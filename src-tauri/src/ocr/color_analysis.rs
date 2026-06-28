//! 颜色分析模块
//! 
//! 从源图片中分析文字区域的实际颜色

use image::{DynamicImage, GenericImageView};
use std::path::PathBuf;
use super::OcrResult;

/// 颜色分析结果
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ColorAnalysis {
    /// 主要文字颜色 (R, G, B)
    pub primary_color: [u8; 3],
    /// 背景颜色 (R, G, B)
    pub background_color: [u8; 3],
    /// 文字类型
    pub text_type: TextType,
    /// 颜色一致性 (0.0 - 1.0)
    pub color_consistency: f32,
}

/// 文字类型分类
#[derive(Debug, Clone, PartialEq)]
pub enum TextType {
    /// 深色文字在浅色背景
    DarkOnLight,
    /// 浅色文字在深色背景
    LightOnDark,
    /// 彩色文字
    Colored,
    /// 渐变文字
    Gradient,
}

/// 分析文字区域的颜色
pub fn analyze_text_color(
    source_path: &PathBuf,
    ocr_result: &OcrResult,
) -> Result<ColorAnalysis, String> {
    let img = image::open(source_path)
        .map_err(|e| format!("Failed to open image: {}", e))?;
    
    let [x, y, w, h] = ocr_result.bbox_px;
    
    // 提取文字区域
    let region = extract_region(&img, x as u32, y as u32, w as u32, h as u32);
    
    // 分析颜色
    analyze_region_colors(&region)
}

/// 提取图片区域
fn extract_region(img: &DynamicImage, x: u32, y: u32, w: u32, h: u32) -> Vec<[u8; 3]> {
    let mut pixels = Vec::new();
    
    let (img_w, img_h) = img.dimensions();
    let x_end = (x + w).min(img_w);
    let y_end = (y + h).min(img_h);
    
    for py in y..y_end {
        for px in x..x_end {
            let pixel = img.get_pixel(px, py);
            pixels.push([pixel[0], pixel[1], pixel[2]]);
        }
    }
    
    pixels
}

/// 分析区域颜色
fn analyze_region_colors(pixels: &[[u8; 3]]) -> Result<ColorAnalysis, String> {
    if pixels.is_empty() {
        return Err("Empty region".to_string());
    }
    
    // 使用 K-means 聚类找出主要颜色
    let colors = kmeans_colors(pixels, 2);
    
    if colors.len() < 2 {
        // 只有一种颜色，可能是纯色背景
        let color = colors.first().unwrap_or(&[128, 128, 128]);
        return Ok(ColorAnalysis {
            primary_color: *color,
            background_color: *color,
            text_type: TextType::Colored,
            color_consistency: 1.0,
        });
    }
    
    // 确定哪个是文字颜色，哪个是背景颜色
    let (text_color, bg_color, text_type) = classify_colors(&colors[0], &colors[1]);
    
    // 计算颜色一致性
    let consistency = calculate_color_consistency(pixels, &text_color, &bg_color);
    
    Ok(ColorAnalysis {
        primary_color: text_color,
        background_color: bg_color,
        text_type,
        color_consistency: consistency,
    })
}

