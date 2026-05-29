# XunFeiMassImage

基于 Rust 的跨平台客户端，用于调用**讯飞星辰 MaaS 平台**的图片生成（文生图）推理服务。

本项目完整实现了讯飞星火图片生成 API 的鉴权、请求构造、响应解析与图片保存流程，支持 **CLI 命令行**、**GUI 可视化界面**、**MCP SSE 服务器**三种使用模式，方便在不同场景下快速接入讯飞星辰大模型的图片推理能力。

> 官方接口文档：https://www.xfyun.cn/doc/spark/%E5%9B%BE%E7%89%87%E7%94%9F%E6%88%90.html

---

## 功能特性

- **完整 API 实现**：支持 HMAC-SHA256 鉴权签名、请求体组装、Base64 图片数据解码与保存
- **CLI 模式**：命令行快速生成，支持参数覆盖配置文件中的默认值
- **GUI 模式**：基于 `egui` + `eframe` 的跨平台可视化界面，支持中文渲染、文件浏览对话框、常用分辨率快捷按钮、随机种子一键生成
- **MCP Server 模式**：基于 SSE 协议的 MCP（Model Context Protocol）服务器，可作为 AI Agent / 大模型助手的图片生成工具，支持返回 Base64 编码图片
- **配置驱动**：通过 `config.yaml` 管理 API 凭证、模型参数与 UI 持久化状态
- **Windows 控制台隐藏**：GUI 模式下自动隐藏命令行窗口，双击即可运行
- **参数覆盖灵活**：CLI 与 MCP 工具参数均可覆盖配置文件中的分辨率、种子、推理步数、CFG 等设置

---

## API 调用流程

本程序对接讯飞星辰 MaaS 平台的 **Text-to-Image（v2.1/tti）** 接口：

1. **鉴权**：使用 `api_key` + `api_secret` 基于 HMAC-SHA256 生成签名，组装带 `host`、`date`、`authorization` 查询参数的鉴权 URL
2. **请求构造**：按接口规范组装 JSON 请求体，包含 `header`（app_id、patch_id）、`parameter`（domain、分辨率、种子、推理步数、CFG、调度器）、`payload`（提示词、负面提示词）
3. **发送与解析**：POST 发送请求，解析返回的 JSON，提取 Base64 编码的图片内容
4. **保存**：将 Base64 解码后的二进制数据写入指定路径

---

## 快速开始

### 1. 准备配置

