//! 基于模板 PPTX 的可编辑导出（增强校准版）
//!
//! 核心特性：
//! 1. 支持模板的三种布局：首页、内容页、尾页
//! 2. 自动读取模板的幻灯片尺寸，确保坐标转换正确
//! 3. 继承模板的默认字体、颜色设置
//! 4. 智能匹配模板的 slideLayout
//! 5. 图片智能切割，避免遮盖模板固定元素

use std::path::PathBuf;
use std::io::{Read, Write};
use std::sync::Arc;
use std::collections::HashMap;
use zip::{ZipArchive, ZipWriter, write::FileOptions};
use serde::{Deserialize, Serialize};
use tauri::{State, AppHandle, Emitter};
use crate::error_log;
use crate::ocr::{TextBoxLayout, NativeShape, NativeLine, TextLayoutManifest, PageData};
use super::editable::ExportProgress;
use image::{DynamicImage, GenericImageView};

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
    
    let _ = app.emit("export-progress", &progress);
}

/// 页面类型
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum PageType {
    FrontCover,
    Content,
    BackCover,
}

/// 模板布局信息
#[derive(Debug, Clone)]
struct TemplateLayout {
    layout_index: u32,
    page_type: PageType,
    /// 固定元素区域（页码、logo、保密提示等）
    fixed_regions: Vec<FixedRegion>,
}

/// 固定元素区域
#[derive(Debug, Clone)]
struct FixedRegion {
    /// 位置 X（EMU）
    x: i64,
    /// 位置 Y（EMU）
    y: i64,
    /// 宽度（EMU）
    width: i64,
    /// 高度（EMU）
    height: i64,
}

/// 图片内容区域
#[derive(Debug, Clone)]
struct ContentRegion {
    /// 区域 X（像素）
    x: u32,
    /// 区域 Y（像素）
    y: u32,
    /// 宽度（像素）
    width: u32,
    /// 高度（像素）
    height: u32,
}

/// 模板分析结果
#[derive(Debug, Clone)]
struct TemplateAnalysis {
    slide_width: i64,
    slide_height: i64,
    theme_colors: HashMap<String, String>,
    default_font: String,
    default_east_asian_font: String,
    layouts: Vec<TemplateLayout>,
}

/// 基于模板导出可编辑 PPTX
#[tauri::command]
pub async fn export_editable_pptx_from_template(
    app: AppHandle,
    cwd: State<'_, Arc<PathBuf>>,
    project_name: String,
    template_name: String,
) -> Result<String, String> {
    let cwd_path = cwd.inner().clone();
    let project_dir = cwd.join("projects").join(&project_name);
    let template_path = cwd.join("templates").join(&template_name).join("template.pptx");

    if !project_dir.exists() {
        return Err(format!("Project not found: {}", project_name));
    }

    if !template_path.exists() {
        return Err(format!("Template PPTX not found: {}", template_name));
    }

    emit_progress(&app, "init", 0, 0, &format!("正在分析模板: {}...", template_name));
    error_log::log_info(&cwd_path, &format!(
        "Exporting from template: {} for project: {}", template_name, project_name
    ));

    let pptx_dir = project_dir.join("pptx");
    if !pptx_dir.exists() {
        tokio::fs::create_dir_all(&pptx_dir).await
            .map_err(|e| format!("Failed to create pptx directory: {}", e))?;
    }

    // 检查是否存在 manifest 文件
    let manifest_path = pptx_dir.join("text_layout_manifest.json");
    if !manifest_path.exists() {
        // 自动执行 OCR 流程生成 manifest
        emit_progress(&app, "ocr", 0, 0, "正在执行 OCR 识别...");
        error_log::log_info(&cwd_path, "Manifest not found, running OCR pipeline...");
        
        run_ocr_pipeline(&app, &cwd_path, &project_dir, &pptx_dir)?;
    }

    let pages = collect_page_data(&project_dir)?;
    if pages.is_empty() {
        return Err("No pages found to export".to_string());
    }

    let total_pages = pages.len();
    emit_progress(&app, "init", 0, total_pages, &format!("找到 {} 页", total_pages));

    // 分析模板
    emit_progress(&app, "analyze", 0, total_pages, "正在分析模板结构...");
    let template_analysis = analyze_template(&template_path)?;
    
    error_log::log_info(&cwd_path, &format!(
        "Template: {}x{} EMU, {} layouts",
        template_analysis.slide_width, template_analysis.slide_height,
        template_analysis.layouts.len()
    ));

    // 读取布局清单
    emit_progress(&app, "load", 0, total_pages, "正在加载布局数据...");
    let manifest = load_manifest(&pptx_dir)?;
    let (all_shapes, all_lines) = load_shapes_and_lines(&pptx_dir, pages.len())?;

    let output_path = pptx_dir.join(format!("{}-from-template.pptx", project_name));

    emit_progress(&app, "generate", 0, total_pages, "正在生成 PPTX...");
    create_pptx_from_template(
        &template_path, &output_path, &pages,
        manifest.as_ref(), &all_shapes, &all_lines, &template_analysis,
        |stage, current, total, message| {
            emit_progress(&app, stage, current, total, message);
        },
    )?;

    let output_str = output_path.to_string_lossy().to_string();
    emit_progress(&app, "done", total_pages, total_pages, "导出完成！");
    error_log::log_info(&cwd_path, &format!("Exported: {}", output_str));
    Ok(output_str)
}

