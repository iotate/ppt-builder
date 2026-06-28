//! 可编辑 PPTX 导出
//! 
//! 基于 OCR 的图片转可编辑 PPT 方案：
//! 1. 对页面图片执行 OCR，获取文字内容和精确位置
//! 2. 基于 OCR 结果去除原图中的文字，生成干净背景
//! 3. 推断文字样式（字号、颜色、角色等）
//! 4. 生成可编辑的 PPTX 文件
//!
//! 参考：ppt-to-editable 方案

use std::path::PathBuf;
use std::io::Write;
use std::sync::Arc;
use zip::ZipWriter;
use zip::write::FileOptions;
use serde::Serialize;
use tauri::{State, AppHandle, Emitter};
use crate::error_log;
use crate::ocr::{
    self, OcrPageResult, TextLayoutManifest, SlideTextLayout,
    TextMaskOptions, LayoutRecoveryOptions, EditabilityReport,
    CalibrationOptions, PageData,
};

/// 导出进度事件
#[derive(Debug, Clone, Serialize)]
pub struct ExportProgress {
    /// 当前阶段
    pub stage: String,
    /// 当前页码
    pub current: usize,
    /// 总页数
    pub total: usize,
    /// 进度消息
    pub message: String,
    /// 进度百分比 (0-100)
    pub percent: u8,
}

/// 发送进度事件
fn emit_progress(app: &AppHandle, stage: &str, current: usize, total: usize, message: &str) {
    let percent = if total > 0 {
        ((current as f64 / total as f64) * 100.0).min(100.0) as u8
    } else {
        0
    };
    
    let progress = ExportProgress {
        stage: stage.to_string(),
        current,
        total,
        message: message.to_string(),
        percent,
    };
    
    // 忽略发送错误
    let _ = app.emit("export-progress", &progress);
}

