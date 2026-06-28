//! 文本遮罩模块
//! 
//! 基于 OCR 结果去除图片中的文字，生成干净背景
//! 支持区域感知的文字去除策略

use std::path::PathBuf;
use image::{DynamicImage, ImageBuffer, Rgba, RgbaImage};
use super::OcrResult;
use super::color_analysis::{analyze_text_color, ColorAnalysis, TextType};

/// 文本遮罩选项
#[derive(Debug, Clone)]
pub struct TextMaskOptions {
    /// 扩展像素（用于遮罩边界扩展）
    pub expand_px: f32,
    /// 是否使用 Inpainting 填充
    pub use_inpainting: bool,
    /// Inpainting 半径
    pub inpaint_radius: f32,
    /// 是否使用区域感知策略
    pub use_region_aware: bool,
}

impl Default for TextMaskOptions {
    fn default() -> Self {
        Self {
            // 进一步增大扩展像素，确保完全覆盖文字边缘、阴影和抗锯齿效果
            expand_px: 12.0,
            use_inpainting: true,
            inpaint_radius: 12.0,
            use_region_aware: true,
        }
    }
}

/// 文本遮罩结果
#[derive(Debug)]
pub struct TextMaskResult {
    /// 干净背景图片
    pub clean_background: DynamicImage,
    /// 文本遮罩图片（调试用）
    pub mask_debug: Option<RgbaImage>,
}

/// 区域策略
#[derive(Debug, Clone, PartialEq)]
pub enum RegionPolicy {
    /// 深色文字在浅色背景
    DarkTextOnLight,
    /// 浅色文字在深色背景（标题栏等）
    LightTextOnDarkHeader,
    /// 彩色标签
    ColoredLabel,
    /// 复合徽章
    CompositeBadge,
    /// 复杂视觉文字
    ComplexVisualText,
}

/// 从图片中去除文字
pub fn remove_text_from_image(
    source_path: &PathBuf,
    ocr_results: &[OcrResult],
    options: &TextMaskOptions,
) -> Result<TextMaskResult, String> {
    // 读取源图片
    let img = image::open(source_path)
        .map_err(|e| format!("Failed to open image: {}", e))?;
    
    let (width, height) = (img.width(), img.height());
    let mut rgba_img = img.to_rgba8();
    
    // 创建遮罩图像（调试用）
    let mut mask_img: RgbaImage = ImageBuffer::from_pixel(width, height, Rgba([0, 0, 0, 0]));
    
    // 分析并处理每个 OCR 结果区域
    for ocr in ocr_results {
        // 只处理已接受或待审核的 OCR 结果
        if ocr.review_status == "omit" || ocr.review_status == "keep_in_background" {
            continue;
        }
        
        // 分析区域颜色
        let color_analysis = if options.use_region_aware {
            analyze_text_color(source_path, ocr).ok()
        } else {
            None
        };
        
        // 确定区域策略
        let policy = determine_region_policy(&color_analysis, ocr);
        
        let [x, y, w, h] = ocr.bbox_px;
        
        // 扩展遮罩边界
        let x1 = ((x - options.expand_px).max(0.0) as u32).min(width - 1);
        let y1 = ((y - options.expand_px).max(0.0) as u32).min(height - 1);
        let x2 = ((x + w + options.expand_px) as u32).min(width);
        let y2 = ((y + h + options.expand_px) as u32).min(height);
        
        // 根据策略应用不同的去除方法
        if options.use_inpainting {
            apply_region_policy(
                &mut rgba_img,
                x1, y1, x2, y2,
                &policy,
                &color_analysis,
                options.inpaint_radius,
            );
        }
        
        // 在遮罩图像上标记区域
        let mask_color = match policy {
            RegionPolicy::DarkTextOnLight => Rgba([255, 0, 0, 128]),    // 红色
            RegionPolicy::LightTextOnDarkHeader => Rgba([0, 0, 255, 128]), // 蓝色
            RegionPolicy::ColoredLabel => Rgba([0, 255, 0, 128]),       // 绿色
            RegionPolicy::CompositeBadge => Rgba([255, 255, 0, 128]),   // 黄色
            RegionPolicy::ComplexVisualText => Rgba([255, 0, 255, 128]), // 品红
        };
        
        for py in y1..y2 {
            for px in x1..x2 {
                mask_img.put_pixel(px, py, mask_color);
            }
        }
    }
    
    Ok(TextMaskResult {
        clean_background: DynamicImage::ImageRgba8(rgba_img),
        mask_debug: Some(mask_img),
    })
}

