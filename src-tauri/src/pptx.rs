use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::Read;
use zip::ZipArchive;
use std::fs::File;

/// PPTX配色方案
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColorScheme {
    /// 主题名称
    pub name: String,
    /// 配色列表：颜色名称 -> HEX颜色代码
    pub colors: HashMap<String, String>,
}

/// 从PPTX文件提取配色方案
pub fn extract_color_scheme(pptx_path: &std::path::Path) -> Result<ColorScheme, String> {
    let file = File::open(pptx_path)
        .map_err(|e| format!("无法打开PPTX文件: {}", e))?;
    
    let mut archive = ZipArchive::new(file)
        .map_err(|e| format!("无法解压PPTX文件: {}", e))?;
    
    // 读取主题文件
    let theme_content = read_zip_text(&mut archive, "ppt/theme/theme1.xml")?;
    
    // 解析XML提取颜色
    let doc = roxmltree::Document::parse(&theme_content)
        .map_err(|e| format!("解析主题XML失败: {}", e))?;
    
    let mut colors = HashMap::new();
    let mut scheme_name = String::from("Unknown");
    
    // 查找配色方案
    // XML结构：<a:clrScheme name="...">
    //   <a:dk1><a:sysClr val="windowText" lastClr="000000"/></a:dk1>
    //   <a:lt1><a:sysClr val="window" lastClr="FFFFFF"/></a:lt1>
    //   <a:accent1><a:srgbClr val="4472C4"/></a:accent1>
    // </a:clrScheme>
    
    for node in doc.descendants() {
        // 查找配色方案节点
        if node.has_tag_name("a:clrScheme") {
            // 获取方案名称
            if let Some(name) = node.attribute("name") {
                scheme_name = name.to_string();
            }
            
            // 遍历子节点提取颜色
            for color_node in node.children() {
                let tag_name = color_node.tag_name().name();
                
                // 颜色名称映射
                let color_name = match tag_name {
                    "a:dk1" => "深色1",
                    "a:lt1" => "浅色1",
                    "a:dk2" => "深色2",
                    "a:lt2" => "浅色2",
                    "a:accent1" => "强调色1",
                    "a:accent2" => "强调色2",
                    "a:accent3" => "强调色3",
                    "a:accent4" => "强调色4",
                    "a:accent5" => "强调色5",
                    "a:accent6" => "强调色6",
                    "a:hlink" => "超链接",
                    "a:folHlink" => "访问过的链接",
                    _ => continue,
                };
                
                // 提取颜色值
                if let Some(hex_color) = extract_color_value(&color_node) {
                    colors.insert(color_name.to_string(), hex_color);
                }
            }
            
            break;
        }
    }
    
    Ok(ColorScheme {
        name: scheme_name,
        colors,
    })
}

/// 从颜色节点提取颜色值
fn extract_color_value(node: &roxmltree::Node) -> Option<String> {
    for child in node.children() {
        let tag = child.tag_name().name();
        
        match tag {
            // srgbClr: 直接RGB值，如 <a:srgbClr val="4472C4"/>
            "a:srgbClr" => {
                if let Some(val) = child.attribute("val") {
                    return Some(format!("#{}", val.to_uppercase()));
                }
            }
            // sysClr: 系统颜色，如 <a:sysClr val="windowText" lastClr="000000"/>
            "a:sysClr" => {
                // 优先使用 lastClr 属性
                if let Some(last_clr) = child.attribute("lastClr") {
                    return Some(format!("#{}", last_clr.to_uppercase()));
                }
                // 如果没有 lastClr，使用 val 属性映射常见系统颜色
                if let Some(val) = child.attribute("val") {
                    let hex = system_color_to_hex(val);
                    return Some(hex);
                }
            }
            _ => {}
        }
    }
    None
}

/// 将系统颜色名称转换为HEX值
fn system_color_to_hex(name: &str) -> String {
    match name {
        "windowText" | "WindowText" => "#000000".to_string(),
        "window" | "Window" => "#FFFFFF".to_string(),
        _ => "#000000".to_string(), // 默认黑色
    }
}

/// 读取ZIP文件中的文本内容
fn read_zip_text(archive: &mut ZipArchive<File>, path: &str) -> Result<String, String> {
    let mut file = archive.by_name(path)
        .map_err(|e| format!("找不到文件 {}: {}", path, e))?;
    
    let mut content = String::new();
    file.read_to_string(&mut content)
        .map_err(|e| format!("读取文件 {} 失败: {}", path, e))?;
    
    Ok(content)
}

/// 提取PPTX的幻灯片数量
pub fn count_slides(pptx_path: &std::path::Path) -> Result<usize, String> {
    let file = File::open(pptx_path)
        .map_err(|e| format!("无法打开PPTX文件: {}", e))?;
    
    let mut archive = ZipArchive::new(file)
        .map_err(|e| format!("无法解压PPTX文件: {}", e))?;
    
    // 统计 ppt/slides/slide*.xml 文件数量
    let mut count = 0;
    for i in 0..archive.len() {
        if let Ok(file) = archive.by_index(i) {
            let name = file.name();
            if name.starts_with("ppt/slides/slide") && name.ends_with(".xml") {
                count += 1;
            }
        }
    }
    
    Ok(count)
}

/// 提取PPTX的幻灯片布局信息
pub fn extract_slide_layouts(pptx_path: &std::path::Path) -> Result<Vec<String>, String> {
    let file = File::open(pptx_path)
        .map_err(|e| format!("无法打开PPTX文件: {}", e))?;
    
    let mut archive = ZipArchive::new(file)
        .map_err(|e| format!("无法解压PPTX文件: {}", e))?;
    
    let mut layouts = Vec::new();
    
    // 读取每个幻灯片
    let slide_count = count_slides(pptx_path)?;
    
    for i in 1..=slide_count {
        let slide_path = format!("ppt/slides/slide{}.xml", i);
        
        if let Ok(content) = read_zip_text(&mut archive, &slide_path) {
            // 解析幻灯片内容，提取形状类型
            if let Ok(doc) = roxmltree::Document::parse(&content) {
                let mut shape_types = Vec::new();
                
                for node in doc.descendants() {
                    if node.has_tag_name("p:sp") || node.has_tag_name("p:pic") {
                        // 提取形状类型
                        if node.children().any(|n| n.has_tag_name("p:nvSpPr")) {
                            shape_types.push("shape");
                        }
                        if node.has_tag_name("p:pic") {
                            shape_types.push("image");
                        }
                    }
                }
                
                // 简单判断布局类型
                let layout_type = if shape_types.iter().any(|t| *t == "image") {
                    "图文混排"
                } else if shape_types.len() > 5 {
                    "多元素布局"
                } else if shape_types.len() > 2 {
                    "中等布局"
                } else {
                    "简洁布局"
                };
                
                layouts.push(layout_type.to_string());
            }
        }
    }
    
    Ok(layouts)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_system_color_mapping() {
        assert_eq!(system_color_to_hex("windowText"), "#000000");
        assert_eq!(system_color_to_hex("window"), "#FFFFFF");
    }
}