/// 导出可编辑 PPTX
/// 
/// 使用 OCR 方案：
/// 1. 对每个页面图片执行 OCR
/// 2. 基于 OCR 结果去除文字，生成干净背景
/// 3. 推断文字样式，生成文本布局清单
/// 4. 打包生成可编辑的 PPTX
#[tauri::command]
pub async fn export_editable_pptx(
    app: AppHandle,
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

    // 发送开始事件
    emit_progress(&app, "init", 0, 0, "正在初始化...");

    // 创建 pptx 临时目录
    let pptx_dir = project_dir.join("pptx");
    if !pptx_dir.exists() {
        tokio::fs::create_dir_all(&pptx_dir)
            .await
            .map_err(|e| format!("Failed to create pptx directory: {}", e))?;
    }

    // 收集页面数据
    let pages = collect_page_data(&project_dir)?;
    
    if pages.is_empty() {
        let error = "No pages found to export. Please generate images first.";
        error_log::log_error(&cwd_path, error);
        return Err(error.to_string());
    }

    let total_pages = pages.len();
    emit_progress(&app, "init", 0, total_pages, &format!("找到 {} 页", total_pages));

    error_log::log_info(&cwd_path, &format!(
        "Exporting editable PPTX for project: {} with {} pages (OCR-based)",
        project_name, total_pages
    ));

    // 检测可用的 OCR 引擎
    let engine_name = ocr::detect_available_engine();
    if engine_name.is_none() {
        let error = "OCR engine not available. Please check the installation.";
        error_log::log_error(&cwd_path, error);
        emit_progress(&app, "error", 0, total_pages, error);
        return Err(error.to_string());
    }
    
    error_log::log_info(&cwd_path, &format!("Using OCR engine: {:?}", engine_name));

    // 处理每个页面
    let mut manifests: Vec<TextLayoutManifest> = Vec::new();
    let mut reports: Vec<EditabilityReport> = Vec::new();
    let mut clean_backgrounds: Vec<PathBuf> = Vec::new();
    let mut image_sizes: Vec<(u32, u32)> = Vec::new();

    for (idx, page) in pages.iter().enumerate() {
        let page_num = page.page_num;
        let progress_msg = format!("正在处理第 {} / {} 页...", idx + 1, total_pages);
        emit_progress(&app, "ocr", idx + 1, total_pages, &progress_msg);
        error_log::log_info(&cwd_path, &format!("Processing page {} with OCR", page_num));

        // 创建页面处理目录
        let page_dir = pptx_dir.join(format!("page_{:02}", page.page_num));
        if !page_dir.exists() {
            tokio::fs::create_dir_all(&page_dir)
                .await
                .map_err(|e| format!("Failed to create page directory: {}", e))?;
        }

        // 检查是否有缓存的 OCR 结果
        let ocr_cache_path = page_dir.join("ocr_results.json");
        let ocr_result = if ocr_cache_path.exists() {
            emit_progress(&app, "ocr", idx + 1, total_pages, &format!("第 {} 页：读取缓存 OCR 结果", idx + 1));
            // 读取缓存的 OCR 结果
            let content = std::fs::read_to_string(&ocr_cache_path)
                .map_err(|e| format!("Failed to read OCR cache: {}", e))?;
            serde_json::from_str::<OcrPageResult>(&content)
                .map_err(|e| format!("Failed to parse OCR cache: {}", e))?
        } else {
            emit_progress(&app, "ocr", idx + 1, total_pages, &format!("第 {} 页：执行 OCR 识别...", idx + 1));
            // 执行 OCR（带超时）
            let ocr_future = tokio::task::spawn_blocking({
                let source_path = page.image_path.clone();
                let output_dir = page_dir.clone();
                move || ocr::run_ocr(&source_path, &output_dir)
            });
            
            // 设置 60 秒超时
            match tokio::time::timeout(std::time::Duration::from_secs(60), ocr_future).await {
                Ok(Ok(result)) => result?,
                Ok(Err(e)) => {
                    let err_msg = format!("OCR failed for page {}: {}", page_num, e);
                    error_log::log_error(&cwd_path, &err_msg);
                    emit_progress(&app, "error", idx + 1, total_pages, &err_msg);
                    return Err(err_msg);
                }
                Err(_) => {
                    let err_msg = format!("OCR timeout for page {} (60s)", page_num);
                    error_log::log_error(&cwd_path, &err_msg);
                    emit_progress(&app, "error", idx + 1, total_pages, &err_msg);
                    return Err(err_msg);
                }
            }
        };

        // 记录图片尺寸
        image_sizes.push((ocr_result.image_size_px[0], ocr_result.image_size_px[1]));

        error_log::log_info(&cwd_path, &format!(
            "Page {}: Found {} text regions",
            page.page_num, ocr_result.records.len()
        ));

        // 去除文字，生成干净背景
        emit_progress(&app, "mask", idx + 1, total_pages, &format!("第 {} 页：去除文字...", idx + 1));
        let clean_bg_path = page_dir.join("clean_background.png");
        let clean_background = if clean_bg_path.exists() {
            clean_bg_path.clone()
        } else {
            let mask_options = TextMaskOptions::default();
            let mask_result = ocr::remove_text_from_image(
                &page.image_path,
                &ocr_result.records,
                &mask_options,
            )?;

            // 保存干净背景
            let (bg_path, mask_path) = ocr::save_text_mask_results(&mask_result, &page_dir, page.page_num)?;
            
            if let Some(mask_p) = mask_path {
                error_log::log_info(&cwd_path, &format!("Saved text mask debug: {:?}", mask_p));
            }
            
            bg_path
        };

        clean_backgrounds.push(clean_background.clone());

        // 生成文本布局清单
        emit_progress(&app, "layout", idx + 1, total_pages, &format!("第 {} 页：生成布局...", idx + 1));
        let layout_options = LayoutRecoveryOptions::default();
        let (mut manifest, report) = ocr::generate_text_layout(
            &ocr_result,
            &page.image_path,
            Some(&clean_background),
            &layout_options,
        )?;
        
        // 校准文本框
        emit_progress(&app, "calibrate", idx + 1, total_pages, &format!("第 {} 页：校准文本框...", idx + 1));
        let calibration_options = CalibrationOptions::default();
        let source_img = image::open(&page.image_path)
            .map_err(|e| format!("Failed to open source image: {}", e))?;
        
        for slide in &mut manifest.slides {
            let calibration = ocr::calibrate_text_boxes(
                &source_img,
                &slide.text_boxes,
                &ocr_result.records,
                &calibration_options,
            );
            
            // 应用校准
            ocr::apply_calibration(&mut slide.text_boxes, &calibration);
            
            error_log::log_info(&cwd_path, &format!(
                "Page {}: Calibration quality score: {:.2}",
                page.page_num, calibration.quality_score
            ));
        }
        
        // 验证背景完整性
        let bg_issues = ocr::verify_background_integrity(
            &page.image_path,
            &clean_background,
            &manifest.slides[0].text_boxes,
        ).unwrap_or_default();
        
        for issue in &bg_issues {
            error_log::log_warning(&cwd_path, &format!("Background issue: {}", issue));
        }

        // 保存布局清单和报告
        let manifest_path = page_dir.join("text_layout_manifest.json");
        ocr::save_text_layout_manifest(&manifest, &manifest_path)?;

        let report_path = page_dir.join("editability_report.json");
        ocr::save_editability_report(&report, &report_path)?;

        manifests.push(manifest);
        reports.push(report);
    }

    // 合并所有页面的布局清单
    emit_progress(&app, "merge", total_pages, total_pages, "正在合并结果...");
    let (merged_manifest, merged_report) = ocr::merge_text_layout_manifests(manifests, reports);

    // 保存合并的清单
    let manifest_path = pptx_dir.join("text_layout_manifest.json");
    let manifest_content = serde_json::to_string_pretty(&merged_manifest)
        .map_err(|e| format!("Failed to serialize manifest: {}", e))?;
    std::fs::write(&manifest_path, manifest_content)
        .map_err(|e| format!("Failed to write manifest: {}", e))?;

    let report_path = pptx_dir.join("editability_report.json");
    let report_content = serde_json::to_string_pretty(&merged_report)
        .map_err(|e| format!("Failed to serialize report: {}", e))?;
    std::fs::write(&report_path, report_content)
        .map_err(|e| format!("Failed to write report: {}", e))?;

    error_log::log_info(&cwd_path, &format!(
        "Total: {} editable text boxes, {} OCR lines, {} issues",
        merged_report.editable_text_bodies,
        merged_report.accepted_ocr_lines,
        merged_report.issues.len()
    ));

    // 输出路径
    let output_path = pptx_dir.join(format!("{}-editable.pptx", project_name));
    let output_path_str = output_path.to_string_lossy().to_string();

    // 生成 PPTX（仅包含文本框和背景图片）
    emit_progress(&app, "generate", total_pages, total_pages, "正在生成 PPTX 文件...");
    match create_editable_pptx_from_manifest(&merged_manifest, &output_path, &image_sizes) {
        Ok(_) => {
            emit_progress(&app, "done", total_pages, total_pages, "导出完成！");
            error_log::log_info(&cwd_path, &format!("Editable PPTX exported successfully: {}", output_path_str));
            Ok(output_path_str)
        }
        Err(e) => {
            emit_progress(&app, "error", total_pages, total_pages, &format!("导出失败: {}", e));
            error_log::log_error(&cwd_path, &format!("Editable PPTX export failed: {}", e));
            Err(format!("PPTX export failed: {}", e))
        }
    }
}

