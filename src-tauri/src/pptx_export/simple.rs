//! 简单 PPTX 导出
//!
//! 将图片按顺序组装为一个 PPT，每页一张图片

use std::path::PathBuf;
use std::io::Write;
use zip::ZipWriter;
use zip::write::FileOptions;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::State;
use crate::error_log;

/// 简单导出选项
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct SimpleExportOptions {
    /// 幻灯片标题
    pub title: Option<String>,
}

/// 导出简单 PPTX（仅图片）
#[tauri::command]
pub async fn export_simple_pptx(
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

    // 创建 pptx 临时目录
    let pptx_dir = project_dir.join("pptx");
    if !pptx_dir.exists() {
        tokio::fs::create_dir_all(&pptx_dir)
            .await
            .map_err(|e| format!("Failed to create pptx directory: {}", e))?;
    }

    // 收集页面图片
    let pages = collect_page_images(&project_dir)?;
    
    if pages.is_empty() {
        let error = "No pages found to export. Please generate images first.";
        error_log::log_error(&cwd_path, error);
        return Err(error.to_string());
    }

    error_log::log_info(&cwd_path, &format!("Exporting simple PPTX for project: {} with {} pages", project_name, pages.len()));

    // 输出路径
    let output_path = pptx_dir.join(format!("{}.pptx", project_name));
    let output_path_str = output_path.to_string_lossy().to_string();

    // 生成 PPTX
    match create_simple_pptx(&pages, &output_path) {
        Ok(_) => {
            error_log::log_info(&cwd_path, &format!("Simple PPTX exported successfully: {}", output_path_str));
            Ok(output_path_str)
        }
        Err(e) => {
            error_log::log_error(&cwd_path, &format!("Simple PPTX export failed: {}", e));
            Err(format!("PPTX export failed: {}", e))
        }
    }
}

/// 页面图片信息
#[allow(dead_code)]
struct PageImage {
    _num: u32,
    path: PathBuf,
    ext: String,
}

/// 收集页面图片
fn collect_page_images(project_dir: &PathBuf) -> Result<Vec<PageImage>, String> {
    let mut pages = Vec::new();
    let entries = std::fs::read_dir(project_dir)
        .map_err(|e| format!("Failed to read project directory: {}", e))?;

    let mut page_files: Vec<(u32, PathBuf, String)> = Vec::new();

    for entry in entries {
        let entry = entry.map_err(|e| e.to_string())?;
        let name = entry.file_name().to_string_lossy().to_string();
        
        // 检查是否是页面图片
        if name.starts_with("page-") {
            let parts: Vec<&str> = name.split('.').collect();
            if parts.len() == 2 {
                let num_str = parts[0].trim_start_matches("page-");
                if let Ok(num) = num_str.parse::<u32>() {
                    let ext = parts[1].to_lowercase();
                    if ["png", "jpg", "jpeg", "webp"].contains(&ext.as_str()) {
                        page_files.push((num, entry.path(), ext));
                    }
                }
            }
        }
    }

    // 排序
    page_files.sort_by_key(|(num, _, _)| *num);

    for (num, path, ext) in page_files {
        pages.push(PageImage { _num: num, path, ext });
    }

    Ok(pages)
}

