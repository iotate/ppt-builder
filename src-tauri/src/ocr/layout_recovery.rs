//! 布局恢复模块
//! 
//! 基于 OCR 结果生成文本布局清单
//! 支持角色推断、字号估算、颜色分析

use std::path::PathBuf;
use image::GenericImageView;
use super::{OcrResult, OcrPageResult, TextLayoutManifest, SlideSize, SlideTextLayout, TextBoxLayout, EditabilityReport};
use super::color_analysis::{color_to_hex, ColorAnalysis};

/// 布局恢复选项
#[derive(Debug, Clone)]
pub struct LayoutRecoveryOptions {
    /// 最小字号
    pub min_font_size: f64,
    /// 最大字号
    pub max_font_size: f64,
    /// 默认字体
    pub default_font: String,
    /// 东亚字体
    pub east_asian_font: String,
    /// 置信度阈值
    pub confidence_threshold: f32,
    /// 是否分析实际颜色
    pub analyze_colors: bool,
}

impl Default for LayoutRecoveryOptions {
    fn default() -> Self {
        Self {
            min_font_size: 8.0,
            max_font_size: 48.0,
            default_font: "Aptos Display".to_string(),
            east_asian_font: "Microsoft YaHei".to_string(),
            confidence_threshold: 0.5,
            analyze_colors: true,
        }
    }
}

/// 角色预设字号范围
#[allow(dead_code)]
pub struct RolePreset {
    pub min_pt: f64,
    pub max_pt: f64,
    pub is_bold: bool,
}

impl RolePreset {
    #[allow(dead_code)]
    pub fn for_role(role: &str) -> Self {
        match role {
            "title" => Self { min_pt: 24.0, max_pt: 34.0, is_bold: true },
            "subtitle" => Self { min_pt: 11.0, max_pt: 16.0, is_bold: false },
            "section" => Self { min_pt: 12.5, max_pt: 18.0, is_bold: true },
            "badge-number" => Self { min_pt: 11.0, max_pt: 17.0, is_bold: false },
            "label" => Self { min_pt: 9.5, max_pt: 12.5, is_bold: false },
            "card-header" => Self { min_pt: 9.5, max_pt: 12.8, is_bold: false },
            "body" => Self { min_pt: 8.8, max_pt: 12.0, is_bold: false },
            "panel-title" => Self { min_pt: 13.0, max_pt: 20.0, is_bold: true },
            "panel-copy" => Self { min_pt: 8.8, max_pt: 12.2, is_bold: false },
            "takeaway" => Self { min_pt: 15.0, max_pt: 21.0, is_bold: true },
            "source" => Self { min_pt: 6.0, max_pt: 8.0, is_bold: false },
            _ => Self { min_pt: 8.8, max_pt: 12.0, is_bold: false },
        }
    }
}

