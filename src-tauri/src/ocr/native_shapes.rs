//! 原生形状重建模块
//!
//! 检测并重建原生 PowerPoint 形状：
//! - 矩形、圆角矩形、卡片
//! - 圆形、徽章、标记点
//! - 箭头、连接线、分割线

#![allow(dead_code)]

use std::path::PathBuf;
use image::{DynamicImage, Rgba, RgbaImage};
use serde::{Serialize, Deserialize};

/// 检测到的原生形状
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NativeShape {
    /// 唯一标识
    pub id: String,
    /// 形状类型
    pub shape_type: ShapeType,
    /// 位置 X（像素）
    pub x: f32,
    /// 位置 Y（像素）
    pub y: f32,
    /// 宽度（像素）
    pub width: f32,
    /// 高度（像素）
    pub height: f32,
    /// 样式
    pub style: ShapeStyle,
    /// 置信度
    pub confidence: f32,
}

/// 形状类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ShapeType {
    /// 矩形
    Rectangle,
    /// 圆角矩形
    RoundedRectangle,
    /// 圆形
    Circle,
    /// 椭圆
    Ellipse,
    /// 线条
    Line,
    /// 箭头
    Arrow,
    /// 徽章（圆形+数字标记）
    Badge,
    /// 卡片
    Card,
}

impl ShapeType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ShapeType::Rectangle => "Rectangle",
            ShapeType::RoundedRectangle => "RoundedRectangle",
            ShapeType::Circle => "Circle",
            ShapeType::Ellipse => "Ellipse",
            ShapeType::Line => "Line",
            ShapeType::Arrow => "Arrow",
            ShapeType::Badge => "Badge",
            ShapeType::Card => "Card",
        }
    }
}

/// 形状样式
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShapeStyle {
    /// 填充颜色（HEX）
    pub fill_color: Option<String>,
    /// 边框颜色（HEX）
    pub stroke_color: Option<String>,
    /// 边框宽度（像素）
    pub stroke_width: Option<f32>,
    /// 圆角半径（像素）
    pub corner_radius: Option<f32>,
    /// 透明度 (0.0 - 1.0)
    pub opacity: Option<f32>,
}

impl Default for ShapeStyle {
    fn default() -> Self {
        Self {
            fill_color: None,
            stroke_color: Some("D9DEE8".to_string()),
            stroke_width: Some(1.0),
            corner_radius: None,
            opacity: Some(1.0),
        }
    }
}

/// 检测到的原生线条
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NativeLine {
    /// 唯一标识
    pub id: String,
    /// 起点坐标
    pub start: (f32, f32),
    /// 终点坐标
    pub end: (f32, f32),
    /// 线条样式
    pub style: LineStyle,
    /// 置信度
    pub confidence: f32,
}

/// 线条样式
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineStyle {
    /// 线条类型：solid, dashed, dotted
    pub line_type: String,
    /// 线条颜色 (HEX)
    pub color: Option<String>,
    /// 线条宽度（像素）
    pub width: Option<f32>,
}

/// 形状检测结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShapeDetectionResult {
    /// 检测到的形状列表
    pub shapes: Vec<NativeShape>,
    /// 检测到的线条列表
    pub lines: Vec<NativeLine>,
    /// 处理耗时（毫秒）
    pub elapsed_ms: u64,
}

/// 形状检测选项
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ShapeDetectionOptions {
    /// 最小形状尺寸（像素）
    pub min_shape_size: f32,
    /// 最大形状尺寸（像素）
    pub max_shape_size: f32,
    /// 是否检测圆形
    pub detect_circles: bool,
    /// 是否检测线条
    pub detect_lines: bool,
    /// 颜色采样步长
    pub color_sample_step: u32,
}

impl Default for ShapeDetectionOptions {
    fn default() -> Self {
        Self {
            min_shape_size: 50.0,
            max_shape_size: 1000.0,
            detect_circles: true,
            detect_lines: true,
            color_sample_step: 4,
        }
    }
}