/// 从文本布局清单创建 PPTX
fn create_editable_pptx_from_manifest(
    manifest: &TextLayoutManifest,
    output_path: &PathBuf,
    image_sizes: &[(u32, u32)],
) -> Result<(), String> {
    let file = std::fs::File::create(output_path)
        .map_err(|e| format!("Failed to create PPTX file: {}", e))?;
    
    let mut zip = ZipWriter::new(file);
    let options = FileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    let slide_count = manifest.slides.len();

    // 写入核心文件
    write_core_files(&mut zip, slide_count)?;

    // 为每个幻灯片写入内容
    for (slide_idx, slide_layout) in manifest.slides.iter().enumerate() {
        let slide_num = slide_layout.slide as usize;
        
        // 写入背景图片
        if let Some(ref bg_path) = slide_layout.background {
            let bg_path = PathBuf::from(bg_path);
            if bg_path.exists() {
                let img_data = std::fs::read(&bg_path)
                    .map_err(|e| format!("Failed to read background image: {}", e))?;
                
                let media_name = format!("ppt/media/image{}.png", slide_num);
                zip.start_file(&media_name, options)
                    .map_err(|e| format!("Failed to write media: {}", e))?;
                zip.write_all(&img_data)
                    .map_err(|e| format!("Failed to write media: {}", e))?;
            }
        }

        let (img_w, img_h) = image_sizes.get(slide_idx).copied().unwrap_or((1920, 1080));

        // 生成幻灯片 XML（仅包含文本框）
        let slide_xml = generate_slide_xml_from_layout(slide_num, slide_layout, img_w, img_h);
        let slide_path = format!("ppt/slides/slide{}.xml", slide_num);
        zip.start_file(&slide_path, options)
            .map_err(|e| format!("Failed to write slide: {}", e))?;
        zip.write_all(slide_xml.as_bytes())
            .map_err(|e| format!("Failed to write slide: {}", e))?;

        // 生成幻灯片关系文件
        let slide_rels = generate_slide_rels_with_background(slide_num);
        let slide_rels_path = format!("ppt/slides/_rels/slide{}.xml.rels", slide_num);
        zip.start_file(&slide_rels_path, options)
            .map_err(|e| format!("Failed to write slide rels: {}", e))?;
        zip.write_all(slide_rels.as_bytes())
            .map_err(|e| format!("Failed to write slide rels: {}", e))?;
    }

    zip.finish()
        .map_err(|e| format!("Failed to finalize PPTX: {}", e))?;

    Ok(())
}

