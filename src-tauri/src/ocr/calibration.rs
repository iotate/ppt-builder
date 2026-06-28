//! 校准与验证模块
//! 
//! 确保导出的 PPTX 与源图片的美观度一致：
//! - 文字位置/大小/颜色校准
//! - 背景完整性验证
//! - 形状检测验证

use std::path::PathBuf;
use image::{DynamicImage, GenericImageView};
use super::{OcrResult, TextBoxLayout, NativeShape, NativeLine};

/// 校准选项
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct CalibrationOptions {
    /// 是否启用字号校准
    pub calibrate_font_size: bool,
    /// 是否启用颜色校准
    pub calibrate_color: bool,
    /// 是否启用位置校准
    pub calibrate_position: bool,
    /// 是否验证背景完整性
    pub verify_background: bool,
    /// 校准容差（像素）
    pub tolerance_px: f32,
    /// 颜色采样半径
    pub color_sample_radius: u32,
}

impl Default for CalibrationOptions {
    fn default() -> Self {
        Self {
            calibrate_font_size: true,
            calibrate_color: true,
            calibrate_position: true,
            verify_background: true,
            tolerance_px: 3.0,
            color_sample_radius: 2,
        }
    }
}

/// 校准结果
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct CalibrationResult {
    /// 字号调整建议
    pub font_size_adjustments: Vec<(String, f64)>,
    /// 颜色修正
    pub color_corrections: Vec<(String, String)>,
    /// 位置微调
    pub position_adjustments: Vec<(String, f64, f64)>,
    /// 背景问题
    pub background_issues: Vec<String>,
    /// 整体质量评分 (0.0 - 1.0)
    pub quality_score: f32,
}

/// 校准文本框布局
pub fn calibrate_text_boxes(
    source_img: &DynamicImage,
    text_boxes: &[TextBoxLayout],
    ocr_results: &[OcrResult],
    options: &CalibrationOptions,
) -> CalibrationResult {
    let mut font_adjustments = Vec::new();
    let mut color_corrections = Vec::new();
    let mut position_adjustments = Vec::new();
    let issues = Vec::new();
    let mut total_score = 0.0f32;
    let mut score_count = 0;

    for (tb, ocr) in text_boxes.iter().zip(ocr_results.iter()) {
        // 1. 字号校准
        if options.calibrate_font_size {
            if let Some(adjusted_size) = calibrate_font_size(source_img, tb, ocr, options) {
                let diff = (adjusted_size - tb.font_size.unwrap_or(12.0)).abs();
                if diff > 1.0 {
                    font_adjustments.push((tb.ocr_id.clone().unwrap_or_default(), adjusted_size));
                }
                total_score += if diff < 2.0 { 1.0 } else if diff < 4.0 { 0.8 } else { 0.6 };
                score_count += 1;
            }
        }

        // 2. 颜色校准
        if options.calibrate_color {
            if let Some(corrected_color) = calibrate_text_color(source_img, ocr, options) {
                if let Some(ref current_color) = tb.color {
                    if corrected_color != *current_color {
                        color_corrections.push((tb.ocr_id.clone().unwrap_or_default(), corrected_color));
                    }
                }
            }
        }

        // 3. 位置校准
        if options.calibrate_position {
            if let Some((dx, dy)) = calibrate_position(source_img, tb, ocr, options) {
                if dx.abs() > options.tolerance_px as f64 || dy.abs() > options.tolerance_px as f64 {
                    position_adjustments.push((tb.ocr_id.clone().unwrap_or_default(), dx, dy));
                }
            }
        }
    }

    // 计算质量评分
    let quality_score = if score_count > 0 {
        total_score / score_count as f32
    } else {
        0.5
    };

    CalibrationResult {
        font_size_adjustments: font_adjustments,
        color_corrections: color_corrections,
        position_adjustments: position_adjustments,
        background_issues: issues,
        quality_score,
    }
}

/// 校准字号
fn calibrate_font_size(
    source_img: &DynamicImage,
    text_box: &TextBoxLayout,
    ocr: &OcrResult,
    _options: &CalibrationOptions,
) -> Option<f64> {
    let [x, y, w, h] = ocr.bbox_px;
    
    // 计算文字区域的平均亮度分布
    let (width, height) = source_img.dimensions();
    let x_start = x as u32;
    let y_start = y as u32;
    let x_end = (x_start + w as u32).min(width);
    let y_end = (y_start + h as u32).min(height);
    
    // 分析文字密度
    let mut dark_pixel_count = 0;
    let mut total_pixels = 0;
    
    for py in y_start..y_end {
        for px in x_start..x_end {
            let pixel = source_img.get_pixel(px, py);
            let luminance = 0.299 * pixel[0] as f32 + 0.587 * pixel[1] as f32 + 0.114 * pixel[2] as f32;
            
            if luminance < 128.0 {
                dark_pixel_count += 1;
            }
            total_pixels += 1;
        }
    }
    
    if total_pixels == 0 {
        return None;
    }
    
    // 根据文字密度调整字号
    let density = dark_pixel_count as f32 / total_pixels as f32;
    
    // 高密度文字可能需要更大字号
    if density > 0.4 {
        let current_size = text_box.font_size.unwrap_or(12.0);
        let adjustment = 1.0 + (density - 0.3) * 0.2;
        return Some((current_size as f32 * adjustment) as f64);
    }
    
    None
}

