use crate::api::generate_image;
use crate::config::Config;
use eframe::egui;
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver};

enum GenResult {
    Success,
    Error(String),
}

const RESOLUTIONS: &[(i32, i32, &str)] = &[
    (512, 512, "512²"),
    (768, 768, "768²"),
    (1024, 1024, "1024²"),
    (576, 1024, "576×1024"),
    (768, 1024, "768×1024"),
    (1024, 576, "1024×576"),
    (1024, 768, "1024×768"),
];

pub struct ImageGenApp {
    config: Config,
    prompt: String,
    negative_prompt: String,
    output_path: String,
    width: String,
    height: String,
    seed: String,
    steps: String,
    guidance: String,
    status: String,
    is_generating: bool,
    generation_succeeded: bool,
    rx: Option<Receiver<GenResult>>,
}

impl ImageGenApp {
    pub fn new(config: Config) -> Self {
        Self {
            prompt: config.last_prompt.clone(),
            negative_prompt: config.last_negative_prompt.clone(),
            output_path: config.last_output_path.clone(),
            width: config.width.to_string(),
            height: config.height.to_string(),
            seed: config.seed.to_string(),
            steps: config.num_inference_steps.to_string(),
            guidance: config.guidance_scale.to_string(),
            config,
            status: String::new(),
            is_generating: false,
            generation_succeeded: false,
            rx: None,
        }
    }

    fn save_config(&mut self) {
        self.config.last_prompt = self.prompt.clone();
        self.config.last_negative_prompt = self.negative_prompt.clone();
        self.config.last_output_path = self.output_path.clone();
        self.config.width = self.width.parse().unwrap_or(self.config.width);
        self.config.height = self.height.parse().unwrap_or(self.config.height);
        self.config.seed = self.seed.parse().unwrap_or(self.config.seed);
        self.config.num_inference_steps = self.steps.parse().unwrap_or(self.config.num_inference_steps);
        self.config.guidance_scale = self.guidance.parse().unwrap_or(self.config.guidance_scale);
        if let Err(e) = self.config.save() {
            self.status = format!("保存配置失败: {}", e);
        }
    }

    fn build_config(&self) -> Result<Config, String> {
        let mut cfg = self.config.clone();
        cfg.width = self.width.parse().map_err(|e| format!("宽度格式错误: {}", e))?;
        cfg.height = self.height.parse().map_err(|e| format!("高度格式错误: {}", e))?;
        cfg.seed = self.seed.parse().map_err(|e| format!("种子格式错误: {}", e))?;
        cfg.num_inference_steps = self.steps.parse().map_err(|e| format!("步数格式错误: {}", e))?;
        cfg.guidance_scale = self.guidance.parse().map_err(|e| format!("CFG格式错误: {}", e))?;
        Ok(cfg)
    }

    fn set_resolution(&mut self, w: i32, h: i32) {
        self.width = w.to_string();
        self.height = h.to_string();
    }

    fn spawn_generate(&mut self) {
        let cfg = match self.build_config() {
            Ok(c) => c,
            Err(e) => {
                self.status = format!("配置错误: {}", e);
                return;
            }
        };

        if self.prompt.trim().is_empty() {
            self.status = "提示词不能为空".to_string();
            return;
        }

        let prompt = self.prompt.clone();
        let neg = self.negative_prompt.clone();
        let out = PathBuf::from(&self.output_path);

        let (tx, rx) = channel::<GenResult>();
        self.rx = Some(rx);
        self.is_generating = true;
        self.generation_succeeded = false;
        self.status = "正在生成图片，请稍候...".to_string();

        let handle = tokio::runtime::Handle::current();
        handle.spawn(async move {
            let neg_opt = if neg.trim().is_empty() {
                None
            } else {
                Some(neg.as_str())
            };
            match generate_image(&cfg, &prompt, neg_opt, &out).await {
                Ok(()) => {
                    let _ = tx.send(GenResult::Success);
                }
                Err(e) => {
                    let _ = tx.send(GenResult::Error(e.to_string()));
                }
            }
        });
    }

