use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tauri::State;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateInfo {
    pub name: String,
    pub path: String,
    pub front_cover_path: Option<String>,
    pub content_path: Option<String>,
    pub back_cover_path: Option<String>,
    pub pptx_path: Option<String>,
    pub has_front_cover: bool,
    pub has_content: bool,
    pub has_back_cover: bool,
    pub has_pptx: bool,
}

fn get_templates_dir(cwd: &PathBuf) -> PathBuf {
    cwd.join("templates")
}

#[tauri::command]
pub async fn list_templates(cwd: State<'_, Arc<PathBuf>>) -> Result<Vec<TemplateInfo>, String> {
    let templates_dir = get_templates_dir(&cwd);
    
    if !templates_dir.exists() {
        return Ok(Vec::new());
    }
    
    let mut entries = tokio::fs::read_dir(&templates_dir)
        .await
        .map_err(|e| format!("Failed to read templates directory: {}", e))?;
    
    let mut templates = Vec::new();
    
    while let Some(entry) = entries.next_entry().await.map_err(|e| e.to_string())? {
        if entry.file_type().await.map(|t| t.is_dir()).unwrap_or(false) {
            let name = entry.file_name().to_string_lossy().to_string();
            let path = entry.path();
            
            let front_cover_path = path.join("front-cover.png");
            let content_path = path.join("content.png");
            let back_cover_path = path.join("back-cover.png");
            let pptx_path = path.join("template.pptx");
            
            let has_front_cover = front_cover_path.exists();
            let has_content = content_path.exists();
            let has_back_cover = back_cover_path.exists();
            let has_pptx = pptx_path.exists();
            
            templates.push(TemplateInfo {
                name,
                path: path.to_string_lossy().to_string(),
                front_cover_path: if has_front_cover { Some(front_cover_path.to_string_lossy().to_string()) } else { None },
                content_path: if has_content { Some(content_path.to_string_lossy().to_string()) } else { None },
                back_cover_path: if has_back_cover { Some(back_cover_path.to_string_lossy().to_string()) } else { None },
                pptx_path: if has_pptx { Some(pptx_path.to_string_lossy().to_string()) } else { None },
                has_front_cover,
                has_content,
                has_back_cover,
                has_pptx,
            });
        }
    }
    
    Ok(templates)
}

#[tauri::command]
pub async fn get_template_info(
    cwd: State<'_, Arc<PathBuf>>,
    name: String,
) -> Result<TemplateInfo, String> {
    let template_path = get_templates_dir(&cwd).join(&name);
    
    if !template_path.exists() {
        return Err(format!("Template not found: {}", name));
    }
    
    let front_cover_path = template_path.join("front-cover.png");
    let content_path = template_path.join("content.png");
    let back_cover_path = template_path.join("back-cover.png");
    let pptx_path = template_path.join("template.pptx");
    
    let has_front_cover = front_cover_path.exists();
    let has_content = content_path.exists();
    let has_back_cover = back_cover_path.exists();
    let has_pptx = pptx_path.exists();
    
    Ok(TemplateInfo {
        name,
        path: template_path.to_string_lossy().to_string(),
        front_cover_path: if has_front_cover { Some(front_cover_path.to_string_lossy().to_string()) } else { None },
        content_path: if has_content { Some(content_path.to_string_lossy().to_string()) } else { None },
        back_cover_path: if has_back_cover { Some(back_cover_path.to_string_lossy().to_string()) } else { None },
        pptx_path: if has_pptx { Some(pptx_path.to_string_lossy().to_string()) } else { None },
        has_front_cover,
        has_content,
        has_back_cover,
        has_pptx,
    })
}