/// 检测图片中的原生形状
pub fn detect_native_shapes(
    source_path: &PathBuf,
    output_dir: &PathBuf,
    options: &ShapeDetectionOptions,
) -> Result<ShapeDetectionResult, String> {
    let start = std::time::Instant::now();
    
    let img = image::open(source_path)
        .map_err(|e| format!("Failed to open image: {}", e))?;
    
    let mut shapes = Vec::new();
    let mut lines = Vec::new();
    
    let rgba_img = img.to_rgba8();
    
    let detected_rects = detect_rectangles(&rgba_img, options);
    shapes.extend(detected_rects);
    
    if options.detect_circles {
        let detected_circles = detect_circles(&rgba_img, options);
        shapes.extend(detected_circles);
    }
    
    if options.detect_lines {
        let detected_lines = detect_straight_lines(&rgba_img, options);
        lines.extend(detected_lines);
    }
    
    let detected_badges = detect_badges(&rgba_img, &shapes, options);
    shapes.extend(detected_badges);
    
    let result = ShapeDetectionResult {
        shapes,
        lines,
        elapsed_ms: start.elapsed().as_millis() as u64,
    };
    
    let json_path = output_dir.join("native_shapes.json");
    let content = serde_json::to_string_pretty(&result)
        .map_err(|e| format!("Failed to serialize shapes: {}", e))?;
    std::fs::write(&json_path, content)
        .map_err(|e| format!("Failed to write shapes: {}", e))?;
    
    let debug_path = output_dir.join("shapes_debug.png");
    if let Err(e) = generate_shapes_debug_image(&img, &result, &debug_path) {
        eprintln!("Warning: Failed to generate shapes debug image: {}", e);
    }
    
    Ok(result)
}


/// 检测矩形
fn detect_rectangles(img: &RgbaImage, options: &ShapeDetectionOptions) -> Vec<NativeShape> {
    let (width, height) = img.dimensions();
    let mut shapes = Vec::new();
    let mut visited = vec![false; (width * height) as usize];
    let mut shape_id = 0;
    
    for y in 0..height {
        for x in 0..width {
            let idx = (y * width + x) as usize;
            if visited[idx] {
                continue;
            }
            
            let pixel = img.get_pixel(x, y);
            if pixel[3] < 128 {
                visited[idx] = true;
                continue;
            }
            
            if let Some(mut shape) = try_detect_rectangle_at(img, x, y, &mut visited, options) {
                shape_id += 1;
                shape.id = format!("shape_{:03}", shape_id);
                shape.shape_type = if shape.style.corner_radius.unwrap_or(0.0) > 0.0 {
                    ShapeType::RoundedRectangle
                } else {
                    ShapeType::Rectangle
                };
                shapes.push(shape);
            }
        }
    }
    
    shapes = merge_overlapping_shapes(shapes);
    shapes = identify_cards(shapes, options);
    
    shapes
}

/// 在指定位置尝试检测矩形
fn try_detect_rectangle_at(
    img: &RgbaImage,
    start_x: u32,
    start_y: u32,
    visited: &mut [bool],
    options: &ShapeDetectionOptions,
) -> Option<NativeShape> {
    let (width, height) = img.dimensions();
    
    let start_pixel = img.get_pixel(start_x, start_y);
    let start_color = [start_pixel[0], start_pixel[1], start_pixel[2]];
    
    let mut right = start_x;
    while right < width - 1 {
        let pixel = img.get_pixel(right + 1, start_y);
        if !is_similar_color(&[pixel[0], pixel[1], pixel[2]], &start_color, 30) {
            break;
        }
        right += 1;
    }
    
    let mut bottom = start_y;
    'outer: while bottom < height - 1 {
        for x in start_x..=right {
            let pixel = img.get_pixel(x, bottom + 1);
            if !is_similar_color(&[pixel[0], pixel[1], pixel[2]], &start_color, 30) {
                break 'outer;
            }
        }
        bottom += 1;
    }
    
    let rect_width = (right - start_x + 1) as f32;
    let rect_height = (bottom - start_y + 1) as f32;
    
    if rect_width < options.min_shape_size || rect_height < options.min_shape_size {
        return None;
    }
    if rect_width > options.max_shape_size || rect_height > options.max_shape_size {
        return None;
    }
    
    let consistency = check_rect_consistency(img, start_x, start_y, right, bottom, &start_color);
    if consistency < 0.7 {
        return None;
    }
    
    for y in start_y..=bottom {
        for x in start_x..=right {
            let idx = (y * width + x) as usize;
            visited[idx] = true;
        }
    }
    
    let corner_radius = detect_corner_radius(img, start_x, start_y, right, bottom, &start_color);
    let style = analyze_shape_style(img, start_x, start_y, right, bottom, &start_color);
    
    Some(NativeShape {
        id: String::new(),
        shape_type: ShapeType::Rectangle,
        x: start_x as f32,
        y: start_y as f32,
        width: rect_width,
        height: rect_height,
        style: ShapeStyle {
            fill_color: Some(color_to_hex(&start_color)),
            stroke_color: style.stroke_color,
            stroke_width: style.stroke_width,
            corner_radius: Some(corner_radius),
            opacity: Some(1.0),
        },
        confidence: consistency,
    })
}