/// 从 OCR 结果生成文本布局清单
pub fn generate_text_layout(
    ocr_result: &OcrPageResult,
    source_image_path: &PathBuf,
    clean_background_path: Option<&PathBuf>,
    options: &LayoutRecoveryOptions,
) -> Result<(TextLayoutManifest, EditabilityReport), String> {
    let (img_width, img_height) = (ocr_result.image_size_px[0], ocr_result.image_size_px[1]);
    
    // 标准幻灯片尺寸（16:9）
    let slide_width = 13.333; // 英寸
    let slide_height = 7.5;   // 英寸
    
    // 像素到英寸的转换比例
    let scale_x = slide_width / img_width as f64;
    let scale_y = slide_height / img_height as f64;
    
    // 加载源图片用于颜色分析
    let source_img = if options.analyze_colors {
        image::open(source_image_path).ok()
    } else {
        None
    };
    
    // 收集所有 OCR 结果的位置信息，用于角色推断
    let position_stats = analyze_position_stats(&ocr_result.records);
    
    // 处理每个 OCR 结果，生成文本框布局
    let mut text_boxes: Vec<TextBoxLayout> = Vec::new();
    let mut accepted_count = 0u32;
    let mut omitted_count = 0u32;
    let mut corrections = 0u32;
    
    for (idx, ocr) in ocr_result.records.iter().enumerate() {
        // 判断是否接受
        let status = classify_ocr_result(ocr, options.confidence_threshold);
        
        match status.as_str() {
            "accepted" => {
                accepted_count += 1;
            }
            "omit" => {
                omitted_count += 1;
                continue;
            }
            "corrected" => {
                corrections += 1;
                accepted_count += 1;
            }
            "needs_review" => {
                accepted_count += 1;
            }
            _ => {
                continue;
            }
        }
        
        // 转换坐标
        let [x, y, w, h] = ocr.bbox_px;
        let x_inch = x as f64 * scale_x;
        let y_inch = y as f64 * scale_y;
        // 文本框宽度增加 20%，避免文字换行
        let w_inch = w as f64 * scale_x * 1.2;
        let h_inch = h as f64 * scale_y;
        
        // 推断文字角色（改进版）
        let role = infer_text_role_advanced(ocr, idx, &position_stats, img_height);
        
        // 估算字号（基于 OCR bbox 高度直接转换）
        // 高度像素 -> 英寸 -> 磅值 (1英寸 = 72磅)
        // 使用 0.8 系数：考虑行距和 OCR bbox 通常略大于实际文字
        let estimated_font_size = h as f64 * scale_y * 72.0 * 0.8;
        
        // 不再使用角色预设限制字号，让实际测量值生效
        // 只限制在全局范围内
        let font_size = estimated_font_size.clamp(options.min_font_size, options.max_font_size);
        
        // 分析实际颜色（使用改进的颜色分析方法）
        let color = if let Some(ref img) = source_img {
            analyze_and_get_color(img, ocr)
        } else {
            "353535".to_string()
        };
        
        // 推断对齐方式
        let align = infer_text_alignment(ocr, img_width);
        
        // 根据角色判断是否粗体
        let is_bold = matches!(role.as_str(), "title" | "subtitle" | "section" | "panel-title" | "takeaway");
        
        let text_box = TextBoxLayout {
            role: Some(role.clone()),
            name: Some(format!("Text {}", idx + 1)),
            text: ocr.text.clone(),
            x: x_inch,
            y: y_inch,
            w: w_inch,
            h: h_inch,
            font_size: Some(font_size),
            font_face: Some(options.default_font.clone()),
            east_asian_font: Some(options.east_asian_font.clone()),
            bold: is_bold,
            italic: false,
            color: Some(color),
            align: Some(align),
            valign: Some("top".to_string()),
            ocr_id: Some(ocr.id.clone()),
            ocr_confidence: Some(ocr.confidence),
            source_bbox_px: Some(ocr.bbox_px),
            trace_level: "line".to_string(),
            no_wrap: true,
            review_status: Some(status),
        };
        
        text_boxes.push(text_box);
    }
    
    // 创建布局清单
    let manifest = TextLayoutManifest {
        title: format!("Slide from {}", source_image_path.display()),
        slide_size: SlideSize {
            width: slide_width,
            height: slide_height,
        },
        units: "inches".to_string(),
        slides: vec![SlideTextLayout {
            slide: 1,
            source_image: source_image_path.to_string_lossy().to_string(),
            background: clean_background_path.map(|p| p.to_string_lossy().to_string()),
            text_boxes,
        }],
    };
    
    // 创建可编辑性报告
    let report = EditabilityReport {
        slide: 1,
        editable_text_bodies: accepted_count,
        accepted_ocr_lines: accepted_count,
        omitted_ocr_lines: omitted_count,
        ocr_corrections: corrections,
        native_shapes: 0,
        native_lines: 0,
        pictures: 0,
        issues: Vec::new(),
        limitations: Vec::new(),
    };
    
    Ok((manifest, report))
}

/// 位置统计信息
#[allow(dead_code)]
struct PositionStats {
    /// 平均 Y 位置
    avg_y: f32,
    /// Y 位置标准差
    std_y: f32,
    /// 最大高度
    max_h: f32,
    /// 平均高度
    avg_h: f32,
}

