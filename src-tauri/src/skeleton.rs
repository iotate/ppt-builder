use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tauri::State;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkeletonInfo {
    pub name: String,
    pub path: String,
}

fn get_skeletons_dir(cwd: &PathBuf) -> PathBuf {
    cwd.join("skeletons")
}

#[tauri::command]
pub async fn list_skeletons(cwd: State<'_, Arc<PathBuf>>) -> Result<Vec<SkeletonInfo>, String> {
    let skeletons_dir = get_skeletons_dir(&cwd);
    
    if !skeletons_dir.exists() {
        return Ok(Vec::new());
    }
    
    let mut entries = tokio::fs::read_dir(&skeletons_dir)
        .await
        .map_err(|e| format!("Failed to read skeletons directory: {}", e))?;
    
    let mut skeletons = Vec::new();
    
    while let Some(entry) = entries.next_entry().await.map_err(|e| e.to_string())? {
        if entry.file_type().await.map(|t| t.is_file()).unwrap_or(false) {
            let file_name = entry.file_name().to_string_lossy().to_string();
            
            if file_name.ends_with(".md") {
                let name = file_name.trim_end_matches(".md").to_string();
                skeletons.push(SkeletonInfo {
                    name,
                    path: entry.path().to_string_lossy().to_string(),
                });
            }
        }
    }
    
    // 按名称排序
    skeletons.sort_by(|a, b| a.name.cmp(&b.name));
    
    Ok(skeletons)
}

#[tauri::command]
pub async fn get_skeleton_content(
    cwd: State<'_, Arc<PathBuf>>,
    name: String,
) -> Result<String, String> {
    let skeleton_path = get_skeletons_dir(&cwd).join(format!("{}.md", name));
    
    if !skeleton_path.exists() {
        return Err(format!("Skeleton not found: {}", name));
    }
    
    let content = tokio::fs::read_to_string(&skeleton_path)
        .await
        .map_err(|e| format!("Failed to read skeleton file: {}", e))?;
    
    Ok(content)
}

#[tauri::command]
pub async fn save_skeleton(
    cwd: State<'_, Arc<PathBuf>>,
    name: String,
    content: String,
) -> Result<(), String> {
    let skeletons_dir = get_skeletons_dir(&cwd);
    
    // Create skeletons directory if not exists
    tokio::fs::create_dir_all(&skeletons_dir)
        .await
        .map_err(|e| format!("Failed to create skeletons directory: {}", e))?;
    
    let skeleton_path = skeletons_dir.join(format!("{}.md", name));
    
    tokio::fs::write(&skeleton_path, content)
        .await
        .map_err(|e| format!("Failed to save skeleton: {}", e))?;
    
    Ok(())
}

#[tauri::command]
pub async fn delete_skeleton(
    cwd: State<'_, Arc<PathBuf>>,
    name: String,
) -> Result<(), String> {
    let skeleton_path = get_skeletons_dir(&cwd).join(format!("{}.md", name));
    
    if skeleton_path.exists() {
        tokio::fs::remove_file(&skeleton_path)
            .await
            .map_err(|e| format!("Failed to delete skeleton: {}", e))?;
    }
    
    Ok(())
}
