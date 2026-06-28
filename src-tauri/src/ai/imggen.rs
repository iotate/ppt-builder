use crate::config::ImageConfig;
use crate::error_log;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::State;
use futures::StreamExt;

use super::{ImageGenerationOptions, ImageJobResult, ImageJobStatus};

#[tauri::command]
pub async fn generate_all_images(
    cwd: State<'_, Arc<PathBuf>>,
    project_name: String,
    options: ImageGenerationOptions,
    config: ImageConfig,
) -> Result<Vec<ImageJobResult>, String> {
    let cwd_path = cwd.inner().clone();
    let project_dir = cwd.join("projects").join(&project_name);
    
    if !project_dir.exists() {
        let error = format!("Project not found: {}", project_name);
        error_log::log_error(&cwd_path, &error);
        return Err(error);
    }
    
    error_log::log_info(&cwd_path, &format!("Starting image generation for project: {}", project_name));
    
    // Get style content if specified
    let style_content = if let Some(style_name) = &options.style {
        let style_path = cwd.join("styles").join(format!("{}.md", style_name));
        if style_path.exists() {
            Some(tokio::fs::read_to_string(&style_path).await.unwrap_or_default())
        } else {
            None
        }
    } else {
        None
    };
    
    // Find all page files
    let mut page_files = Vec::new();
    let mut entries = tokio::fs::read_dir(&project_dir)
        .await
        .map_err(|e| {
            let error = format!("Failed to read project directory: {}", e);
            error_log::log_error(&cwd_path, &error);
            error
        })?;
    
    while let Some(entry) = entries.next_entry().await.map_err(|e| e.to_string())? {
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with("page-") && name.ends_with(".md") {
            page_files.push(entry.path());
        }
    }
    
    page_files.sort_by_key(|p| p.file_name().unwrap().to_string_lossy().to_string());
    
    let total_pages = page_files.len();
    
    // Use width/height from options, fallback to 1920x1080 (16:9)
    let width = options.width.unwrap_or(1920);
    let height = options.height.unwrap_or(1080);
    
    // 并发生成图片，最多 3 个并发
    const MAX_CONCURRENT: usize = 3;
    
    // 创建生成任务
    let futures_stream = futures::stream::iter(page_files.into_iter().map(|page_path| {
        let cwd = cwd.clone();
        let cwd_path = cwd_path.clone();
        let style_content = style_content.clone();
        let config = config.clone();
        let project_dir = project_dir.clone();
        let template_opt = options.template.clone();
        
        async move {
            let page_name = page_path.file_stem().unwrap().to_string_lossy().to_string();
            let page_num: u32 = page_name
                .strip_prefix("page-")
                .and_then(|s| s.parse().ok())
                .unwrap_or(0);
            
            // Read page content
            let page_content = match tokio::fs::read_to_string(&page_path).await {
                Ok(content) => content,
                Err(e) => {
                    let error = format!("Failed to read page file {}: {}", page_name, e);
                    error_log::log_error(&cwd_path, &error);
                    return ImageJobResult {
                        page_num,
                        status: ImageJobStatus::Failed(error.clone()),
                        output_path: None,
                        error: Some(error),
                    };
                }
            };
            
            // Determine page type and get template image
            let page_type = determine_page_type(&page_content, page_num, total_pages);
            let template_image = get_template_image(&cwd, &template_opt, page_type).await;
            
            // Build prompt (without style guide for backward compatibility)
            let prompt = build_image_prompt(&page_content, style_content.as_deref(), page_type, None, None, false);
            
            // Generate image - use two-digit format for output path
            let output_path = project_dir.join(format!("page-{:02}.png", page_num));
            
            let result = if let Some(template_path) = template_image {
                generate_image_with_template(&prompt, width, height, &config, &output_path, &template_path).await
            } else {
                generate_single_image(&prompt, width, height, &config, &output_path).await
            };
            
            match result {
                Ok(_) => {
                    error_log::log_info(&cwd_path, &format!("Image generated successfully for page {}", page_num));
                    ImageJobResult {
                        page_num,
                        status: ImageJobStatus::Success,
                        output_path: Some(output_path.to_string_lossy().to_string()),
                        error: None,
                    }
                },
                Err(e) => {
                    error_log::log_error(&cwd_path, &format!("Failed to generate image for page {}: {}", page_num, e));
                    ImageJobResult {
                        page_num,
                        status: ImageJobStatus::Failed(e.clone()),
                        output_path: None,
                        error: Some(e),
                    }
                }
            }
        }
    }));
    
    // 使用 buffer_unordered 控制并发数
    let results: Vec<ImageJobResult> = futures_stream
        .buffer_unordered(MAX_CONCURRENT)
        .collect()
        .await;
    
    // 按页码排序结果
    let mut sorted_results = results;
    sorted_results.sort_by_key(|r| r.page_num);
    
    error_log::log_info(&cwd_path, &format!("Image generation completed for project: {}", project_name));
    Ok(sorted_results)
}

