//! OCR 引擎实现
//! 
//! 使用 rust-paddle-ocr (ocr-rs crate)
//! 基于 PaddleOCR v4/v5/v6，使用 MNN 后端
//! 支持中文等多种语言，高性能

use std::path::PathBuf;
use std::sync::OnceLock;
use std::sync::Mutex;
use super::{OcrResult, OcrPageResult};

/// 全局 OCR 引擎实例
static OCR_ENGINE: OnceLock<Mutex<Option<OcrEngineWrapper>>> = OnceLock::new();

/// OCR 引擎包装
pub struct OcrEngineWrapper {
    engine: ocr_rs::OcrEngine,
}

impl OcrEngineWrapper {
    /// 创建新的 OCR 引擎
    pub fn new(models_dir: &PathBuf) -> Result<Self, String> {
        let det_model = models_dir.join("PP-OCRv6_small_det.mnn");
        let rec_model = models_dir.join("PP-OCRv6_small_rec.mnn");
        let keys_path = models_dir.join("ppocr_keys_v6_small.txt");
        
        // 检查模型文件是否存在
        if !det_model.exists() {
            return Err(format!("Detection model not found: {:?}", det_model));
        }
        if !rec_model.exists() {
            return Err(format!("Recognition model not found: {:?}", rec_model));
        }
        if !keys_path.exists() {
            return Err(format!("Keys file not found: {:?}", keys_path));
        }
        
        println!("Loading OCR models from: {:?}", models_dir);
        println!("  Detection model: {:?}", det_model);
        println!("  Recognition model: {:?}", rec_model);
        println!("  Keys file: {:?}", keys_path);
        
        // 创建引擎配置
        let config = ocr_rs::OcrEngineConfig::fast()
            .with_min_result_confidence(0.5);
        
        // 创建引擎
        let engine = ocr_rs::OcrEngine::new(
            det_model.to_str().unwrap(),
            rec_model.to_str().unwrap(),
            keys_path.to_str().unwrap(),
            Some(config),
        ).map_err(|e| format!("Failed to create OCR engine: {:?}", e))?;
        
        println!("OCR engine initialized successfully");
        
        Ok(Self { engine })
    }
    
    /// 执行 OCR 识别
    pub fn recognize(&self, img: &image::DynamicImage) -> Result<Vec<OcrResult>, String> {
        let results = self.engine.recognize(img)
            .map_err(|e| format!("OCR recognition failed: {:?}", e))?;
        
        // 转换结果格式
        let mut ocr_results = Vec::new();
        for (idx, result) in results.iter().enumerate() {
            let bbox = &result.bbox;
            
            // 从角点计算多边形
            let polygon: Vec<(f32, f32)> = if let Some(points) = &bbox.points {
                points.iter().map(|p| (p.x, p.y)).collect()
            } else {
                // 从矩形生成四角点
                vec![
                    (bbox.rect.left() as f32, bbox.rect.top() as f32),
                    (bbox.rect.right() as f32, bbox.rect.top() as f32),
                    (bbox.rect.right() as f32, bbox.rect.bottom() as f32),
                    (bbox.rect.left() as f32, bbox.rect.bottom() as f32),
                ]
            };
            
            // 计算包围盒
            let xs: Vec<f32> = polygon.iter().map(|p| p.0).collect();
            let ys: Vec<f32> = polygon.iter().map(|p| p.1).collect();
            
            let x_min = xs.iter().cloned().fold(f32::INFINITY, f32::min);
            let y_min = ys.iter().cloned().fold(f32::INFINITY, f32::min);
            let x_max = xs.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
            let y_max = ys.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
            
            ocr_results.push(OcrResult {
                id: format!("ocr_{:03}", idx + 1),
                text: result.text.clone(),
                confidence: result.confidence,
                polygon_px: polygon,
                bbox_px: [x_min, y_min, x_max - x_min, y_max - y_min],
                role: None,
                review_status: "pending".to_string(),
            });
        }
        
        Ok(ocr_results)
    }
}

/// 初始化 OCR 引擎（在应用启动时调用）
pub fn init_engine(cwd: &PathBuf) -> Result<(), String> {
    let models_dir = cwd.join("models");
    
    if !models_dir.exists() {
        return Err(format!("Models directory not found: {:?}", models_dir));
    }
    
    let engine = OcrEngineWrapper::new(&models_dir)?;
    
    // 存储到全局
    let cell = OCR_ENGINE.get_or_init(|| Mutex::new(None));
    let mut guard = cell.lock().map_err(|e| format!("Lock error: {}", e))?;
    *guard = Some(engine);
    
    Ok(())
}