/// 确定区域策略
fn determine_region_policy(
    color_analysis: &Option<ColorAnalysis>,
    ocr: &OcrResult,
) -> RegionPolicy {
    if let Some(analysis) = color_analysis {
        match analysis.text_type {
            TextType::DarkOnLight => {
                // 检查是否可能是徽章
                if is_likely_badge(ocr) {
                    return RegionPolicy::CompositeBadge;
                }
                RegionPolicy::DarkTextOnLight
            }
            TextType::LightOnDark => {
                // 检查是否在深色标题栏
                if is_likely_header(ocr) {
                    return RegionPolicy::LightTextOnDarkHeader;
                }
                RegionPolicy::DarkTextOnLight // 反转处理
            }
            TextType::Colored => {
                // 彩色标签
                if is_likely_label(ocr) {
                    return RegionPolicy::ColoredLabel;
                }
                RegionPolicy::DarkTextOnLight
            }
            TextType::Gradient => {
                RegionPolicy::ComplexVisualText
            }
        }
    } else {
        // 默认策略
        RegionPolicy::DarkTextOnLight
    }
}

/// 判断是否可能是徽章
fn is_likely_badge(ocr: &OcrResult) -> bool {
    let [_, _, w, h] = ocr.bbox_px;
    let aspect_ratio = w / h;
    
    // 徽章通常是方形或接近方形，且较小
    aspect_ratio > 0.7 && aspect_ratio < 1.5 && w < 80.0 && h < 80.0
}

/// 判断是否可能是标题栏
fn is_likely_header(ocr: &OcrResult) -> bool {
    let [_x, y, w, _h] = ocr.bbox_px;
    
    // 标题通常在页面顶部，且宽度较大
    y < 150.0 && w > 200.0
}

/// 判断是否可能是标签
fn is_likely_label(ocr: &OcrResult) -> bool {
    let [_, _, w, h] = ocr.bbox_px;
    
    // 标签通常较小
    w < 150.0 && h < 40.0
}

/// 应用区域策略
fn apply_region_policy(
    img: &mut RgbaImage,
    x1: u32, y1: u32, x2: u32, y2: u32,
    policy: &RegionPolicy,
    color_analysis: &Option<ColorAnalysis>,
    radius: f32,
) {
    let (_width, _height) = img.dimensions();
    
    match policy {
        RegionPolicy::DarkTextOnLight => {
            // 遮罩深色或彩色墨迹，保留浅色背景
            inpaint_dark_text_on_light(img, x1, y1, x2, y2, color_analysis, radius);
        }
        RegionPolicy::LightTextOnDarkHeader => {
            // 使用完整标题栏区域进行填充
            inpaint_light_text_on_dark(img, x1, y1, x2, y2, color_analysis, radius);
        }
        RegionPolicy::ColoredLabel => {
            // 遮罩饱和彩色墨迹，保留浅色规则
            inpaint_colored_label(img, x1, y1, x2, y2, color_analysis, radius);
        }
        RegionPolicy::CompositeBadge => {
            // 移除源徽章区域，准备原生重建
            inpaint_composite_badge(img, x1, y1, x2, y2, color_analysis, radius);
        }
        RegionPolicy::ComplexVisualText => {
            // 移除文字像素或小区域，保留复杂视觉
            inpaint_complex_visual_text(img, x1, y1, x2, y2, color_analysis, radius);
        }
    }
}