/// 创建简单的 PPTX 文件
fn create_simple_pptx(pages: &[PageImage], output_path: &PathBuf) -> Result<(), String> {
    let file = std::fs::File::create(output_path)
        .map_err(|e| format!("Failed to create PPTX file: {}", e))?;
    
    let mut zip = ZipWriter::new(file);
    let options = FileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    // 1. [Content_Types].xml
    let content_types = generate_content_types(pages.len());
    zip.start_file("[Content_Types].xml", options)
        .map_err(|e| format!("Failed to write Content_Types: {}", e))?;
    zip.write_all(content_types.as_bytes())
        .map_err(|e| format!("Failed to write Content_Types: {}", e))?;

    // 2. _rels/.rels
    let rels = generate_rels();
    zip.start_file("_rels/.rels", options)
        .map_err(|e| format!("Failed to write rels: {}", e))?;
    zip.write_all(rels.as_bytes())
        .map_err(|e| format!("Failed to write rels: {}", e))?;

    // 3. ppt/presentation.xml
    let presentation = generate_presentation(pages.len());
    zip.start_file("ppt/presentation.xml", options)
        .map_err(|e| format!("Failed to write presentation: {}", e))?;
    zip.write_all(presentation.as_bytes())
        .map_err(|e| format!("Failed to write presentation: {}", e))?;

    // 4. ppt/_rels/presentation.xml.rels
    let presentation_rels = generate_presentation_rels(pages.len());
    zip.start_file("ppt/_rels/presentation.xml.rels", options)
        .map_err(|e| format!("Failed to write presentation rels: {}", e))?;
    zip.write_all(presentation_rels.as_bytes())
        .map_err(|e| format!("Failed to write presentation rels: {}", e))?;

    // 5. ppt/slideLayouts/slideLayout1.xml
    let slide_layout = generate_slide_layout();
    zip.start_file("ppt/slideLayouts/slideLayout1.xml", options)
        .map_err(|e| format!("Failed to write slide layout: {}", e))?;
    zip.write_all(slide_layout.as_bytes())
        .map_err(|e| format!("Failed to write slide layout: {}", e))?;

    // 5.1 ppt/slideLayouts/_rels/slideLayout1.xml.rels
    let slide_layout_rels = generate_slide_layout_rels();
    zip.start_file("ppt/slideLayouts/_rels/slideLayout1.xml.rels", options)
        .map_err(|e| format!("Failed to write slide layout rels: {}", e))?;
    zip.write_all(slide_layout_rels.as_bytes())
        .map_err(|e| format!("Failed to write slide layout rels: {}", e))?;

    // 6. ppt/slideMasters/slideMaster1.xml
    let slide_master = generate_slide_master();
    zip.start_file("ppt/slideMasters/slideMaster1.xml", options)
        .map_err(|e| format!("Failed to write slide master: {}", e))?;
    zip.write_all(slide_master.as_bytes())
        .map_err(|e| format!("Failed to write slide master: {}", e))?;

    // 6.1 ppt/slideMasters/_rels/slideMaster1.xml.rels
    let slide_master_rels = generate_slide_master_rels();
    zip.start_file("ppt/slideMasters/_rels/slideMaster1.xml.rels", options)
        .map_err(|e| format!("Failed to write slide master rels: {}", e))?;
    zip.write_all(slide_master_rels.as_bytes())
        .map_err(|e| format!("Failed to write slide master rels: {}", e))?;

    // 6.2 ppt/theme/theme1.xml
    let theme = generate_theme();
    zip.start_file("ppt/theme/theme1.xml", options)
        .map_err(|e| format!("Failed to write theme: {}", e))?;
    zip.write_all(theme.as_bytes())
        .map_err(|e| format!("Failed to write theme: {}", e))?;

    // 7. 为每个页面创建幻灯片
    for (index, page) in pages.iter().enumerate() {
        let slide_num = index + 1;
        
        // 读取并添加图片
        let img_data = std::fs::read(&page.path)
            .map_err(|e| format!("Failed to read image {}: {}", page.path.display(), e))?;
        
        let media_name = format!("ppt/media/image{}.{}", slide_num, page.ext);
        zip.start_file(&media_name, options)
            .map_err(|e| format!("Failed to write image: {}", e))?;
        zip.write_all(&img_data)
            .map_err(|e| format!("Failed to write image: {}", e))?;

        // 创建幻灯片 XML
        let slide_xml = generate_slide_with_image(slide_num, &page.ext);
        let slide_path = format!("ppt/slides/slide{}.xml", slide_num);
        zip.start_file(&slide_path, options)
            .map_err(|e| format!("Failed to write slide: {}", e))?;
        zip.write_all(slide_xml.as_bytes())
            .map_err(|e| format!("Failed to write slide: {}", e))?;

        // 创建幻灯片关系文件
        let slide_rels = generate_slide_rels(slide_num, &page.ext);
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

/// 生成包含图片的幻灯片
fn generate_slide_with_image(slide_num: usize, _ext: &str) -> String {
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
<p:pic>
<p:nvPicPr>
<p:cNvPr id="2" name="Image {}"/>
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
<a:ext cx="12192000" cy="6858000"/>
</a:xfrm>
<a:prstGeom prst="rect">
<a:avLst/>
</a:prstGeom>
</p:spPr>
</p:pic>
</p:spTree>
</p:cSld>
<p:clrMapOvr>
<a:masterClrMapping/>
</p:clrMapOvr>
</p:sld>"#,
        slide_num
    )
}

/// 生成幻灯片关系文件
fn generate_slide_rels(slide_num: usize, ext: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="../media/image{}.{}"/>
<Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideLayout" Target="../slideLayouts/slideLayout1.xml"/>
</Relationships>"#,
        slide_num, ext
    )
}

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
    
    relationships.push_str(&format!(
        r#"<Relationship Id="rId1000" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideMaster" Target="slideMasters/slideMaster1.xml"/>"#
    ));

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