/// 分析 OCR 结果的位置统计
fn analyze_position_stats(ocr_results: &[OcrResult]) -> PositionStats {
    if ocr_results.is_empty() {
        return PositionStats {
            avg_y: 0.0,
            std_y: 0.0,
            max_h: 0.0,
            avg_h: 0.0,
        };
    }
    
    let y_values: Vec<f32> = ocr_results.iter().map(|o| o.bbox_px[1]).collect();
    let h_values: Vec<f32> = ocr_results.iter().map(|o| o.bbox_px[3]).collect();
    
    let avg_y = y_values.iter().sum::<f32>() / y_values.len() as f32;
    let avg_h = h_values.iter().sum::<f32>() / h_values.len() as f32;
    let max_h = h_values.iter().cloned().fold(0.0, f32::max);
    
    // 计算标准差
    let variance_y: f32 = y_values.iter()
        .map(|y| (y - avg_y).powi(2))
        .sum::<f32>() / y_values.len() as f32;
    let std_y = variance_y.sqrt();
    
    PositionStats {
        avg_y,
        std_y,
        max_h,
        avg_h,
    }
}

/// 改进的角色推断
fn infer_text_role_advanced(
    ocr: &OcrResult,
    index: usize,
    stats: &PositionStats,
    img_height: u32,
) -> String {
    let text = &ocr.text;
    let [_x, y, w, h] = ocr.bbox_px;
    
    // 计算相对位置（0.0 - 1.0）
    let rel_y = y / img_height as f32;
    
    // 1. 标题检测
    // 标题通常在顶部 15% 区域，且字号较大
    if rel_y < 0.15 {
        // 检查高度是否显著高于平均
        if h > stats.avg_h * 1.3 || h > 40.0 {
            return "title".to_string();
        }
        // 第一个文本在顶部区域
        if index == 0 {
            return "title".to_string();
        }
    }
    
    // 2. 副标题检测
    // 副标题通常在标题下方，顶部 25% 区域
    if rel_y < 0.25 && rel_y >= 0.10 {
        if h > stats.avg_h * 1.1 && text.len() > 10 {
            return "subtitle".to_string();
        }
    }
    
    // 3. 徽章数字检测
    // 短数字文本，通常较小
    let trimmed = text.trim();
    if trimmed.len() <= 3 && trimmed.chars().all(|c| c.is_numeric() || c == '.' || c == '%' || c == '+') {
        return "badge-number".to_string();
    }
    
    // 4. 标签检测
    // 短文本，通常较小，用于标注
    if text.len() <= 15 && h < stats.avg_h * 0.9 {
        // 检查是否为列表项标记
        if trimmed.starts_with(|c: char| c.is_numeric()) && trimmed.contains(|c| c == '.' || c == ')' || c == ':') {
            return "label".to_string();
        }
        // 检查是否为关键词标签
        if w < 150.0 {
            return "label".to_string();
        }
    }
    
    // 5. 章节标题检测
    // 中等大小，在内容区域上方
    if h > stats.avg_h * 1.2 && h < stats.max_h * 0.9 {
        if rel_y > 0.15 && rel_y < 0.7 {
            return "section".to_string();
        }
    }
    
    // 6. 面板标题检测
    // 检查是否有面板结构
    if is_likely_panel_title(ocr, stats) {
        return "panel-title".to_string();
    }
    
    // 7. 来源/注释检测
    // 通常在底部 10% 区域，字号较小
    if rel_y > 0.85 {
        return "source".to_string();
    }
    
    // 8. 默认为正文
    "body".to_string()
}

/// 检查是否可能是面板标题
fn is_likely_panel_title(ocr: &OcrResult, stats: &PositionStats) -> bool {
    let [_x, _y, w, h] = ocr.bbox_px;
    
    // 面板标题通常：
    // - 在卡片/面板区域内
    // - 宽度适中
    // - 高度略高于平均值
    // - 在垂直方向的特定位置
    
    h > stats.avg_h * 1.1 && h < stats.avg_h * 1.5 && w > 100.0 && w < 400.0
}

