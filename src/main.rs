use clap::Parser;
use std::path::PathBuf;

mod api;
mod cli;
mod config;
mod gui;
mod mcp;

use cli::{run_cli, CliArgs};
use config::Config;
use gui::run_gui;
use mcp::run_mcp_server;

#[derive(Parser, Debug)]
#[command(name = "xunfei-image-gen")]
#[command(about = "讯飞星火图片生成工具")]
struct Args {
    /// 强制启动GUI模式
    #[arg(long)]
    gui: bool,

    /// 仅启动 MCP SSE 服务（不启动 GUI）
    #[arg(long)]
    mcp: bool,

    /// 输出示例配置文件到控制台
    #[arg(long)]
    example_config: bool,

    /// 图片生成提示词（提供时进入CLI模式，除非同时使用 --gui 或 --mcp）
    prompt: Option<String>,

    /// 输出图片路径
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// 负面提示词
    #[arg(short, long)]
    negative: Option<String>,

    /// 图片宽度
    #[arg(long)]
    width: Option<i32>,

    /// 图片高度
    #[arg(long)]
    height: Option<i32>,

    /// 随机种子
    #[arg(long)]
    seed: Option<i32>,

    /// 推理步数
    #[arg(long)]
    steps: Option<i32>,

    /// CFG Scale (guidance_scale)
    #[arg(long)]
    guidance: Option<f32>,
}

#[cfg(windows)]
fn hide_console_window() {
    #[link(name = "kernel32")]
    extern "system" {
        fn GetConsoleWindow() -> *mut std::ffi::c_void;
    }
    #[link(name = "user32")]
    extern "system" {
        fn ShowWindow(hWnd: *mut std::ffi::c_void, nCmdShow: i32) -> i32;
    }
    const SW_HIDE: i32 = 0;
    unsafe {
        let hwnd = GetConsoleWindow();
        if !hwnd.is_null() {
            ShowWindow(hwnd, SW_HIDE);
        }
    }
}

#[cfg(not(windows))]
fn hide_console_window() {}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    if args.example_config {
        println!("{}", Config::generate_example());
        return;
    }

    if args.mcp {
        let config = match Config::load() {
            Ok(c) => c,
            Err(e) => {
                eprintln!("错误: {}", e);
                std::process::exit(1);
            }
        };
        if let Err(e) = run_mcp_server(config).await {
            eprintln!("MCP 服务启动失败: {}", e);
            std::process::exit(1);
        }
        return;
    }

    let use_gui = args.gui || args.prompt.is_none();

    if use_gui {
        let config = match Config::load() {
            Ok(c) => c,
            Err(e) => {
                eprintln!("错误: {}", e);
                eprintln!("\n提示: 使用 --example-config 查看配置文件示例");
                std::process::exit(1);
            }
        };

        // 若配置启用了 MCP，后台启动 SSE 服务
        if config.mcp_enabled {
            let mcp_config = config.clone();
            tokio::spawn(async move {
                if let Err(e) = run_mcp_server(mcp_config).await {
                    eprintln!("MCP 服务异常: {}", e);
                }
            });
        }

        hide_console_window();
        if let Err(e) = run_gui(config) {
            eprintln!("GUI启动失败: {}", e);
            std::process::exit(1);
        }
    } else {
        let cli_args = CliArgs {
            prompt: args.prompt.unwrap(),
            output: args.output.unwrap_or_else(|| PathBuf::from("output.png")),
            negative: args.negative,
            width: args.width,
            height: args.height,
            seed: args.seed,
            steps: args.steps,
            guidance: args.guidance,
        };
        if let Err(e) = run_cli(cli_args).await {
            eprintln!("错误: {}", e);
            std::process::exit(1);
        }
    }
}
