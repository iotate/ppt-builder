use lopdf::{Document, Object, Stream, Dictionary};
use std::path::PathBuf;
use ::image::{GenericImageView, DynamicImage};
use image::codecs::jpeg::JpegEncoder;
use std::io::Cursor;

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::State;
use crate::error_log;


#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PdfExportOptions {
    pub paper_size: PaperSize,
    pub orientation: Orientation,
    pub margins: Margins,
}

impl Default for PdfExportOptions {
    fn default() -> Self {
        Self {
            paper_size: PaperSize::A4,
            orientation: Orientation::Landscape,
            margins: Margins::default(),
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PaperSize {
    A4,
    Letter,
    Custom { width: f64, height: f64 },
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Orientation {
    Portrait,
    Landscape,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Margins {
    pub top: f64,
    pub right: f64,
    pub bottom: f64,
    pub left: f64,
}

impl Default for Margins {
    fn default() -> Self {
        Self {
            top: 10.0,
            right: 10.0,
            bottom: 10.0,
            left: 10.0,
        }
    }
}

#[tauri::command]
pub async fn export_pdf(
    cwd: State<'_, Arc<PathBuf>>,
    project_name: String,
) -> Result<String, String> {
    let cwd_path = cwd.inner().clone();
    let project_dir = cwd.join("projects").join(&project_name);

    if !project_dir.exists() {
        let error = format!("Project not found: {}", project_name);
        error_log::log_error(&cwd_path, &error);
        return Err(error);
    }

    // Find all page images
    let mut image_files = Vec::new();
    let mut entries = tokio::fs::read_dir(&project_dir)
        .await
        .map_err(|e| {
            let error = format!("Failed to read project directory: {}", e);
            error_log::log_error(&cwd_path, &error);
            error
        })?;

    while let Some(entry) = entries.next_entry().await.map_err(|e| e.to_string())? {
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with("page-") && (name.ends_with(".png") || name.ends_with(".jpg")) {
            image_files.push(entry.path());
        }
    }

    image_files.sort_by_key(|p| p.file_name().unwrap().to_string_lossy().to_string());

    if image_files.is_empty() {
        let error = "No images found to export. Please generate images first.";
        error_log::log_error(&cwd_path, error);
        return Err(error.to_string());
    }

    error_log::log_info(&cwd_path, &format!("Exporting PDF for project: {} with {} images", project_name, image_files.len()));

    let output_path = project_dir.join(format!("{}.pdf", project_name));
    let output_path_str = output_path.to_string_lossy().to_string();

    // Generate PDF
    match create_pdf_from_images(&image_files, &output_path) {
        Ok(_) => {
            error_log::log_info(&cwd_path, &format!("PDF exported successfully: {}", output_path_str));
            Ok(output_path_str)
        }
        Err(e) => {
            error_log::log_error(&cwd_path, &format!("PDF export failed: {}", e));
            Err(format!("PDF export failed: {}", e))
        }
    }
}



/// Create a PDF file from images with JPEG compression
fn create_pdf_from_images(image_paths: &[PathBuf], output_path: &PathBuf) -> Result<(), String> {
    // 压缩配置：超过这个宽度的图片会被等比缩小，以减小 PDF 体积
    const MAX_WIDTH: u32 = 1920; 
    // JPEG 压缩质量 (1-100)，值越小文件越小，但质量越差
    const JPEG_QUALITY: u8 = 95;

    let mut doc = Document::with_version("1.5");
    let mut page_ids = Vec::new();

    // 1. 创建 Catalog 对象 (文档根)
    let catalog_id = doc.new_object_id();
    let mut catalog_dict = Dictionary::new();
    catalog_dict.set("Type", "Catalog");
    
    // 2. 预先创建"页面树根节点" (Pages Object)
    let pages_id = doc.new_object_id();
    let mut pages_dict = Dictionary::new();
    pages_dict.set("Type", "Pages");
    pages_dict.set("Kids", Vec::<Object>::new()); 
    pages_dict.set("Count", 0);
    
    // 设置 Catalog 的 Pages 引用
    catalog_dict.set("Pages", pages_id);
    
    // 先插入 Pages
    doc.objects.insert(pages_id, Object::Dictionary(pages_dict));
    // 再插入 Catalog
    doc.objects.insert(catalog_id, Object::Dictionary(catalog_dict));
    // 设置 trailer 的 Root 为 Catalog
    doc.trailer.set("Root", catalog_id);

    for img_path in image_paths.iter() {
        // 3. 读取并压缩图像
        let img = image::open(img_path)
            .map_err(|e| format!("Failed to open image: {}", e))?;
        
        let (orig_width, orig_height) = img.dimensions();
        let (new_width, new_height) = if orig_width > MAX_WIDTH {
            let ratio = MAX_WIDTH as f32 / orig_width as f32;
            (MAX_WIDTH, (orig_height as f32 * ratio) as u32)
        } else {
            (orig_width, orig_height)
        };

        let img_scaled = DynamicImage::ImageRgb8(img.to_rgb8()).resize_exact(
            new_width, new_height, ::image::imageops::FilterType::Lanczos3
        );
        
        // 转换为 RGB8 格式
        let rgb_img = img_scaled.to_rgb8();

        // 4. 页面尺寸 = 压缩后的图片尺寸 (1px = 1 PDF point)
        let page_width = new_width as f32;
        let page_height = new_height as f32;

        // 5. 将图像编码为 JPEG 格式以大幅减小体积
        let mut jpeg_buffer = Cursor::new(Vec::new());
        {
            let mut encoder = JpegEncoder::new_with_quality(&mut jpeg_buffer, JPEG_QUALITY);
            encoder.encode(
                &rgb_img,
                new_width,
                new_height,
                image::ExtendedColorType::Rgb8,
            ).map_err(|e| format!("Failed to encode JPEG: {}", e))?;
        }
        let jpeg_data = jpeg_buffer.into_inner();

        // 6. 创建图像流，使用 DCTDecode (JPEG) 压缩
        let mut img_dict = Dictionary::new();
        img_dict.set("Type", "XObject");
        img_dict.set("Subtype", "Image");
        img_dict.set("Width", new_width as i64);
        img_dict.set("Height", new_height as i64);
        img_dict.set("ColorSpace", "DeviceRGB");
        img_dict.set("BitsPerComponent", 8);
        img_dict.set("Filter", "DCTDecode");  // JPEG 压缩
        
        let image_stream = Stream::new(img_dict, jpeg_data);
        let image_obj_id = doc.add_object(image_stream);

        // 7. 构建内容流 - 绘制图像
        let content_str = format!(
            "q {} 0 0 {} 0 0 cm /Im1 Do Q",
            page_width, page_height
        );
        let content_stream = Stream::new(Dictionary::new(), content_str.into_bytes());
        let content_obj_id = doc.add_object(content_stream);

        // 8. 设置资源字典
        let mut xobject_dict = Dictionary::new();
        xobject_dict.set("Im1", image_obj_id);
        let mut resources_dict = Dictionary::new();
        resources_dict.set("XObject", xobject_dict);

        // 9. 创建页面并记录 ID
        let mut page_dict = Dictionary::new();
        page_dict.set("Type", "Page");
        page_dict.set("Parent", pages_id);
        page_dict.set("MediaBox", vec![0.into(), 0.into(), page_width.into(), page_height.into()]);
        page_dict.set("Contents", content_obj_id);
        page_dict.set("Resources", resources_dict);

        let page_obj_id = doc.add_object(Object::Dictionary(page_dict));
        page_ids.push(Object::Reference(page_obj_id));
    }

    // 10. 更新页面树
    if let Some(Object::Dictionary(ref mut pages_dict)) = doc.objects.get_mut(&pages_id) {
        let count = page_ids.len() as i64;
        pages_dict.set("Kids", page_ids);
        pages_dict.set("Count", count);
    }

    // 11. 保存文件
    doc.save(output_path)
        .map_err(|e| format!("Failed to save PDF: {}", e))?;

    Ok(())
}