/// 检测圆角半径
fn detect_corner_radius(
    img: &RgbaImage,
    x1: u32, y1: u32,
    _x2: u32, _y2: u32,
    fill_color: &[u8; 3],
) -> f32 {
    let mut radius = 0.0f32;
    
    for r in 0..20u32 {
        let check_x = x1 + r;
        let check_y = y1 + r;
        
        let (width, height) = img.dimensions();
        if check_x >= width || check_y >= height {
            break;
        }
        
        let pixel = img.get_pixel(check_x, check_y);
        if is_similar_color(&[pixel[0], pixel[1], pixel[2]], fill_color, 30) {
            radius = r as f32;
        } else {
            break;
        }
    }
    
    radius
}


/// 分析形状样式（检查所有四边边框并检测边框宽度）
fn analyze_shape_style(
    img: &RgbaImage,
    x1: u32, y1: u32,
    x2: u32, y2: u32,
    fill_color: &[u8; 3],
) -> ShapeStyle {
    let (width, height) = img.dimensions();
    
    let mut border_colors: Vec<[u8; 3]> = Vec::new();
    let mut border_widths: Vec<f32> = Vec::new();
    
    if y1 > 0 {
        let (colors, w) = detect_border_at_edge(img, x1, x2, y1, -1, width, height, fill_color);
        border_colors.extend(colors);
        if w > 0.0 { border_widths.push(w); }
    }
    
    if y2 < height - 1 {
        let (colors, w) = detect_border_at_edge(img, x1, x2, y2, 1, width, height, fill_color);
        border_colors.extend(colors);
        if w > 0.0 { border_widths.push(w); }
    }
    
    if x1 > 0 {
        let (colors, w) = detect_border_at_edge_vertical(img, y1, y2, x1, -1, width, height, fill_color);
        border_colors.extend(colors);
        if w > 0.0 { border_widths.push(w); }
    }
    
    if x2 < width - 1 {
        let (colors, w) = detect_border_at_edge_vertical(img, y1, y2, x2, 1, width, height, fill_color);
        border_colors.extend(colors);
        if w > 0.0 { border_widths.push(w); }
    }
    
    if border_colors.is_empty() {
        return ShapeStyle::default();
    }
    
    let avg_border_color = average_color(&border_colors);
    let avg_border_width = if !border_widths.is_empty() {
        border_widths.iter().sum::<f32>() / border_widths.len() as f32
    } else {
        1.0
    };
    
    if !is_similar_color(&avg_border_color, fill_color, 30) {
        ShapeStyle {
            fill_color: Some(color_to_hex(fill_color)),
            stroke_color: Some(color_to_hex(&avg_border_color)),
            stroke_width: Some(avg_border_width),
            corner_radius: None,
            opacity: Some(1.0),
        }
    } else {
        ShapeStyle {
            fill_color: Some(color_to_hex(fill_color)),
            stroke_color: None,
            stroke_width: None,
            corner_radius: None,
            opacity: Some(1.0),
        }
    }
}

