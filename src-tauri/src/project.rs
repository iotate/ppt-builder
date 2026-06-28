use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tauri::State;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ProjectStatus {
    Draft,
    OutlineDraft,
    OutlineConfirmed,
    PagesSplit,
    ImagesGenerating,
    ImagesDone,
    Exported,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectInfo {
    pub name: String,
    pub topic: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub template: String,
    #[serde(default)]
    pub style: Option<String>,
    pub status: ProjectStatus,
    pub page_count: u32,
    pub image_count: u32,
    pub has_brainstorm: bool,
    pub has_outline: bool,
    pub has_images: bool,
    pub export_path: Option<String>,
    pub dir: String,
    #[serde(default)]
    pub size_index: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageFileInfo {
    pub page_num: u32,
    pub title: String,
    pub md_path: PathBuf,
    pub png_path: Option<PathBuf>,
    pub status: PageStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PageStatus {
    Unprocessed,
    ImageGenerated,
    ImageFailed,
}

fn get_projects_dir(cwd: &PathBuf) -> PathBuf {
    cwd.join("projects")
}

fn get_project_dir(cwd: &PathBuf, name: &str) -> PathBuf {
    get_projects_dir(cwd).join(name)
}

/// Scan project directory and count files
fn scan_project_files(project_dir: &PathBuf) -> (bool, bool, u32, u32) {
    let mut has_brainstorm = false;
    let mut has_outline = false;
    let mut page_count = 0u32;
    let mut image_count = 0u32;
    
    if let Ok(entries) = std::fs::read_dir(project_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            
            if name == "builder.db" {
                has_brainstorm = true;
            } else if name == "outline.md" {
                has_outline = true;
            } else if name.starts_with("page-") {
                if name.ends_with(".md") {
                    page_count += 1;
                } else if name.ends_with(".png") || name.ends_with(".jpg") || name.ends_with(".webp") {
                    image_count += 1;
                }
            }
        }
    }
    
    (has_brainstorm, has_outline, page_count, image_count)
}

#[tauri::command]
pub async fn create_project(
    cwd: State<'_, Arc<PathBuf>>,
    topic: String,
    template: String,
) -> Result<ProjectInfo, String> {
    let projects_dir = get_projects_dir(&cwd);
    
    // Create projects directory if not exists
    tokio::fs::create_dir_all(&projects_dir)
        .await
        .map_err(|e| format!("Failed to create projects directory: {}", e))?;
    
    // Generate unique name
    let name = format!("{}-{}", 
        topic.chars().take(20).collect::<String>().replace([' ', '/', '\\'], "-"),
        Uuid::new_v4().to_string()[..8].to_string()
    );
    
    let project_dir = get_project_dir(&cwd, &name);
    tokio::fs::create_dir_all(&project_dir)
        .await
        .map_err(|e| format!("Failed to create project directory: {}", e))?;
    
    let now = Utc::now();
    let project_info = ProjectInfo {
        name: name.clone(),
        topic: topic.clone(),
        created_at: now,
        updated_at: now,
        template,
        style: None,
        status: ProjectStatus::Draft,
        page_count: 0,
        image_count: 0,
        has_brainstorm: false,
        has_outline: false,
        has_images: false,
        export_path: None,
        dir: project_dir.to_string_lossy().to_string(),
        size_index: 3, // 默认 4:3 标准纵向
    };
    
    // Save project.json
    let project_json = project_dir.join("project.json");
    let content = serde_json::to_string_pretty(&project_info)
        .map_err(|e| format!("Failed to serialize project: {}", e))?;
    
    tokio::fs::write(&project_json, content)
        .await
        .map_err(|e| format!("Failed to write project.json: {}", e))?;
    
    Ok(project_info)
}

#[tauri::command]
pub async fn list_projects(cwd: State<'_, Arc<PathBuf>>) -> Result<Vec<ProjectInfo>, String> {
    let projects_dir = get_projects_dir(&cwd);
    
    if !projects_dir.exists() {
        return Ok(Vec::new());
    }
    
    let mut entries = tokio::fs::read_dir(&projects_dir)
        .await
        .map_err(|e| format!("Failed to read projects directory: {}", e))?;
    
    let mut projects = Vec::new();
    
    while let Some(entry) = entries.next_entry().await.map_err(|e| e.to_string())? {
        if entry.file_type().await.map(|t| t.is_dir()).unwrap_or(false) {
            let project_json = entry.path().join("project.json");
            if project_json.exists() {
                let content = tokio::fs::read_to_string(&project_json)
                    .await
                    .map_err(|e| format!("Failed to read project.json: {}", e))?;
                
                if let Ok(mut project) = serde_json::from_str::<ProjectInfo>(&content) {
                    // Scan project directory for file status
                    let (has_brainstorm, has_outline, page_count, image_count) = scan_project_files(&entry.path());
                    project.has_brainstorm = has_brainstorm;
                    project.has_outline = has_outline;
                    project.page_count = page_count;
                    project.image_count = image_count;
                    project.has_images = image_count > 0;
                    
                    projects.push(project);
                }
            }
        }
    }
    
    // Sort by updated_at descending
    projects.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    
    Ok(projects)
}

#[tauri::command]
pub async fn open_project(
    cwd: State<'_, Arc<PathBuf>>,
    name: String,
) -> Result<ProjectInfo, String> {
    let project_dir = get_project_dir(&cwd, &name);
    let project_json = project_dir.join("project.json");
    
    if !project_json.exists() {
        return Err(format!("Project not found: {}", name));
    }
    
    let content = tokio::fs::read_to_string(&project_json)
        .await
        .map_err(|e| format!("Failed to read project.json: {}", e))?;
    
    let mut project: ProjectInfo = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse project.json: {}", e))?;
    
    // Update file status
    let (has_brainstorm, has_outline, page_count, image_count) = scan_project_files(&project_dir);
    project.has_brainstorm = has_brainstorm;
    project.has_outline = has_outline;
    project.page_count = page_count;
    project.image_count = image_count;
    project.has_images = image_count > 0;
    
    Ok(project)
}

#[tauri::command]
pub async fn delete_project(
    cwd: State<'_, Arc<PathBuf>>,
    name: String,
) -> Result<(), String> {
    let project_dir = get_project_dir(&cwd, &name);
    
    if project_dir.exists() {
        tokio::fs::remove_dir_all(&project_dir)
            .await
            .map_err(|e| format!("Failed to delete project: {}", e))?;
    }
    
    Ok(())
}

#[tauri::command]
pub async fn update_project_status(
    cwd: State<'_, Arc<PathBuf>>,
    name: String,
    status: ProjectStatus,
) -> Result<(), String> {
    let project_dir = get_project_dir(&cwd, &name);
    let project_json = project_dir.join("project.json");
    
    let mut project: ProjectInfo = {
        let content = tokio::fs::read_to_string(&project_json)
            .await
            .map_err(|e| format!("Failed to read project.json: {}", e))?;
        serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse project.json: {}", e))?
    };
    
    project.status = status;
    project.updated_at = Utc::now();
    
    let content = serde_json::to_string_pretty(&project)
        .map_err(|e| format!("Failed to serialize project: {}", e))?;
    
    tokio::fs::write(&project_json, content)
        .await
        .map_err(|e| format!("Failed to write project.json: {}", e))?;
    
    Ok(())
}

#[tauri::command]
pub async fn update_project_size_index(
    cwd: State<'_, Arc<PathBuf>>,
    name: String,
    size_index: usize,
) -> Result<(), String> {
    let project_dir = get_project_dir(&cwd, &name);
    let project_json = project_dir.join("project.json");
    
    let mut project: ProjectInfo = {
        let content = tokio::fs::read_to_string(&project_json)
            .await
            .map_err(|e| format!("Failed to read project.json: {}", e))?;
        serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse project.json: {}", e))?
    };
    
    project.size_index = size_index;
    project.updated_at = Utc::now();
    
    let content = serde_json::to_string_pretty(&project)
        .map_err(|e| format!("Failed to serialize project: {}", e))?;
    
    tokio::fs::write(&project_json, content)
        .await
        .map_err(|e| format!("Failed to write project.json: {}", e))?;
    
    Ok(())
}

#[tauri::command]
pub async fn update_project_settings(
    cwd: State<'_, Arc<PathBuf>>,
    name: String,
    template: Option<String>,
    style: Option<String>,
    size_index: Option<usize>,
) -> Result<(), String> {
    let project_dir = get_project_dir(&cwd, &name);
    let project_json = project_dir.join("project.json");
    
    let mut project: ProjectInfo = {
        let content = tokio::fs::read_to_string(&project_json)
            .await
            .map_err(|e| format!("Failed to read project.json: {}", e))?;
        serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse project.json: {}", e))?
    };
    
    if let Some(t) = template {
        project.template = t;
    }
    if let Some(s) = style {
        project.style = Some(s);
    }
    if let Some(si) = size_index {
        project.size_index = si;
    }
    project.updated_at = Utc::now();
    
    let content = serde_json::to_string_pretty(&project)
        .map_err(|e| format!("Failed to serialize project: {}", e))?;
    
    tokio::fs::write(&project_json, content)
        .await
        .map_err(|e| format!("Failed to write project.json: {}", e))?;
    
    Ok(())
}

#[tauri::command]
pub async fn split_pages(
    cwd: State<'_, Arc<PathBuf>>,
    name: String,
) -> Result<Vec<PageFileInfo>, String> {
    let project_dir = get_project_dir(&cwd, &name);
    let outline_path = project_dir.join("outline.md");
    
    if !outline_path.exists() {
        return Err("大纲文件不存在，请先生成或保存大纲".to_string());
    }
    
    let content = tokio::fs::read_to_string(&outline_path)
        .await
        .map_err(|e| format!("读取大纲文件失败: {}", e))?;
    
    if content.trim().is_empty() {
        return Err("大纲内容为空".to_string());
    }
    
    // Clean up old page files before splitting
    cleanup_old_pages(&project_dir).await?;
    
    // Split by multiple possible delimiters
    let pages = split_outline_content(&content);
    
    if pages.is_empty() {
        return Err("未能从大纲中解析出页面内容，请检查大纲格式".to_string());
    }
    
    // First pass: filter valid pages
    let valid_pages: Vec<(usize, &String)> = pages
        .iter()
        .enumerate()
        .filter(|(_, page_content)| {
            let trimmed = page_content.trim();
            if trimmed.is_empty() {
                return false;
            }
            // Skip content with only one line (insufficient content)
            let line_count = trimmed.lines().filter(|l| !l.trim().is_empty()).count();
            line_count > 1
        })
        .collect();
    
    if valid_pages.is_empty() {
        return Err("没有有效的页面内容。所有页面内容过少（只有一行或为空）。".to_string());
    }
    
    let mut page_files = Vec::new();
    
    // Second pass: write pages with sequential numbering starting from 1
    for (seq_num, (_, page_content)) in valid_pages.iter().enumerate() {
        let page_num = (seq_num + 1) as u32;  // Sequential page number starting from 1
        let trimmed_content = page_content.trim();
        
        let title = extract_page_title(trimmed_content);
        
        // Use two-digit format for page numbers (01, 02, ..., 99)
        let md_path = project_dir.join(format!("page-{:02}.md", page_num));
        tokio::fs::write(&md_path, trimmed_content)
            .await
            .map_err(|e| format!("写入页面文件失败: {}", e))?;
        
        page_files.push(PageFileInfo {
            page_num,
            title,
            md_path,
            png_path: None,
            status: PageStatus::Unprocessed,
        });
    }
    
    let final_page_count = page_files.len();
    
    // Update project info
    let project_json = project_dir.join("project.json");
    if project_json.exists() {
        let mut project: ProjectInfo = {
            let content = tokio::fs::read_to_string(&project_json)
                .await
                .map_err(|e| format!("读取项目配置失败: {}", e))?;
            serde_json::from_str(&content)
                .map_err(|e| format!("解析项目配置失败: {}", e))?
        };
        
        project.page_count = final_page_count as u32;
        project.has_outline = true;
        project.status = ProjectStatus::PagesSplit;
        project.updated_at = Utc::now();
        
        let content = serde_json::to_string_pretty(&project)
            .map_err(|e| format!("序列化项目配置失败: {}", e))?;
        
        tokio::fs::write(&project_json, content)
            .await
            .map_err(|e| format!("写入项目配置失败: {}", e))?;
    }
    
    Ok(page_files)
}

/// Clean up old page files (both .md and .png)
async fn cleanup_old_pages(project_dir: &PathBuf) -> Result<(), String> {
    let mut entries = tokio::fs::read_dir(project_dir)
        .await
        .map_err(|e| format!("读取项目目录失败: {}", e))?;
    
    let mut files_to_delete = Vec::new();
    
    while let Some(entry) = entries.next_entry().await.map_err(|e| e.to_string())? {
        let file_name = entry.file_name().to_string_lossy().to_string();
        
        // Match page-{N}.md and page-{N}.png files
        if file_name.starts_with("page-") && 
           (file_name.ends_with(".md") || file_name.ends_with(".png") || 
            file_name.ends_with(".jpg") || file_name.ends_with(".webp")) {
            files_to_delete.push(entry.path());
        }
    }
    
    for file_path in files_to_delete {
        tokio::fs::remove_file(&file_path)
            .await
            .map_err(|e| format!("删除旧页面文件失败: {}", e))?;
    }
    
    Ok(())
}

/// Split outline content by standard delimiter
fn split_outline_content(content: &str) -> Vec<String> {
    // Use standard markdown horizontal rule as delimiter
    let delimiter = "\n---\n";
    
    let parts: Vec<&str> = content.split(delimiter).collect();
    
    if parts.len() > 1 {
        // Found delimiter, return split pages
        parts.iter().map(|s| s.to_string()).collect()
    } else {
        // No delimiter found, treat entire content as one page
        vec![content.to_string()]
    }
}

fn extract_page_title(content: &str) -> String {
    // Try multiple patterns to extract title
    for line in content.lines() {
        let trimmed = line.trim();
        
        // Pattern 1: ## 第N页：XXX
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
        
        // Pattern 2: **标题**: XXX
        if trimmed.starts_with("**标题**") {
            let title = trimmed
                .trim_start_matches(|c: char| c == '*' || c == ':')
                .trim_start_matches(':')
                .trim();
            if !title.is_empty() {
                return title.to_string();
            }
        }
        
        // Pattern 3: # XXX (main heading)
        if trimmed.starts_with("# ") && !trimmed.starts_with("## ") {
            let title = trimmed.trim_start_matches('#').trim();
            if !title.is_empty() && !title.starts_with("主题") {
                return title.to_string();
            }
        }
    }
    
    "未命名页面".to_string()
}

#[tauri::command]
pub async fn load_outline(
    cwd: State<'_, Arc<PathBuf>>,
    project_name: String,
) -> Result<String, String> {
    let project_dir = get_project_dir(&cwd, &project_name);
    let outline_path = project_dir.join("outline.md");
    
    if !outline_path.exists() {
        return Ok(String::new());
    }
    
    let content = tokio::fs::read_to_string(&outline_path)
        .await
        .map_err(|e| format!("Failed to read outline: {}", e))?;
    
    Ok(content)
}

#[tauri::command]
pub async fn list_pages(
    cwd: State<'_, Arc<PathBuf>>,
    project_name: String,
) -> Result<Vec<PageFileInfo>, String> {
    let project_dir = get_project_dir(&cwd, &project_name);
    
    if !project_dir.exists() {
        return Ok(Vec::new());
    }
    
    let mut entries = tokio::fs::read_dir(&project_dir)
        .await
        .map_err(|e| format!("Failed to read project directory: {}", e))?;
    
    let mut pages = Vec::new();
    
    while let Some(entry) = entries.next_entry().await.map_err(|e| e.to_string())? {
        let file_name = entry.file_name().to_string_lossy().to_string();
        
        if file_name.starts_with("page-") && file_name.ends_with(".md") {
            // Extract page number from filename like "page-01.md" or "page-1.md"
            let page_num_str = file_name
                .trim_start_matches("page-")
                .trim_end_matches(".md");
            let page_num: u32 = page_num_str.parse().unwrap_or(0);
            
            if page_num > 0 {
                let content = tokio::fs::read_to_string(entry.path())
                    .await
                    .unwrap_or_default();
                let title = extract_page_title(&content);
                
                // Use two-digit format for png path
                let png_path = project_dir.join(format!("page-{:02}.png", page_num));
                let has_image = png_path.exists();
                
                pages.push(PageFileInfo {
                    page_num,
                    title,
                    md_path: entry.path(),
                    png_path: if has_image { Some(png_path) } else { None },
                    status: if has_image {
                        PageStatus::ImageGenerated
                    } else {
                        PageStatus::Unprocessed
                    },
                });
            }
        }
    }
    
    // Sort by page number
    pages.sort_by_key(|p| p.page_num);
    
    Ok(pages)
}

#[tauri::command]
pub async fn read_page(
    cwd: State<'_, Arc<PathBuf>>,
    project_name: String,
    page_num: u32,
) -> Result<String, String> {
    let project_dir = get_project_dir(&cwd, &project_name);
    // Use two-digit format for page numbers
    let page_path = project_dir.join(format!("page-{:02}.md", page_num));
    
    if !page_path.exists() {
        return Err(format!("Page {} not found", page_num));
    }
    
    let content = tokio::fs::read_to_string(&page_path)
        .await
        .map_err(|e| format!("Failed to read page: {}", e))?;
    
    Ok(content)
}

#[tauri::command]
pub async fn save_page(
    cwd: State<'_, Arc<PathBuf>>,
    project_name: String,
    page_num: u32,
    content: String,
) -> Result<(), String> {
    let project_dir = get_project_dir(&cwd, &project_name);
    // Use two-digit format for page numbers
    let page_path = project_dir.join(format!("page-{:02}.md", page_num));
    
    tokio::fs::write(&page_path, content)
        .await
        .map_err(|e| format!("Failed to save page: {}", e))?;
    
    Ok(())
}


#[tauri::command]
pub async fn open_folder(path: String) -> Result<(), String> {
    // Use tauri-plugin-opener to open the folder in the file manager
    let path_buf = std::path::PathBuf::from(&path);
    
    if !path_buf.exists() {
        return Err(format!("Folder does not exist: {}", path));
    }
    
    // Open the folder using the system's default file manager
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(&path)
            .spawn()
            .map_err(|e| format!("Failed to open folder: {}", e))?;
    }
    
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(&path)
            .spawn()
            .map_err(|e| format!("Failed to open folder: {}", e))?;
    }
    
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(&path)
            .spawn()
            .map_err(|e| format!("Failed to open folder: {}", e))?;
    }
    
    Ok(())
}

