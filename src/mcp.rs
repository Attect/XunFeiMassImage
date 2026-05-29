use crate::api::generate_image;
use crate::config::Config;
use anyhow::Result;
use axum::{
    extract::{Query, State},
    response::sse::{Event, Sse},
    routing::{get, post},
    Json, Router,
};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::convert::Infallible;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tokio_stream::{StreamExt, wrappers::ReceiverStream};

// ─── MCP 协议类型 ───

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct JsonRpcRequest {
    jsonrpc: String,
    id: Option<serde_json::Value>,
    method: String,
    #[serde(default)]
    params: serde_json::Value,
}

#[derive(Debug, Serialize)]
struct JsonRpcResponse {
    jsonrpc: String,
    id: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<JsonRpcError>,
}

#[derive(Debug, Serialize)]
struct JsonRpcError {
    code: i32,
    message: String,
}

#[derive(Debug, Deserialize)]
struct MessageParams {
    session_id: String,
}

// ─── 共享状态 ───

type Sessions = Arc<RwLock<HashMap<String, mpsc::Sender<String>>>>;

#[derive(Clone)]
struct AppState {
    sessions: Sessions,
    config: Config,
}

// ─── SSE 端点 ───

async fn sse_handler(State(state): State<AppState>) -> Sse<impl tokio_stream::Stream<Item = Result<Event, Infallible>>> {
    let session_id = uuid::Uuid::new_v4().to_string();
    let (tx, rx) = mpsc::channel::<String>(100);

    state.sessions.write().await.insert(session_id.clone(), tx);

    let endpoint = format!("/message?session_id={}", session_id);
    let init = tokio_stream::once(Ok(
        Event::default().event("endpoint").data(endpoint)
    ));

    let stream = ReceiverStream::new(rx)
        .map(|msg| Ok(Event::default().data(msg)));

    Sse::new(init.chain(stream))
}

// ─── Message 端点 ───

async fn message_handler(
    State(state): State<AppState>,
    Query(params): Query<MessageParams>,
    Json(req): Json<JsonRpcRequest>,
) {
    let resp = handle_request(req, &state.config).await;
    let resp_str = match serde_json::to_string(&resp) {
        Ok(s) => s,
        Err(e) => {
            let err = JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id: resp.id,
                result: None,
                error: Some(JsonRpcError {
                    code: -32603,
                    message: format!("序列化响应失败: {}", e),
                }),
            };
            serde_json::to_string(&err).unwrap_or_default()
        }
    };

    let sessions = state.sessions.read().await;
    if let Some(tx) = sessions.get(&params.session_id) {
        let _ = tx.send(resp_str).await;
    }
}

// ─── 请求处理 ───

async fn handle_request(req: JsonRpcRequest, config: &Config) -> JsonRpcResponse {
    let id = req.id.clone();

    match req.method.as_str() {
        "initialize" => handle_initialize(id),
        "notifications/initialized" => JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id: None,
            result: None,
            error: None,
        },
        "tools/list" => handle_tools_list(id),
        "tools/call" => handle_tools_call(id, req.params, config).await,
        _ => JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id,
            result: None,
            error: Some(JsonRpcError {
                code: -32601,
                message: format!("未知方法: {}", req.method),
            }),
        },
    }
}

fn handle_initialize(id: Option<serde_json::Value>) -> JsonRpcResponse {
    JsonRpcResponse {
        jsonrpc: "2.0".to_string(),
        id,
        result: Some(serde_json::json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {
                "tools": {}
            },
            "serverInfo": {
                "name": "xunfei-image-gen",
                "version": "0.1.0"
            }
        })),
        error: None,
    }
}

fn handle_tools_list(id: Option<serde_json::Value>) -> JsonRpcResponse {
    JsonRpcResponse {
        jsonrpc: "2.0".to_string(),
        id,
        result: Some(serde_json::json!({
            "tools": [
                {
                    "name": "generate_image",
                    "description": "使用讯飞星火大模型 API 根据文本提示词生成图片",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "prompt": {
                                "type": "string",
                                "description": "图片生成提示词，描述你想要生成的图片内容"
                            },
                            "negative_prompt": {
                                "type": "string",
                                "description": "负面提示词，描述你不希望在图片中出现的内容"
                            },
                            "width": {
                                "type": "integer",
                                "description": "图片宽度，可选值: 512, 576, 768, 1024"
                            },
                            "height": {
                                "type": "integer",
                                "description": "图片高度，可选值: 512, 576, 768, 1024"
                            },
                            "seed": {
                                "type": "integer",
                                "description": "随机种子，范围 0 ~ INT_MAX"
                            },
                            "output_path": {
                                "type": "string",
                                "description": "输出图片的文件路径，默认为 output.png"
                            }
                        },
                        "required": ["prompt"]
                    }
                }
            ]
        })),
        error: None,
    }
}

async fn handle_tools_call(
    id: Option<serde_json::Value>,
    params: serde_json::Value,
    config: &Config,
) -> JsonRpcResponse {
    let tool_name = params["name"].as_str().unwrap_or("");
    if tool_name != "generate_image" {
        return JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id,
            result: None,
            error: Some(JsonRpcError {
                code: -32602,
                message: format!("未知工具: {}", tool_name),
            }),
        };
    }

    let args = &params["arguments"];
    let prompt = match args["prompt"].as_str() {
        Some(p) if !p.is_empty() => p,
        _ => {
            return JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: None,
                error: Some(JsonRpcError {
                    code: -32602,
                    message: "缺少必填参数: prompt".to_string(),
                }),
            };
        }
    };

    let negative = args["negative_prompt"].as_str();
    let output_path = args["output_path"]
        .as_str()
        .unwrap_or("output.png");
    let out = std::path::PathBuf::from(output_path);

    let mut cfg = config.clone();
    if let Some(w) = args["width"].as_i64() {
        cfg.width = w as i32;
    }
    if let Some(h) = args["height"].as_i64() {
        cfg.height = h as i32;
    }
    if let Some(s) = args["seed"].as_i64() {
        cfg.seed = s as i32;
    }

    match generate_image(&cfg, prompt, negative, &out).await {
        Ok(()) => {
            let mut content = vec![serde_json::json!({
                "type": "text",
                "text": format!("图片生成成功，已保存至 {}", output_path)
            })];

            // 尝试读取图片并转为 base64 返回
            if let Ok(img_bytes) = std::fs::read(&out) {
                let b64 = BASE64.encode(&img_bytes);
                content.push(serde_json::json!({
                    "type": "image",
                    "data": b64,
                    "mimeType": "image/png"
                }));
            }

            JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: Some(serde_json::json!({
                    "content": content,
                    "isError": false
                })),
                error: None,
            }
        }
        Err(e) => JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(serde_json::json!({
                "content": [
                    {
                        "type": "text",
                        "text": format!("图片生成失败: {}", e)
                    }
                ],
                "isError": true
            })),
            error: None,
        },
    }
}

// ─── 公共接口 ───

pub async fn run_mcp_server(config: Config) -> Result<()> {
    let state = AppState {
        sessions: Arc::new(RwLock::new(HashMap::new())),
        config,
    };

    let host = state.config.mcp_host.clone();
    let port = state.config.mcp_port;

    let app = Router::new()
        .route("/sse", get(sse_handler))
        .route("/message", post(message_handler))
        .with_state(state);

    let addr = format!("{}:{}", host, port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    println!("MCP SSE 服务已启动: http://{}/sse", addr);

    axum::serve(listener, app).await?;
    Ok(())
}