#[tauri::command]
pub async fn regenerate_image(
    cwd: State<'_, Arc<PathBuf>>,
    project_name: String,
    page_num: u32,
    custom_prompt: Option<String>,
    width: Option<u32>,
    height: Option<u32>,
    config: ImageConfig,
) -> Result<String, String> {
    let project_dir = cwd.join("projects").join(&project_name);
    // Use two-digit format for page numbers
    let page_path = project_dir.join(format!("page-{:02}.md", page_num));
    
    if !page_path.exists() {
        return Err(format!("Page {} not found", page_num));
    }
    
    let page_content = tokio::fs::read_to_string(&page_path)
        .await
        .map_err(|e| format!("Failed to read page file: {}", e))?;
    
    let prompt = custom_prompt.unwrap_or_else(|| {
        let page_type = determine_page_type(&page_content, page_num, 0);
        build_image_prompt(&page_content, None, page_type, None, None, false)
    });
    
    // Use two-digit format for output path
    let output_path = project_dir.join(format!("page-{:02}.png", page_num));
    
    let img_width = width.unwrap_or(1920);
    let img_height = height.unwrap_or(1080);
    
    generate_single_image(&prompt, img_width, img_height, &config, &output_path).await?;
    
    Ok(output_path.to_string_lossy().to_string())
}

#[tauri::command]
pub async fn generate_image(
    cwd: State<'_, Arc<PathBuf>>,
    project_name: String,
    page_num: u32,
    template: Option<String>,
    style: Option<String>,
    width: u32,
    height: u32,
    config: ImageConfig,
    _layout_family: Option<String>,
    _adherence_level: Option<String>,
    _llm_config: Option<crate::config::ApiConfig>,
) -> Result<String, String> {
    let cwd_path = cwd.inner().clone();
    let project_dir = cwd.join("projects").join(&project_name);
    // Use two-digit format for page numbers
    let page_path = project_dir.join(format!("page-{:02}.md", page_num));
    
    if !page_path.exists() {
        return Err(format!("Page {} not found", page_num));
    }
    
    // Get style content and parse style guide
    let (style_content, style_guide) = if let Some(style_name) = &style {
        let style_path = cwd.join("styles").join(format!("{}.md", style_name));
        if style_path.exists() {
            let content = tokio::fs::read_to_string(&style_path).await.unwrap_or_default();
            let guide = crate::style_guide::parse_style_guide_from_markdown(&content);
            (Some(content), Some(guide))
        } else {
            (None, None)
        }
    } else {
        (None, None)
    };
    
    // Read page content
    let page_content = tokio::fs::read_to_string(&page_path)
        .await
        .map_err(|e| format!("Failed to read page file: {}", e))?;
    
    // Determine page type and get template image
    let page_type = determine_page_type(&page_content, page_num, 0);
    let template_image = if let Some(template_name) = &template {
        get_template_image(&cwd, &Some(template_name.clone()), page_type).await
    } else {
        None
    };
    
    // Check if we have reference images (template images)
    let has_reference_images = template_image.is_some();
    
    // Build prompt with style guide
    let prompt = build_image_prompt(
        &page_content,
        style_content.as_deref(),
        page_type,
        _layout_family.as_deref(),
        style_guide.as_ref(),
        has_reference_images,
    );
    
    // Generate image - use two-digit format for output path
    let output_path = project_dir.join(format!("page-{:02}.png", page_num));
    
    let result = if let Some(template_path) = template_image {
        generate_image_with_template(&prompt, width, height, &config, &output_path, &template_path).await
    } else {
        generate_single_image(&prompt, width, height, &config, &output_path).await
    };
    
    match result {
        Ok(_) => {
            error_log::log_info(&cwd_path, &format!("Image generated successfully for page {}", page_num));
            Ok(output_path.to_string_lossy().to_string())
        }
        Err(e) => {
            error_log::log_error(&cwd_path, &format!("Failed to generate image for page {}: {}", page_num, e));
            Err(e)
        }
    }
}