/// 基于角色预设估算字号（已弃用，保留供参考）
#[allow(dead_code)]
fn estimate_font_size_with_role(
    height_px: f64,
    img_height: f64,
    preset: &RolePreset,
    options: &LayoutRecoveryOptions,
) -> f64 {
    // 根据文字高度占图片高度的比例估算字号
    let height_ratio = height_px / img_height;
    let estimated_pt = height_ratio * 7.5 * 72.0; // 7.5 是幻灯片高度（英寸）
    
    // 先限制在角色预设范围内
    let role_limited = estimated_pt.clamp(preset.min_pt, preset.max_pt);
    
    // 再限制在全局范围内
    role_limited.clamp(options.min_font_size, options.max_font_size)
}

/// 分析并获取文字颜色
/// 
/// 改进策略：
/// 1. 先识别整张图片的主导颜色作为背景色
/// 2. 分析文字区域的颜色
/// 3. 如果文字颜色与背景色相近，使用对比色（与背景色相反）
fn analyze_and_get_color(img: &image::DynamicImage, ocr: &OcrResult) -> String {
    // 1. 获取整张图片的背景色（主导颜色）
    let bg_color = get_image_dominant_color(img);
    
    // 2. 分析文字区域的颜色
    if let Ok(analysis) = analyze_text_color_from_image(img, ocr) {
        let text_color = analysis.primary_color;
        
        // 3. 检查文字颜色是否与背景色相近
        if colors_are_similar(&text_color, &bg_color) {
            // 相近时使用对比色
            let contrast_color = get_contrast_color(&bg_color);
            color_to_hex(&contrast_color)
        } else {
            color_to_hex(&text_color)
        }
    } else {
        // 回退：根据背景色亮度决定文字颜色
        let contrast_color = get_contrast_color(&bg_color);
        color_to_hex(&contrast_color)
    }
}

/// 获取图片的主导颜色（背景色）
/// 
/// 采样整张图片，找出占比最大的颜色
fn get_image_dominant_color(img: &image::DynamicImage) -> [u8; 3] {
    let (width, height) = img.dimensions();
    
    // 采样：每隔一定像素采样一次，提高性能
    let sample_step = 4u32;
    let mut pixels: Vec<[u8; 3]> = Vec::new();
    
    for y in (0..height).step_by(sample_step as usize) {
        for x in (0..width).step_by(sample_step as usize) {
            let pixel = img.get_pixel(x, y);
            pixels.push([pixel[0], pixel[1], pixel[2]]);
        }
    }
    
    if pixels.is_empty() {
        return [255, 255, 255]; // 默认白色
    }
    
    // 统计颜色
    let color_counts = count_colors(&pixels);
    
    // 找出出现次数最多的颜色
    let mut sorted_colors: Vec<([u8; 3], usize)> = color_counts.into_iter().collect();
    sorted_colors.sort_by(|a, b| b.1.cmp(&a.1));
    
    sorted_colors.first().map(|(c, _)| *c).unwrap_or([255, 255, 255])
}

/// 判断两个颜色是否相近
/// 
/// 使用欧几里得距离，阈值设为 100（满分 441）
fn colors_are_similar(c1: &[u8; 3], c2: &[u8; 3]) -> bool {
    let dr = (c1[0] as i32 - c2[0] as i32).abs();
    let dg = (c1[1] as i32 - c2[1] as i32).abs();
    let db = (c1[2] as i32 - c2[2] as i32).abs();
    
    // 欧几里得距离
    let distance = ((dr * dr + dg * dg + db * db) as f64).sqrt();
    
    // 阈值 100（约 23% 的最大距离）
    distance < 100.0
}

/// 获取对比色（与背景色相反的颜色）
/// 
/// 策略：根据背景色亮度选择黑色或白色
/// 如果背景是彩色的，可以选择互补色
fn get_contrast_color(bg_color: &[u8; 3]) -> [u8; 3] {
    let lum = luminance(bg_color);
    
    if lum > 0.5 {
        // 浅色背景 → 深色文字
        [53, 53, 53] // 深灰色
    } else {
        // 深色背景 → 浅色文字
        [255, 255, 255] // 白色
    }
}