/// 检测水平边缘的边框（上/下边框）
fn detect_border_at_edge(
    img: &RgbaImage,
    x1: u32, x2: u32,
    edge_y: u32,
    dy: i32,
    width: u32,
    height: u32,
    fill_color: &[u8; 3],
) -> (Vec<[u8; 3]>, f32) {
    let mut colors = Vec::new();
    let mut border_width = 0.0f32;
    
    let sample_step = ((x2 - x1) / 20).max(1);
    for x in (x1..=x2.min(width - 1)).step_by(sample_step as usize) {
        let check_y = if dy > 0 { edge_y + 1 } else { edge_y.saturating_sub(1) };
        if check_y >= height {
            continue;
        }
        
        let pixel = img.get_pixel(x, check_y);
        let color = [pixel[0], pixel[1], pixel[2]];
        
        if !is_similar_color(&color, fill_color, 30) {
            colors.push(color);
            
            let mut w = 1.0f32;
            for offset in 2..10u32 {
                let cy = if dy > 0 { edge_y + offset } else { edge_y.saturating_sub(offset) };
                if cy >= height { break; }
                
                let p = img.get_pixel(x, cy);
                let c = [p[0], p[1], p[2]];
                if is_similar_color(&c, &color, 30) { w += 1.0; } else { break; }
            }
            
            if w > border_width { border_width = w; }
        }
    }
    
    (colors, border_width)
}

/// 检测垂直边缘的边框（左/右边框）
fn detect_border_at_edge_vertical(
    img: &RgbaImage,
    y1: u32, y2: u32,
    edge_x: u32,
    dx: i32,
    width: u32,
    height: u32,
    fill_color: &[u8; 3],
) -> (Vec<[u8; 3]>, f32) {
    let mut colors = Vec::new();
    let mut border_width = 0.0f32;
    
    let sample_step = ((y2 - y1) / 20).max(1);
    for y in (y1..=y2.min(height - 1)).step_by(sample_step as usize) {
        let check_x = if dx > 0 { edge_x + 1 } else { edge_x.saturating_sub(1) };
        if check_x >= width {
            continue;
        }
        
        let pixel = img.get_pixel(check_x, y);
        let color = [pixel[0], pixel[1], pixel[2]];
        
        if !is_similar_color(&color, fill_color, 30) {
            colors.push(color);
            
            let mut w = 1.0f32;
            for offset in 2..10u32 {
                let cx = if dx > 0 { edge_x + offset } else { edge_x.saturating_sub(offset) };
                if cx >= width { break; }
                
                let p = img.get_pixel(cx, y);
                let c = [p[0], p[1], p[2]];
                if is_similar_color(&c, &color, 30) { w += 1.0; } else { break; }
            }
            
            if w > border_width { border_width = w; }
        }
    }
    
    (colors, border_width)
}


/// 检测圆形
fn detect_circles(img: &RgbaImage, options: &ShapeDetectionOptions) -> Vec<NativeShape> {
    let (width, height) = img.dimensions();
    let mut shapes = Vec::new();
    let mut visited = vec![false; (width * height) as usize];
    let mut shape_id = 100;
    
    for y in 0..height {
        for x in 0..width {
            let idx = (y * width + x) as usize;
            if visited[idx] { continue; }
            
            let pixel = img.get_pixel(x, y);
            if pixel[3] < 128 {
                visited[idx] = true;
                continue;
            }
            
            if let Some(mut shape) = try_detect_circle_at(img, x, y, &mut visited, options) {
                shape_id += 1;
                shape.id = format!("shape_{:03}", shape_id);
                shapes.push(shape);
            }
        }
    }
    
    shapes
}

