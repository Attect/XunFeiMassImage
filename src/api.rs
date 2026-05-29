use crate::config::Config;
use anyhow::{Context, Result};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use chrono::Utc;
use hmac::{Hmac, KeyInit, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::path::Path;

#[derive(Debug, Serialize)]
struct RequestBody {
    header: RequestHeader,
    parameter: RequestParameter,
    payload: RequestPayload,
}

#[derive(Debug, Serialize)]
struct RequestHeader {
    app_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    uid: Option<String>,
    patch_id: Vec<String>,
}

#[derive(Debug, Serialize)]
struct RequestParameter {
    chat: ChatParameter,
}

#[derive(Debug, Serialize)]
struct ChatParameter {
    domain: String,
    width: i32,
    height: i32,
    seed: i32,
    num_inference_steps: i32,
    guidance_scale: f32,
    scheduler: String,
}

#[derive(Debug, Serialize)]
struct RequestPayload {
    message: MessagePayload,
}

#[derive(Debug, Serialize)]
struct MessagePayload {
    text: Vec<MessageText>,
    #[serde(skip_serializing_if = "Option::is_none")]
    negative_prompts: Option<String>,
}

#[derive(Debug, Serialize)]
struct MessageText {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct ResponseBody {
    header: ResponseHeader,
    payload: ResponsePayload,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct ResponseHeader {
    code: i32,
    message: String,
    sid: String,
    status: i32,
}

#[derive(Debug, Deserialize)]
struct ResponsePayload {
    choices: ResponseChoices,
}

#[derive(Debug, Deserialize)]
struct ResponseChoices {
    text: Vec<ResponseText>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct ResponseText {
    content: String,
    role: String,
    index: i32,
    #[serde(default)]
    content_type: String,
}

type HmacSha256 = Hmac<Sha256>;

fn assemble_auth_url(url: &str, api_key: &str, api_secret: &str) -> Result<String> {
    let parsed = reqwest::Url::parse(url).context("解析API地址失败")?;
    let host = parsed.host_str().context("API地址缺少host")?;
    let path = parsed.path();

    let date = Utc::now().format("%a, %d %b %Y %H:%M:%S GMT").to_string();

    let signature_origin = format!("host: {}\ndate: {}\nPOST {} HTTP/1.1", host, date, path);

    let mut mac = HmacSha256::new_from_slice(api_secret.as_bytes())
        .context("创建HMAC失败")?;
    mac.update(signature_origin.as_bytes());
    let signature_sha = mac.finalize().into_bytes();
    let signature = BASE64.encode(signature_sha);

    let authorization_origin = format!(
        r#"api_key="{}", algorithm="hmac-sha256", headers="host date request-line", signature="{}""#,
        api_key, signature
    );
    let authorization = BASE64.encode(authorization_origin.as_bytes());

    let auth_url = format!(
        "{}?host={}&date={}&authorization={}",
        url,
        urlencoding::encode(host),
        urlencoding::encode(&date),
        urlencoding::encode(&authorization)
    );

    Ok(auth_url)
}

pub async fn generate_image(
    config: &Config,
    prompt: &str,
    negative_prompt: Option<&str>,
    output_path: &Path,
) -> Result<()> {
    let auth_url = assemble_auth_url(&config.api_url, &config.api_key, &config.api_secret)?;

    let body = RequestBody {
        header: RequestHeader {
            app_id: config.app_id.clone(),
            uid: None,
            patch_id: config.patch_id.clone(),
        },
        parameter: RequestParameter {
            chat: ChatParameter {
                domain: config.model_id.clone(),
                width: config.width,
                height: config.height,
                seed: config.seed,
                num_inference_steps: config.num_inference_steps,
                guidance_scale: config.guidance_scale,
                scheduler: config.scheduler.clone(),
            },
        },
        payload: RequestPayload {
            message: MessagePayload {
                text: vec![MessageText {
                    role: "user".to_string(),
                    content: prompt.to_string(),
                }],
                negative_prompts: negative_prompt.map(|s| s.to_string()),
            },
        },
    };

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .context("创建HTTP客户端失败")?;

    let response = client
        .post(&auth_url)
        .header("Content-Type", "application/json; charset=UTF-8")
        .json(&body)
        .send()
        .await
        .context("发送请求失败")?;

    let status = response.status();
    let text = response.text().await.context("读取响应体失败")?;

    if !status.is_success() {
        anyhow::bail!("HTTP请求失败: {} - {}", status, text);
    }

    let resp: ResponseBody = serde_json::from_str(&text)
        .with_context(|| format!("解析响应JSON失败: {}", text))?;

    if resp.header.code != 0 {
        anyhow::bail!(
            "API返回错误: [{}] {} (sid: {})",
            resp.header.code,
            resp.header.message,
            resp.header.sid
        );
    }

    let image_text = resp
        .payload
        .choices
        .text
        .into_iter()
        .next()
        .context("响应中没有图片数据")?;

    let image_data = BASE64
        .decode(&image_text.content)
        .context("解码Base64图片数据失败")?;

    std::fs::write(output_path, &image_data)
        .with_context(|| format!("保存图片失败: {}", output_path.display()))?;

    Ok(())
}