/// 执行 OCR 流程生成 manifest
fn run_ocr_pipeline(
    app: &AppHandle,
    cwd_path: &PathBuf,
    project_dir: &PathBuf,
    pptx_dir: &PathBuf,
) -> Result<(), String> {
    use crate::ocr::{
        self, TextMaskOptions, LayoutRecoveryOptions, 
        CalibrationOptions, OcrPageResult,
    };

    // 检测 OCR 引擎
    let engine_name = ocr::detect_available_engine();
    if engine_name.is_none() {
        return Err("OCR engine not available. Please install OCR engine first.".to_string());
    }

    // 收集页面数据
    let pages = collect_page_data(project_dir)?;
    if pages.is_empty() {
        return Err("No pages found".to_string());
    }

    let total_pages = pages.len();
    let mut manifests: Vec<crate::ocr::TextLayoutManifest> = Vec::new();
    let mut reports: Vec<crate::ocr::EditabilityReport> = Vec::new();

    for (idx, page) in pages.iter().enumerate() {
        emit_progress(app, "ocr", idx + 1, total_pages, &format!("正在处理第 {} 页...", idx + 1));

        // 创建页面目录
        let page_dir = pptx_dir.join(format!("page_{:02}", page.page_num));
        if !page_dir.exists() {
            std::fs::create_dir_all(&page_dir)
                .map_err(|e| format!("Failed to create page directory: {}", e))?;
        }

        // 执行 OCR
        let ocr_cache_path = page_dir.join("ocr_results.json");
        let ocr_result: OcrPageResult = if ocr_cache_path.exists() {
            let content = std::fs::read_to_string(&ocr_cache_path)
                .map_err(|e| format!("Failed to read OCR cache: {}", e))?;
            serde_json::from_str(&content)
                .map_err(|e| format!("Failed to parse OCR cache: {}", e))?
        } else {
            ocr::run_ocr(&page.image_path, &page_dir)?
        };

        // 去除文字
        emit_progress(app, "mask", idx + 1, total_pages, &format!("第 {} 页：去除文字...", idx + 1));
        let clean_bg_path = page_dir.join(format!("page_{:02}_clean.png", page.page_num));
        let clean_background = if clean_bg_path.exists() {
            clean_bg_path.clone()
        } else {
            let mask_options = TextMaskOptions::default();
            let mask_result = ocr::remove_text_from_image(
                &page.image_path,
                &ocr_result.records,
                &mask_options,
            )?;
            let (bg_path, _) = ocr::save_text_mask_results(&mask_result, &page_dir, page.page_num)?;
            bg_path
        };

        // 生成布局
        emit_progress(app, "layout", idx + 1, total_pages, &format!("第 {} 页：生成布局...", idx + 1));
        let layout_options = LayoutRecoveryOptions::default();
        let (mut manifest, report) = ocr::generate_text_layout(
            &ocr_result,
            &page.image_path,
            Some(&clean_background),
            &layout_options,
        )?;

        // 校准文本框
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
            ocr::apply_calibration(&mut slide.text_boxes, &calibration);
        }

        // 保存单页 manifest
        let manifest_path = page_dir.join("text_layout_manifest.json");
        ocr::save_text_layout_manifest(&manifest, &manifest_path)?;

        manifests.push(manifest);
        reports.push(report);
    }

    // 合并并保存
    emit_progress(app, "merge", total_pages, total_pages, "正在合并结果...");
    let (merged_manifest, merged_report) = ocr::merge_text_layout_manifests(manifests, reports);

    let manifest_content = serde_json::to_string_pretty(&merged_manifest)
        .map_err(|e| format!("Failed to serialize manifest: {}", e))?;
    std::fs::write(pptx_dir.join("text_layout_manifest.json"), manifest_content)
        .map_err(|e| format!("Failed to write manifest: {}", e))?;

    let report_content = serde_json::to_string_pretty(&merged_report)
        .map_err(|e| format!("Failed to serialize report: {}", e))?;
    std::fs::write(pptx_dir.join("editability_report.json"), report_content)
        .map_err(|e| format!("Failed to write report: {}", e))?;

    // 检测形状和线条（带缓存）
    emit_progress(app, "shapes", total_pages, total_pages, "正在检测形状和线条...");
    
    // 检查是否有缓存的形状数据
    let shapes_cache_path = pptx_dir.join("all_shapes.json");
    let lines_cache_path = pptx_dir.join("all_lines.json");
    
    let (all_shapes, all_lines) = if shapes_cache_path.exists() && lines_cache_path.exists() {
        // 读取缓存
        let shapes_content = std::fs::read_to_string(&shapes_cache_path)
            .map_err(|e| format!("Failed to read shapes cache: {}", e))?;
        let lines_content = std::fs::read_to_string(&lines_cache_path)
            .map_err(|e| format!("Failed to read lines cache: {}", e))?;
        
        let shapes: Vec<Vec<ocr::NativeShape>> = serde_json::from_str(&shapes_content)
            .unwrap_or_else(|_| vec![Vec::new(); total_pages]);
        let lines: Vec<Vec<ocr::NativeLine>> = serde_json::from_str(&lines_content)
            .unwrap_or_else(|_| vec![Vec::new(); total_pages]);
        
        (shapes, lines)
    } else {
        // 执行形状检测
        let mut all_shapes: Vec<Vec<ocr::NativeShape>> = Vec::new();
        let mut all_lines: Vec<Vec<ocr::NativeLine>> = Vec::new();
        
        let shape_options = ocr::ShapeDetectionOptions::default();
        
        for (idx, page) in pages.iter().enumerate() {
            emit_progress(app, "shapes", idx + 1, total_pages, &format!("第 {} 页：检测形状...", idx + 1));
            
            // 使用页面目录作为输出目录，避免覆盖
            let page_dir = pptx_dir.join(format!("page_{:02}", page.page_num));
            
            match ocr::detect_native_shapes(&page.image_path, &page_dir, &shape_options) {
                Ok(result) => {
                    all_shapes.push(result.shapes);
                    all_lines.push(result.lines);
                }
                Err(e) => {
                    error_log::log_warning(cwd_path, &format!("Shape detection failed for page {}: {}", page.page_num, e));
                    all_shapes.push(Vec::new());
                    all_lines.push(Vec::new());
                }
            }
        }
        
        // 保存形状和线条数据到缓存
        let shapes_content = serde_json::to_string_pretty(&all_shapes)
            .map_err(|e| format!("Failed to serialize shapes: {}", e))?;
        std::fs::write(&shapes_cache_path, &shapes_content)
            .map_err(|e| format!("Failed to write shapes: {}", e))?;

        let lines_content = serde_json::to_string_pretty(&all_lines)
            .map_err(|e| format!("Failed to serialize lines: {}", e))?;
        std::fs::write(&lines_cache_path, &lines_content)
            .map_err(|e| format!("Failed to write lines: {}", e))?;
        
        (all_shapes, all_lines)
    };
    
    // 统计形状和线条数量
    let total_shapes = all_shapes.iter().map(|s| s.len()).sum::<usize>();
    let total_lines = all_lines.iter().map(|l| l.len()).sum::<usize>();
    
    if total_shapes == 0 && total_lines == 0 {
        error_log::log_warning(cwd_path, "No shapes or lines detected in any page");
    }
    
    error_log::log_info(cwd_path, &format!(
        "OCR completed: {} text boxes, {} shapes, {} lines extracted", 
        merged_report.editable_text_bodies,
        total_shapes,
        total_lines
    ));

    Ok(())
}

/// 分析模板 PPTX
fn analyze_template(template_path: &PathBuf) -> Result<TemplateAnalysis, String> {
    let file = std::fs::File::open(template_path)
        .map_err(|e| format!("Failed to open template: {}", e))?;
    let mut archive = ZipArchive::new(file)
        .map_err(|e| format!("Failed to read template as ZIP: {}", e))?;

    let (slide_width, slide_height) = read_slide_dimensions(&mut archive)?;
    let theme_colors = read_theme_colors(&mut archive)?;
    let (default_font, default_east_asian_font) = read_default_fonts(&mut archive)?;
    let layouts = analyze_layouts(&mut archive)?;

    Ok(TemplateAnalysis {
        slide_width, slide_height, theme_colors,
        default_font, default_east_asian_font, layouts,
    })
}

/// 读取幻灯片尺寸
fn read_slide_dimensions(archive: &mut ZipArchive<std::fs::File>) -> Result<(i64, i64), String> {
    let xml = read_zip_text(archive, "ppt/presentation.xml")?;
    let mut width = 12192000i64;
    let mut height = 6858000i64;

    if let Ok(doc) = roxmltree::Document::parse(&xml) {
        for node in doc.descendants() {
            if node.has_tag_name("p:sldSz") {
                if let Some(cx) = node.attribute("cx") { width = cx.parse().unwrap_or(width); }
                if let Some(cy) = node.attribute("cy") { height = cy.parse().unwrap_or(height); }
                break;
            }
        }
    }
    Ok((width, height))
}

/// 读取主题颜色
fn read_theme_colors(archive: &mut ZipArchive<std::fs::File>) -> Result<HashMap<String, String>, String> {
    let mut colors = HashMap::new();
    let xml = match read_zip_text(archive, "ppt/theme/theme1.xml") { Ok(x) => x, Err(_) => return Ok(colors) };

    if let Ok(doc) = roxmltree::Document::parse(&xml) {
        for node in doc.descendants() {
            if node.has_tag_name("a:clrScheme") {
                for color_node in node.children() {
                    let tag = color_node.tag_name().name();
                    let name = match tag {
                        "a:dk1" => "dk1", "a:lt1" => "lt1", "a:dk2" => "dk2", "a:lt2" => "lt2",
                        "a:accent1" => "accent1", "a:accent2" => "accent2", "a:accent3" => "accent3",
                        "a:accent4" => "accent4", "a:accent5" => "accent5", "a:accent6" => "accent6",
                        "a:hlink" => "hlink", "a:folHlink" => "folHlink", _ => continue,
                    };
                    for child in color_node.children() {
                        if child.has_tag_name("a:srgbClr") {
                            if let Some(val) = child.attribute("val") {
                                colors.insert(name.to_string(), val.to_uppercase());
                            }
                        } else if child.has_tag_name("a:sysClr") {
                            if let Some(last) = child.attribute("lastClr") {
                                colors.insert(name.to_string(), last.to_uppercase());
                            }
                        }
                    }
                }
                break;
            }
        }
    }
    Ok(colors)
}