/// 深色文字在浅色背景上的 Inpainting
fn inpaint_dark_text_on_light(
    img: &mut RgbaImage,
    x1: u32, y1: u32, x2: u32, y2: u32,
    _color_analysis: &Option<ColorAnalysis>,
    _radius: f32,
) {
    let (width, height) = img.dimensions();
    
    // 获取背景颜色 - 从边界采样，更准确
    let bg_color = calculate_border_average_color(img, x1, y1, x2, y2, width, height);
    
    // 直接用背景颜色填充整个区域，确保完全擦除文字且颜色一致
    fill_region_with_color(img, x1, y1, x2, y2, bg_color);
}

/// 检查是否为边缘像素（抗锯齿像素）
/// 边缘像素的颜色介于文字颜色和背景颜色之间
fn is_edge_pixel(pixel: &[u8], text_color: &[u8; 3], bg_color: &[u8; 3]) -> bool {
    // 计算像素到文字颜色和背景颜色的距离
    let dist_to_text = color_distance(pixel, text_color);
    let dist_to_bg = color_distance(pixel, bg_color);
    
    // 如果像素介于文字和背景之间，且与背景有一定距离，则认为是边缘像素
    // 放宽条件：只要与背景有距离就可能是边缘
    dist_to_bg > 20.0 && dist_to_text < dist_to_bg + 80.0
}

/// 检查是否为暗色像素（可能是文字阴影或描边）
fn is_dark_pixel(pixel: &[u8], bg_color: &[u8; 3]) -> bool {
    let brightness = (pixel[0] as f32 + pixel[1] as f32 + pixel[2] as f32) / 3.0;
    let bg_brightness = (bg_color[0] as f32 + bg_color[1] as f32 + bg_color[2] as f32) / 3.0;
    
    // 如果像素比背景暗很多，可能是文字阴影
    bg_brightness - brightness > 30.0
}

/// 计算两个颜色之间的欧几里得距离
fn color_distance(a: &[u8], b: &[u8]) -> f32 {
    let dr = a[0] as f32 - b[0] as f32;
    let dg = a[1] as f32 - b[1] as f32;
    let db = a[2] as f32 - b[2] as f32;
    (dr * dr + dg * dg + db * db).sqrt()
}

/// 浅色文字在深色背景上的 Inpainting
fn inpaint_light_text_on_dark(
    img: &mut RgbaImage,
    x1: u32, y1: u32, x2: u32, y2: u32,
    color_analysis: &Option<ColorAnalysis>,
    _radius: f32,
) {
    let (width, height) = img.dimensions();
    
    // 获取背景颜色（深色）
    let bg_color = color_analysis
        .as_ref()
        .map(|a| Rgba([a.background_color[0], a.background_color[1], a.background_color[2], 255]))
        .unwrap_or_else(|| {
            calculate_border_average_color(img, x1, y1, x2, y2, width, height)
        });
    
    // 使用背景颜色填充整个区域
    fill_region_with_color(img, x1, y1, x2, y2, bg_color);
}

/// 彩色标签的 Inpainting
fn inpaint_colored_label(
    img: &mut RgbaImage,
    x1: u32, y1: u32, x2: u32, y2: u32,
    color_analysis: &Option<ColorAnalysis>,
    _radius: f32,
) {
    let (width, height) = img.dimensions();
    
    // 获取背景颜色
    let bg_color = color_analysis
        .as_ref()
        .map(|a| Rgba([a.background_color[0], a.background_color[1], a.background_color[2], 255]))
        .unwrap_or_else(|| {
            calculate_border_average_color(img, x1, y1, x2, y2, width, height)
        });
    
    if let Some(analysis) = color_analysis {
        let text_color = analysis.primary_color;
        
        // 只遮罩饱和彩色像素
        for py in y1..y2.min(height) {
            for px in x1..x2.min(width) {
                let pixel = img.get_pixel(px, py);
                
                // 检查是否为高饱和度像素
                if is_high_saturation(&pixel.0[..3]) && is_similar_color(&pixel.0[..3], &text_color, 60) {
                    img.put_pixel(px, py, bg_color);
                }
            }
        }
    } else {
        fill_region_with_color(img, x1, y1, x2, y2, bg_color);
    }
}

