use image::{GrayImage, Rgb, RgbImage};

struct CalibData {
    alpha_png: &'static [u8],
    wcolor_png: &'static [u8],
    x: u32,
    y: u32,
}

const CALIB_512X512: CalibData = CalibData {
    alpha_png: include_bytes!("../assets/alpha_crop_512x512.png"),
    wcolor_png: include_bytes!("../assets/W_crop_512x512.png"),
    x: 394,
    y: 462,
};

const CALIB_1024X1024: CalibData = CalibData {
    alpha_png: include_bytes!("../assets/alpha_crop_1024x1024.png"),
    wcolor_png: include_bytes!("../assets/W_crop_1024x1024.png"),
    x: 824,
    y: 923,
};

const CALIB_576X1024: CalibData = CalibData {
    alpha_png: include_bytes!("../assets/alpha_crop_576x1024.png"),
    wcolor_png: include_bytes!("../assets/W_crop_576x1024.png"),
    x: 446,
    y: 967,
};

const CALIB_1024X576: CalibData = CalibData {
    alpha_png: include_bytes!("../assets/alpha_crop_1024x576.png"),
    wcolor_png: include_bytes!("../assets/W_crop_1024x576.png"),
    x: 889,
    y: 519,
};

fn get_calib(w: u32, h: u32) -> Option<&'static CalibData> {
    match (w, h) {
        (512, 512) => Some(&CALIB_512X512),
        (1024, 1024) => Some(&CALIB_1024X1024),
        (576, 1024) => Some(&CALIB_576X1024),
        (1024, 576) => Some(&CALIB_1024X576),
        _ => None,
    }
}

/// 对单个像素做 Alpha 逆运算
fn alpha_inverse(pixel: [u8; 3], wcolor: [u8; 3], alpha: f32) -> [u8; 3] {
    if alpha < 0.01 || alpha > 0.99 {
        return pixel;
    }
    let denom = 1.0 - alpha;
    let r = ((pixel[0] as f32 - wcolor[0] as f32 * alpha) / denom).clamp(0.0, 255.0);
    let g = ((pixel[1] as f32 - wcolor[1] as f32 * alpha) / denom).clamp(0.0, 255.0);
    let b = ((pixel[2] as f32 - wcolor[2] as f32 * alpha) / denom).clamp(0.0, 255.0);
    [r as u8, g as u8, b as u8]
}

/// Alpha 混合逆运算去水印（多分辨率独立标定，固定位置）
pub fn remove_watermark_auto(img: &mut RgbImage) {
    let (w, h) = img.dimensions();

    let calib = match get_calib(w, h) {
        Some(c) => c,
        None => {
            eprintln!("警告: 暂不支持 {}x{} 分辨率的去水印", w, h);
            return;
        }
    };

    let alpha_crop = image::load_from_memory(calib.alpha_png)
        .expect("加载 alpha 标定数据失败")
        .to_luma8();
    let wcolor_crop = image::load_from_memory(calib.wcolor_png)
        .expect("加载 W 标定数据失败")
        .to_rgb8();

    let cw = alpha_crop.width();
    let ch = alpha_crop.height();
    let x0 = calib.x;
    let y0 = calib.y;

    for dy in 0..ch {
        for dx in 0..cw {
            let x = x0 + dx;
            let y = y0 + dy;
            if x >= w || y >= h {
                continue;
            }

            let a = alpha_crop.get_pixel(dx, dy)[0] as f32 / 255.0;
            let wc = wcolor_crop.get_pixel(dx, dy);
            let p = img.get_pixel(x, y);

            let recon = alpha_inverse(
                [p[0], p[1], p[2]],
                [wc[0], wc[1], wc[2]],
                a,
            );
            img.put_pixel(x, y, Rgb(recon));
        }
    }
}
