use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tauri::State;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StyleInfo {
    pub name: String,
    pub path: String,
}

fn get_styles_dir(cwd: &PathBuf) -> PathBuf {
    cwd.join("styles")
}

#[tauri::command]
pub async fn list_styles(cwd: State<'_, Arc<PathBuf>>) -> Result<Vec<StyleInfo>, String> {
    let styles_dir = get_styles_dir(&cwd);
    
    if !styles_dir.exists() {
        return Ok(Vec::new());
    }
    
    let mut entries = tokio::fs::read_dir(&styles_dir)
        .await
        .map_err(|e| format!("Failed to read styles directory: {}", e))?;
    
    let mut styles = Vec::new();
    
    while let Some(entry) = entries.next_entry().await.map_err(|e| e.to_string())? {
        if entry.file_type().await.map(|t| t.is_file()).unwrap_or(false) {
            let file_name = entry.file_name().to_string_lossy().to_string();
            
            if file_name.ends_with(".md") {
                let name = file_name.trim_end_matches(".md").to_string();
                styles.push(StyleInfo {
                    name,
                    path: entry.path().to_string_lossy().to_string(),
                });
            }
        }
    }
    
    Ok(styles)
}

#[tauri::command]
pub async fn get_style_content(
    cwd: State<'_, Arc<PathBuf>>,
    name: String,
) -> Result<String, String> {
    let style_path = get_styles_dir(&cwd).join(format!("{}.md", name));
    
    if !style_path.exists() {
        return Err(format!("Style not found: {}", name));
    }
    
    let content = tokio::fs::read_to_string(&style_path)
        .await
        .map_err(|e| format!("Failed to read style file: {}", e))?;
    
    Ok(content)
}

#[tauri::command]
pub async fn save_style(
    cwd: State<'_, Arc<PathBuf>>,
    name: String,
    content: String,
) -> Result<(), String> {
    let styles_dir = get_styles_dir(&cwd);
    
    // Create styles directory if not exists
    tokio::fs::create_dir_all(&styles_dir)
        .await
        .map_err(|e| format!("Failed to create styles directory: {}", e))?;
    
    let style_path = styles_dir.join(format!("{}.md", name));
    
    tokio::fs::write(&style_path, content)
        .await
        .map_err(|e| format!("Failed to save style: {}", e))?;
    
    Ok(())
}

#[tauri::command]
pub async fn delete_style(
    cwd: State<'_, Arc<PathBuf>>,
    name: String,
) -> Result<(), String> {
    let style_path = get_styles_dir(&cwd).join(format!("{}.md", name));
    
    if style_path.exists() {
        tokio::fs::remove_file(&style_path)
            .await
            .map_err(|e| format!("Failed to delete style: {}", e))?;
    }
    
    Ok(())
}