/// 对图片执行 OCR
pub fn run_ocr(
    source_path: &PathBuf,
    output_dir: &PathBuf,
) -> Result<OcrPageResult, String> {
    let start = std::time::Instant::now();
    
    // 读取图片
    let img = image::open(source_path)
        .map_err(|e| format!("Failed to open image: {}", e))?;
    let (width, height) = (img.width(), img.height());
    
    // 获取 OCR 引擎
    let results = {
        let cell = OCR_ENGINE.get_or_init(|| Mutex::new(None));
        let mut guard = cell.lock().map_err(|e| format!("Lock error: {}", e))?;
        
        if guard.is_none() {
            // 尝试自动初始化
            if let Some(models_dir) = find_models_dir() {
                match OcrEngineWrapper::new(&models_dir) {
                    Ok(engine) => {
                        *guard = Some(engine);
                    }
                    Err(e) => {
                        return Err(format!("Failed to initialize OCR engine: {}", e));
                    }
                }
            } else {
                return Err("OCR engine not initialized. Models directory not found.".to_string());
            }
        }
        
        let engine = guard.as_ref().unwrap();
        engine.recognize(&img)?
    };
    
    // 生成调试覆盖图
    let overlay_path = output_dir.join("ocr_overlay_debug.png");
    if let Err(e) = generate_ocr_overlay(source_path, &results, &overlay_path) {
        eprintln!("Warning: Failed to generate OCR overlay: {}", e);
    }
    
    // 构建结果
    let result = OcrPageResult {
        source: source_path.to_string_lossy().to_string(),
        image_size_px: [width, height],
        engine: "rust-paddle-ocr".to_string(),
        elapsed_ms: start.elapsed().as_millis() as u64,
        records: results,
    };
    
    // 保存结果到 JSON
    let json_path = output_dir.join("ocr_results.json");
    let content = serde_json::to_string_pretty(&result)
        .map_err(|e| format!("Failed to serialize OCR result: {}", e))?;
    std::fs::write(&json_path, content)
        .map_err(|e| format!("Failed to write OCR result: {}", e))?;
    
    Ok(result)
}

/// 查找模型目录（用于自动初始化）
fn find_models_dir() -> Option<PathBuf> {
    // 1. 检查当前工作目录下的 models
    if let Ok(cwd) = std::env::current_dir() {
        let models_dir = cwd.join("models");
        if models_dir.exists() {
            return Some(models_dir);
        }
        
        // 检查 src-tauri/models (开发时)
        let src_models = cwd.join("src-tauri").join("models");
        if src_models.exists() {
            return Some(src_models);
        }
    }
    
    // 2. 检查可执行文件目录下的 models
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            let models_dir = exe_dir.join("models");
            if models_dir.exists() {
                return Some(models_dir);
            }
        }
    }
    
    None
}

/// 生成 OCR 覆盖图
fn generate_ocr_overlay(
    source_path: &PathBuf,
    results: &[OcrResult],
    output_path: &PathBuf,
) -> Result<(), String> {
    use image::{Rgb, RgbImage};
    
    let img = image::open(source_path)
        .map_err(|e| format!("Failed to open image: {}", e))?;
    let mut overlay: RgbImage = img.to_rgb8();
    
    // 预定义颜色
    let colors = [
        Rgb([220u8, 38, 38]),   // 红色
        Rgb([0, 200, 0]),       // 绿色
        Rgb([0, 100, 255]),     // 蓝色
        Rgb([255, 200, 0]),     // 黄色
        Rgb([200, 0, 200]),     // 品红
        Rgb([0, 200, 200]),     // 青色
    ];
    
    for (i, result) in results.iter().enumerate() {
        let color = colors[i % colors.len()];
        let polygon = &result.polygon_px;
        
        // 绘制多边形边框
        if polygon.len() >= 2 {
            for j in 0..polygon.len() {
                let start = polygon[j];
                let end = polygon[(j + 1) % polygon.len()];
                draw_line(&mut overlay, start, end, color);
            }
        }
        
        // 绘制 ID 标签背景
        let [x, y, _, _] = result.bbox_px;
        let label_y = (y as i32 - 14).max(0) as u32;
        let label_x = x as u32;
        
        for py in label_y..(label_y + 12).min(overlay.height()) {
            for px in label_x..(label_x + 35).min(overlay.width()) {
                overlay.put_pixel(px, py, color);
            }
        }
    }
    
    // 创建输出目录
    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create output directory: {}", e))?;
    }
    
    overlay.save(output_path)
        .map_err(|e| format!("Failed to save overlay image: {}", e))?;
    
    Ok(())
}

/// Bresenham 线段绘制
fn draw_line(img: &mut image::RgbImage, start: (f32, f32), end: (f32, f32), color: image::Rgb<u8>) {
    let x0 = start.0 as i32;
    let y0 = start.1 as i32;
    let x1 = end.0 as i32;
    let y1 = end.1 as i32;
    
    let dx = (x1 - x0).abs();
    let dy = (y1 - y0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx - dy;
    
    let mut x = x0;
    let mut y = y0;
    
    let (width, height) = img.dimensions();
    
    loop {
        if x >= 0 && x < width as i32 && y >= 0 && y < height as i32 {
            img.put_pixel(x as u32, y as u32, color);
        }
        
        if x == x1 && y == y1 {
            break;
        }
        
        let e2 = 2 * err;
        if e2 > -dy {
            err -= dy;
            x += sx;
        }
        if e2 < dx {
            err += dx;
            y += sy;
        }
    }
}

/// 检查 OCR 是否可用
pub fn is_ocr_available() -> bool {
    // 检查引擎是否已初始化
    if let Some(cell) = OCR_ENGINE.get() {
        if let Ok(guard) = cell.lock() {
            if guard.is_some() {
                return true;
            }
        }
    }
    
    // 检查是否可以自动初始化
    find_models_dir().is_some()
}

/// 检测可用的 OCR 引擎
pub fn detect_available_engine() -> Option<String> {
    if is_ocr_available() {
        Some("rust-paddle-ocr".to_string())
    } else {
        None
    }
}