/// 写入 PPTX 核心文件
fn write_core_files(zip: &mut ZipWriter<std::fs::File>, slide_count: usize) -> Result<(), String> {
    let options = FileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    // [Content_Types].xml
    let content_types = generate_content_types(slide_count);
    zip.start_file("[Content_Types].xml", options)
        .map_err(|e| format!("Failed to write Content_Types: {}", e))?;
    zip.write_all(content_types.as_bytes())
        .map_err(|e| format!("Failed to write Content_Types: {}", e))?;

    // _rels/.rels
    let rels = generate_rels();
    zip.start_file("_rels/.rels", options)
        .map_err(|e| format!("Failed to write rels: {}", e))?;
    zip.write_all(rels.as_bytes())
        .map_err(|e| format!("Failed to write rels: {}", e))?;

    // ppt/presentation.xml
    let presentation = generate_presentation(slide_count);
    zip.start_file("ppt/presentation.xml", options)
        .map_err(|e| format!("Failed to write presentation: {}", e))?;
    zip.write_all(presentation.as_bytes())
        .map_err(|e| format!("Failed to write presentation: {}", e))?;

    // ppt/_rels/presentation.xml.rels
    let presentation_rels = generate_presentation_rels(slide_count);
    zip.start_file("ppt/_rels/presentation.xml.rels", options)
        .map_err(|e| format!("Failed to write presentation rels: {}", e))?;
    zip.write_all(presentation_rels.as_bytes())
        .map_err(|e| format!("Failed to write presentation rels: {}", e))?;

    // ppt/slideLayouts/slideLayout1.xml
    let slide_layout = generate_slide_layout();
    zip.start_file("ppt/slideLayouts/slideLayout1.xml", options)
        .map_err(|e| format!("Failed to write slide layout: {}", e))?;
    zip.write_all(slide_layout.as_bytes())
        .map_err(|e| format!("Failed to write slide layout: {}", e))?;

    // ppt/slideLayouts/_rels/slideLayout1.xml.rels
    let slide_layout_rels = generate_slide_layout_rels();
    zip.start_file("ppt/slideLayouts/_rels/slideLayout1.xml.rels", options)
        .map_err(|e| format!("Failed to write slide layout rels: {}", e))?;
    zip.write_all(slide_layout_rels.as_bytes())
        .map_err(|e| format!("Failed to write slide layout rels: {}", e))?;

    // ppt/slideMasters/slideMaster1.xml
    let slide_master = generate_slide_master();
    zip.start_file("ppt/slideMasters/slideMaster1.xml", options)
        .map_err(|e| format!("Failed to write slide master: {}", e))?;
    zip.write_all(slide_master.as_bytes())
        .map_err(|e| format!("Failed to write slide master: {}", e))?;

    // ppt/slideMasters/_rels/slideMaster1.xml.rels
    let slide_master_rels = generate_slide_master_rels();
    zip.start_file("ppt/slideMasters/_rels/slideMaster1.xml.rels", options)
        .map_err(|e| format!("Failed to write slide master rels: {}", e))?;
    zip.write_all(slide_master_rels.as_bytes())
        .map_err(|e| format!("Failed to write slide master rels: {}", e))?;

    // ppt/theme/theme1.xml
    let theme = generate_theme();
    zip.start_file("ppt/theme/theme1.xml", options)
        .map_err(|e| format!("Failed to write theme: {}", e))?;
    zip.write_all(theme.as_bytes())
        .map_err(|e| format!("Failed to write theme: {}", e))?;

    Ok(())
}