#[tauri::command]
pub async fn refine_image(
    cwd: State<'_, Arc<PathBuf>>,
    project_name: String,
    page_num: u32,
    refine_prompt: String,
    width: u32,
    height: u32,
    config: ImageConfig,
) -> Result<String, String> {
    let project_dir = cwd.join("projects").join(&project_name);
    // Use two-digit format for output path
    let output_path = project_dir.join(format!("page-{:02}.png", page_num));
    
    if !output_path.exists() {
        return Err(format!("Image for page {} not found", page_num));
    }
    
    // Read existing image and encode as base64
    let image_data = tokio::fs::read(&output_path)
        .await
        .map_err(|e| format!("Failed to read image: {}", e))?;
    
    let base64_image = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &image_data);
    
    // Call image editing API
    let client = reqwest::Client::new();
    
    let response = client
        .post(config.endpoint.replace("/generations", "/edits"))
        .header("Authorization", format!("Bearer {}", config.api_key))
        .json(&serde_json::json!({
            "image": base64_image,
            "prompt": refine_prompt,
            "n": 1,
            "size": format!("{}x{}", width, height),
        }))
        .send()
        .await
        .map_err(|e| format!("Image edit API request failed: {}", e))?;
    
    if !response.status().is_success() {
        let error_text = response.text().await.unwrap_or_default();
        return Err(format!("Image edit API error: {}", error_text));
    }
    
    let json: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse API response: {}", e))?;
    
    // Try URL first, then base64
      let image_url = if let Some(url) = json["data"][0]["url"].as_str() {
          url.to_string()
      } else if let Some(b64) = json["data"][0]["b64_json"].as_str() {
          // Handle base64 response - decode and save directly
          let image_bytes = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, b64)
              .map_err(|e| format!("Failed to decode base64 image: {}", e))?;
          tokio::fs::write(&output_path, &image_bytes)
              .await
              .map_err(|e| format!("Failed to save image: {}", e))?;
          return Ok(output_path.to_string_lossy().to_string());
      } else {
          return Err(format!("Failed to extract image from response. Response keys: {:?}", 
              json.as_object().map(|m| m.keys().collect::<Vec<_>>())));
      };

      // Download and save the image
    let image_response = reqwest::get(image_url)
        .await
        .map_err(|e| format!("Failed to download image: {}", e))?;
    
    let image_bytes = image_response
        .bytes()
        .await
        .map_err(|e| format!("Failed to read image data: {}", e))?;
    
    tokio::fs::write(&output_path, &image_bytes)
        .await
        .map_err(|e| format!("Failed to save image: {}", e))?;
    
    Ok(output_path.to_string_lossy().to_string())
}

