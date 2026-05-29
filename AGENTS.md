# AGENTS.md — xunfei-image-gen

## 项目概述

跨平台 Rust 程序，调用讯飞星火图片生成 API（v2.1/tti），支持三种入口模式：CLI、GUI（egui）、MCP SSE 服务器。

## 关键约定

### 文件布局

```
.
├── Cargo.toml          # 依赖：reqwest, egui/eframe, axum, tokio, clap, hmac/sha2, serde_yaml
├── src/
│   ├── main.rs         # 入口：模式分发（GUI / MCP / CLI），console 隐藏（Windows）
│   ├── api.rs          # 讯飞 API：HMAC-SHA256 鉴权、请求体构造、Base64 解码保存
│   ├── config.rs       # Config 结构体：加载 / 保存 / 验证 config.yaml
│   ├── gui.rs          # egui GUI：中文系统字体 fallback、参数面板、异步生成
│   ├── mcp.rs          # SSE MCP 服务器：/sse、/message?session_id=、generate_image 工具
│   └── watermark.rs    # 去水印算法（Alpha 混合逆运算），当前未编译进主程序
├── config.yaml         # 运行时配置（**含敏感 API key，不得进入 Git**）
└── assets/             # 字体资源（msyh.ttc、msgothic.ttc 等）
```

### 敏感信息

- **`config.yaml` 不得提交到 Git**。已有 `.gitignore` 排除。若修改了 `.gitignore`，必须保持 `config.yaml` 在排除列表中。
- 已提供干净的 `config.yaml.example`，不含任何真实凭证。

### 模式选择逻辑

```rust
use_gui = args.gui || (args.prompt.is_none() && !args.mcp)
```

- 无参数 → GUI
- `--gui` → GUI
- `--mcp` → MCP 服务器
- 提供 `prompt` → CLI

### 去水印模块（watermark.rs）

当前 **未链接到主程序**（`main.rs` 未声明 `mod watermark;`，`api.rs` 未调用去水印函数）。

- 保留原因：算法代码完整，但标定数据无法通用（API 根据内容动态调整水印 Alpha 值），未来若 API 水印策略变化，可在此基础上迭代。
- **若恢复使用**：需在 `main.rs` 加回 `mod watermark;`，在 `api.rs` 的 `generate_image` 中恢复调用。

### 配置文件字段

```rust
pub struct Config {
    pub app_id: String,
    pub api_key: String,
    pub api_secret: String,
    pub model_id: String,
    pub patch_id: Vec<String>,      // LoRA patch ID
    pub api_url: String,
    pub width: i32,
    pub height: i32,
    pub seed: i32,
    pub num_inference_steps: i32,
    pub guidance_scale: f32,
    pub scheduler: String,
    // UI 持久化
    pub last_prompt: String,
    pub last_negative_prompt: String,
    pub last_output_path: String,
    // MCP
    pub mcp_enabled: bool,
    pub mcp_host: String,
    pub mcp_port: u16,
}
```

### GUI 中文显示

`gui.rs::setup_custom_fonts()` 加载系统中日文字体作为 fallback，确保 Windows 平台中文正常渲染。

### MCP 协议

- JSON-RPC 2.0 over SSE
- 工具名：`generate_image`
- 会话隔离：通过 `session_id` 区分多客户端

## 构建

```bash
cargo build --release
```

## 测试

- 无单元测试（项目为 API 调用工具，核心逻辑依赖外部服务）
- 手动验证方式：
  1. 准备有效 `config.yaml`
  2. CLI: `cargo run -- "test prompt" -o test.png`
  3. GUI: `cargo run`
  4. MCP: `cargo run -- --mcp`，然后用 MCP 客户端连接