/// 生成幻灯片布局关系文件
fn generate_slide_layout_rels() -> String {
    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideMaster" Target="../slideMasters/slideMaster1.xml"/>
</Relationships>"#.to_string()
}

/// 生成幻灯片母版关系文件
fn generate_slide_master_rels() -> String {
    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/theme" Target="../theme/theme1.xml"/>
</Relationships>"#.to_string()
}

/// 生成主题文件
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
<a:gradFill rotWithShape="1">
<a:gsLst>
<a:gs pos="0">
<a:schemeClr val="phClr">
<a:tint val="50000"/>
<a:satMod val="300000"/>
</a:schemeClr>
</a:gs>
<a:gs pos="35000">
<a:schemeClr val="phClr">
<a:tint val="37000"/>
<a:satMod val="300000"/>
</a:schemeClr>
</a:gs>
<a:gs pos="100000">
<a:schemeClr val="phClr">
<a:tint val="15000"/>
<a:satMod val="350000"/>
</a:schemeClr>
</a:gs>
</a:gsLst>
<a:lin ang="16200000" scaled="1"/>
</a:gradFill>
<a:gradFill rotWithShape="1">
<a:gsLst>
<a:gs pos="0">
<a:schemeClr val="phClr">
<a:shade val="51000"/>
<a:satMod val="130000"/>
</a:schemeClr>
</a:gs>
<a:gs pos="80000">
<a:schemeClr val="phClr">
<a:shade val="93000"/>
<a:satMod val="130000"/>
</a:schemeClr>
</a:gs>
<a:gs pos="100000">
<a:schemeClr val="phClr">
<a:shade val="94000"/>
<a:satMod val="135000"/>
</a:schemeClr>
</a:gs>
</a:gsLst>
<a:lin ang="16200000" scaled="0"/>
</a:gradFill>
</a:fillStyleLst>
<a:lnStyleLst>
<a:ln w="9525" cap="flat" cmpd="sng" algn="ctr">
<a:solidFill>
<a:schemeClr val="phClr">
<a:shade val="95000"/>
<a:satMod val="105000"/>
</a:schemeClr>
</a:solidFill>
<a:prstDash val="solid"/>
</a:ln>
<a:ln w="25400" cap="flat" cmpd="sng" algn="ctr">
<a:solidFill>
<a:schemeClr val="phClr"/>
</a:solidFill>
<a:prstDash val="solid"/>
</a:ln>
<a:ln w="38100" cap="flat" cmpd="sng" algn="ctr">
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
<a:effectStyle>
<a:effectLst/>
</a:effectStyle>
<a:effectStyle>
<a:effectLst>
<a:outerShdw blurRad="40000" dist="20000" dir="5400000" algn="ctr">
<a:srgbClr val="000000">
<a:alpha val="38000"/>
</a:srgbClr>
</a:outerShdw>
</a:effectLst>
</a:effectStyle>
</a:effectStyleLst>
<a:bgFillStyleLst>
<a:solidFill>
<a:schemeClr val="phClr"/>
</a:solidFill>
<a:gradFill rotWithShape="1">
<a:gsLst>
<a:gs pos="0">
<a:schemeClr val="phClr">
<a:tint val="40000"/>
<a:satMod val="350000"/>
</a:schemeClr>
</a:gs>
<a:gs pos="40000">
<a:schemeClr val="phClr">
<a:tint val="45000"/>
<a:shade val="99000"/>
<a:satMod val="350000"/>
</a:schemeClr>
</a:gs>
<a:gs pos="100000">
<a:schemeClr val="phClr">
<a:shade val="20000"/>
<a:satMod val="255000"/>
</a:schemeClr>
</a:gs>
</a:gsLst>
<a:lin ang="16200000" scaled="1"/>
</a:gradFill>
<a:gradFill rotWithShape="1">
<a:gsLst>
<a:gs pos="0">
<a:schemeClr val="phClr">
<a:tint val="80000"/>
<a:satMod val="300000"/>
</a:schemeClr>
</a:gs>
<a:gs pos="100000">
<a:schemeClr val="phClr">
<a:shade val="30000"/>
<a:satMod val="200000"/>
</a:schemeClr>
</a:gs>
</a:gsLst>
<a:lin ang="16200000" scaled="0"/>
</a:gradFill>
</a:bgFillStyleLst>
</a:fmtScheme>
</a:themeElements>
</a:theme>"#.to_string()
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
