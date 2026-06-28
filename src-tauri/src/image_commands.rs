//! 图像处理命令
//! 
//! 提供元素图生成和图像分割的 Tauri 命令

use std::path::PathBuf;
use std::sync::Arc;
use tauri::State;
use serde::{Deserialize, Serialize};
use crate::error_log;
use crate::config::load_config_sync;

/// 元素图生成请求
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct GenerateElementsRequest {
    pub project_name: String,
    pub page_num: u32,
}

/// 元素图生成结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerateElementsResponse {
    pub elements_image_path: String,
    pub assets_dir: String,
    pub asset_count: usize,
}

/// 生成元素图并分割元素
#[tauri::command]
pub async fn generate_page_elements(
    cwd: State<'_, Arc<PathBuf>>,
    project_name: String,
    page_num: u32,
) -> Result<GenerateElementsResponse, String> {
    let cwd_path = cwd.inner().clone();
    let project_dir = cwd.join("projects").join(&project_name);

    if !project_dir.exists() {
        let error = format!("Project not found: {}", project_name);
        error_log::log_error(&cwd_path, &error);
        return Err(error);
    }

    // 创建 pptx 目录
    let pptx_dir = project_dir.join("pptx");
    if !pptx_dir.exists() {
        tokio::fs::create_dir_all(&pptx_dir)
            .await
            .map_err(|e| format!("Failed to create pptx directory: {}", e))?;
    }

    // 查找参考图
    let reference_page_path = find_reference_page(&project_dir, page_num)?;

    // 加载配置
    let config = load_config_sync(&cwd_path)?;

    // 生成元素图
    let elements_dir = pptx_dir.join(format!("page_{:02}_elements", page_num));
    if !elements_dir.exists() {
        tokio::fs::create_dir_all(&elements_dir)
            .await
            .map_err(|e| format!("Failed to create elements directory: {}", e))?;
    }

    let elements_image_path = elements_dir.join("elements.png");
    let assets_dir = elements_dir.join("assets");

    error_log::log_info(&cwd_path, &format!(
        "Generating elements image for page {} in project {}",
        page_num, project_name
    ));

    // 调用 API 生成元素图
    let elements_result = crate::image::elements_gen::generate_elements_image(
        &reference_page_path,
        &elements_image_path,
        &config.img.api_key,
        &config.img.endpoint,
        &config.img.model,
    ).await?;

    error_log::log_info(&cwd_path, &format!(
        "Elements image generated: {}",
        elements_result.output_path
    ));

    // 分割元素
    let split_result = crate::image::splitter::split_elements(
        &elements_image_path,
        &assets_dir,
        8,  // alpha_threshold
        48, // alpha_core_threshold
        8,  // min_area
        0,  // padding
    )?;

    let response = GenerateElementsResponse {
        elements_image_path: elements_image_path.to_string_lossy().to_string(),
        assets_dir: assets_dir.to_string_lossy().to_string(),
        asset_count: split_result.count,
    };

    error_log::log_info(&cwd_path, &format!(
        "Elements split completed: {} assets",
        split_result.count
    ));

    Ok(response)
}

/// 仅生成元素图（不分割）
#[tauri::command]
pub async fn generate_elements_image_only(
    cwd: State<'_, Arc<PathBuf>>,
    project_name: String,
    page_num: u32,
) -> Result<String, String> {
    let cwd_path = cwd.inner().clone();
    let project_dir = cwd.join("projects").join(&project_name);

    if !project_dir.exists() {
        return Err(format!("Project not found: {}", project_name));
    }

    let pptx_dir = project_dir.join("pptx");
    if !pptx_dir.exists() {
        tokio::fs::create_dir_all(&pptx_dir)
            .await
            .map_err(|e| format!("Failed to create pptx directory: {}", e))?;
    }

    let reference_page_path = find_reference_page(&project_dir, page_num)?;
    let config = load_config_sync(&cwd_path)?;

    let elements_image_path = pptx_dir.join(format!("page_{:02}_elements.png", page_num));

    let result = crate::image::elements_gen::generate_elements_image(
        &reference_page_path,
        &elements_image_path,
        &config.img.api_key,
        &config.img.endpoint,
        &config.img.model,
    ).await?;

    Ok(result.output_path)
}

/// 仅分割元素图
#[tauri::command]
pub async fn split_elements_image(
    cwd: State<'_, Arc<PathBuf>>,
    project_name: String,
    page_num: u32,
    alpha_threshold: Option<u8>,
    alpha_core_threshold: Option<u8>,
    min_area: Option<usize>,
) -> Result<crate::image::splitter::SplitResult, String> {
    let cwd_path = cwd.inner().clone();
    let project_dir = cwd.join("projects").join(&project_name);

    if !project_dir.exists() {
        return Err(format!("Project not found: {}", project_name));
    }

    let pptx_dir = project_dir.join("pptx");
    let elements_image_path = pptx_dir.join(format!("page_{:02}_elements.png", page_num));

    if !elements_image_path.exists() {
        return Err(format!(
            "Elements image not found for page {}. Please generate it first.",
            page_num
        ));
    }

    let assets_dir = pptx_dir.join(format!("page_{:02}_assets", page_num));

    let result = crate::image::splitter::split_elements(
        &elements_image_path,
        &assets_dir,
        alpha_threshold.unwrap_or(8),
        alpha_core_threshold.unwrap_or(48),
        min_area.unwrap_or(8),
        0, // padding
    )?;

    error_log::log_info(&cwd_path, &format!(
        "Split completed: {} elements for page {}",
        result.count, page_num
    ));

    Ok(result)
}

/// 查找参考页面图片
fn find_reference_page(project_dir: &PathBuf, page_num: u32) -> Result<PathBuf, String> {
    let png_path = project_dir.join(format!("page-{:02}.png", page_num));
    if png_path.exists() {
        return Ok(png_path);
    }

    let jpg_path = project_dir.join(format!("page-{:02}.jpg", page_num));
    if jpg_path.exists() {
        return Ok(jpg_path);
    }

    let webp_path = project_dir.join(format!("page-{:02}.webp", page_num));
    if webp_path.exists() {
        return Ok(webp_path);
    }

    Err(format!("Reference page image not found for page {}", page_num))
}