/// 在指定位置尝试检测圆形
fn try_detect_circle_at(
    img: &RgbaImage,
    start_x: u32,
    start_y: u32,
    visited: &mut [bool],
    options: &ShapeDetectionOptions,
) -> Option<NativeShape> {
    let (width, height) = img.dimensions();
    
    let start_pixel = img.get_pixel(start_x, start_y);
    let start_color = [start_pixel[0], start_pixel[1], start_pixel[2]];
    
    let mut radius = 0u32;
    
    loop {
        let test_radius = radius + 1;
        if start_x + test_radius >= width || start_y + test_radius >= height { break; }
        
        let mut consistent = true;
        for angle in (0..360).step_by(15) {
            let rad = (angle as f32).to_radians();
            let check_x = (start_x as f32 + test_radius as f32 * rad.cos()) as u32;
            let check_y = (start_y as f32 + test_radius as f32 * rad.sin()) as u32;
            
            if check_x < width && check_y < height {
                let pixel = img.get_pixel(check_x, check_y);
                if !is_similar_color(&[pixel[0], pixel[1], pixel[2]], &start_color, 40) {
                    consistent = false;
                    break;
                }
            }
        }
        
        if !consistent { break; }
        radius = test_radius;
    }
    
    if radius < (options.min_shape_size / 2.0) as u32 { return None; }
    
    let diameter = radius * 2;
    
    for dy in 0..diameter {
        for dx in 0..diameter {
            let px = start_x + dx;
            let py = start_y + dy;
            if px < width && py < height {
                visited[(py * width + px) as usize] = true;
            }
        }
    }
    
    Some(NativeShape {
        id: String::new(),
        shape_type: ShapeType::Circle,
        x: start_x as f32,
        y: start_y as f32,
        width: diameter as f32,
        height: diameter as f32,
        style: ShapeStyle {
            fill_color: Some(color_to_hex(&start_color)),
            stroke_color: None,
            stroke_width: None,
            corner_radius: Some(radius as f32),
            opacity: Some(1.0),
        },
        confidence: 0.7,
    })
}


/// 检测直线
fn detect_straight_lines(img: &RgbaImage, options: &ShapeDetectionOptions) -> Vec<NativeLine> {
    let (width, height) = img.dimensions();
    let mut lines = Vec::new();
    let mut line_id = 0;
    
    // 检测水平线
    for y in 0..height {
        let mut line_start: Option<u32> = None;
        let mut line_color: Option<[u8; 3]> = None;
        
        for x in 0..width {
            let pixel = img.get_pixel(x, y);
            let color = [pixel[0], pixel[1], pixel[2]];
            
            if pixel[3] > 200 && !is_similar_color(&color, &[255, 255, 255], 30) {
                if line_start.is_none() {
                    line_start = Some(x);
                    line_color = Some(color);
                } else if let Some(lc) = line_color {
                    if !is_similar_color(&color, &lc, 30) {
                        if let Some(start) = line_start {
                            if x - start > options.min_shape_size as u32 {
                                line_id += 1;
                                lines.push(NativeLine {
                                    id: format!("line_{:03}", line_id),
                                    start: (start as f32, y as f32),
                                    end: (x as f32, y as f32),
                                    style: LineStyle {
                                        line_type: "solid".to_string(),
                                        color: line_color.map(|c| color_to_hex(&c)),
                                        width: Some(1.0),
                                    },
                                    confidence: 0.8,
                                });
                            }
                        }
                        line_start = Some(x);
                        line_color = Some(color);
                    }
                }
            } else {
                if let Some(start) = line_start {
                    if x - start > options.min_shape_size as u32 {
                        line_id += 1;
                        lines.push(NativeLine {
                            id: format!("line_{:03}", line_id),
                            start: (start as f32, y as f32),
                            end: (x as f32, y as f32),
                            style: LineStyle {
                                line_type: "solid".to_string(),
                                color: line_color.map(|c| color_to_hex(&c)),
                                width: Some(1.0),
                            },
                            confidence: 0.8,
                        });
                    }
                }
                line_start = None;
                line_color = None;
            }
        }
        
        if let Some(start) = line_start {
            if width - start > options.min_shape_size as u32 {
                line_id += 1;
                lines.push(NativeLine {
                    id: format!("line_{:03}", line_id),
                    start: (start as f32, y as f32),
                    end: (width as f32, y as f32),
                    style: LineStyle {
                        line_type: "solid".to_string(),
                        color: line_color.map(|c| color_to_hex(&c)),
                        width: Some(1.0),
                    },
                    confidence: 0.8,
                });
            }
        }
    }
    
    // 检测垂直线
    for x in 0..width {
        let mut line_start: Option<u32> = None;
        let mut line_color: Option<[u8; 3]> = None;
        
        for y in 0..height {
            let pixel = img.get_pixel(x, y);
            let color = [pixel[0], pixel[1], pixel[2]];
            
            if pixel[3] > 200 && !is_similar_color(&color, &[255, 255, 255], 30) {
                if line_start.is_none() {
                    line_start = Some(y);
                    line_color = Some(color);
                } else if let Some(lc) = line_color {
                    if !is_similar_color(&color, &lc, 30) {
                        if let Some(start) = line_start {
                            if y - start > options.min_shape_size as u32 {
                                line_id += 1;
                                lines.push(NativeLine {
                                    id: format!("line_{:03}", line_id),
                                    start: (x as f32, start as f32),
                                    end: (x as f32, y as f32),
                                    style: LineStyle {
                                        line_type: "solid".to_string(),
                                        color: line_color.map(|c| color_to_hex(&c)),
                                        width: Some(1.0),
                                    },
                                    confidence: 0.8,
                                });
                            }
                        }
                        line_start = Some(y);
                        line_color = Some(color);
                    }
                }
            } else {
                if let Some(start) = line_start {
                    if y - start > options.min_shape_size as u32 {
                        line_id += 1;
                        lines.push(NativeLine {
                            id: format!("line_{:03}", line_id),
                            start: (x as f32, start as f32),
                            end: (x as f32, y as f32),
                            style: LineStyle {
                                line_type: "solid".to_string(),
                                color: line_color.map(|c| color_to_hex(&c)),
                                width: Some(1.0),
                            },
                            confidence: 0.8,
                        });
                    }
                }
                line_start = None;
                line_color = None;
            }
        }
        
        if let Some(start) = line_start {
            if height - start > options.min_shape_size as u32 {
                line_id += 1;
                lines.push(NativeLine {
                    id: format!("line_{:03}", line_id),
                    start: (x as f32, start as f32),
                    end: (x as f32, height as f32),
                    style: LineStyle {
                        line_type: "solid".to_string(),
                        color: line_color.map(|c| color_to_hex(&c)),
                        width: Some(1.0),
                    },
                    confidence: 0.8,
                });
            }
        }
    }
    
    lines
}