/// 从图片分析文字颜色（使用 OCR 多边形轮廓精确定位文字像素）
fn analyze_text_color_from_image(
    img: &image::DynamicImage,
    ocr: &OcrResult,
) -> Result<ColorAnalysis, String> {
    let (img_w, img_h) = img.dimensions();
    
    // 方法1：使用多边形轮廓提取文字像素（更精确）
    if ocr.polygon_px.len() >= 3 {
        let text_pixels = extract_pixels_inside_polygon(img, &ocr.polygon_px, img_w, img_h);
        if !text_pixels.is_empty() {
            // 分析文字像素的颜色分布
            if let Ok(analysis) = analyze_text_pixels(&text_pixels) {
                return Ok(analysis);
            }
        }
    }
    
    // 方法2：使用收缩后的 bbox（避开边缘的背景像素）
    let [x, y, w, h] = ocr.bbox_px;
    
    // 收缩区域：取 bbox 中心 60% 的区域
    let shrink_ratio = 0.2; // 每边收缩 20%
    let new_x = x + w * shrink_ratio;
    let new_y = y + h * shrink_ratio;
    let new_w = w * (1.0 - 2.0 * shrink_ratio);
    let new_h = h * (1.0 - 2.0 * shrink_ratio);
    
    let x_start = (new_x as u32).min(img_w);
    let y_start = (new_y as u32).min(img_h);
    let x_end = ((new_x + new_w) as u32).min(img_w);
    let y_end = ((new_y + new_h) as u32).min(img_h);
    
    // 提取收缩区域的像素
    let mut pixels: Vec<[u8; 3]> = Vec::new();
    for py in y_start..y_end {
        for px in x_start..x_end {
            let pixel = img.get_pixel(px, py);
            pixels.push([pixel[0], pixel[1], pixel[2]]);
        }
    }
    
    if pixels.is_empty() {
        return Err("Empty region".to_string());
    }
    
    // 分析收缩区域的颜色
    analyze_region_colors_advanced(&pixels)
}

/// 从多边形内部提取像素
fn extract_pixels_inside_polygon(
    img: &image::DynamicImage,
    polygon: &[(f32, f32)],
    img_w: u32,
    img_h: u32,
) -> Vec<[u8; 3]> {
    let mut pixels = Vec::new();
    
    // 计算多边形包围盒
    let min_x = polygon.iter().map(|p| p.0).fold(f32::INFINITY, f32::min).max(0.0) as u32;
    let max_x = polygon.iter().map(|p| p.0).fold(f32::NEG_INFINITY, f32::max).min(img_w as f32) as u32;
    let min_y = polygon.iter().map(|p| p.1).fold(f32::INFINITY, f32::min).max(0.0) as u32;
    let max_y = polygon.iter().map(|p| p.1).fold(f32::NEG_INFINITY, f32::max).min(img_h as f32) as u32;
    
    // 检查每个像素是否在多边形内
    for py in min_y..max_y {
        for px in min_x..max_x {
            if is_point_in_polygon(px as f32, py as f32, polygon) {
                let pixel = img.get_pixel(px, py);
                pixels.push([pixel[0], pixel[1], pixel[2]]);
            }
        }
    }
    
    pixels
}

/// 判断点是否在多边形内（射线法）
fn is_point_in_polygon(x: f32, y: f32, polygon: &[(f32, f32)]) -> bool {
    let n = polygon.len();
    if n < 3 {
        return false;
    }
    
    let mut inside = false;
    let mut j = n - 1;
    
    for i in 0..n {
        let xi = polygon[i].0;
        let yi = polygon[i].1;
        let xj = polygon[j].0;
        let yj = polygon[j].1;
        
        if ((yi > y) != (yj > y)) && (x < (xj - xi) * (y - yi) / (yj - yi) + xi) {
            inside = !inside;
        }
        j = i;
    }
    
    inside
}