/// 使用多模态模型微调图片（基于参考图的编辑）
#[tauri::command]
pub async fn refine_image_with_reference(
    cwd: State<'_, Arc<PathBuf>>,
    project_name: String,
    page_num: u32,
    refine_prompt: String,
    width: u32,
    height: u32,
    config: ImageConfig,
) -> Result<String, String> {
    let project_dir = cwd.join("projects").join(&project_name);
    let output_path = project_dir.join(format!("page-{:02}.png", page_num));
    
    if !output_path.exists() {
        return Err(format!("Image for page {} not found", page_num));
    }
    
    // 读取现有图片并转为 base64
    let image_data = tokio::fs::read(&output_path)
        .await
        .map_err(|e| format!("Failed to read image: {}", e))?;
    
    let base64_image = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &image_data);
    let data_uri = format!("data:image/png;base64,{}", base64_image);
    
    // 构建提示词
    let prompt = format!(
        "请根据以下要求对图片进行微调，保持整体风格和布局不变，只修改指定部分：\n\n{}\n\n请生成修改后的图片。",
        refine_prompt
    );
    
    // 使用图片生成 API（带参考图）
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;
    
    let model_lower = config.model.to_lowercase();
    let is_gpt_image = model_lower.contains("gpt");
    let is_agnes = model_lower.starts_with("agnes");
    
    if is_gpt_image {
        // GPT-Image: 使用 /images/edits 端点
        let edit_endpoint = config.endpoint.replace("/generations", "/edits");
        
        let form = reqwest::multipart::Form::new()
            .text("model", config.model.clone())
            .text("prompt", prompt.clone())
            .text("size", format!("{}x{}", width, height))
            .text("n", "1")
            .part("image[]", reqwest::multipart::Part::bytes(image_data.clone())
                .file_name("image.png")
                .mime_str("image/png")
                .map_err(|e| format!("Failed to set mime type: {}", e))?);
        
        let mut request = client
            .post(&edit_endpoint)
            .header("Authorization", format!("Bearer {}", config.api_key));
        
        for header in &config.extra_headers {
            request = request.header(&header.key, &header.value);
        }
        
        let response = request
            .multipart(form)
            .send()
            .await
            .map_err(|e| format!("Image edit API request failed: {}", e))?;
        
        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(format!("Image edit API error (HTTP {}): {}", status, error_text));
        }
        
        let json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse API response: {}", e))?;
        
        handle_image_response(&json, &output_path).await?;
        Ok(output_path.to_string_lossy().to_string())
    } else if is_agnes {
        // Agnes: 使用 extra_body.image 参数
        let request_body = serde_json::json!({
            "model": config.model,
            "prompt": prompt,
            "size": format!("{}x{}", width, height),
            "extra_body": {
                "image": [data_uri],
                "response_format": "b64_json"
            }
        });
        
        let mut request = client
            .post(&config.endpoint)
            .header("Authorization", format!("Bearer {}", config.api_key))
            .header("Content-Type", "application/json");
        
        for header in &config.extra_headers {
            request = request.header(&header.key, &header.value);
        }
        
        let response = request
            .json(&request_body)
            .send()
            .await
            .map_err(|e| format!("Image generation API request failed: {}", e))?;
        
        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(format!("Image generation API error (HTTP {}): {}", status, error_text));
        }
        
        let json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse API response: {}", e))?;
        
        handle_image_response(&json, &output_path).await?;
        Ok(output_path.to_string_lossy().to_string())
    } else {
        // 其他模型：使用标准格式
        let request_body = serde_json::json!({
            "model": config.model,
            "prompt": prompt,
            "image": [data_uri],
            "size": format!("{}x{}", width, height),
            "n": 1,
        });
        
        let mut request = client
            .post(&config.endpoint)
            .header("Authorization", format!("Bearer {}", config.api_key))
            .header("Content-Type", "application/json");
        
        for header in &config.extra_headers {
            request = request.header(&header.key, &header.value);
        }
        
        let response = request
            .json(&request_body)
            .send()
            .await
            .map_err(|e| format!("Image generation API request failed: {}", e))?;
        
        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(format!("Image generation API error (HTTP {}): {}", status, error_text));
        }
        
        let json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse API response: {}", e))?;
        
        handle_image_response(&json, &output_path).await?;
        Ok(output_path.to_string_lossy().to_string())
    }
}

/// Determine page type based on content and position
fn determine_page_type(content: &str, page_num: u32, total_pages: usize) -> &'static str {
    // First page is always front cover
    if page_num == 1 {
        return "front-cover";
    }
    
    // Last page is back cover (if we know total)
    if total_pages > 0 && page_num as usize == total_pages {
        return "back-cover";
    }
    
    // Check content for keywords
    let lower = content.to_lowercase();
    if lower.contains("封面") || lower.contains("cover") {
        return "front-cover";
    }
    if lower.contains("封底") || lower.contains("thank") || lower.contains("谢谢") || lower.contains("感谢") {
        return "back-cover";
    }
    
    "content"
}