/// 读取默认字体
fn read_default_fonts(archive: &mut ZipArchive<std::fs::File>) -> Result<(String, String), String> {
    let mut default_font = "Aptos Display".to_string();
    let mut east_asian = "Microsoft YaHei".to_string();
    let xml = match read_zip_text(archive, "ppt/theme/theme1.xml") { Ok(x) => x, Err(_) => return Ok((default_font, east_asian)) };

    if let Ok(doc) = roxmltree::Document::parse(&xml) {
        for node in doc.descendants() {
            if node.has_tag_name("a:fontScheme") {
                for child in node.descendants() {
                    if child.has_tag_name("a:latin") {
                        if let Some(t) = child.attribute("typeface") { default_font = t.to_string(); }
                    }
                    if child.has_tag_name("a:ea") {
                        if let Some(t) = child.attribute("typeface") { east_asian = t.to_string(); }
                    }
                }
                break;
            }
        }
    }
    Ok((default_font, east_asian))
}

/// 分析布局
fn analyze_layouts(archive: &mut ZipArchive<std::fs::File>) -> Result<Vec<TemplateLayout>, String> {
    let mut layouts_by_index: HashMap<u32, TemplateLayout> = HashMap::new();
    
    // 默认布局映射（按常见的 PowerPoint 模板结构）
    // slideLayout1 通常是标题页/封面
    // slideLayout2 通常是内容页
    // slideLayout3 通常是节标题或封底
    layouts_by_index.insert(1, TemplateLayout { 
        layout_index: 1, 
        page_type: PageType::FrontCover, 
        fixed_regions: Vec::new() 
    });
    layouts_by_index.insert(2, TemplateLayout { 
        layout_index: 2, 
        page_type: PageType::Content, 
        fixed_regions: Vec::new() 
    });
    layouts_by_index.insert(3, TemplateLayout { 
        layout_index: 3, 
        page_type: PageType::BackCover, 
        fixed_regions: Vec::new() 
    });

    // 尝试从模板中读取实际的布局信息
    for i in 1..=10 {
        let path = format!("ppt/slideLayouts/slideLayout{}.xml", i);
        if let Ok(xml) = read_zip_text(archive, &path) {
            if let Ok(doc) = roxmltree::Document::parse(&xml) {
                let mut page_type = PageType::Content;
                let mut fixed_regions = Vec::new();
                
                // 解析布局名称和类型
                for node in doc.descendants() {
                    // 从 cNvPr 读取布局名称
                    if node.has_tag_name("p:cNvPr") {
                        if let Some(name) = node.attribute("name") {
                            // 根据名称推断页面类型（支持中英文）
                            let name_lower = name.to_lowercase();
                            page_type = if name_lower.contains("title") || 
                                          name_lower.contains("cover") || 
                                          name_lower.contains("front") ||
                                          name_lower.contains("封面") ||
                                          name_lower.contains("首页") {
                                PageType::FrontCover
                            } else if name_lower.contains("back") || 
                                      name_lower.contains("end") || 
                                      name_lower.contains("closing") ||
                                      name_lower.contains("section") ||
                                      name_lower.contains("封底") ||
                                      name_lower.contains("尾页") ||
                                      name_lower.contains("结束") {
                                PageType::BackCover
                            } else {
                                PageType::Content
                            };
                        }
                    }
                    
                    // 解析固定元素（ph 类型为 body 以外的占位符）
                    if node.has_tag_name("p:sp") || node.has_tag_name("p:pic") {
                        if let Some(region) = parse_fixed_element(&node) {
                            fixed_regions.push(region);
                        }
                    }
                }
                
                layouts_by_index.insert(i, TemplateLayout {
                    layout_index: i,
                    page_type,
                    fixed_regions,
                });
            }
        }
    }
    
    // 转换为有序列表
    let mut layouts: Vec<TemplateLayout> = layouts_by_index.into_values().collect();
    layouts.sort_by_key(|l| l.layout_index);
    
    // 确保至少有三种布局类型
    let has_front = layouts.iter().any(|l| l.page_type == PageType::FrontCover);
    let has_content = layouts.iter().any(|l| l.page_type == PageType::Content);
    let has_back = layouts.iter().any(|l| l.page_type == PageType::BackCover);
    
    // 如果缺少必要类型，使用索引映射
    if !has_front && layouts.len() >= 1 {
        layouts[0].page_type = PageType::FrontCover;
    }
    if !has_content && layouts.len() >= 2 {
        layouts[1].page_type = PageType::Content;
    }
    if !has_back && layouts.len() >= 3 {
        layouts[2].page_type = PageType::BackCover;
    }
    
    Ok(layouts)
}

/// 解析固定元素
fn parse_fixed_element(node: &roxmltree::Node) -> Option<FixedRegion> {
    // 检查是否为占位符
    let ph_type = node.descendants()
        .find(|n| n.has_tag_name("p:ph"))
        .and_then(|n| n.attribute("type"));
    
    // 只处理非 body 占位符（如 sldNum, dt, ftr, hdr 等）
    let is_fixed = match ph_type {
        Some("sldNum") | Some("ftr") | Some("hdr") | Some("dt") => true,
        Some("body") | Some("title") | Some("ctrTitle") | Some("subTitle") => false,
        _ => {
            // 检查是否为图片（可能是 logo）
            node.has_tag_name("p:pic")
        }
    };
    
    if !is_fixed {
        return None;
    }
    
    // 获取位置和大小
    let (x, y, width, height) = node.descendants()
        .find(|n| n.has_tag_name("a:xfrm"))
        .map(|xfrm| {
            let off = xfrm.descendants().find(|n| n.has_tag_name("a:off"));
            let ext = xfrm.descendants().find(|n| n.has_tag_name("a:ext"));
            
            let x = off.as_ref().and_then(|o| o.attribute("x")).and_then(|v| v.parse().ok()).unwrap_or(0);
            let y = off.as_ref().and_then(|o| o.attribute("y")).and_then(|v| v.parse().ok()).unwrap_or(0);
            let w = ext.as_ref().and_then(|e| e.attribute("cx")).and_then(|v| v.parse().ok()).unwrap_or(0);
            let h = ext.as_ref().and_then(|e| e.attribute("cy")).and_then(|v| v.parse().ok()).unwrap_or(0);
            
            (x, y, w, h)
        })
        .unwrap_or((0, 0, 0, 0));
    
    // 过滤掉无效区域
    if width < 10000 || height < 10000 { // 小于约 0.1 英寸
        return None;
    }
    
    Some(FixedRegion {
        x,
        y,
        width,
        height,
    })
}

/// 加载布局清单
fn load_manifest(pptx_dir: &PathBuf) -> Result<Option<TextLayoutManifest>, String> {
    let path = pptx_dir.join("text_layout_manifest.json");
    if !path.exists() { return Ok(None); }
    let content = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    Ok(serde_json::from_str(&content).ok())
}

/// 加载形状和线条
fn load_shapes_and_lines(pptx_dir: &PathBuf, page_count: usize) -> Result<(Vec<Vec<NativeShape>>, Vec<Vec<NativeLine>>), String> {
    let shapes_path = pptx_dir.join("all_shapes.json");
    let shapes: Vec<Vec<NativeShape>> = if shapes_path.exists() {
        let content = std::fs::read_to_string(&shapes_path).map_err(|e| e.to_string())?;
        serde_json::from_str(&content).unwrap_or_else(|_| vec![Vec::new(); page_count])
    } else {
        vec![Vec::new(); page_count]
    };

    let lines_path = pptx_dir.join("all_lines.json");
    let lines: Vec<Vec<NativeLine>> = if lines_path.exists() {
        let content = std::fs::read_to_string(&lines_path).map_err(|e| e.to_string())?;
        serde_json::from_str(&content).unwrap_or_else(|_| vec![Vec::new(); page_count])
    } else {
        vec![Vec::new(); page_count]
    };

    Ok((shapes, lines))
}