/// 校准文字颜色
fn calibrate_text_color(
    source_img: &DynamicImage,
    ocr: &OcrResult,
    options: &CalibrationOptions,
) -> Option<String> {
    let [x, y, w, h] = ocr.bbox_px;
    let (width, height) = source_img.dimensions();
    
    let x_center = (x + w / 2.0) as u32;
    let y_center = (y + h / 2.0) as u32;
    let r = options.color_sample_radius;
    
    // 采样中心区域的颜色
    let mut colors: Vec<[u8; 3]> = Vec::new();
    
    for dy in -((r as i32).min(y_center as i32))..=((r as i32).min((height - y_center - 1) as i32)) {
        for dx in -((r as i32).min(x_center as i32))..=((r as i32).min((width - x_center - 1) as i32)) {
            let px = (x_center as i32 + dx) as u32;
            let py = (y_center as i32 + dy) as u32;
            
            let pixel = source_img.get_pixel(px, py);
            colors.push([pixel[0], pixel[1], pixel[2]]);
        }
    }
    
    if colors.is_empty() {
        return None;
    }
    
    // 找出最深的颜色（文字颜色）
    let darkest = colors.iter()
        .min_by_key(|c| c[0] as u32 + c[1] as u32 + c[2] as u32)?;
    
    Some(format!("{:02X}{:02X}{:02X}", darkest[0], darkest[1], darkest[2]))
}

/// 校准位置
fn calibrate_position(
    _source_img: &DynamicImage,
    text_box: &TextBoxLayout,
    ocr: &OcrResult,
    _options: &CalibrationOptions,
) -> Option<(f64, f64)> {
    // 计算预期位置和实际位置的差异
    // 像素坐标 -> 英寸
    let scale_x = 13.333 / 1920.0;
    let scale_y = 7.5 / 1080.0;
    
    let expected_x = ocr.bbox_px[0] as f64 * scale_x;
    let expected_y = ocr.bbox_px[1] as f64 * scale_y;
    
    let dx = text_box.x - expected_x;
    let dy = text_box.y - expected_y;
    
    Some((dx, dy))
}

/// 验证背景完整性
pub fn verify_background_integrity(
    original_path: &PathBuf,
    clean_bg_path: &PathBuf,
    text_boxes: &[TextBoxLayout],
) -> Result<Vec<String>, String> {
    let original = image::open(original_path)
        .map_err(|e| format!("Failed to open original: {}", e))?;
    let clean_bg = image::open(clean_bg_path)
        .map_err(|e| format!("Failed to open clean background: {}", e))?;
    
    let mut issues = Vec::new();
    
    let (width, height) = original.dimensions();
    
    // 检查是否有文字残留
    for tb in text_boxes {
        let Some(ref ocr_id) = tb.ocr_id else { continue };
        let Some(ref source_bbox) = tb.source_bbox_px else { continue };
        
        let [x, y, w, h] = source_bbox;
        let x_end = (x + w) as u32;
        let y_end = (y + h) as u32;
        
        // 采样检查文字区域
        let mut residual_count = 0;
        let mut sample_count = 0;
        
        for py in (*y as u32..y_end).step_by(3) {
            for px in (*x as u32..x_end).step_by(3) {
                if px >= width || py >= height {
                    continue;
                }
                
                let orig_pixel = original.get_pixel(px, py);
                let clean_pixel = clean_bg.get_pixel(px, py);
                
                // 计算差异
                let diff = (orig_pixel[0] as i32 - clean_pixel[0] as i32).abs()
                         + (orig_pixel[1] as i32 - clean_pixel[1] as i32).abs()
                         + (orig_pixel[2] as i32 - clean_pixel[2] as i32).abs();
                
                if diff > 50 {
                    residual_count += 1;
                }
                sample_count += 1;
            }
        }
        
        if sample_count > 0 && residual_count as f32 / sample_count as f32 > 0.3 {
            issues.push(format!("Text residual detected in region: {}", ocr_id));
        }
    }
    
    Ok(issues)
}

/// 验证形状检测
#[allow(dead_code)]
pub fn verify_shape_detection(
    _source_img: &DynamicImage,
    shapes: &[NativeShape],
    lines: &[NativeLine],
) -> Vec<String> {
    let mut issues = Vec::new();
    
    for shape in shapes {
        // 检查形状是否在有效范围内
        if shape.x < 0.0 || shape.y < 0.0 {
            issues.push(format!("Shape {} has invalid position", shape.id));
        }
        
        if shape.width < 5.0 || shape.height < 5.0 {
            issues.push(format!("Shape {} is too small", shape.id));
        }
        
        // 检查置信度
        if shape.confidence < 0.5 {
            issues.push(format!("Shape {} has low confidence: {:.2}", shape.id, shape.confidence));
        }
    }
    
    for line in lines {
        // 检查线条长度
        let length = ((line.end.0 - line.start.0).powi(2) + (line.end.1 - line.start.1).powi(2)).sqrt();
        if length < 10.0 {
            issues.push(format!("Line {} is too short", line.id));
        }
    }
    
    issues
}

/// 应用校准结果到文本框
pub fn apply_calibration(
    text_boxes: &mut [TextBoxLayout],
    calibration: &CalibrationResult,
) {
    // 应用字号调整
    for (ocr_id, adjusted_size) in &calibration.font_size_adjustments {
        for tb in text_boxes.iter_mut() {
            if tb.ocr_id.as_ref() == Some(ocr_id) {
                tb.font_size = Some(*adjusted_size);
            }
        }
    }
    
    // 应用颜色修正
    for (ocr_id, corrected_color) in &calibration.color_corrections {
        for tb in text_boxes.iter_mut() {
            if tb.ocr_id.as_ref() == Some(ocr_id) {
                tb.color = Some(corrected_color.clone());
            }
        }
    }
    
    // 应用位置调整
    for (ocr_id, dx, dy) in &calibration.position_adjustments {
        for tb in text_boxes.iter_mut() {
            if tb.ocr_id.as_ref() == Some(ocr_id) {
                tb.x += dx;
                tb.y += dy;
            }
        }
    }
}