#[tauri::command]
pub async fn open_project_folder(
    cwd: tauri::State<'_, std::sync::Arc<std::path::PathBuf>>,
    project_name: String,
) -> Result<(), String> {
    let project_dir = cwd.join("projects").join(&project_name);
    
    if !project_dir.exists() {
        return Err(format!("Project folder does not exist: {}", project_name));
    }
    
    let path = project_dir.to_string_lossy().to_string();
    
    // Open the project folder using the system's default file manager
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(&path)
            .spawn()
            .map_err(|e| format!("Failed to open folder: {}", e))?;
    }
    
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(&path)
            .spawn()
            .map_err(|e| format!("Failed to open folder: {}", e))?;
    }
    
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(&path)
            .spawn()
            .map_err(|e| format!("Failed to open folder: {}", e))?;
    }
    
    Ok(())
}

#[tauri::command]
pub async fn open_project_attachs_folder(
    cwd: tauri::State<'_, std::sync::Arc<std::path::PathBuf>>,
    project_name: String,
) -> Result<(), String> {
    let attachs_dir = cwd.join("projects").join(&project_name).join("attachs");
    
    // 如果附件目录不存在，创建它
    if !attachs_dir.exists() {
        tokio::fs::create_dir_all(&attachs_dir)
            .await
            .map_err(|e| format!("Failed to create attachs folder: {}", e))?;
    }
    
    let path = attachs_dir.to_string_lossy().to_string();
    
    // Open the attachs folder using the system's default file manager
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(&path)
            .spawn()
            .map_err(|e| format!("Failed to open folder: {}", e))?;
    }
    
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(&path)
            .spawn()
            .map_err(|e| format!("Failed to open folder: {}", e))?;
    }
    
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(&path)
            .spawn()
            .map_err(|e| format!("Failed to open folder: {}", e))?;
    }
    
    Ok(())
}