/// 从模板创建 PPTX
fn create_pptx_from_template<F>(
    template_path: &PathBuf,
    output_path: &PathBuf,
    pages: &[PageData],
    manifest: Option<&TextLayoutManifest>,
    all_shapes: &[Vec<NativeShape>],
    all_lines: &[Vec<NativeLine>],
    template_analysis: &TemplateAnalysis,
    mut emit_progress: F,
) -> Result<(), String>
where
    F: FnMut(&str, usize, usize, &str),
{
    let template_file = std::fs::File::open(template_path)
        .map_err(|e| format!("Failed to open template: {}", e))?;
    let mut template_archive = ZipArchive::new(template_file)
        .map_err(|e| format!("Failed to read template: {}", e))?;

    let output_file = std::fs::File::create(output_path)
        .map_err(|e| format!("Failed to create output: {}", e))?;
    let mut output_zip = ZipWriter::new(output_file);
    let options = FileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    // 复制模板结构
    emit_progress("copy", 0, pages.len(), "正在复制模板结构...");
    copy_template_structure(&mut template_archive, &mut output_zip)?;

    let slide_count = pages.len();
    let total_pages = pages.len();

    for (idx, page) in pages.iter().enumerate() {
        let slide_num = idx + 1;
        let progress_msg = format!("正在处理第 {} / {} 页...", slide_num, total_pages);
        emit_progress("slide", slide_num, total_pages, &progress_msg);
        
        // 智能判断页面类型：结合位置和标题关键词
        let page_type = determine_page_type(slide_num, total_pages, &page.title);
        
        // 根据页面类型获取布局索引
        let layout_index = get_layout_for_page_type(page_type, &template_analysis.layouts);
        
        // 获取该布局的固定区域（模板中的页码、logo等）
        let fixed_regions = get_fixed_regions(page_type, &template_analysis.layouts);
        
        // 获取文本框（文字区域）
        let all_text_boxes = manifest.and_then(|m| m.slides.get(idx))
            .map(|s| s.text_boxes.as_slice()).unwrap_or(&[]);
        let filtered_text_boxes = filter_text_boxes(
            all_text_boxes,
            &fixed_regions,
            template_analysis.slide_width,
            template_analysis.slide_height,
        );
        
        // 使用干净背景图片（已去除文字的背景）
        let background_path = manifest
            .and_then(|m| m.slides.get(idx))
            .and_then(|s| s.background.as_ref())
            .map(|p| PathBuf::from(p))
            .filter(|p| p.exists())
            .unwrap_or_else(|| page.image_path.clone());
        
        let (img_w, img_h) = get_image_dimensions(&background_path)?;
        
        // 切割干净背景图片，基于内容检测分成多个片段
        emit_progress("analyze", slide_num, total_pages, "正在切割背景图片...");
        let sliced_images = write_sliced_images(
            &mut output_zip,
            &background_path,
            slide_num,
            &options,
            &fixed_regions,           // 模板固定区域
            &filtered_text_boxes,     // 文字区域
            template_analysis.slide_width,
            template_analysis.slide_height,
            img_w,
            img_h,
        )?;
        
        let shapes = all_shapes.get(idx).map(|s| s.as_slice()).unwrap_or(&[]);
        let lines = all_lines.get(idx).map(|l| l.as_slice()).unwrap_or(&[]);

        // 使用切割后的图片生成幻灯片（先放图片，再放文字）
        let slide_xml = generate_slide_xml_with_sliced_images(
            slide_num, &filtered_text_boxes, shapes, lines, &sliced_images,
            img_w, img_h, template_analysis, layout_index,
        );

        let slide_path = format!("ppt/slides/slide{}.xml", slide_num);
        output_zip.start_file(&slide_path, options.clone()).map_err(|e| e.to_string())?;
        output_zip.write_all(slide_xml.as_bytes()).map_err(|e| e.to_string())?;

        // 生成包含多个图片关系的关系文件
        let slide_rels = generate_slide_rels_for_sliced_images(slide_num, layout_index, &sliced_images);
        let rels_path = format!("ppt/slides/_rels/slide{}.xml.rels", slide_num);
        output_zip.start_file(&rels_path, options.clone()).map_err(|e| e.to_string())?;
        output_zip.write_all(slide_rels.as_bytes()).map_err(|e| e.to_string())?;
    }

    // 写入 presentation.xml
    emit_progress("finalize", slide_count, slide_count, "正在写入演示文稿结构...");
    let presentation = generate_presentation_xml(slide_count, template_analysis);
    output_zip.start_file("ppt/presentation.xml", options.clone()).map_err(|e| e.to_string())?;
    output_zip.write_all(presentation.as_bytes()).map_err(|e| e.to_string())?;

    let pres_rels = generate_presentation_rels(slide_count);
    output_zip.start_file("ppt/_rels/presentation.xml.rels", options.clone()).map_err(|e| e.to_string())?;
    output_zip.write_all(pres_rels.as_bytes()).map_err(|e| e.to_string())?;

    let content_types = generate_content_types(slide_count);
    output_zip.start_file("[Content_Types].xml", options.clone()).map_err(|e| e.to_string())?;
    output_zip.write_all(content_types.as_bytes()).map_err(|e| e.to_string())?;

    copy_file_from_archive(&mut template_archive, &mut output_zip, "_rels/.rels")?;

    output_zip.finish().map_err(|e| e.to_string())?;
    Ok(())
}

/// 智能判断页面类型：结合位置和标题关键词
fn determine_page_type(slide_num: usize, total_pages: usize, title: &str) -> PageType {
    // 位置判断：第一页和最后一页
    if slide_num == 1 {
        return PageType::FrontCover;
    }
    if slide_num == total_pages {
        return PageType::BackCover;
    }
    
    // 标题关键词判断（支持中英文）
    let title_lower = title.to_lowercase();
    
    // 封面/首页关键词
    let front_keywords = ["封面", "首页", "cover", "front", "title", "封面页"];
    for kw in &front_keywords {
        if title_lower.contains(kw) {
            return PageType::FrontCover;
        }
    }
    
    // 封底/尾页/结束关键词
    let back_keywords = ["封底", "尾页", "结束", "back", "end", "closing", "感谢", "谢谢", "thanks", "thank you", "q&a", "问答"];
    for kw in &back_keywords {
        if title_lower.contains(kw) {
            return PageType::BackCover;
        }
    }
    
    // 默认为内容页
    PageType::Content
}

fn get_layout_for_page_type(page_type: PageType, layouts: &[TemplateLayout]) -> u32 {
    layouts.iter().find(|l| l.page_type == page_type).map(|l| l.layout_index).unwrap_or(1)
}

/// 获取指定布局的固定区域
fn get_fixed_regions(page_type: PageType, layouts: &[TemplateLayout]) -> Vec<FixedRegion> {
    layouts.iter()
        .find(|l| l.page_type == page_type)
        .map(|l| l.fixed_regions.clone())
        .unwrap_or_default()
}