/// 检测徽章
fn detect_badges(
    _img: &RgbaImage,
    existing_shapes: &[NativeShape],
    _options: &ShapeDetectionOptions,
) -> Vec<NativeShape> {
    let mut badges = Vec::new();
    let mut badge_id = 200;
    
    for shape in existing_shapes {
        if shape.shape_type == ShapeType::Circle {
            if shape.width < 60.0 && shape.height < 60.0 {
                badge_id += 1;
                badges.push(NativeShape {
                    id: format!("shape_{:03}", badge_id),
                    shape_type: ShapeType::Badge,
                    x: shape.x,
                    y: shape.y,
                    width: shape.width,
                    height: shape.height,
                    style: shape.style.clone(),
                    confidence: 0.6,
                });
            }
        }
    }
    
    badges
}

/// 合并重叠的形状
fn merge_overlapping_shapes(shapes: Vec<NativeShape>) -> Vec<NativeShape> {
    let mut result = Vec::new();
    
    for shape in &shapes {
        let mut is_contained = false;
        
        for other in &shapes {
            if shape.id != other.id {
                if shape.x >= other.x && shape.y >= other.y &&
                   shape.x + shape.width <= other.x + other.width &&
                   shape.y + shape.height <= other.y + other.height {
                    is_contained = true;
                    break;
                }
            }
        }
        
        if !is_contained {
            result.push(shape.clone());
        }
    }
    
    result
}

/// 识别卡片
fn identify_cards(shapes: Vec<NativeShape>, _options: &ShapeDetectionOptions) -> Vec<NativeShape> {
    shapes.into_iter().map(|mut shape| {
        if shape.shape_type == ShapeType::RoundedRectangle &&
           shape.width > 100.0 && shape.height > 60.0 {
            if shape.style.corner_radius.unwrap_or(0.0) > 4.0 {
                shape.shape_type = ShapeType::Card;
            }
        }
        shape
    }).collect()
}

/// 检查矩形区域的颜色一致性
fn check_rect_consistency(
    img: &RgbaImage,
    x1: u32, y1: u32,
    x2: u32, y2: u32,
    target_color: &[u8; 3],
) -> f32 {
    let step = 4;
    let mut matching = 0;
    let mut total = 0;
    
    for y in (y1..=y2).step_by(step) {
        for x in (x1..=x2).step_by(step) {
            let pixel = img.get_pixel(x, y);
            let color = [pixel[0], pixel[1], pixel[2]];
            
            if is_similar_color(&color, target_color, 40) {
                matching += 1;
            }
            total += 1;
        }
    }
    
    if total == 0 { 0.0 } else { matching as f32 / total as f32 }
}