/// 复合徽章的 Inpainting
fn inpaint_composite_badge(
    img: &mut RgbaImage,
    x1: u32, y1: u32, x2: u32, y2: u32,
    _color_analysis: &Option<ColorAnalysis>,
    _radius: f32,
) {
    let (width, height) = img.dimensions();
    
    // 对于徽章，使用边界平均颜色填充
    let bg_color = calculate_border_average_color(img, x1, y1, x2, y2, width, height);
    fill_region_with_color(img, x1, y1, x2, y2, bg_color);
    
    // 标记需要原生重建（后续步骤会添加圆形徽章）
}

/// 复杂视觉文字的 Inpainting
fn inpaint_complex_visual_text(
    img: &mut RgbaImage,
    x1: u32, y1: u32, x2: u32, y2: u32,
    color_analysis: &Option<ColorAnalysis>,
    _radius: f32,
) {
    let (width, height) = img.dimensions();
    
    // 对于复杂视觉，只移除文字像素，保留背景
    if let Some(analysis) = color_analysis {
        let text_color = analysis.primary_color;
        let bg_color = Rgba([analysis.background_color[0], analysis.background_color[1], analysis.background_color[2], 255]);
        
        for py in y1..y2.min(height) {
            for px in x1..x2.min(width) {
                let pixel = img.get_pixel(px, py);
                
                // 只遮罩非常接近文字颜色的像素
                if is_similar_color(&pixel.0[..3], &text_color, 50) {
                    img.put_pixel(px, py, bg_color);
                }
            }
        }
    } else {
        let bg_color = calculate_border_average_color(img, x1, y1, x2, y2, width, height);
        fill_region_with_color(img, x1, y1, x2, y2, bg_color);
    }
}

/// 计算边界平均颜色（多圈采样，更准确）
fn calculate_border_average_color(
    img: &RgbaImage,
    x1: u32, y1: u32, x2: u32, y2: u32,
    width: u32, height: u32,
) -> Rgba<u8> {
    let mut border_colors: Vec<[u8; 4]> = Vec::new();
    
    // 扩展采样范围：取边界外 1-3 像素的多圈采样
    let sample_range = 3u32;
    
    // 上边界采样
    for offset in 1..=sample_range {
        if y1 >= offset {
            let sample_y = y1 - offset;
            for px in x1.saturating_sub(offset)..(x2 + offset).min(width) {
                let pixel = img.get_pixel(px, sample_y);
                border_colors.push(pixel.0);
            }
        }
    }
    
    // 下边界采样
    for offset in 1..=sample_range {
        if y2 + offset < height {
            let sample_y = y2 + offset;
            for px in x1.saturating_sub(offset)..(x2 + offset).min(width) {
                let pixel = img.get_pixel(px, sample_y);
                border_colors.push(pixel.0);
            }
        }
    }
    
    // 左边界采样
    for offset in 1..=sample_range {
        if x1 >= offset {
            let sample_x = x1 - offset;
            for py in y1.saturating_sub(offset)..(y2 + offset).min(height) {
                let pixel = img.get_pixel(sample_x, py);
                border_colors.push(pixel.0);
            }
        }
    }
    
    // 右边界采样
    for offset in 1..=sample_range {
        if x2 + offset < width {
            let sample_x = x2 + offset;
            for py in y1.saturating_sub(offset)..(y2 + offset).min(height) {
                let pixel = img.get_pixel(sample_x, py);
                border_colors.push(pixel.0);
            }
        }
    }
    
    // 四角区域额外采样
    let corners = [
        (x1.saturating_sub(sample_range), y1.saturating_sub(sample_range)),
        (x2.min(width - 1), y1.saturating_sub(sample_range)),
        (x1.saturating_sub(sample_range), y2.min(height - 1)),
        (x2.min(width - 1), y2.min(height - 1)),
    ];
    
    for (cx, cy) in corners {
        if cx < width && cy < height {
            // 每个角落采样 5x5 区域
            for dx in 0..5 {
                for dy in 0..5 {
                    let px = cx.saturating_add(dx).min(width - 1);
                    let py = cy.saturating_add(dy).min(height - 1);
                    let pixel = img.get_pixel(px, py);
                    border_colors.push(pixel.0);
                }
            }
        }
    }
    
    // 计算平均颜色
    if border_colors.is_empty() {
        return Rgba([255, 255, 255, 255]);
    }
    
    // 使用中位数过滤异常值，然后取平均
    border_colors.sort_by(|a, b| {
        let avg_a = (a[0] as u32 + a[1] as u32 + a[2] as u32) / 3;
        let avg_b = (b[0] as u32 + b[1] as u32 + b[2] as u32) / 3;
        avg_a.cmp(&avg_b)
    });
    
    // 去掉最亮和最暗的 20%，取中间 60% 的平均值
    let total = border_colors.len();
    let start = total / 5;
    let end = total - total / 5;
    
    if start >= end {
        let mut sum = [0u32; 4];
        for c in &border_colors {
            sum[0] += c[0] as u32;
            sum[1] += c[1] as u32;
            sum[2] += c[2] as u32;
            sum[3] += c[3] as u32;
        }
        let n = border_colors.len() as u32;
        return Rgba([
            (sum[0] / n) as u8,
            (sum[1] / n) as u8,
            (sum[2] / n) as u8,
            (sum[3] / n) as u8,
        ]);
    }
    
    let middle_colors: Vec<[u8; 4]> = border_colors[start..end].to_vec();
    let mut sum = [0u32; 4];
    for c in &middle_colors {
        sum[0] += c[0] as u32;
        sum[1] += c[1] as u32;
        sum[2] += c[2] as u32;
        sum[3] += c[3] as u32;
    }
    let n = middle_colors.len() as u32;
    
    Rgba([
        (sum[0] / n) as u8,
        (sum[1] / n) as u8,
        (sum[2] / n) as u8,
        (sum[3] / n) as u8,
    ])
}