/// 从布局生成幻灯片 XML（底层图片 + 文本框）
fn generate_slide_xml_from_layout(
    slide_num: usize,
    layout: &SlideTextLayout,
    _img_width: u32,
    _img_height: u32,
) -> String {
    let mut shapes_xml = String::new();
    
    // 幻灯片尺寸 (EMU)
    let slide_width_emu = 12192000i64; // 13.333 英寸
    let slide_height_emu = 6858000i64; // 7.5 英寸
    
    // 添加底层图片（干净背景）
    // id 从 2 开始，因为 id=1 被 nvGrpSpPr 占用
    shapes_xml.push_str(&format!(
        r#"<p:pic>
<p:nvPicPr>
<p:cNvPr id="2" name="Background"/>
<p:cNvPicPr/>
<p:nvPr/>
</p:nvPicPr>
<p:blipFill>
<a:blip r:embed="rId1"/>
<a:stretch>
<a:fillRect/>
</a:stretch>
</p:blipFill>
<p:spPr>
<a:xfrm>
<a:off x="0" y="0"/>
<a:ext cx="{}" cy="{}"/>
</a:xfrm>
<a:prstGeom prst="rect">
<a:avLst/>
</a:prstGeom>
</p:spPr>
</p:pic>"#,
        slide_width_emu, slide_height_emu
    ));
    
    // 下一个可用的 shape id
    let mut next_shape_id = 3;
    
    // 添加文本框
    for tb in layout.text_boxes.iter() {
        // 转换坐标：英寸 -> EMU
        let x_emu = (tb.x * 914400.0) as i64;
        let y_emu = (tb.y * 914400.0) as i64;
        let w_emu = (tb.w * 914400.0) as i64;
        let h_emu = (tb.h * 914400.0) as i64;
        
        // 字号 (单位: hundredths of a point)
        let font_size = tb.font_size.unwrap_or(12.0) as i64;
        let font_size_hpt = font_size * 100;
        
        // 颜色
        let color = tb.color.as_deref().unwrap_or("353535");
        
        // 粗体
        let bold = if tb.bold { "1" } else { "0" };
        
        // 对齐
        let align = match tb.align.as_deref() {
            Some("center") => "ctr",
            Some("right") => "r",
            _ => "l",
        };
        
        // 字体
        let font_face = tb.font_face.as_deref().unwrap_or("Aptos Display");
        let east_asian_font = tb.east_asian_font.as_deref().unwrap_or("Microsoft YaHei");

        // 处理多行文本
        let paragraphs: Vec<&str> = tb.text.split('\n').collect();
        let mut paragraphs_xml = String::new();
        
        for para in paragraphs {
            paragraphs_xml.push_str(&format!(
                r#"<a:p>
<a:pPr algn="{}">
<a:defRPr/>
</a:pPr>
<a:r>
<a:rPr lang="zh-CN" b="{}" sz="{}">
<a:solidFill>
<a:srgbClr val="{}"/>
</a:solidFill>
<a:latin typeface="{}"/>
<a:ea typeface="{}"/>
<a:cs typeface="{}"/>
</a:rPr>
<a:t>{}</a:t>
</a:r>
</a:p>"#,
                align,
                bold,
                font_size_hpt,
                color,
                font_face,
                east_asian_font,
                east_asian_font,
                escape_xml_text(para)
            ));
        }

        shapes_xml.push_str(&format!(
            r#"<p:sp>
<p:nvSpPr>
<p:cNvPr id="{}" name="TextBox {}"/>
<p:cNvSpPr txBox="1"/>
<p:nvPr/>
</p:nvSpPr>
<p:spPr>
<a:xfrm>
<a:off x="{}" y="{}"/>
<a:ext cx="{}" cy="{}"/>
</a:xfrm>
<a:prstGeom prst="rect">
<a:avLst/>
</a:prstGeom>
<a:noFill/>
</p:spPr>
<p:txBody>
<a:bodyPr wrap="square" anchor="t">
<a:noAutofit/>
</a:bodyPr>
<a:lstStyle/>
{}
</p:txBody>
</p:sp>"#,
            next_shape_id,
            slide_num,
            x_emu,
            y_emu,
            w_emu,
            h_emu,
            paragraphs_xml
        ));
        next_shape_id += 1;
    }

    // 生成完整的幻灯片 XML（使用空白背景，底层图片作为第一个形状）
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
<p:cSld>
<p:spTree>
<p:nvGrpSpPr>
<p:cNvPr id="1" name=""/>
<p:nvPr/>
</p:nvGrpSpPr>
<p:grpSpPr>
<a:xfrm>
<a:off x="0" y="0"/>
<a:ext cx="0" cy="0"/>
<a:chOff x="0" y="0"/>
<a:chExt cx="0" cy="0"/>
</a:xfrm>
</p:grpSpPr>
{}
</p:spTree>
</p:cSld>
<p:clrMapOvr>
<a:masterClrMapping/>
</p:clrMapOvr>
</p:sld>"#,
        shapes_xml
    )
}