/// Get template image path for the given page type
async fn get_template_image(cwd: &PathBuf, template_name: &Option<String>, page_type: &str) -> Option<PathBuf> {
    let template_name = template_name.as_ref()?;
    
    let template_dir = cwd.join("templates").join(template_name);
    if !template_dir.exists() {
        error_log::log_error(cwd, &format!("Template directory not found: {:?}", template_dir));
        return None;
    }
    
    let template_file = match page_type {
        "front-cover" => template_dir.join("front-cover.png"),
        "back-cover" => template_dir.join("back-cover.png"),
        _ => template_dir.join("content.png"),
    };
    
    if template_file.exists() {
        error_log::log_info(cwd, &format!("Using template image: {:?}", template_file));
        Some(template_file)
    } else {
        error_log::log_error(cwd, &format!("Template image not found: {:?}", template_file));
        None
    }
}

/// Build image generation prompt with Chinese descriptions
fn build_image_prompt(
    page_content: &str,
    style_content: Option<&str>,
    page_type: &str,
    _layout_family: Option<&str>,
    style_guide: Option<&crate::style_guide::StyleGuide>,
    has_reference_images: bool,
) -> String {
    let mut prompt_parts = Vec::new();
    
    // 如果有参考图，强化模板遵循要求
    if has_reference_images {
        prompt_parts.push("【核心任务】这是一张模板图片，你需要在此基础上生成新的页面。".to_string());
        prompt_parts.push("".to_string());
        prompt_parts.push("【必须严格遵守】参考图片中的以下元素是模板固有元素，必须保持原样不变：".to_string());
        prompt_parts.push("1. 所有 Logo、品牌标识、商标 - 位置、大小、颜色完全不变".to_string());
        prompt_parts.push("2. 背景色块、装饰图形、边框线条 - 形状、位置、颜色完全不变".to_string());
        prompt_parts.push("3. 标题区域的位置和样式 - 仅替换标题文字，不改变样式".to_string());
        prompt_parts.push("4. 页眉页脚、导航栏等固定区域 - 完全不变".to_string());
        prompt_parts.push("5. 整体配色方案 - 主色、辅色、背景色完全一致".to_string());
        prompt_parts.push("".to_string());
        prompt_parts.push("【允许修改】只有内容区域的文字和数据可以更新。".to_string());
        prompt_parts.push("".to_string());
    }
    
    // 如果有风格指南，使用增强版提示词
    if let Some(guide) = style_guide {
        // 风格锚点
        if !guide.prompt_anchor.is_empty() {
            prompt_parts.push(format!("风格基调：{}", guide.prompt_anchor));
        }
        
        // 配色
        if !guide.style_core.palette.is_empty() {
            prompt_parts.push(format!("配色方案：{}", guide.style_core.palette.join("、")));
        }
    } else if let Some(style) = style_content {
        // 向后兼容：使用原始风格 Markdown
        prompt_parts.push(format!("风格要求：\n{}", style));
    }
    
    // 页面类型描述
    let page_type_desc = match page_type {
        "front-cover" => "\n页面类型：封面页（保持封面标题区、副标题区、装饰元素的布局不变）",
        "back-cover" => "\n页面类型：封底页（保持致谢区、联系方式区的布局不变）",
        _ => "\n页面类型：内容页（保持内容区域的卡片布局、图表样式不变）",
    };
    prompt_parts.push(page_type_desc.to_string());
    
    // 过滤掉讲稿备注行
    let filtered_content: String = page_content
        .lines()
        .filter(|line| {
            let trimmed = line.trim();
            !trimmed.starts_with("**讲稿备注**") && !trimmed.starts_with("**讲稿**")
        })
        .collect::<Vec<_>>()
        .join("\n");
    
    // 页面内容
    prompt_parts.push(format!("\n需要更新的内容：\n{}", filtered_content));
    
    // 最终强调
    if has_reference_images {
        prompt_parts.push("".to_string());
        prompt_parts.push("【最终检查】生成前请确认：模板固有元素是否保持不变？只修改了内容区域的文字？".to_string());
    }
    
    prompt_parts.join("\n")
}