/// 检查两个颜色是否相似
fn is_similar_color(a: &[u8; 3], b: &[u8; 3], threshold: u32) -> bool {
    let dr = (a[0] as i32 - b[0] as i32).abs() as u32;
    let dg = (a[1] as i32 - b[1] as i32).abs() as u32;
    let db = (a[2] as i32 - b[2] as i32).abs() as u32;
    
    dr < threshold && dg < threshold && db < threshold
}

/// 计算平均颜色
fn average_color(colors: &[[u8; 3]]) -> [u8; 3] {
    if colors.is_empty() {
        return [128, 128, 128];
    }
    
    let mut sum = [0u32; 3];
    for c in colors {
        sum[0] += c[0] as u32;
        sum[1] += c[1] as u32;
        sum[2] += c[2] as u32;
    }
    let n = colors.len() as u32;
    
    [(sum[0] / n) as u8, (sum[1] / n) as u8, (sum[2] / n) as u8]
}

/// 颜色转 HEX
fn color_to_hex(color: &[u8; 3]) -> String {
    format!("{:02X}{:02X}{:02X}", color[0], color[1], color[2])
}


/// 生成形状调试图像
fn generate_shapes_debug_image(
    source_img: &DynamicImage,
    result: &ShapeDetectionResult,
    output_path: &PathBuf,
) -> Result<(), String> {
    let mut debug_img = source_img.to_rgba8();
    let (width, height) = debug_img.dimensions();
    
    for shape in &result.shapes {
        let color = match shape.shape_type {
            ShapeType::Rectangle => Rgba([255, 0, 0, 200]),
            ShapeType::RoundedRectangle => Rgba([255, 128, 0, 200]),
            ShapeType::Circle => Rgba([0, 255, 0, 200]),
            ShapeType::Badge => Rgba([0, 0, 255, 200]),
            ShapeType::Card => Rgba([128, 0, 255, 200]),
            _ => Rgba([255, 255, 0, 200]),
        };
        
        let x1 = shape.x as u32;
        let y1 = shape.y as u32;
        let x2 = (shape.x + shape.width) as u32;
        let y2 = (shape.y + shape.height) as u32;
        
        for x in x1..x2.min(width) {
            if y1 < height { debug_img.put_pixel(x, y1, color); }
            if y2 < height { debug_img.put_pixel(x, y2, color); }
        }
        for y in y1..y2.min(height) {
            if x1 < width { debug_img.put_pixel(x1, y, color); }
            if x2 < width { debug_img.put_pixel(x2, y, color); }
        }
    }
    
    for line in &result.lines {
        let color = Rgba([0, 255, 255, 200]);
        
        let x1 = line.start.0 as u32;
        let y1 = line.start.1 as u32;
        let x2 = line.end.0 as u32;
        let y2 = line.end.1 as u32;
        
        let dx = (x2 as i32 - x1 as i32).abs();
        let dy = (y2 as i32 - y1 as i32).abs();
        
        if dx > dy {
            let (start_x, end_x, y) = if x1 < x2 { (x1, x2, y1) } else { (x2, x1, y2) };
            for x in start_x..=end_x {
                if x < width && y < height { debug_img.put_pixel(x, y, color); }
            }
        } else {
            let (start_y, end_y, x) = if y1 < y2 { (y1, y2, x1) } else { (y2, y1, x2) };
            for y in start_y..=end_y {
                if x < width && y < height { debug_img.put_pixel(x, y, color); }
            }
        }
    }
    
    debug_img.save(output_path)
        .map_err(|e| format!("Failed to save debug image: {}", e))
}