/// 分析文字像素的颜色（从多边形内提取的像素，大部分是文字）
fn analyze_text_pixels(pixels: &[[u8; 3]]) -> Result<ColorAnalysis, String> {
    if pixels.is_empty() {
        return Err("No text pixels".to_string());
    }
    
    // 文字像素中，使用颜色直方图找出主要颜色
    let color_counts = count_colors(pixels);
    
    // 找出出现次数最多的颜色
    let mut sorted_colors: Vec<([u8; 3], usize)> = color_counts.into_iter().collect();
    sorted_colors.sort_by(|a, b| b.1.cmp(&a.1));
    
    if sorted_colors.is_empty() {
        return Err("No color counts".to_string());
    }
    
    // 如果只有一个主要颜色，那就是文字颜色
    if sorted_colors.len() == 1 {
        let text_color = sorted_colors[0].0;
        return Ok(ColorAnalysis {
            primary_color: text_color,
            background_color: [255, 255, 255], // 默认白色背景
            text_type: super::color_analysis::TextType::Colored,
            color_consistency: 1.0,
        });
    }
    
    // 多个颜色时：出现次数最多的是文字颜色（因为是从多边形内提取的）
    // 出现次数第二多的可能是背景或其他颜色
    let text_color = sorted_colors[0].0;
    let second_color = sorted_colors[1].0;
    
    // 判断文字类型
    let lum_text = luminance(&text_color);
    let _lum_second = luminance(&second_color); // 保留用于后续扩展
    
    let text_type = if lum_text > 0.7 {
        // 文字很亮，可能是白色文字
        super::color_analysis::TextType::LightOnDark
    } else if lum_text < 0.3 {
        // 文字很暗，可能是深色文字
        super::color_analysis::TextType::DarkOnLight
    } else {
        super::color_analysis::TextType::Colored
    };
    
    // 背景色取第二频繁的颜色或最亮的颜色
    let bg_color = second_color;
    
    Ok(ColorAnalysis {
        primary_color: text_color,
        background_color: bg_color,
        text_type,
        color_consistency: 0.8,
    })
}

/// 统计颜色出现次数（使用颜色量化减少颜色数量）
fn count_colors(pixels: &[[u8; 3]]) -> std::collections::HashMap<[u8; 3], usize> {
    let mut counts: std::collections::HashMap<[u8; 3], usize> = std::collections::HashMap::new();
    
    // 颜色量化：将每个通道除以 16 然后乘回，减少颜色种类
    let quantize = |v: u8| -> u8 { (v / 16) * 16 };
    
    for pixel in pixels {
        let quantized = [quantize(pixel[0]), quantize(pixel[1]), quantize(pixel[2])];
        *counts.entry(quantized).or_insert(0) += 1;
    }
    
    counts
}

/// 高级区域颜色分析
fn analyze_region_colors_advanced(pixels: &[[u8; 3]]) -> Result<ColorAnalysis, String> {
    // 使用颜色直方图方法
    let color_counts = count_colors(pixels);
    
    let mut sorted_colors: Vec<([u8; 3], usize)> = color_counts.into_iter().collect();
    sorted_colors.sort_by(|a, b| b.1.cmp(&a.1));
    
    if sorted_colors.is_empty() {
        return Err("No colors".to_string());
    }
    
    // 假设出现最多的颜色是背景色，第二多的是文字颜色
    let bg_color = sorted_colors.first().map(|(c, _)| *c).unwrap_or([255, 255, 255]);
    let text_color = sorted_colors.get(1).map(|(c, _)| *c).unwrap_or(bg_color);
    
    // 如果只有一个主要颜色，使用亮度判断
    let (final_text, final_bg) = if sorted_colors.len() == 1 {
        let lum = luminance(&bg_color);
        if lum < 0.5 {
            // 深色背景，文字可能是浅色
            ([255, 255, 255], bg_color)
        } else {
            // 浅色背景，文字可能是深色
            ([53, 53, 53], bg_color)
        }
    } else {
        // 使用亮度判断哪个是文字
        let lum_text = luminance(&text_color);
        let lum_bg = luminance(&bg_color);
        
        if lum_text < lum_bg {
            (text_color, bg_color)
        } else {
            (bg_color, text_color)
        }
    };
    
    Ok(ColorAnalysis {
        primary_color: final_text,
        background_color: final_bg,
        text_type: super::color_analysis::TextType::DarkOnLight,
        color_consistency: 0.8,
    })
}