/// Generate image without template (text-to-image)
async fn generate_single_image(
    prompt: &str,
    width: u32,
    height: u32,
    config: &ImageConfig,
    output_path: &PathBuf,
) -> Result<(), String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;
    
    // Build request body - handle different API formats
    let request_body = if config.provider == "dashscope" {
        // DashScope (阿里云) format
        serde_json::json!({
            "model": config.model,
            "input": {
                "prompt": prompt,
            },
            "parameters": {
                "size": format!("{}x{}", width, height),
                "n": 1,
            }
        })
    } else {
        // OpenAI-compatible format (no response_format needed)
        serde_json::json!({
            "model": config.model,
            "prompt": prompt,
            "n": 1,
            "size": format!("{}x{}", width, height),
        })
    };
    
    let mut request = client
        .post(&config.endpoint)
        .header("Authorization", format!("Bearer {}", config.api_key))
        .header("Content-Type", "application/json");
    
    // Add extra headers
    for header in &config.extra_headers {
        request = request.header(&header.key, &header.value);
    }
    
    let response = request
        .json(&request_body)
        .send()
        .await
        .map_err(|e| format!("Image generation API request failed: {}", e))?;
    
    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_default();
        return Err(format!("Image generation API error (HTTP {}): {}", status, error_text));
    }
    
    let json: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse API response: {}", e))?;
    
    // Try to extract image URL from different response formats
    let image_url = if config.provider == "dashscope" {
        // DashScope format
        json["output"]["results"][0]["url"]
            .as_str()
            .ok_or("Failed to extract image URL from DashScope response")?
            .to_string()
    } else {
        // OpenAI-compatible format - try URL first, then base64
        if let Some(url) = json["data"][0]["url"].as_str() {
            url.to_string()
        } else if let Some(b64) = json["data"][0]["b64_json"].as_str() {
            // Handle base64 response - decode and save directly
            let image_bytes = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, b64)
                .map_err(|e| format!("Failed to decode base64 image: {}", e))?;
            tokio::fs::write(output_path, &image_bytes)
                .await
                .map_err(|e| format!("Failed to save image: {}", e))?;
            return Ok(());
        } else {
            return Err(format!("Failed to extract image from response. Response keys: {:?}", 
                json.as_object().map(|m| m.keys().collect::<Vec<_>>())));
        }
    };
    
    // Download and save the image
    let image_response = reqwest::get(image_url)
        .await
        .map_err(|e| format!("Failed to download image: {}", e))?;
    
    let image_bytes = image_response
        .bytes()
        .await
        .map_err(|e| format!("Failed to read image data: {}", e))?;
    
    tokio::fs::write(output_path, &image_bytes)
        .await
        .map_err(|e| format!("Failed to save image: {}", e))?;
    
    Ok(())
}