/// 生成幻灯片关系文件（底层图片 + slideLayout）
fn generate_slide_rels_with_background(slide_num: usize) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="../media/image{}.png"/>
<Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideLayout" Target="../slideLayouts/slideLayout1.xml"/>
</Relationships>"#,
        slide_num
    )
}

/// 导出可编辑 PPTX（带元素处理）
/// 
/// 这是 export_editable_pptx 的别名，使用 OCR 方案
#[tauri::command]
pub async fn export_editable_pptx_with_elements(
    app: AppHandle,
    cwd: State<'_, Arc<PathBuf>>,
    project_name: String,
) -> Result<String, String> {
    // 直接调用 OCR 版本
    export_editable_pptx(app, cwd, project_name).await
}

/// 收集页面数据
fn collect_page_data(project_dir: &PathBuf) -> Result<Vec<PageData>, String> {
    let mut pages = Vec::new();
    let entries = std::fs::read_dir(project_dir)
        .map_err(|e| format!("Failed to read project directory: {}", e))?;

    let mut page_files: Vec<(u32, PathBuf, Option<PathBuf>)> = Vec::new();

    for entry in entries {
        let entry = entry.map_err(|e| e.to_string())?;
        let name = entry.file_name().to_string_lossy().to_string();
        
        if name.starts_with("page-") && name.ends_with(".md") {
            let num_str = name.trim_start_matches("page-").trim_end_matches(".md");
            if let Ok(num) = num_str.parse::<u32>() {
                let md_path = entry.path();
                let png_path = project_dir.join(format!("page-{:02}.png", num));
                let jpg_path = project_dir.join(format!("page-{:02}.jpg", num));
                let webp_path = project_dir.join(format!("page-{:02}.webp", num));
                
                let img_path = if png_path.exists() {
                    Some(png_path)
                } else if jpg_path.exists() {
                    Some(jpg_path)
                } else if webp_path.exists() {
                    Some(webp_path)
                } else {
                    None
                };
                
                page_files.push((num, md_path, img_path));
            }
        }
    }

    // 排序
    page_files.sort_by_key(|(num, _, _)| *num);

    // 读取内容
    for (num, md_path, img_path) in page_files {
        if let Some(image_path) = img_path {
            let markdown = std::fs::read_to_string(&md_path).unwrap_or_default();
            let title = extract_title(&markdown);
            
            pages.push(PageData {
                page_num: num,
                title,
                markdown,
                image_path,
            });
        }
    }

    Ok(pages)
}