/// 检查文本框是否与固定区域重叠
fn overlaps_with_fixed_region(tb: &TextBoxLayout, fixed_regions: &[FixedRegion], _slide_width: i64, _slide_height: i64) -> bool {
    // 转换文本框坐标到 EMU
    let tb_x = (tb.x * 914400.0) as i64;
    let tb_y = (tb.y * 914400.0) as i64;
    let tb_w = (tb.w * 914400.0) as i64;
    let tb_h = (tb.h * 914400.0) as i64;
    
    for region in fixed_regions {
        // 检查是否有重叠（允许 10% 的容差）
        let tolerance = 50000i64; // 约 0.05 英寸
        
        let overlaps_x = tb_x < region.x + region.width + tolerance && tb_x + tb_w > region.x - tolerance;
        let overlaps_y = tb_y < region.y + region.height + tolerance && tb_y + tb_h > region.y - tolerance;
        
        if overlaps_x && overlaps_y {
            // 检查重叠面积比例
            let overlap_x = (tb_x + tb_w).min(region.x + region.width) - tb_x.max(region.x);
            let overlap_y = (tb_y + tb_h).min(region.y + region.height) - tb_y.max(region.y);
            let overlap_area = overlap_x.max(0) * overlap_y.max(0);
            let tb_area = tb_w * tb_h;
            
            // 如果重叠超过 30%，则认为冲突
            if tb_area > 0 && overlap_area * 100 / tb_area > 30 {
                return true;
            }
        }
    }
    
    false
}

/// 过滤掉与固定区域重叠的文本框
fn filter_text_boxes(
    text_boxes: &[TextBoxLayout],
    fixed_regions: &[FixedRegion],
    slide_width: i64,
    slide_height: i64,
) -> Vec<TextBoxLayout> {
    text_boxes
        .iter()
        .filter(|tb| !overlaps_with_fixed_region(tb, fixed_regions, slide_width, slide_height))
        .cloned()
        .collect()
}

