use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tauri::State;

use crate::config::ApiConfig;
use crate::error_log;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageOutline {
    pub page_num: u32,
    pub title: String,
    pub content: String,
    pub layout: String,
    pub image_desc: String,
    pub speaker_notes: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OutlineMode {
    Simple,   // 3-5 pages
    Medium,   // 6-10 pages
    Detailed, // 10-15 pages
}

impl OutlineMode {
    pub fn page_range(&self) -> (u32, u32) {
        match self {
            OutlineMode::Simple => (3, 5),
            OutlineMode::Medium => (6, 10),
            OutlineMode::Detailed => (10, 15),
        }
    }
}

fn get_projects_dir(cwd: &PathBuf) -> PathBuf {
    cwd.join("projects")
}

#[tauri::command]
pub async fn generate_outline(
    cwd: State<'_, Arc<PathBuf>>,
    topic: String,
    mode: String,
    expected_pages: Option<u32>,
    config: ApiConfig,
) -> Result<String, String> {
    let cwd_path = cwd.inner().clone();
    
    let (min_pages, max_pages) = if let Some(pages) = expected_pages {
        // 自定义模式：使用用户指定的页数，允许 1-50 页
        let pages = pages.clamp(1, 50);
        (pages, pages)
    } else {
        // 预设模式
        let outline_mode = match mode.as_str() {
            "simple" => OutlineMode::Simple,
            "detailed" => OutlineMode::Detailed,
            _ => OutlineMode::Medium,
        };
        outline_mode.page_range()
    };
    
    let prompt = format!(
        r#"请为以下主题生成一个信息图表大纲，包含{}到{}页。

主题：{}

请按以下格式输出（使用Markdown）：

# 主题：[主题名称]

---

## 第1页：封面

**标题**: [主标题]
**页面内容**: [副标题/日期/作者]
**布局设计**: [布局描述]
**配图设计**: [配图描述]
**讲稿备注**: [讲稿内容]

---

## 第2页：[页面标题]

**标题**: [标题]
**页面内容**: [内容]
**布局设计**: [布局描述]
**配图设计**: [配图描述]
**讲稿备注**: [讲稿内容]

---

... (更多页面)

## 第N页：封底

**标题**: 谢谢
**页面内容**: [联系方式/二维码]
**布局设计**: 居中布局
**配图设计**: 简洁背景
**讲稿备注**: 结束语

请确保：
1. 第一页是封面
2. 最后一页是封底
3. 内容页之间用 --- 分隔
4. 每页都包含所有必需字段
"#,
        min_pages, max_pages, topic
    );

    // Log the start of generation (只记录动作，不记录具体内容)
    error_log::log_info(&cwd_path, "Starting outline generation");

    // Call LLM API
    let client = reqwest::Client::new();
    
    let mut request = client
        .post(format!("{}/chat/completions", config.endpoint))
        .header("Authorization", format!("Bearer {}", config.api_key))
        .header("Content-Type", "application/json");
    
    // Add extra headers
    for header in &config.extra_headers {
        request = request.header(&header.key, &header.value);
    }
    
    let response = request
        .json(&serde_json::json!({
            "model": config.model,
            "messages": [
                {"role": "system", "content": "你是一个专业的信息图表设计师，擅长创建结构清晰、内容丰富的演示文稿大纲。"},
                {"role": "user", "content": prompt}
            ],
            "temperature": 0.7,
        }))
        .send()
        .await;

    match response {
        Ok(resp) => {
            if !resp.status().is_success() {
                let error_text = resp.text().await.unwrap_or_default();
                let error_msg = format!("LLM API error: {}", error_text);
                error_log::log_error(&cwd_path, &error_msg);
                return Err(error_msg);
            }

            let json_result = resp.json::<serde_json::Value>().await;
            match json_result {
                Ok(json) => {
                    let content = json["choices"][0]["message"]["content"]
                        .as_str()
                        .ok_or("Failed to extract content from LLM response");

                    match content {
                        Ok(content) => {
                            error_log::log_info(&cwd_path, "Outline generation completed successfully");
                            Ok(content.to_string())
                        }
                        Err(e) => {
                            let error_msg = e.to_string();
                            error_log::log_error(&cwd_path, &error_msg);
                            Err(error_msg)
                        }
                    }
                }
                Err(e) => {
                    let error_msg = format!("Failed to parse LLM response: {}", e);
                    error_log::log_error(&cwd_path, &error_msg);
                    Err(error_msg)
                }
            }
        }
        Err(e) => {
            let error_msg = format!("LLM API request failed: {}", e);
            error_log::log_error(&cwd_path, &error_msg);
            Err(error_msg)
        }
    }
}

#[tauri::command]
pub async fn parse_outline(content: String) -> Result<Vec<PageOutline>, String> {
    let pages: Vec<&str> = content.split("\n---\n").collect();
    let mut outlines = Vec::new();
    
    for (i, page_content) in pages.iter().enumerate() {
        let page_num = (i + 1) as u32;
        
        let title = extract_field(page_content, "标题");
        let content = extract_field(page_content, "页面内容");
        let layout = extract_field(page_content, "布局设计");
        let image_desc = extract_field(page_content, "配图设计");
        let speaker_notes = extract_field(page_content, "讲稿备注");
        
        outlines.push(PageOutline {
            page_num,
            title,
            content,
            layout,
            image_desc,
            speaker_notes,
        });
    }
    
    Ok(outlines)
}

fn extract_field(content: &str, field_name: &str) -> String {
    for line in content.lines() {
        if line.starts_with(&format!("**{}**", field_name)) {
            return line
                .split(':')
                .skip(1)
                .collect::<Vec<_>>()
                .join(":")
                .trim()
                .to_string();
        }
    }
    String::new()
}

#[tauri::command]
pub async fn save_outline(
    cwd: State<'_, Arc<PathBuf>>,
    project_name: String,
    content: String,
) -> Result<(), String> {
    let project_dir = get_projects_dir(&cwd).join(&project_name);
    let outline_path = project_dir.join("outline.md");
    
    tokio::fs::write(&outline_path, content)
        .await
        .map_err(|e| format!("Failed to save outline: {}", e))?;
    
    Ok(())
}

#[tauri::command]
pub async fn load_prompt(
    cwd: State<'_, Arc<PathBuf>>,
    project_name: String,
) -> Result<String, String> {
    let project_dir = get_projects_dir(&cwd).join(&project_name);
    let prompt_path = project_dir.join("prompt.md");
    
    if !prompt_path.exists() {
        return Ok(String::new());
    }
    
    let content = tokio::fs::read_to_string(&prompt_path)
        .await
        .map_err(|e| format!("Failed to read prompt: {}", e))?;
    
    Ok(content)
}

#[tauri::command]
pub async fn save_prompt(
    cwd: State<'_, Arc<PathBuf>>,
    project_name: String,
    content: String,
) -> Result<(), String> {
    let project_dir = get_projects_dir(&cwd).join(&project_name);
    let prompt_path = project_dir.join("prompt.md");
    
    tokio::fs::write(&prompt_path, content)
        .await
        .map_err(|e| format!("Failed to save prompt: {}", e))?;
    
    Ok(())
}

#[tauri::command]
pub async fn regenerate_page(
    cwd: State<'_, Arc<PathBuf>>,
    project_name: String,
    page_num: u32,
    prompt: String,
    config: ApiConfig,
) -> Result<String, String> {
    let project_dir = get_projects_dir(&cwd).join(&project_name);
    let outline_path = project_dir.join("outline.md");
    
    let content = tokio::fs::read_to_string(&outline_path)
        .await
        .map_err(|e| format!("Failed to read outline: {}", e))?;
    
    let pages: Vec<&str> = content.split("\n---\n").collect();
    let page_index = (page_num - 1) as usize;
    
    if page_index >= pages.len() {
        return Err(format!("Page {} not found", page_num));
    }
    
    let current_page = pages[page_index];
    
    let system_prompt = "你是一个专业的信息图表设计师，擅长优化和改进演示文稿内容。";
    let user_prompt = format!(
        "请根据以下要求重新生成这一页的内容：\n\n原始内容：\n{}\n\n修改要求：{}\n\n请保持原有格式输出。",
        current_page, prompt
    );
    
    let client = reqwest::Client::new();
    
    let mut request = client
        .post(format!("{}/chat/completions", config.endpoint))
        .header("Authorization", format!("Bearer {}", config.api_key))
        .header("Content-Type", "application/json");
    
    // Add extra headers
    for header in &config.extra_headers {
        request = request.header(&header.key, &header.value);
    }
    
    let response = request
        .json(&serde_json::json!({
            "model": config.model,
            "messages": [
                {"role": "system", "content": system_prompt},
                {"role": "user", "content": user_prompt}
            ],
            "temperature": 0.7,
        }))
        .send()
        .await
        .map_err(|e| format!("LLM API request failed: {}", e))?;

    if !response.status().is_success() {
        let error_text = response.text().await.unwrap_or_default();
        return Err(format!("LLM API error: {}", error_text));
    }

    let json: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse LLM response: {}", e))?;

    let new_content = json["choices"][0]["message"]["content"]
        .as_str()
        .ok_or("Failed to extract content from LLM response")?
        .to_string();
    
    // Update the page in outline
    let mut pages_vec: Vec<String> = pages.iter().map(|s| s.to_string()).collect();
    pages_vec[page_index] = new_content.clone();
    
    let updated_content = pages_vec.join("\n---\n");
    tokio::fs::write(&outline_path, updated_content)
        .await
        .map_err(|e| format!("Failed to update outline: {}", e))?;
    
    Ok(new_content)
}