/// 从 Markdown 提取标题
fn extract_title(markdown: &str) -> String {
    for line in markdown.lines() {
        let trimmed = line.trim();
        
        if trimmed.starts_with("## 第") && trimmed.contains("页") {
            let title = trimmed
                .trim_start_matches('#')
                .trim()
                .trim_start_matches(|c: char| c.is_numeric())
                .trim_start_matches(|c: char| c == '页' || c == '：' || c == ':')
                .trim();
            if !title.is_empty() {
                return title.to_string();
            }
        }
        
        if trimmed.starts_with("**标题**") {
            let title = trimmed
                .trim_start_matches(|c: char| c == '*' || c == ':')
                .trim_start_matches(':')
                .trim();
            if !title.is_empty() {
                return title.to_string();
            }
        }
        
        if trimmed.starts_with("# ") && !trimmed.starts_with("## ") {
            let title = trimmed.trim_start_matches('#').trim();
            if !title.is_empty() {
                return title.to_string();
            }
        }
    }
    
    "未命名页面".to_string()
}

/// 转义 XML 文本
fn escape_xml_text(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

// ============================================================================
// 以下为 PPTX 生成辅助函数
// ============================================================================

fn generate_content_types(slide_count: usize) -> String {
    let mut slide_parts = String::new();
    for i in 1..=slide_count {
        slide_parts.push_str(&format!(
            r#"<Override PartName="/ppt/slides/slide{}.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slide+xml"/>"#,
            i
        ));
    }

    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
<Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
<Default Extension="xml" ContentType="application/xml"/>
<Default Extension="png" ContentType="image/png"/>
<Default Extension="jpg" ContentType="image/jpeg"/>
<Default Extension="webp" ContentType="image/webp"/>
<Override PartName="/ppt/presentation.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.presentation.main+xml"/>
<Override PartName="/ppt/slideMasters/slideMaster1.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slideMaster+xml"/>
<Override PartName="/ppt/slideLayouts/slideLayout1.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slideLayout+xml"/>
<Override PartName="/ppt/theme/theme1.xml" ContentType="application/vnd.openxmlformats-officedocument.theme+xml"/>
{}
</Types>"#,
        slide_parts
    )
}

fn generate_rels() -> String {
    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="ppt/presentation.xml"/>
</Relationships>"#.to_string()
}

fn generate_presentation(slide_count: usize) -> String {
    let mut slide_ids = String::new();
    for i in 1..=slide_count {
        slide_ids.push_str(&format!(
            r#"<p:sldId id="{}" r:id="rId{}"/>"#,
            255 + i,
            i
        ));
    }

    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:presentation xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
<p:sldMasterIdLst>
<p:sldMasterId id="2147483648" r:id="rId1000"/>
</p:sldMasterIdLst>
<p:sldIdLst>
{}
</p:sldIdLst>
<p:sldSz cx="12192000" cy="6858000" type="screen16x9"/>
<p:notesSz cx="6858000" cy="9144000"/>
</p:presentation>"#,
        slide_ids
    )
}

fn generate_presentation_rels(slide_count: usize) -> String {
    let mut relationships = String::new();
    
    for i in 1..=slide_count {
        relationships.push_str(&format!(
            r#"<Relationship Id="rId{}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide" Target="slides/slide{}.xml"/>"#,
            i, i
        ));
    }
    
    relationships.push_str(
        r#"<Relationship Id="rId1000" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideMaster" Target="slideMasters/slideMaster1.xml"/>"#
    );

    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
{}
</Relationships>"#,
        relationships
    )
}

fn generate_slide_layout() -> String {
    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sldLayout xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main" type="blank">
<p:cSld>
<p:spTree>
<p:nvGrpSpPr>
<p:cNvPr id="1" name=""/>
<p:nvPr/>
</p:nvGrpSpPr>
<p:grpSpPr>
<a:xfrm>
<a:off x="0" y="0"/>
<a:ext cx="0" cy="0"/>
<a:chOff x="0" y="0"/>
<a:chExt cx="0" cy="0"/>
</a:xfrm>
</p:grpSpPr>
</p:spTree>
</p:cSld>
<p:clrMapOvr>
<a:masterClrMapping/>
</p:clrMapOvr>
</p:sldLayout>"#.to_string()
}

