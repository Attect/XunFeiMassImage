use crate::api::generate_image;
use crate::config::Config;
use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "xunfei-image-gen")]
#[command(about = "讯飞星火图片生成工具 (CLI模式)")]
pub struct CliArgs {
    /// 图片生成提示词
    pub prompt: String,

    /// 输出图片路径
    #[arg(short, long, default_value = "output.png")]
    pub output: PathBuf,

    /// 负面提示词
    #[arg(short, long)]
    pub negative: Option<String>,

    /// 图片宽度
    #[arg(long)]
    pub width: Option<i32>,

    /// 图片高度
    #[arg(long)]
    pub height: Option<i32>,

    /// 随机种子
    #[arg(long)]
    pub seed: Option<i32>,

    /// 推理步数
    #[arg(long)]
    pub steps: Option<i32>,

    /// CFG Scale
    #[arg(long)]
    pub guidance: Option<f32>,
}

pub async fn run_cli(args: CliArgs) -> Result<()> {
    let mut config = Config::load()?;

    // CLI参数覆盖配置文件
    if let Some(w) = args.width {
        config.width = w;
    }
    if let Some(h) = args.height {
        config.height = h;
    }
    if let Some(s) = args.seed {
        config.seed = s;
    }
    if let Some(st) = args.steps {
        config.num_inference_steps = st;
    }
    if let Some(g) = args.guidance {
        config.guidance_scale = g;
    }

    println!("正在生成图片...");
    println!("提示词: {}", args.prompt);
    println!("输出路径: {}", args.output.display());
    println!(
        "分辨率: {}x{} | 种子: {} | 步数: {} | CFG: {}",
        config.width, config.height, config.seed, config.num_inference_steps, config.guidance_scale
    );

    generate_image(&config, &args.prompt, args.negative.as_deref(), &args.output).await?;

    println!("图片生成成功! 保存至: {}", args.output.display());
    Ok(())
}