/// 简单的 K-means 颜色聚类（保留供参考）
#[allow(dead_code)]
fn kmeans_colors_simple(pixels: &[[u8; 3]], k: usize) -> Vec<[u8; 3]> {
    if pixels.is_empty() {
        return vec![[128, 128, 128]];
    }
    
    // 初始化中心
    let mut centers: Vec<[f64; 3]> = Vec::new();
    
    // 第一个中心：最暗的像素
    let darkest = pixels.iter()
        .min_by_key(|p| p[0] as u32 + p[1] as u32 + p[2] as u32)
        .unwrap();
    centers.push([darkest[0] as f64, darkest[1] as f64, darkest[2] as f64]);
    
    if k > 1 {
        // 第二个中心：最亮的像素
        let brightest = pixels.iter()
            .max_by_key(|p| p[0] as u32 + p[1] as u32 + p[2] as u32)
            .unwrap();
        centers.push([brightest[0] as f64, brightest[1] as f64, brightest[2] as f64]);
    }
    
    // 迭代优化
    for _ in 0..5 {
        let mut clusters: Vec<Vec<[u8; 3]>> = vec![Vec::new(); centers.len()];
        
        for pixel in pixels {
            let mut min_dist = f64::MAX;
            let mut min_idx = 0;
            
            for (idx, center) in centers.iter().enumerate() {
                let dist = ((pixel[0] as f64 - center[0]).powi(2) +
                           (pixel[1] as f64 - center[1]).powi(2) +
                           (pixel[2] as f64 - center[2]).powi(2)).sqrt();
                if dist < min_dist {
                    min_dist = dist;
                    min_idx = idx;
                }
            }
            
            clusters[min_idx].push(*pixel);
        }
        
        for (idx, cluster) in clusters.iter().enumerate() {
            if !cluster.is_empty() {
                let sum: [f64; 3] = cluster.iter().fold([0.0, 0.0, 0.0], |acc, p| {
                    [acc[0] + p[0] as f64, acc[1] + p[1] as f64, acc[2] + p[2] as f64]
                });
                let n = cluster.len() as f64;
                centers[idx] = [sum[0] / n, sum[1] / n, sum[2] / n];
            }
        }
    }
    
    centers.iter().map(|c| [c[0] as u8, c[1] as u8, c[2] as u8]).collect()
}

/// 计算亮度
fn luminance(color: &[u8; 3]) -> f64 {
    (0.299 * color[0] as f64 + 0.587 * color[1] as f64 + 0.114 * color[2] as f64) / 255.0
}

/// 分类 OCR 结果
fn classify_ocr_result(ocr: &OcrResult, threshold: f32) -> String {
    // 低置信度
    if ocr.confidence < threshold {
        return "needs_review".to_string();
    }
    
    // 空文本
    if ocr.text.trim().is_empty() {
        return "omit".to_string();
    }
    
    // 单个字符且置信度低
    if ocr.text.chars().count() == 1 && ocr.confidence < 0.7 {
        return "omit".to_string();
    }
    
    // 特殊字符（图标、符号等）
    if is_special_symbol(&ocr.text) {
        return "omit".to_string();
    }
    
    "accepted".to_string()
}

/// 检查是否为特殊符号
fn is_special_symbol(text: &str) -> bool {
    let text = text.trim();
    
    // 常见的装饰性符号
    let decorative_symbols = ["●", "○", "■", "□", "▲", "△", "◆", "◇", "★", "☆", "→", "←", "↑", "↓", "•", "·", "►", "▶", "◀", "▼"];
    
    decorative_symbols.contains(&text)
}