fn generate_slide_layout_rels() -> String {
    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideMaster" Target="../slideMasters/slideMaster1.xml"/>
</Relationships>"#.to_string()
}

fn generate_slide_master() -> String {
    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sldMaster xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
<p:cSld>
<p:bg>
<p:bgRef idx="1001">
<a:scrgbClr r="100%" g="100%" b="100%"/>
</p:bgRef>
</p:bg>
<p:spTree>
<p:nvGrpSpPr>
<p:cNvPr id="1" name=""/>
<p:nvPr/>
</p:nvGrpSpPr>
<p:grpSpPr>
<a:xfrm>
<a:off x="0" y="0"/>
<a:ext cx="0" cy="0"/>
<a:chOff x="0" y="0"/>
<a:chExt cx="0" cy="0"/>
</a:xfrm>
</p:grpSpPr>
</p:spTree>
</p:cSld>
<p:clrMap>
<a:clrMap bg1="lt1" tx1="dk1" bg2="lt2" tx2="dk2" accent1="accent1" accent2="accent2" accent3="accent3" accent4="accent4" accent5="accent5" accent6="accent6" hlink="hlink" folHlink="folHlink"/>
</p:clrMap>
</p:sldMaster>"#.to_string()
}

fn generate_slide_master_rels() -> String {
    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/theme" Target="../theme/theme1.xml"/>
</Relationships>"#.to_string()
}

fn generate_theme() -> String {
    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<a:theme xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" name="Office Theme">
<a:themeElements>
<a:clrScheme name="Office">
<a:dk1>
<a:sysClr val="windowText" lastClr="000000"/>
</a:dk1>
<a:lt1>
<a:sysClr val="window" lastClr="FFFFFF"/>
</a:lt1>
<a:dk2>
<a:srgbClr val="1F497D"/>
</a:dk2>
<a:lt2>
<a:srgbClr val="EEECE1"/>
</a:lt2>
<a:accent1>
<a:srgbClr val="4F81BD"/>
</a:accent1>
<a:accent2>
<a:srgbClr val="C0504D"/>
</a:accent2>
<a:accent3>
<a:srgbClr val="9BBB59"/>
</a:accent3>
<a:accent4>
<a:srgbClr val="8064A2"/>
</a:accent4>
<a:accent5>
<a:srgbClr val="4BACC6"/>
</a:accent5>
<a:accent6>
<a:srgbClr val="F79646"/>
</a:accent6>
<a:hlink>
<a:srgbClr val="0000FF"/>
</a:hlink>
<a:folHlink>
<a:srgbClr val="800080"/>
</a:folHlink>
</a:clrScheme>
<a:fontScheme name="Office">
<a:majorFont>
<a:latin typeface="Calibri"/>
<a:ea typeface=""/>
<a:cs typeface=""/>
</a:majorFont>
<a:minorFont>
<a:latin typeface="Calibri"/>
<a:ea typeface=""/>
<a:cs typeface=""/>
</a:minorFont>
</a:fontScheme>
<a:fmtScheme name="Office">
<a:fillStyleLst>
<a:solidFill>
<a:schemeClr val="phClr"/>
</a:solidFill>
</a:fillStyleLst>
<a:lnStyleLst>
<a:ln w="9525" cap="flat" cmpd="sng" algn="ctr">
<a:solidFill>
<a:schemeClr val="phClr"/>
</a:solidFill>
<a:prstDash val="solid"/>
</a:ln>
</a:lnStyleLst>
<a:effectStyleLst>
<a:effectStyle>
<a:effectLst/>
</a:effectStyle>
</a:effectStyleLst>
<a:bgFillStyleLst>
<a:solidFill>
<a:schemeClr val="phClr"/>
</a:solidFill>
</a:bgFillStyleLst>
</a:fmtScheme>
</a:themeElements>
</a:theme>"#.to_string()
}