/// Generate image with template as reference (using /images/edits endpoint for GPT-Image)
async fn generate_image_with_template(
    prompt: &str,
    width: u32,
    height: u32,
    config: &ImageConfig,
    output_path: &PathBuf,
    template_path: &PathBuf,
) -> Result<(), String> {
    // Read template image
    let template_data = tokio::fs::read(template_path)
        .await
        .map_err(|e| format!("Failed to read template image: {}", e))?;
    
    let model_lower = config.model.to_lowercase();
    let is_gpt_image = model_lower.contains("gpt");
    let is_agnes = model_lower.starts_with("agnes");
    
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;
    
    if is_gpt_image {
        // GPT-Image: use /images/edits endpoint with multipart/form-data
        let edit_endpoint = config.endpoint.replace("/generations", "/edits");
        
        // Build multipart form
        let form = reqwest::multipart::Form::new()
            .text("model", config.model.clone())
            .text("prompt", prompt.to_string())
            .text("size", format!("{}x{}", width, height))
            .text("n", "1")
            .part("image[]", reqwest::multipart::Part::bytes(template_data)
                .file_name("template.png")
                .mime_str("image/png")
                .map_err(|e| format!("Failed to set mime type: {}", e))?);
        
        let mut request = client
            .post(&edit_endpoint)
            .header("Authorization", format!("Bearer {}", config.api_key));
        
        // Add extra headers
        for header in &config.extra_headers {
            request = request.header(&header.key, &header.value);
        }
        
        let response = request
            .multipart(form)
            .send()
            .await
            .map_err(|e| format!("Image generation API request failed: {}", e))?;
        
        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(format!("Image generation API error (HTTP {}): {}", status, error_text));
        }
        
        let json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse API response: {}", e))?;
        
        // Handle response
        handle_image_response(&json, output_path).await?;
        Ok(())
    } else if is_agnes {
        // Agnes models: use extra_body.image parameter with data URI
        let base64_template = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &template_data);
        let data_uri = format!("data:image/png;base64,{}", base64_template);
        
        let request_body = serde_json::json!({
            "model": config.model,
            "prompt": prompt,
            "size": format!("{}x{}", width, height),
            "extra_body": {
                "image": [data_uri],
                "response_format": "b64_json"
            }
        });
        
        let mut request = client
            .post(&config.endpoint)
            .header("Authorization", format!("Bearer {}", config.api_key))
            .header("Content-Type", "application/json");
        
        // Add extra headers
        for header in &config.extra_headers {
            request = request.header(&header.key, &header.value);
        }
        
        let response = request
            .json(&request_body)
            .send()
            .await
            .map_err(|e| format!("Image generation API request failed: {}", e))?;
        
        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(format!("Image generation API error (HTTP {}): {}", status, error_text));
        }
        
        let json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse API response: {}", e))?;
        
        // Handle response
        handle_image_response(&json, output_path).await?;
        Ok(())
    } else {
        // Other APIs: use /images/generations with JSON body
        let base64_template = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &template_data);
        let data_uri = format!("data:image/png;base64,{}", base64_template);
        
        let request_body = serde_json::json!({
            "model": config.model,
            "prompt": prompt,
            "image": [data_uri],
            "size": format!("{}x{}", width, height),
            "n": 1,
        });
        
        let mut request = client
            .post(&config.endpoint)
            .header("Authorization", format!("Bearer {}", config.api_key))
            .header("Content-Type", "application/json");
        
        // Add extra headers
        for header in &config.extra_headers {
            request = request.header(&header.key, &header.value);
        }
        
        let response = request
            .json(&request_body)
            .send()
            .await
            .map_err(|e| format!("Image generation API request failed: {}", e))?;
        
        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(format!("Image generation API error (HTTP {}): {}", status, error_text));
        }
        
        let json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse API response: {}", e))?;
        
        // Handle response
        handle_image_response(&json, output_path).await?;
        Ok(())
    }
}

/// Handle image response from API (URL or base64)
async fn handle_image_response(json: &serde_json::Value, output_path: &PathBuf) -> Result<(), String> {
    // Try URL first, then base64
    let image_url = if let Some(url) = json["data"][0]["url"].as_str() {
        url.to_string()
    } else if let Some(b64) = json["data"][0]["b64_json"].as_str() {
        // Handle base64 response - decode and save directly
        let image_bytes = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, b64)
            .map_err(|e| format!("Failed to decode base64 image: {}", e))?;
        tokio::fs::write(output_path, &image_bytes)
            .await
            .map_err(|e| format!("Failed to save image: {}", e))?;
        return Ok(());
    } else {
        return Err(format!("Failed to extract image from response. Response keys: {:?}", 
            json.as_object().map(|m| m.keys().collect::<Vec<_>>())));
    };
    
    // Download and save the image
    let image_response = reqwest::get(&image_url)
        .await
        .map_err(|e| format!("Failed to download image: {}", e))?;
    
    let image_bytes = image_response
        .bytes()
        .await
        .map_err(|e| format!("Failed to read image data: {}", e))?;
    
    tokio::fs::write(output_path, &image_bytes)
        .await
        .map_err(|e| format!("Failed to save image: {}", e))?;
    
    Ok(())
}
