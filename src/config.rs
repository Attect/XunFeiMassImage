use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

fn default_api_url() -> String {
    "https://maas-api.cn-huabei-1.xf-yun.com/v2.1/tti".to_string()
}

fn default_width() -> i32 {
    512
}

fn default_height() -> i32 {
    512
}

fn default_seed() -> i32 {
    12345
}

fn default_num_inference_steps() -> i32 {
    20
}

fn default_guidance_scale() -> f32 {
    5.0
}

fn default_scheduler() -> String {
    "DPM++ 2M Karras".to_string()
}

fn default_output_path() -> String {
    "output.png".to_string()
}

fn default_mcp_host() -> String {
    "127.0.0.1".to_string()
}

fn default_mcp_port() -> u16 {
    8080
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    pub app_id: String,
    pub api_key: String,
    pub api_secret: String,
    pub model_id: String,

    #[serde(default)]
    pub patch_id: Vec<String>,

    #[serde(default = "default_api_url")]
    pub api_url: String,

    #[serde(default = "default_width")]
    pub width: i32,

    #[serde(default = "default_height")]
    pub height: i32,

    #[serde(default = "default_seed")]
    pub seed: i32,

    #[serde(default = "default_num_inference_steps")]
    pub num_inference_steps: i32,

    #[serde(default = "default_guidance_scale")]
    pub guidance_scale: f32,

    #[serde(default = "default_scheduler")]
    pub scheduler: String,

    // 上次使用的 UI 状态
    #[serde(default)]
    pub last_prompt: String,
    #[serde(default)]
    pub last_negative_prompt: String,
    #[serde(default = "default_output_path")]
    pub last_output_path: String,

    // MCP 服务配置
    #[serde(default)]
    pub mcp_enabled: bool,

    #[serde(default = "default_mcp_host")]
    pub mcp_host: String,

    #[serde(default = "default_mcp_port")]
    pub mcp_port: u16,

    // 运行时字段，不序列化
    #[serde(skip)]
    pub path: PathBuf,
}

impl Config {
    pub fn load() -> Result<Self> {
        let exe_path = std::env::current_exe().context("无法获取当前可执行文件路径")?;
        let exe_dir = exe_path
            .parent()
            .context("无法获取可执行文件所在目录")?;
        let config_path = exe_dir.join("config.yaml");

        if !config_path.exists() {
            anyhow::bail!(
                "配置文件不存在: {}\n请在该路径创建包含 app_id, api_key, api_secret, model_id 的 config.yaml 文件",
                config_path.display()
            );
        }

        let content = std::fs::read_to_string(&config_path)
            .with_context(|| format!("读取配置文件失败: {}", config_path.display()))?;
        let mut config: Config = serde_yaml::from_str(&content)
            .with_context(|| format!("解析配置文件失败: {}", config_path.display()))?;

        config.path = config_path;
        config.validate()?;
        Ok(config)
    }

    pub fn save(&self) -> Result<()> {
        // 创建一份不包含 path 字段的副本用于序列化
        let content = serde_yaml::to_string(self)
            .context("序列化配置失败")?;
        std::fs::write(&self.path, content)
            .with_context(|| format!("写入配置文件失败: {}", self.path.display()))?;
        Ok(())
    }

    fn validate(&self) -> Result<()> {
        if self.app_id.is_empty() {
            anyhow::bail!("config.yaml 中 app_id 不能为空");
        }
        if self.api_key.is_empty() {
            anyhow::bail!("config.yaml 中 api_key 不能为空");
        }
        if self.api_secret.is_empty() {
            anyhow::bail!("config.yaml 中 api_secret 不能为空");
        }
        if self.model_id.is_empty() {
            anyhow::bail!("config.yaml 中 model_id 不能为空");
        }
        Ok(())
    }

    pub fn generate_example() -> String {
        let example = Config {
            app_id: "your_app_id".to_string(),
            api_key: "your_api_key".to_string(),
            api_secret: "your_api_secret".to_string(),
            model_id: "your_model_id".to_string(),
            patch_id: vec!["your_patch_id".to_string()],
            api_url: default_api_url(),
            width: default_width(),
            height: default_height(),
            seed: default_seed(),
            num_inference_steps: default_num_inference_steps(),
            guidance_scale: default_guidance_scale(),
            scheduler: default_scheduler(),
            last_prompt: String::new(),
            last_negative_prompt: String::new(),
            last_output_path: default_output_path(),
            mcp_enabled: false,
            mcp_host: default_mcp_host(),
            mcp_port: default_mcp_port(),
            path: PathBuf::new(),
        };
        serde_yaml::to_string(&example).unwrap_or_default()
    }
}