    fn open_output_dir(&self) {
        let path = std::path::Path::new(&self.output_path);
        let dir = path.parent().unwrap_or(path);
        #[cfg(target_os = "windows")]
        {
            let _ = std::process::Command::new("explorer").arg(dir).spawn();
        }
        #[cfg(target_os = "macos")]
        {
            let _ = std::process::Command::new("open").arg(dir).spawn();
        }
        #[cfg(target_os = "linux")]
        {
            let _ = std::process::Command::new("xdg-open").arg(dir).spawn();
        }
    }

    fn open_output_file(&self) {
        let path = &self.output_path;
        #[cfg(target_os = "windows")]
        {
            let _ = std::process::Command::new("cmd")
                .args(["/C", "start", "", path])
                .spawn();
        }
        #[cfg(target_os = "macos")]
        {
            let _ = std::process::Command::new("open").arg(path).spawn();
        }
        #[cfg(target_os = "linux")]
        {
            let _ = std::process::Command::new("xdg-open").arg(path).spawn();
        }
    }
}

impl eframe::App for ImageGenApp {
    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.save_config();
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // 检查结果
        if let Some(rx) = &self.rx {
            if let Ok(result) = rx.try_recv() {
                self.is_generating = false;
                match result {
                    GenResult::Success => {
                        self.generation_succeeded = true;
                        self.status = format!("图片生成成功! 已保存至 {}", self.output_path);
                        self.save_config();
                    }
                    GenResult::Error(e) => {
                        self.generation_succeeded = false;
                        self.status = format!("生成失败: {}", e);
                    }
                }
            }
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("讯飞星火图片生成");
            ui.separator();

            // 提示词
            ui.label("提示词:");
            ui.add(
                egui::TextEdit::multiline(&mut self.prompt)
                    .desired_width(ui.available_width())
                    .desired_rows(3),
            );

            ui.add_space(6.0);

            // 负面提示词
            ui.label("负面提示词 (可选):");
            ui.add(
                egui::TextEdit::multiline(&mut self.negative_prompt)
                    .desired_width(ui.available_width())
                    .desired_rows(2),
            );

            ui.add_space(6.0);

            // 输出路径
            ui.horizontal(|ui| {
                ui.label("输出路径:");
                ui.text_edit_singleline(&mut self.output_path);
                if ui.button("浏览").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("图片", &["png", "jpg", "jpeg"])
                        .save_file()
                    {
                        self.output_path = path.to_string_lossy().to_string();
                    }
                }
            });

            ui.add_space(6.0);

            // 常用分辨率按钮
            ui.label("常用分辨率:");
            ui.horizontal_wrapped(|ui| {
                for &(w, h, label) in RESOLUTIONS {
                    if ui.button(label).clicked() {
                        self.set_resolution(w, h);
                    }
                }
            });

            ui.add_space(6.0);

            // 参数网格
            egui::Grid::new("params_grid")
                .num_columns(6)
                .spacing([10.0, 6.0])
                .show(ui, |ui| {
                    ui.label("宽度:");
                    ui.add(egui::TextEdit::singleline(&mut self.width).desired_width(55.0));
                    ui.label("高度:");
                    ui.add(egui::TextEdit::singleline(&mut self.height).desired_width(55.0));
                    ui.label("种子:");
                    ui.horizontal(|ui| {
                        ui.add(egui::TextEdit::singleline(&mut self.seed).desired_width(80.0));
                        if ui.button("🎲").clicked() {
                            self.seed = rand::random::<u32>().to_string();
                        }
                    });
                    ui.end_row();

                    ui.label("步数:");
                    ui.add(egui::TextEdit::singleline(&mut self.steps).desired_width(55.0));
                    ui.label("CFG:");
                    ui.add(egui::TextEdit::singleline(&mut self.guidance).desired_width(55.0));
                    ui.end_row();
                });

            ui.add_space(6.0);


            ui.add_space(6.0);