#[tauri::command]
pub async fn delete_page(
    cwd: State<'_, Arc<PathBuf>>,
    project_name: String,
    page_num: u32,
) -> Result<(), String> {
    let project_dir = get_project_dir(&cwd, &project_name);
    
    // Delete the page markdown file
    let md_path = project_dir.join(format!("page-{:02}.md", page_num));
    if md_path.exists() {
        tokio::fs::remove_file(&md_path)
            .await
            .map_err(|e| format!("删除页面文件失败: {}", e))?;
    }
    
    // Delete the corresponding image file if exists
    let png_path = project_dir.join(format!("page-{:02}.png", page_num));
    if png_path.exists() {
        tokio::fs::remove_file(&png_path)
            .await
            .map_err(|e| format!("删除图片文件失败: {}", e))?;
    }
    
    // Reorganize remaining pages
    let mut entries = tokio::fs::read_dir(&project_dir)
        .await
        .map_err(|e| format!("读取项目目录失败: {}", e))?;
    
    let mut page_files: Vec<(u32, PathBuf)> = Vec::new();
    
    while let Some(entry) = entries.next_entry().await.map_err(|e| e.to_string())? {
        let file_name = entry.file_name().to_string_lossy().to_string();
        
        if file_name.starts_with("page-") && file_name.ends_with(".md") {
            let num_str = file_name.trim_start_matches("page-").trim_end_matches(".md");
            if let Ok(num) = num_str.parse::<u32>() {
                if num != page_num {
                    page_files.push((num, entry.path()));
                }
            }
        }
    }
    
    // Sort by page number
    page_files.sort_by_key(|(num, _)| *num);
    
    // Rename files to sequential order
    for (new_num, (old_num, old_path)) in page_files.iter().enumerate() {
        let new_num = (new_num + 1) as u32;
        
        if *old_num != new_num {
            // Rename markdown file
            let new_md_path = project_dir.join(format!("page-{:02}.md", new_num));
            tokio::fs::rename(old_path, &new_md_path)
                .await
                .map_err(|e| format!("重命名页面文件失败: {}", e))?;
            
            // Rename image file if exists
            let old_png_path = project_dir.join(format!("page-{:02}.png", old_num));
            if old_png_path.exists() {
                let new_png_path = project_dir.join(format!("page-{:02}.png", new_num));
                tokio::fs::rename(&old_png_path, &new_png_path)
                    .await
                    .map_err(|e| format!("重命名图片文件失败: {}", e))?;
            }
        }
    }
    
    // Update project info
    let project_json = project_dir.join("project.json");
    if project_json.exists() {
        let mut project: ProjectInfo = {
            let content = tokio::fs::read_to_string(&project_json)
                .await
                .map_err(|e| format!("读取项目配置失败: {}", e))?;
            serde_json::from_str(&content)
                .map_err(|e| format!("解析项目配置失败: {}", e))?
        };
        
        let (_, _, page_count, image_count) = scan_project_files(&project_dir);
        project.page_count = page_count;
        project.image_count = image_count;
        project.updated_at = Utc::now();
        
        let content = serde_json::to_string_pretty(&project)
            .map_err(|e| format!("序列化项目配置失败: {}", e))?;
        
        tokio::fs::write(&project_json, content)
            .await
            .map_err(|e| format!("写入项目配置失败: {}", e))?;
    }
    
    Ok(())
}