/// 推断对齐方式
fn infer_text_alignment(ocr: &OcrResult, img_width: u32) -> String {
    let [x, _, w, _] = ocr.bbox_px;
    
    // 计算文字区域中心
    let center = x + w / 2.0;
    let img_center = img_width as f32 / 2.0;
    
    // 判断是否居中
    if (center - img_center).abs() < img_width as f32 * 0.1 {
        return "center".to_string();
    }
    
    // 判断是否右对齐
    if x > img_width as f32 * 0.6 {
        return "right".to_string();
    }
    
    // 默认左对齐
    "left".to_string()
}

/// 保存布局清单到文件
pub fn save_text_layout_manifest(
    manifest: &TextLayoutManifest,
    output_path: &PathBuf,
) -> Result<(), String> {
    let content = serde_json::to_string_pretty(manifest)
        .map_err(|e| format!("Failed to serialize manifest: {}", e))?;
    
    std::fs::write(output_path, content)
        .map_err(|e| format!("Failed to write manifest: {}", e))?;
    
    Ok(())
}

/// 保存可编辑性报告
pub fn save_editability_report(
    report: &EditabilityReport,
    output_path: &PathBuf,
) -> Result<(), String> {
    let content = serde_json::to_string_pretty(report)
        .map_err(|e| format!("Failed to serialize report: {}", e))?;
    
    std::fs::write(output_path, content)
        .map_err(|e| format!("Failed to write report: {}", e))?;
    
    Ok(())
}

/// 合并多个页面的布局清单
pub fn merge_text_layout_manifests(
    manifests: Vec<TextLayoutManifest>,
    reports: Vec<EditabilityReport>,
) -> (TextLayoutManifest, EditabilityReport) {
    if manifests.is_empty() {
        return (
            TextLayoutManifest {
                title: "Empty Presentation".to_string(),
                slide_size: SlideSize { width: 13.333, height: 7.5 },
                units: "inches".to_string(),
                slides: Vec::new(),
            },
            EditabilityReport {
                slide: 0,
                editable_text_bodies: 0,
                accepted_ocr_lines: 0,
                omitted_ocr_lines: 0,
                ocr_corrections: 0,
                native_shapes: 0,
                native_lines: 0,
                pictures: 0,
                issues: vec!["No slides processed".to_string()],
                limitations: Vec::new(),
            },
        );
    }
    
    let first = &manifests[0];
    
    // 合并幻灯片
    let mut all_slides: Vec<SlideTextLayout> = Vec::new();
    let mut slide_num = 1;
    
    for manifest in &manifests {
        for mut slide in manifest.slides.clone() {
            slide.slide = slide_num;
            all_slides.push(slide);
            slide_num += 1;
        }
    }
    
    // 合并报告
    let mut total_editable = 0u32;
    let mut total_accepted = 0u32;
    let mut total_omitted = 0u32;
    let mut total_corrections = 0u32;
    let mut all_issues: Vec<String> = Vec::new();
    let mut all_limitations: Vec<String> = Vec::new();
    
    for (idx, report) in reports.iter().enumerate() {
        total_editable += report.editable_text_bodies;
        total_accepted += report.accepted_ocr_lines;
        total_omitted += report.omitted_ocr_lines;
        total_corrections += report.ocr_corrections;
        
        for issue in &report.issues {
            all_issues.push(format!("Slide {}: {}", idx + 1, issue));
        }
        for limit in &report.limitations {
            all_limitations.push(format!("Slide {}: {}", idx + 1, limit));
        }
    }
    
    let merged_manifest = TextLayoutManifest {
        title: first.title.clone(),
        slide_size: first.slide_size.clone(),
        units: first.units.clone(),
        slides: all_slides,
    };
    
    let merged_report = EditabilityReport {
        slide: 0,
        editable_text_bodies: total_editable,
        accepted_ocr_lines: total_accepted,
        omitted_ocr_lines: total_omitted,
        ocr_corrections: total_corrections,
        native_shapes: 0,
        native_lines: 0,
        pictures: 0,
        issues: all_issues,
        limitations: all_limitations,
    };
    
    (merged_manifest, merged_report)
}