/// 简单的 K-means 聚类
fn kmeans_colors(pixels: &[[u8; 3]], k: usize) -> Vec<[u8; 3]> {
    if pixels.is_empty() {
        return vec![[128, 128, 128]];
    }
    
    // 初始化：使用像素的极值作为初始中心
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
    for _ in 0..10 {
        // 分配像素到最近的中心
        let mut clusters: Vec<Vec<[u8; 3]>> = vec![Vec::new(); centers.len()];
        
        for pixel in pixels {
            let mut min_dist = f64::MAX;
            let mut min_idx = 0;
            
            for (idx, center) in centers.iter().enumerate() {
                let dist = color_distance(pixel, center);
                if dist < min_dist {
                    min_dist = dist;
                    min_idx = idx;
                }
            }
            
            clusters[min_idx].push(*pixel);
        }
        
        // 更新中心
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
    
    // 转换为 u8
    centers.iter()
        .map(|c| [c[0] as u8, c[1] as u8, c[2] as u8])
        .collect()
}

/// 计算颜色距离
fn color_distance(a: &[u8; 3], b: &[f64; 3]) -> f64 {
    let dr = a[0] as f64 - b[0];
    let dg = a[1] as f64 - b[1];
    let db = a[2] as f64 - b[2];
    (dr * dr + dg * dg + db * db).sqrt()
}

/// 分类颜色：确定哪个是文字颜色，哪个是背景颜色
fn classify_colors(color1: &[u8; 3], color2: &[u8; 3]) -> ([u8; 3], [u8; 3], TextType) {
    let lum1 = luminance(color1);
    let lum2 = luminance(color2);
    
    // 计算饱和度
    let sat1 = saturation(color1);
    let sat2 = saturation(color2);
    
    // 判断文字类型
    let text_type = if sat1 > 0.3 || sat2 > 0.3 {
        TextType::Colored
    } else if (lum1 - lum2).abs() < 0.3 {
        TextType::Gradient
    } else if lum1 < lum2 {
        TextType::DarkOnLight
    } else {
        TextType::LightOnDark
    };
    
    // 文字通常是对比度更高的那个
    if lum1 < lum2 {
        (*color1, *color2, text_type)
    } else {
        (*color2, *color1, text_type)
    }
}

/// 计算亮度 (0.0 - 1.0)
fn luminance(color: &[u8; 3]) -> f64 {
    (0.299 * color[0] as f64 + 0.587 * color[1] as f64 + 0.114 * color[2] as f64) / 255.0
}

/// 计算饱和度 (0.0 - 1.0)
fn saturation(color: &[u8; 3]) -> f64 {
    let max = color[0].max(color[1]).max(color[2]) as f64;
    let min = color[0].min(color[1]).min(color[2]) as f64;
    
    if max == 0.0 {
        0.0
    } else {
        (max - min) / max
    }
}

/// 计算颜色一致性
fn calculate_color_consistency(pixels: &[[u8; 3]], text_color: &[u8; 3], bg_color: &[u8; 3]) -> f32 {
    let mut text_count = 0;
    let mut bg_count = 0;
    
    for pixel in pixels {
        let dist_text = color_distance_int(pixel, text_color);
        let dist_bg = color_distance_int(pixel, bg_color);
        
        if dist_text < dist_bg {
            text_count += 1;
        } else {
            bg_count += 1;
        }
    }
    
    let total = text_count + bg_count;
    if total == 0 {
        return 0.0;
    }
    
    // 一致性是较大类的比例
    let max_count = text_count.max(bg_count);
    max_count as f32 / total as f32
}

/// 计算颜色距离（整数版本）
fn color_distance_int(a: &[u8; 3], b: &[u8; 3]) -> u32 {
    let dr = (a[0] as i32 - b[0] as i32).abs() as u32;
    let dg = (a[1] as i32 - b[1] as i32).abs() as u32;
    let db = (a[2] as i32 - b[2] as i32).abs() as u32;
    dr * dr + dg * dg + db * db
}

/// 将颜色转换为 HEX 格式
pub fn color_to_hex(color: &[u8; 3]) -> String {
    format!("{:02X}{:02X}{:02X}", color[0], color[1], color[2])
}

/// 批量分析多个 OCR 结果的颜色
#[allow(dead_code)]
pub fn analyze_all_text_colors(
    source_path: &PathBuf,
    ocr_results: &[OcrResult],
) -> Result<Vec<(String, ColorAnalysis)>, String> {
    let img = image::open(source_path)
        .map_err(|e| format!("Failed to open image: {}", e))?;
    
    let mut results = Vec::new();
    
    for ocr in ocr_results {
        let [x, y, w, h] = ocr.bbox_px;
        let region = extract_region(&img, x as u32, y as u32, w as u32, h as u32);
        
        if let Ok(analysis) = analyze_region_colors(&region) {
            results.push((ocr.id.clone(), analysis));
        }
    }
    
    Ok(results)
}