            // 配置信息
            ui.collapsing("配置信息 (来自 config.yaml)", |ui| {
                ui.label(format!("App ID: {}", self.config.app_id));
                ui.label(format!("Model ID: {}", self.config.model_id));
                if !self.config.patch_id.is_empty() {
                    ui.label(format!("Patch ID: {:?}", self.config.patch_id));
                }
                ui.label(format!("API URL: {}", self.config.api_url));
                ui.label(format!("调度器: {}", self.config.scheduler));
                ui.separator();
                if self.config.mcp_enabled {
                    ui.colored_label(egui::Color32::from_rgb(30, 120, 60), format!(
                        "MCP 服务: 已启用 | http://{}:{}/sse",
                        self.config.mcp_host, self.config.mcp_port
                    ));
                } else {
                    ui.colored_label(egui::Color32::GRAY, "MCP 服务: 未启用");
                }
            });

            ui.add_space(10.0);

            // 按钮行
            ui.horizontal(|ui| {
                let generate_btn = ui.add_sized(
                    [100.0, 32.0],
                    egui::Button::new("生成图片"),
                );
                if generate_btn.clicked() && !self.is_generating {
                    self.spawn_generate();
                }

                if self.is_generating {
                    ui.spinner();
                    ui.label("生成中...");
                }

                if ui.add_sized([80.0, 32.0], egui::Button::new("打开目录")).clicked() {
                    self.open_output_dir();
                }

                if ui
                    .add_sized([80.0, 32.0], egui::Button::new("打开图片"))
                    .clicked()
                {
                    self.open_output_file();
                }
            });

            ui.add_space(6.0);

            // 状态显示
            if !self.status.is_empty() {
                let (fg, bg) = if self.status.starts_with("生成失败") || self.status.starts_with("配置错误") {
                    (egui::Color32::from_rgb(160, 40, 40), egui::Color32::from_rgb(255, 230, 230))
                } else if self.status.starts_with("图片生成成功") {
                    (egui::Color32::from_rgb(30, 120, 60), egui::Color32::from_rgb(220, 255, 230))
                } else {
                    (egui::Color32::from_rgb(160, 110, 20), egui::Color32::from_rgb(255, 250, 210))
                };
                egui::Frame::none()
                    .fill(bg)
                    .inner_margin(egui::vec2(10.0, 6.0))
                    .rounding(4.0)
                    .show(ui, |ui| {
                        ui.colored_label(fg, &self.status);
                    });
            }
        });

        if self.is_generating {
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
        }
    }
}

fn setup_custom_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();

    let font_paths: &[&str] = &[
        // Windows
        r"C:\Windows\Fonts\msyh.ttc",
        r"C:\Windows\Fonts\msyhbd.ttc",
        r"C:\Windows\Fonts\simhei.ttf",
        r"C:\Windows\Fonts\simsun.ttc",
        // macOS
        "/System/Library/Fonts/PingFang.ttc",
        "/System/Library/Fonts/STHeiti Light.ttc",
        "/Library/Fonts/Arial Unicode.ttf",
        // Linux
        "/usr/share/fonts/truetype/wqy/wqy-zenhei.ttc",
        "/usr/share/fonts/truetype/wqy/wqy-microhei.ttc",
        "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
    ];

    for path in font_paths {
        if let Ok(font_data) = std::fs::read(path) {
            let name = std::path::Path::new(path)
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();

            fonts
                .font_data
                .insert(name.clone(), egui::FontData::from_owned(font_data));

            fonts
                .families
                .entry(egui::FontFamily::Proportional)
                .or_default()
                .push(name.clone());
            fonts
                .families
                .entry(egui::FontFamily::Monospace)
                .or_default()
                .push(name);

            break;
        }
    }

    ctx.set_fonts(fonts);
}

pub fn run_gui(config: Config) -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([620.0, 540.0]),
        ..Default::default()
    };
    eframe::run_native(
        "讯飞星火图片生成",
        options,
        Box::new(|cc| {
            setup_custom_fonts(&cc.egui_ctx);
            Ok(Box::new(ImageGenApp::new(config)))
        }),
    )
}