在可执行文件同级目录创建 `config.yaml`，填入从 [讯飞开放平台](https://console.xfyun.cn) 获取的凭证与模型信息：

```yaml
app_id: your_app_id
api_key: your_api_key
api_secret: your_api_secret
model_id: your_model_id

# 可选：LoRA patch ID 列表
# patch_id:
#   - "patch_id_1"

api_url: https://maas-api.cn-huabei-1.xf-yun.com/v2.1/tti
width: 512
height: 512
seed: 12345
num_inference_steps: 20
guidance_scale: 5.0
scheduler: DPM++ 2M Karras

# MCP 服务配置
mcp_enabled: false
mcp_host: 127.0.0.1
mcp_port: 8080
```

或使用命令生成示例配置：

```bash
xunfei-image-gen --example-config
```

### 2. CLI 模式

```bash
# 基本用法（使用 config.yaml 中的默认参数）
xunfei-image-gen "一只可爱的猫" -o cat.png

# 指定负面提示词
xunfei-image-gen "一只可爱的猫" -n "模糊, 变形" -o cat.png

# 覆盖分辨率、种子、推理步数和 CFG
xunfei-image-gen "赛博朋克风格的城市夜景" --width 1024 --height 576 --seed 42 --steps 30 --guidance 7.0 -o cyberpunk.png
```

### 3. GUI 模式

```bash
# 无参数默认启动 GUI
xunfei-image-gen

# 或显式指定
xunfei-image-gen --gui
```

GUI 特性：
- 提示词与负面提示词多行输入
- 文件浏览对话框选择输出路径
- 常用分辨率快捷按钮（512² / 768² / 1024² / 576×1024 / 768×1024 / 1024×576 / 1024×768）
- 宽度、高度、种子、步数、CFG 参数实时编辑
- 🎲 一键生成随机种子
- 配置信息折叠面板（显示当前加载的 App ID / Model ID / API URL / MCP 状态）
- 生成成功后可直接「打开目录」或「打开图片」
- 退出时自动保存 UI 状态到 `config.yaml`

### 4. MCP Server 模式

```bash
# 独立启动 MCP 服务
xunfei-image-gen --mcp
```

或在 GUI 模式下后台启用（设置 `mcp_enabled: true`）：

```yaml
mcp_enabled: true
mcp_host: 127.0.0.1
mcp_port: 8080
```

服务暴露两个端点：
- `GET /sse` — SSE 事件流连接，返回会话 ID 与消息推送通道
- `POST /message?session_id=<id>` — JSON-RPC 2.0 消息通道

MCP 工具 `generate_image` 参数：

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `prompt` | string | ✅ | 图片生成提示词 |
| `negative_prompt` | string | ❌ | 负面提示词 |
| `width` | integer | ❌ | 图片宽度 |
| `height` | integer | ❌ | 图片高度 |
| `seed` | integer | ❌ | 随机种子 |
| `output_path` | string | ❌ | 输出文件路径（默认 `output.png`） |

调用成功后返回文本消息与 Base64 编码的图片数据。

---

## 配置文件字段说明

| 字段 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `app_id` | string | — | 讯飞开放平台应用 ID |
| `api_key` | string | — | API Key |
| `api_secret` | string | — | API Secret（用于 HMAC-SHA256 签名） |
| `model_id` | string | — | 模型/服务 ID |
| `patch_id` | string[] | `[]` | LoRA Patch ID 列表 |
| `api_url` | string | `https://maas-api.cn-huabei-1.xf-yun.com/v2.1/tti` | 图片生成接口地址 |
| `width` | int | `512` | 图片宽度 |
| `height` | int | `512` | 图片高度 |
| `seed` | int | `12345` | 随机种子 |
| `num_inference_steps` | int | `20` | 推理步数 |
| `guidance_scale` | float | `5.0` | CFG Scale（引导强度） |
| `scheduler` | string | `DPM++ 2M Karras` | 调度器 |
| `last_prompt` | string | `""` | GUI 上次使用的提示词（自动保存） |
| `last_negative_prompt` | string | `""` | GUI 上次使用的负面提示词（自动保存） |
| `last_output_path` | string | `"output.png"` | GUI 上次输出路径（自动保存） |
| `mcp_enabled` | bool | `false` | 是否在 GUI 模式下后台启动 MCP 服务 |
| `mcp_host` | string | `127.0.0.1` | MCP 服务绑定地址 |
| `mcp_port` | int | `8080` | MCP 服务端口 |

---

## 构建

```bash
cargo build --release
```

编译完成后，可执行文件位于 `target/release/xunfei-image-gen`（Linux/macOS）或 `target/release/xunfei-image-gen.exe`（Windows）。

### 依赖说明

| 功能 | 依赖 |
|------|------|
| HTTP 请求 | `reqwest` |
| GUI 框架 | `egui` + `eframe` |
| 文件对话框 | `rfd` |
| CLI 解析 | `clap` |
| 鉴权签名 | `hmac` + `sha2` |
| 配置序列化 | `serde_yaml` |
| Web 服务 | `axum` + `tokio` |
| MCP SSE | `tokio-stream` + `uuid` |

---

## 技术细节

### 鉴权签名

本程序严格遵循讯飞星火 API 的 HMAC-SHA256 鉴权规范：

1. 以 `host`、`date`、`request-line` 构造签名原文
2. 使用 `api_secret` 进行 HMAC-SHA256 签名，Base64 编码
3. 将 `api_key`、算法、headers、签名组装为 authorization 原文，再次 Base64
4. 将 `host`、`date`、`authorization` 作为查询参数附加到原始 URL

### GUI 中文渲染

`gui.rs` 中通过 `setup_custom_fonts()` 自动探测系统中日文字体作为 fallback，支持：
- Windows：`msyh.ttc`、`simhei.ttf`、`simsun.ttc`
- macOS：`PingFang.ttc`、`STHeiti`、`Arial Unicode`
- Linux：`wqy-zenhei`、`wqy-microhei`、`NotoSansCJK`

### MCP 会话管理

基于 `axum` + `tokio::sync::mpsc` 实现多客户端 SSE 会话隔离，每个连接分配独立 `session_id`，JSON-RPC 消息按会话路由。

---

## 项目结构

```
.
├── src/
│   ├── main.rs       # 程序入口，模式分发，Windows 控制台隐藏
│   ├── api.rs        # API 鉴权、请求构造、响应解析、图片保存
│   ├── config.rs     # Config 结构体，YAML 加载/保存/验证
│   ├── cli.rs        # CLI 参数解析与执行
│   ├── gui.rs        # egui GUI 实现
│   └── mcp.rs        # MCP SSE 服务器（JSON-RPC 2.0）
├── config.yaml.example
├── Cargo.toml
└── README.md
```

---

## License

MIT License © 2026 Attect

详见 [LICENSE](LICENSE) 文件。
