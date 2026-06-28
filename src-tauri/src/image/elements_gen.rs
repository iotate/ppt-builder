//! 元素图生成
//! 
//! 通过 Images Edits API 生成去文字后的元素图
//! 参考 ai-ppt-maker 的 openai_image_provider.py 中的 generate_elements_page 实现

use std::path::Path;
use std::fs::File;
use std::io::Read;
use serde::{Deserialize, Serialize};

/// 元素图生成结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElementsImageResult {
    pub output_path: String,
    pub provider: String,
    pub model: String,
    pub revised_prompt: Option<String>,
}

/// 生成元素图（去文字版本）
/// 
/// 使用 Images Edits API，以 reference_page 为输入
/// prompt: "remove all text, keep visual elements only"
pub async fn generate_elements_image(
    reference_page_path: &Path,
    output_path: &Path,
    api_key: &str,
    api_base_url: &str,
    model: &str,
) -> Result<ElementsImageResult, String> {
    // 读取参考图
    let mut image_data = Vec::new();
    let mut file = File::open(reference_page_path)
        .map_err(|e| format!("Failed to open reference image: {}", e))?;
    file.read_to_end(&mut image_data)
        .map_err(|e| format!("Failed to read reference image: {}", e))?;

    // 确定 MIME 类型
    let ext = reference_page_path
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_else(|| "png".to_string());
    
    let mime_type = match ext.as_str() {
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        _ => "image/png",
    };

    // 构建 multipart form
    let client = reqwest::Client::new();
    
    let part = reqwest::multipart::Part::bytes(image_data)
        .file_name(reference_page_path.file_name().unwrap().to_string_lossy().to_string())
        .mime_str(mime_type)
        .map_err(|e| format!("Failed to set mime type: {}", e))?;

    let form = reqwest::multipart::Form::new()
        .part("image", part)
        .text("model", model.to_string())
        .text("prompt", "remove all text, keep visual elements only")
        .text("size", "2048x1152")
        .text("n", "1")
        .text("background", "opaque")
        .text("response_format", "url");

    // 发送请求
    let url = format!("{}/images/edits", api_base_url.trim_end_matches('/'));
    
    let response = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", api_key))
        .multipart(form)
        .send()
        .await
        .map_err(|e| format!("API request failed: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        return Err(format!("API error {}: {}", status, text));
    }

    let json: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse API response: {}", e))?;

    // 提取图片 URL 或 base64
    let image_url = json["data"][0]["url"]
        .as_str()
        .or_else(|| json["data"][0]["b64_json"].as_str())
        .ok_or("No image data in response")?;

    // 检查是 URL 还是 base64
    if image_url.starts_with("http") {
        // 下载图片
        let image_response = client
            .get(image_url)
            .send()
            .await
            .map_err(|e| format!("Failed to download image: {}", e))?;

        let image_bytes = image_response
            .bytes()
            .await
            .map_err(|e| format!("Failed to read image bytes: {}", e))?;

        // 保存图片
        std::fs::write(output_path, &image_bytes)
            .map_err(|e| format!("Failed to save image: {}", e))?;
    } else {
        // 解码 base64
        let decoded = base64::Engine::decode(
            &base64::engine::general_purpose::STANDARD,
            image_url,
        ).map_err(|e| format!("Failed to decode base64: {}", e))?;

        std::fs::write(output_path, &decoded)
            .map_err(|e| format!("Failed to save image: {}", e))?;
    }

    let revised_prompt = json["data"][0]["revised_prompt"]
        .as_str()
        .map(|s| s.to_string());

    Ok(ElementsImageResult {
        output_path: output_path.to_string_lossy().to_string(),
        provider: "openai_compatible".to_string(),
        model: model.to_string(),
        revised_prompt,
    })
}

/// 使用本地图像处理生成元素图（当 API 不可用时的备用方案）
/// 
/// 简单地将 alpha 通道低于阈值的区域视为可能的文字区域
pub fn generate_elements_image_local(
    reference_page_path: &Path,
    output_path: &Path,
    _alpha_threshold: u8,
) -> Result<String, String> {
    let img = image::open(reference_page_path)
        .map_err(|e| format!("Failed to open image: {}", e))?;

    let mut rgba_img = img.to_rgba8();

    // 简单处理：检测高对比度文字区域并移除
    // 这是一个简化的实现，实际效果不如 AI 生成
    for pixel in rgba_img.pixels_mut() {
        let r = pixel[0];
        let g = pixel[1];
        let b = pixel[2];

        // 检测是否可能是文字（高对比度、颜色单一）
        let is_likely_text = {
            let max = r.max(g).max(b);
            let min = r.min(g).min(b);
            let contrast = max as i32 - min as i32;
            
            // 高对比度且颜色较深或较浅的区域可能是文字
            contrast > 150 && (max < 50 || min > 200)
        };

        if is_likely_text {
            // 将可能的文字区域变透明
            pixel[3] = pixel[3].saturating_sub(128);
        }
    }

    // 保存图片
    rgba_img.save(output_path)
        .map_err(|e| format!("Failed to save image: {}", e))?;

    Ok(output_path.to_string_lossy().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_elements_result_serialization() {
        let result = ElementsImageResult {
            output_path: "elements.png".to_string(),
            provider: "openai".to_string(),
            model: "gpt-image-1".to_string(),
            revised_prompt: Some("test prompt".to_string()),
        };

        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("elements.png"));
    }
}