/// 使用颜色填充区域
fn fill_region_with_color(
    img: &mut RgbaImage,
    x1: u32, y1: u32, x2: u32, y2: u32,
    color: Rgba<u8>,
) {
    let (width, height) = img.dimensions();
    
    for py in y1..y2.min(height) {
        for px in x1..x2.min(width) {
            img.put_pixel(px, py, color);
        }
    }
}

/// 检查两个颜色是否相似
fn is_similar_color(a: &[u8], b: &[u8; 3], threshold: u32) -> bool {
    let dr = (a[0] as i32 - b[0] as i32).abs() as u32;
    let dg = (a[1] as i32 - b[1] as i32).abs() as u32;
    let db = (a[2] as i32 - b[2] as i32).abs() as u32;
    
    let distance = ((dr * dr + dg * dg + db * db) as f64).sqrt() as u32;
    distance < threshold
}

/// 检查是否为高饱和度颜色
fn is_high_saturation(color: &[u8]) -> bool {
    let max = color[0].max(color[1]).max(color[2]) as f32;
    let min = color[0].min(color[1]).min(color[2]) as f32;
    
    if max < 10.0 {
        return false;
    }
    
    let saturation = (max - min) / max;
    saturation > 0.3
}

/// 保存干净背景和调试图像
pub fn save_text_mask_results(
    result: &TextMaskResult,
    output_dir: &PathBuf,
    page_num: u32,
) -> Result<(PathBuf, Option<PathBuf>), String> {
    // 保存干净背景
    let background_path = output_dir.join(format!("page_{:02}_clean.png", page_num));
    result.clean_background.save(&background_path)
        .map_err(|e| format!("Failed to save clean background: {}", e))?;
    
    // 保存遮罩调试图像
    let mask_path = if let Some(ref mask) = result.mask_debug {
        let path = output_dir.join(format!("page_{:02}_mask_debug.png", page_num));
        mask.save(&path)
            .map_err(|e| format!("Failed to save mask debug: {}", e))?;
        Some(path)
    } else {
        None
    };
    
    Ok((background_path, mask_path))
}