/// 复制模板结构
fn copy_template_structure(template: &mut ZipArchive<std::fs::File>, output: &mut ZipWriter<std::fs::File>) -> Result<(), String> {
    let options = FileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    let paths = ["ppt/theme/", "ppt/slideMasters/", "ppt/slideLayouts/", "ppt/notesMasters/", "ppt/handoutMasters/"];

    for i in 0..template.len() {
        let name = template.by_index(i).map_err(|e| e.to_string())?.name().to_string();
        let should_copy = paths.iter().any(|p| name.starts_with(p));
        if should_copy {
            let mut content = Vec::new();
            let mut file = template.by_index(i).map_err(|e| e.to_string())?;
            file.read_to_end(&mut content).map_err(|e| e.to_string())?;
            output.start_file(&name, options.clone()).map_err(|e| e.to_string())?;
            output.write_all(&content).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

fn get_image_dimensions(image_path: &PathBuf) -> Result<(u32, u32), String> {
    let img = image::open(image_path).map_err(|e| format!("Failed to open image: {}", e))?;
    Ok((img.width(), img.height()))
}

/// 分析图片并检测内容区域
/// 
/// 策略：使用颜色变化检测内容区域边界
/// 1. 扫描图片寻找颜色变化明显的区域
/// 2. 将相邻的高变化区域合并
/// 3. 如果没有检测到明显边界，返回整张图片
fn analyze_image_content(image_path: &PathBuf) -> Result<Vec<ContentRegion>, String> {
    let img = image::open(image_path)
        .map_err(|e| format!("Failed to open image: {}", e))?;
    
    let (width, height) = (img.width(), img.height());
    
    // 使用更精细的网格
    let grid_size = 30u32; // 30x30 像素的网格
    let cols = (width + grid_size - 1) / grid_size;
    let rows = (height + grid_size - 1) / grid_size;
    
    // 计算每个网格的内容分数
    let mut grid_scores: Vec<Vec<f32>> = vec![vec![0.0; cols as usize]; rows as usize];
    
    for row in 0..rows {
        for col in 0..cols {
            let x1 = col * grid_size;
            let y1 = row * grid_size;
            let x2 = ((col + 1) * grid_size).min(width);
            let y2 = ((row + 1) * grid_size).min(height);
            
            let score = calculate_region_content_score(&img, x1, y1, x2, y2);
            grid_scores[row as usize][col as usize] = score;
        }
    }
    
    // 降低阈值，更容易检测到内容
    let threshold = 0.15f32;
    
    // 合并相邻的高分网格为区域
    let mut visited = vec![vec![false; cols as usize]; rows as usize];
    let mut regions = Vec::new();
    
    for row in 0..rows {
        for col in 0..cols {
            if visited[row as usize][col as usize] {
                continue;
            }
            
            if grid_scores[row as usize][col as usize] > threshold {
                // 找到内容区域的起始点，使用洪水填充扩展
                if let Some(region) = flood_fill_region(&grid_scores, &mut visited, col, row, cols, rows, grid_size, width, height, threshold) {
                    regions.push(region);
                }
            } else {
                visited[row as usize][col as usize] = true;
            }
        }
    }
    
    // 过滤太小的区域（至少占图片 0.5%）
    let min_area = (width * height) / 200;
    regions.retain(|r| r.width * r.height >= min_area);
    
    // 如果没有检测到明显区域，或者只检测到一个接近整图的区域，返回整张图片
    if regions.is_empty() {
        regions.push(ContentRegion {
            x: 0,
            y: 0,
            width,
            height,
        });
    } else if regions.len() == 1 {
        let r = &regions[0];
        // 如果检测到的区域接近整张图片（>90%），则不切割
        let area_ratio = (r.width * r.height) as f64 / (width * height) as f64;
        if area_ratio > 0.9 {
            regions[0] = ContentRegion {
                x: 0,
                y: 0,
                width,
                height,
            };
        }
    }
    
    Ok(regions)
}

/// 计算区域的内容分数（边缘密度和非均匀性）
fn calculate_region_content_score(img: &DynamicImage, x1: u32, y1: u32, x2: u32, y2: u32) -> f32 {
    let mut edge_count = 0u32;
    let mut total_pixels = 0u32;
    let mut color_variance = 0.0f32;
    let mut avg_color = [0.0f32; 3];
    
    // 先计算平均颜色
    let mut pixel_count = 0u32;
    for y in y1..y2 {
        for x in x1..x2 {
            let pixel = img.get_pixel(x, y);
            avg_color[0] += pixel.0[0] as f32;
            avg_color[1] += pixel.0[1] as f32;
            avg_color[2] += pixel.0[2] as f32;
            pixel_count += 1;
        }
    }
    
    if pixel_count == 0 {
        return 0.0;
    }
    
    avg_color[0] /= pixel_count as f32;
    avg_color[1] /= pixel_count as f32;
    avg_color[2] /= pixel_count as f32;
    
    // 计算颜色方差和边缘
    for y in y1..y2 {
        for x in x1..x2 {
            let pixel = img.get_pixel(x, y);
            
            // 计算与平均颜色的差异
            let diff = ((pixel.0[0] as f32 - avg_color[0]).powi(2) +
                       (pixel.0[1] as f32 - avg_color[1]).powi(2) +
                       (pixel.0[2] as f32 - avg_color[2]).powi(2)).sqrt();
            color_variance += diff;
            
            // 简单的边缘检测（与右边和下边像素的差异）
            if x + 1 < x2 {
                let right = img.get_pixel(x + 1, y);
                let edge_diff = ((pixel.0[0] as i32 - right.0[0] as i32).abs() +
                                (pixel.0[1] as i32 - right.0[1] as i32).abs() +
                                (pixel.0[2] as i32 - right.0[2] as i32).abs()) as f32;
                if edge_diff > 30.0 {
                    edge_count += 1;
                }
            }
            if y + 1 < y2 {
                let below = img.get_pixel(x, y + 1);
                let edge_diff = ((pixel.0[0] as i32 - below.0[0] as i32).abs() +
                                (pixel.0[1] as i32 - below.0[1] as i32).abs() +
                                (pixel.0[2] as i32 - below.0[2] as i32).abs()) as f32;
                if edge_diff > 30.0 {
                    edge_count += 1;
                }
            }
            
            total_pixels += 1;
        }
    }
    
    if total_pixels == 0 {
        return 0.0;
    }
    
    let edge_ratio = edge_count as f32 / total_pixels as f32;
    let avg_variance = color_variance / total_pixels as f32;
    
    // 综合评分：边缘密度和颜色变化
    (edge_ratio * 2.0 + avg_variance / 50.0).min(1.0)
}

/// 使用洪水填充算法扩展区域
fn flood_fill_region(
    grid_scores: &[Vec<f32>],
    visited: &mut [Vec<bool>],
    start_col: u32,
    start_row: u32,
    cols: u32,
    rows: u32,
    grid_size: u32,
    img_width: u32,
    img_height: u32,
    threshold: f32,
) -> Option<ContentRegion> {
    let mut min_col = start_col;
    let mut max_col = start_col;
    let mut min_row = start_row;
    let mut max_row = start_row;
    
    let mut stack = vec![(start_col, start_row)];
    
    while let Some((col, row)) = stack.pop() {
        if col >= cols || row >= rows {
            continue;
        }
        if visited[row as usize][col as usize] {
            continue;
        }
        if grid_scores[row as usize][col as usize] < threshold {
            visited[row as usize][col as usize] = true;
            continue;
        }
        
        visited[row as usize][col as usize] = true;
        
        min_col = min_col.min(col);
        max_col = max_col.max(col);
        min_row = min_row.min(row);
        max_row = max_row.max(row);
        
        // 添加相邻格子
        if col > 0 { stack.push((col - 1, row)); }
        if col + 1 < cols { stack.push((col + 1, row)); }
        if row > 0 { stack.push((col, row - 1)); }
        if row + 1 < rows { stack.push((col, row + 1)); }
    }
    
    // 计算实际像素坐标
    let x = min_col * grid_size;
    let y = min_row * grid_size;
    let width = ((max_col - min_col + 1) * grid_size).min(img_width - x);
    let height = ((max_row - min_row + 1) * grid_size).min(img_height - y);
    
    // 过滤太小的区域
    if width < grid_size * 2 || height < grid_size * 2 {
        return None;
    }
    
    Some(ContentRegion {
        x,
        y,
        width,
        height,
    })
}

/// 写入切割后的背景图片到 PPTX
/// 
/// 策略：基于内容检测将背景图片切割成多个片段
/// 好处：用户可以单独选中、移动、替换某个图片片段
/// 
/// 优化：
/// - 避开模板固定区域（页码、logo等）
/// - 避开文本框区域（文字内容区域）
fn write_sliced_images(
    output: &mut ZipWriter<std::fs::File>,
    image_path: &PathBuf,
    slide_num: usize,
    options: &FileOptions,
    fixed_regions: &[FixedRegion],       // 模板固定区域
    text_boxes: &[TextBoxLayout],        // 文字区域
    slide_width: i64,
    slide_height: i64,
    img_width: u32,
    img_height: u32,
) -> Result<Vec<(String, i64, i64, i64, i64)>, String> {
    let mut img = image::open(image_path)
        .map_err(|e| format!("Failed to open image: {}", e))?;
    
    // 检测内容区域
    let mut regions = analyze_image_content(image_path)?;
    
    // 计算像素到 EMU 的转换比例
    let x_scale = slide_width as f64 / img_width as f64;
    let y_scale = slide_height as f64 / img_height as f64;
    
    // 将固定区域和文本框转换为像素坐标，用于过滤切割区域
    let exclude_regions = build_exclude_regions(fixed_regions, text_boxes, x_scale, y_scale);
    
    // 过滤掉与排除区域重叠过多的切割区域
    if !exclude_regions.is_empty() {
        regions = filter_regions_by_exclusions(&regions, &exclude_regions, img_width, img_height);
    }
    
    let mut result: Vec<(String, i64, i64, i64, i64)> = Vec::new();
    
    // 如果没有有效区域或只有一个覆盖整图的区域，直接使用整张图片
    if regions.is_empty() || (regions.len() == 1 && {
        let r = &regions[0];
        r.x == 0 && r.y == 0 && r.width == img_width && r.height == img_height
    }) {
        let image_id = slide_num;
        let media_name = format!("ppt/media/image{}.png", image_id);
        output.start_file(&media_name, options.clone())
            .map_err(|e| format!("Failed to create media: {}", e))?;
        
        let mut img_bytes: Vec<u8> = Vec::new();
        img.to_rgba8().write_to(&mut std::io::Cursor::new(&mut img_bytes), image::ImageFormat::Png)
            .map_err(|e| format!("Failed to encode image: {}", e))?;
        output.write_all(&img_bytes)
            .map_err(|e| format!("Failed to write image: {}", e))?;
        
        return Ok(vec![("rId1".to_string(), 0, 0, slide_width, slide_height)]);
    }
    
    // 切割图片为多个片段
    for (idx, region) in regions.iter().enumerate() {
        // 裁剪图片片段
        let cropped = img.crop(region.x, region.y, region.width, region.height);
        
        // 计算图片 ID：slide_num * 100 + idx + 1
        // 例如：第 1 页的第 2 个片段 → image102
        let image_id = slide_num * 100 + idx + 1;
        let media_name = format!("ppt/media/image{}.png", image_id);
        
        output.start_file(&media_name, options.clone())
            .map_err(|e| format!("Failed to create media: {}", e))?;
        
        let mut img_bytes: Vec<u8> = Vec::new();
        cropped.to_rgba8().write_to(&mut std::io::Cursor::new(&mut img_bytes), image::ImageFormat::Png)
            .map_err(|e| format!("Failed to encode image: {}", e))?;
        output.write_all(&img_bytes)
            .map_err(|e| format!("Failed to write image: {}", e))?;
        
        // 计算该片段在幻灯片中的位置和大小（EMU）
        let x_emu = (region.x as f64 * x_scale) as i64;
        let y_emu = (region.y as f64 * y_scale) as i64;
        let w_emu = (region.width as f64 * x_scale) as i64;
        let h_emu = (region.height as f64 * y_scale) as i64;
        
        // rId 从 1 开始
        let rid = format!("rId{}", idx + 1);
        result.push((rid, x_emu, y_emu, w_emu, h_emu));
    }
    
    Ok(result)
}

/// 构建排除区域（固定区域 + 文本框区域，转换为像素坐标）
fn build_exclude_regions(
    fixed_regions: &[FixedRegion],
    text_boxes: &[TextBoxLayout],
    x_scale: f64,
    y_scale: f64,
) -> Vec<(u32, u32, u32, u32)> {
    let mut exclude_regions = Vec::new();
    
    // 添加固定区域（EMU -> 像素）
    for region in fixed_regions {
        let x = (region.x as f64 / x_scale) as u32;
        let y = (region.y as f64 / y_scale) as u32;
        let w = (region.width as f64 / x_scale) as u32;
        let h = (region.height as f64 / y_scale) as u32;
        exclude_regions.push((x, y, w, h));
    }
    
    // 添加文本框区域（英寸 -> 像素）
    for tb in text_boxes {
        let x = (tb.x * 914400.0 / x_scale) as u32;
        let y = (tb.y * 914400.0 / y_scale) as u32;
        let w = (tb.w * 914400.0 / x_scale) as u32;
        let h = (tb.h * 914400.0 / y_scale) as u32;
        exclude_regions.push((x, y, w, h));
    }
    
    exclude_regions
}

/// 过滤掉与排除区域重叠过多的切割区域
fn filter_regions_by_exclusions(
    regions: &[ContentRegion],
    exclude_regions: &[(u32, u32, u32, u32)],
    _img_width: u32,
    _img_height: u32,
) -> Vec<ContentRegion> {
    regions.iter().filter(|region| {
        // 检查该区域是否与任何排除区域重叠过多
        for &(ex_x, ex_y, ex_w, ex_h) in exclude_regions {
            // 计算重叠面积
            let overlap_x = (region.x + region.width).min(ex_x + ex_w).saturating_sub(region.x.max(ex_x));
            let overlap_y = (region.y + region.height).min(ex_y + ex_h).saturating_sub(region.y.max(ex_y));
            let overlap_area = overlap_x as u64 * overlap_y as u64;
            let region_area = region.width as u64 * region.height as u64;
            
            // 如果重叠超过 50%，则跳过该区域
            if region_area > 0 && overlap_area * 100 / region_area > 50 {
                return false;
            }
        }
        true
    }).cloned().collect()
}

fn read_zip_text(archive: &mut ZipArchive<std::fs::File>, path: &str) -> Result<String, String> {
    let mut file = archive.by_name(path).map_err(|_| format!("File not found: {}", path))?;
    let mut content = String::new();
    file.read_to_string(&mut content).map_err(|e| e.to_string())?;
    Ok(content)
}

fn copy_file_from_archive(template: &mut ZipArchive<std::fs::File>, output: &mut ZipWriter<std::fs::File>, path: &str) -> Result<(), String> {
    let options = FileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    let mut content = Vec::new();
    let mut file = template.by_name(path).map_err(|_| format!("File not found: {}", path))?;
    file.read_to_end(&mut content).map_err(|e| e.to_string())?;
    output.start_file(path, options).map_err(|e| e.to_string())?;
    output.write_all(&content).map_err(|e| e.to_string())?;
    Ok(())
}

/// 生成幻灯片 XML（使用切割后的多张图片）
fn generate_slide_xml_with_sliced_images(
    slide_num: usize, text_boxes: &[TextBoxLayout], shapes: &[NativeShape], lines: &[NativeLine],
    sliced_images: &[(String, i64, i64, i64, i64)],
    img_width: u32, img_height: u32,
    template_analysis: &TemplateAnalysis, _layout_index: u32,
) -> String {
    let mut shapes_xml = String::new();
    let x_scale = template_analysis.slide_width as f64 / img_width as f64;
    let y_scale = template_analysis.slide_height as f64 / img_height as f64;

    // 添加所有切割后的图片（id 从 2 开始，因为 id=1 被 nvGrpSpPr 占用）
    for (idx, (rid, x_emu, y_emu, w_emu, h_emu)) in sliced_images.iter().enumerate() {
        let shape_id = idx + 2; // id=1 被 nvGrpSpPr 占用
        shapes_xml.push_str(&format!(
            r#"<p:pic>
<p:nvPicPr>
<p:cNvPr id="{}" name="Image {}"/>
<p:cNvPicPr><a:picLocks noChangeAspect="1"/></p:cNvPicPr>
<p:nvPr/>
</p:nvPicPr>
<p:blipFill>
<a:blip r:embed="{}"/>
<a:stretch>
<a:fillRect/>
</a:stretch>
</p:blipFill>
<p:spPr>
<a:xfrm>
<a:off x="{}" y="{}"/>
<a:ext cx="{}" cy="{}"/>
</a:xfrm>
<a:prstGeom prst="rect">
<a:avLst/>
</a:prstGeom>
</p:spPr>
</p:pic>"#,
            shape_id, idx + 1, rid, x_emu, y_emu, w_emu, h_emu
        ));
    }
    
    // 计算下一个可用的 shape id
    let mut next_shape_id = 2 + sliced_images.len();
    
    // 添加形状
    for shape in shapes {
        shapes_xml.push_str(&generate_shape_xml_with_id(shape, x_scale, y_scale, next_shape_id));
        next_shape_id += 1;
    }
    for line in lines {
        shapes_xml.push_str(&generate_line_xml_with_id(line, x_scale, y_scale, next_shape_id));
        next_shape_id += 1;
    }
    
    // 添加文本框
    for tb in text_boxes.iter() {
        shapes_xml.push_str(&generate_textbox_xml_with_id(tb, slide_num, next_shape_id, template_analysis));
        next_shape_id += 1;
    }

    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
<p:cSld>
<p:spTree>
<p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:nvPr/></p:nvGrpSpPr>
<p:grpSpPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="0" cy="0"/><a:chOff x="0" y="0"/><a:chExt cx="0" cy="0"/></a:xfrm></p:grpSpPr>
{}
</p:spTree>
</p:cSld>
<p:clrMapOvr><a:masterClrMapping/></p:clrMapOvr>
</p:sld>"#,
        shapes_xml
    )
}

fn generate_shape_xml_with_id(shape: &NativeShape, x_scale: f64, y_scale: f64, shape_id: usize) -> String {
    let x_emu = (shape.x as f64 * x_scale) as i64;
    let y_emu = (shape.y as f64 * y_scale) as i64;
    let w_emu = (shape.width as f64 * x_scale) as i64;
    let h_emu = (shape.height as f64 * y_scale) as i64;
    let fill = shape.style.fill_color.as_deref().unwrap_or("FFFFFF");
    let stroke = shape.style.stroke_color.as_deref().unwrap_or("D9DEE8");
    let stroke_w = (shape.style.stroke_width.unwrap_or(1.0) * 12700.0) as i64;
    let geom = match shape.shape_type {
        crate::ocr::ShapeType::Circle | crate::ocr::ShapeType::Badge => "ellipse",
        crate::ocr::ShapeType::RoundedRectangle | crate::ocr::ShapeType::Card => "roundRect",
        _ => "rect",
    };

    format!(r#"<p:sp><p:nvSpPr><p:cNvPr id="{}" name="{}"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr><a:xfrm><a:off x="{}" y="{}"/><a:ext cx="{}" cy="{}"/></a:xfrm><a:prstGeom prst="{}"><a:avLst/></a:prstGeom><a:solidFill><a:srgbClr val="{}"/></a:solidFill><a:ln w="{}"><a:solidFill><a:srgbClr val="{}"/></a:solidFill></a:ln></p:spPr></p:sp>"#,
        shape_id, shape.shape_type.as_str(), x_emu, y_emu, w_emu, h_emu, geom, fill, stroke_w, stroke)
}

fn generate_line_xml_with_id(line: &NativeLine, x_scale: f64, y_scale: f64, shape_id: usize) -> String {
    let x1 = (line.start.0 as f64 * x_scale) as i64;
    let y1 = (line.start.1 as f64 * y_scale) as i64;
    let x2 = (line.end.0 as f64 * x_scale) as i64;
    let y2 = (line.end.1 as f64 * y_scale) as i64;
    let dash = match line.style.line_type.as_str() {
        "solid" => "solid",
        "dashed" => "dash",
        "dotted" => "dot",
        _ => "solid",
    };

    format!(r#"<p:cxnSp><p:nvCxnSpPr><p:cNvPr id="{}" name="Line"/><p:nvPr/></p:nvCxnSpPr><p:spPr><a:xfrm><a:off x="{}" y="{}"/><a:ext cx="{}" cy="{}"/></a:xfrm><a:prstGeom prst="line"><a:avLst/></a:prstGeom><a:noFill/><a:ln w="12700"><a:solidFill><a:srgbClr val="888888"/></a:solidFill><a:prstDash val="{}"/></a:ln></p:spPr></p:cxnSp>"#,
        shape_id, x1, y1, (x2-x1).abs(), (y2-y1).abs(), dash)
}

fn generate_textbox_xml_with_id(tb: &TextBoxLayout, slide_num: usize, shape_id: usize, template: &TemplateAnalysis) -> String {
    let x_emu = (tb.x * 914400.0) as i64;
    let y_emu = (tb.y * 914400.0) as i64;
    let w_emu = (tb.w * 914400.0) as i64;
    let h_emu = (tb.h * 914400.0) as i64;
    let font_size = tb.font_size.unwrap_or(12.0) as i64 * 100;

    // 使用模板颜色
    let color = if let Some(ref c) = tb.color {
        c.trim_start_matches('#').to_string()
    } else if let Some(dk1) = template.theme_colors.get("dk1") {
        dk1.clone()
    } else { "353535".into() };

    let bold = if tb.bold { "1" } else { "0" };
    let align = match tb.align.as_deref() { Some("center") => "ctr", Some("right") => "r", _ => "l" };
    let font = tb.font_face.as_deref().unwrap_or(&template.default_font);
    let ea_font = tb.east_asian_font.as_deref().unwrap_or(&template.default_east_asian_font);

    let mut paras = String::new();
    for para in tb.text.split('\n') {
        paras.push_str(&format!(r#"<a:p><a:pPr algn="{}"><a:defRPr/></a:pPr><a:r><a:rPr lang="zh-CN" b="{}" sz="{}"><a:solidFill><a:srgbClr val="{}"/></a:solidFill><a:latin typeface="{}"/><a:ea typeface="{}"/></a:rPr><a:t>{}</a:t></a:r></a:p>"#,
            align, bold, font_size, color, font, ea_font, escape_xml(para)));
    }

    format!(r#"<p:sp><p:nvSpPr><p:cNvPr id="{}" name="TextBox {}"/><p:cNvSpPr txBox="1"/><p:nvPr/></p:nvSpPr><p:spPr><a:xfrm><a:off x="{}" y="{}"/><a:ext cx="{}" cy="{}"/></a:xfrm><a:prstGeom prst="rect"><a:avLst/></a:prstGeom><a:noFill/></p:spPr><p:txBody><a:bodyPr wrap="square" anchor="t"><a:noAutofit/></a:bodyPr><a:lstStyle/>{}</p:txBody></p:sp>"#,
        shape_id, slide_num, x_emu, y_emu, w_emu, h_emu, paras)
}

/// 生成幻灯片关系文件（支持多张切割图片）
fn generate_slide_rels_for_sliced_images(slide_num: usize, layout_index: u32, sliced_images: &[(String, i64, i64, i64, i64)]) -> String {
    let mut rels = String::new();
    
    // 判断是单张图片还是多张切割图片
    if sliced_images.len() == 1 && sliced_images[0].2 == 0 && sliced_images[0].3 == 0 {
        // 单张图片（整张背景，位置为 0,0），图片 ID 就是 slide_num
        let (rid, _, _, _, _) = &sliced_images[0];
        rels.push_str(&format!(
            r#"<Relationship Id="{}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="../media/image{}.png"/>"#,
            rid, slide_num
        ));
    } else {
        // 多张切割图片，图片 ID 格式：slide_num * 100 + idx + 1
        for (idx, (rid, _, _, _, _)) in sliced_images.iter().enumerate() {
            let image_num = slide_num * 100 + idx + 1;
            rels.push_str(&format!(
                r#"<Relationship Id="{}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="../media/image{}.png"/>"#,
                rid, image_num
            ));
        }
    }
    
    // 添加布局关系
    let layout_rid = format!("rId{}", sliced_images.len() + 1);
    rels.push_str(&format!(
        r#"<Relationship Id="{}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideLayout" Target="../slideLayouts/slideLayout{}.xml"/>"#,
        layout_rid, layout_index
    ));
    
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">{}</Relationships>"#,
        rels
    )
}

fn generate_presentation_xml(slide_count: usize, template: &TemplateAnalysis) -> String {
    let mut ids = String::new();
    for i in 1..=slide_count { ids.push_str(&format!(r#"<p:sldId id="{}" r:id="rId{}"/>"#, 255+i, i)); }
    format!(r#"<?xml version="1.0" encoding="UTF-8"?><p:presentation xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"><p:sldMasterIdLst><p:sldMasterId id="2147483648" r:id="rId1000"/></p:sldMasterIdLst><p:sldIdLst>{}</p:sldIdLst><p:sldSz cx="{}" cy="{}" type="screen16x9"/><p:notesSz cx="6858000" cy="9144000"/></p:presentation>"#,
        ids, template.slide_width, template.slide_height)
}

fn generate_presentation_rels(slide_count: usize) -> String {
    let mut rels = String::new();
    for i in 1..=slide_count { rels.push_str(&format!(r#"<Relationship Id="rId{}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide" Target="slides/slide{}.xml"/>"#, i, i)); }
    rels.push_str(r#"<Relationship Id="rId1000" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideMaster" Target="slideMasters/slideMaster1.xml"/>"#);
    format!(r#"<?xml version="1.0" encoding="UTF-8"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">{}</Relationships>"#, rels)
}

fn generate_content_types(slide_count: usize) -> String {
    let mut parts = String::new();
    for i in 1..=slide_count { parts.push_str(&format!(r#"<Override PartName="/ppt/slides/slide{}.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slide+xml"/>"#, i)); }
    
    // 添加所有可能的 slideLayout 声明
    let mut layout_parts = String::new();
    for i in 1..=10 {
        layout_parts.push_str(&format!(r#"<Override PartName="/ppt/slideLayouts/slideLayout{}.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slideLayout+xml"/>"#, i));
    }
    
    format!(r#"<?xml version="1.0" encoding="UTF-8"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Default Extension="png" ContentType="image/png"/><Default Extension="jpg" ContentType="image/jpeg"/><Override PartName="/ppt/presentation.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.presentation.main+xml"/><Override PartName="/ppt/slideMasters/slideMaster1.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slideMaster+xml"/>{}<Override PartName="/ppt/theme/theme1.xml" ContentType="application/vnd.openxmlformats-officedocument.theme+xml"/>{}</Types>"#, layout_parts, parts)
}

fn escape_xml(text: &str) -> String {
    text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;").replace('\'', "&apos;")
}

/// 收集页面数据
fn collect_page_data(project_dir: &PathBuf) -> Result<Vec<PageData>, String> {
    let mut pages = Vec::new();
    let entries = std::fs::read_dir(project_dir).map_err(|e| e.to_string())?;
    let mut files: Vec<(u32, PathBuf, Option<PathBuf>)> = Vec::new();

    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with("page-") && name.ends_with(".md") {
            if let Ok(num) = name.trim_start_matches("page-").trim_end_matches(".md").parse::<u32>() {
                let md = entry.path();
                let png = project_dir.join(format!("page-{:02}.png", num));
                let jpg = project_dir.join(format!("page-{:02}.jpg", num));
                let webp = project_dir.join(format!("page-{:02}.webp", num));
                let img = if png.exists() { Some(png) } else if jpg.exists() { Some(jpg) } else if webp.exists() { Some(webp) } else { None };
                files.push((num, md, img));
            }
        }
    }
    files.sort_by_key(|(n, _, _)| *n);
    for (num, md, img) in files {
        if let Some(image) = img {
            let markdown = std::fs::read_to_string(&md).unwrap_or_default();
            pages.push(PageData { page_num: num, title: extract_title(&markdown), markdown, image_path: image });
        }
    }
    Ok(pages)
}

fn extract_title(markdown: &str) -> String {
    for line in markdown.lines() {
        let t = line.trim();
        if t.starts_with("## 第") && t.contains("页") {
            let title = t.trim_start_matches('#').trim().trim_start_matches(|c: char| c.is_numeric()).trim_start_matches(|c| c == '页' || c == '：' || c == ':').trim();
            if !title.is_empty() { return title.into(); }
        }
        if t.starts_with("**标题**") {
            let title = t.trim_start_matches(|c| c == '*' || c == ':').trim_start_matches(':').trim();
            if !title.is_empty() { return title.into(); }
        }
        if t.starts_with("# ") && !t.starts_with("## ") {
            let title = t.trim_start_matches('#').trim();
            if !title.is_empty() { return title.into(); }
        }
    }
    "未命名页面".into()
}