/// 将形状信息转换为 PPTX XML
pub fn shape_to_pptx_xml(shape: &NativeShape, img_width: u32, img_height: u32) -> String {
    let slide_width_emu = 12192000i64;
    let slide_height_emu = 6858000i64;
    
    let x_emu = (shape.x as f64 / img_width as f64 * slide_width_emu as f64) as i64;
    let y_emu = (shape.y as f64 / img_height as f64 * slide_height_emu as f64) as i64;
    let w_emu = (shape.width as f64 / img_width as f64 * slide_width_emu as f64) as i64;
    let h_emu = (shape.height as f64 / img_height as f64 * slide_height_emu as f64) as i64;
    
    let fill_color = shape.style.fill_color.as_deref().unwrap_or("FFFFFF");
    let stroke_color = shape.style.stroke_color.as_deref().unwrap_or("D9DEE8");
    let stroke_width_emu = (shape.style.stroke_width.unwrap_or(1.0) * 12700.0) as i64;
    
    let geometry = match shape.shape_type {
        ShapeType::Circle | ShapeType::Badge => "ellipse",
        ShapeType::RoundedRectangle | ShapeType::Card => "roundRect",
        _ => "rect",
    };
    
    let corner_adj = if matches!(shape.shape_type, ShapeType::RoundedRectangle | ShapeType::Card) {
        let r = shape.style.corner_radius.unwrap_or(8.0);
        let adj = (r / shape.width.min(shape.height) * 16667.0) as i32;
        format!("<a:avLst><a:gd name=\"adj\" fmla=\"{}\"/></a:avLst>", format!("#{}", adj))
    } else {
        String::from("<a:avLst/>")
    };
    
    let stroke_xml = if let Some(_) = shape.style.stroke_color {
        format!(r#"<a:ln w="{}"><a:solidFill><a:srgbClr val="{}"/></a:solidFill></a:ln>"#, stroke_width_emu, stroke_color)
    } else {
        r#"<a:noFill/>"#.to_string()
    };
    
    format!(
        r#"<p:sp>
<p:nvSpPr>
<p:cNvPr id="{}" name="{}"/>
<p:nvPr/>
</p:nvSpPr>
<p:spPr>
<a:xfrm>
<a:off x="{}" y="{}"/>
<a:ext cx="{}" cy="{}"/>
</a:xfrm>
<a:prstGeom prst="{}">
{}
</a:prstGeom>
<a:solidFill>
<a:srgbClr val="{}"/>
</a:solidFill>
{}
</p:spPr>
</p:sp>"#,
        shape.id, shape.shape_type.as_str(),
        x_emu, y_emu, w_emu, h_emu,
        geometry, corner_adj,
        fill_color, stroke_xml
    )
}

/// 将线条转换为 PPTX XML
pub fn line_to_pptx_xml(line: &NativeLine, img_width: u32, img_height: u32) -> String {
    let slide_width_emu = 12192000i64;
    let slide_height_emu = 6858000i64;
    
    let x1_emu = (line.start.0 as f64 / img_width as f64 * slide_width_emu as f64) as i64;
    let y1_emu = (line.start.1 as f64 / img_height as f64 * slide_height_emu as f64) as i64;
    let x2_emu = (line.end.0 as f64 / img_width as f64 * slide_width_emu as f64) as i64;
    let y2_emu = (line.end.1 as f64 / img_height as f64 * slide_height_emu as f64) as i64;
    
    let dash_style = line.style.line_type.as_str();
    let line_color = line.style.color.as_deref().unwrap_or("888888");
    let line_width_emu = (line.style.width.unwrap_or(1.0) * 12700.0) as i64;
    
    format!(
        r#"<p:cxnSp>
<p:nvCxnSpPr>
<p:cNvPr id="{}" name="Line"/>
<p:nvPr/>
</p:nvCxnSpPr>
<p:spPr>
<a:xfrm>
<a:off x="{}" y="{}"/>
<a:ext cx="{}" cy="{}"/>
</a:xfrm>
<a:prstGeom prst="line">
<a:avLst/>
</a:prstGeom>
<a:noFill/>
<a:ln w="{}">
<a:solidFill>
<a:srgbClr val="{}"/>
</a:solidFill>
<a:prstDash val="{}"/>
</a:ln>
</p:spPr>
</p:cxnSp>"#,
        line.id, x1_emu, y1_emu, (x2_emu - x1_emu).abs(), (y2_emu - y1_emu).abs(), 
        line_width_emu, line_color, dash_style
    )
}
