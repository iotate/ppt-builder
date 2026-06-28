use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tauri::State;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ExtraHeader {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiConfig {
    pub provider: String,
    pub endpoint: String,
    pub api_key: String,
    pub model: String,
    #[serde(default)]
    pub extra_headers: Vec<ExtraHeader>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageSize {
    pub name: String,
    pub width: u32,
    pub height: u32,
}

/// 图像生成配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageConfig {
    pub provider: String,
    pub endpoint: String,
    pub api_key: String,
    pub model: String,
    #[serde(default)]
    pub extra_headers: Vec<ExtraHeader>,
}

/// 导出给其他模块使用的配置类型别名
pub type ImgConfig = ImageConfig;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub llm: ApiConfig,
    pub img: ImageConfig,
    pub image_sizes: Vec<ImageSize>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            llm: ApiConfig {
                provider: "openai".to_string(),
                endpoint: "https://api.openai.com/v1".to_string(),
                api_key: String::new(),
                model: "gpt-4o".to_string(),
                extra_headers: Vec::new(),
            },
            img: ImgConfig {
                provider: "openai".to_string(),
                endpoint: "https://api.openai.com/v1/images/generations".to_string(),
                api_key: String::new(),
                model: "gpt-image-2".to_string(),
                extra_headers: Vec::new(),
            },
            image_sizes: vec![
                ImageSize { name: "16:9 横屏".to_string(), width: 1920, height: 1072 },
                ImageSize { name: "9:16 竖屏".to_string(), width: 1072, height: 1920 },
                ImageSize { name: "4:3 横向".to_string(), width: 1440, height: 1072 },
                ImageSize { name: "3:4 纵向".to_string(), width: 1072, height: 1440 },
                ImageSize { name: "1:1 方形".to_string(), width: 1072, height: 1072 },
            ],
        }
    }
}

fn get_config_path(cwd: &PathBuf) -> PathBuf {
    cwd.join("config.yaml")
}

#[tauri::command]
pub async fn load_config(cwd: State<'_, Arc<PathBuf>>) -> Result<AppConfig, String> {
    let config_path = get_config_path(&cwd);
    
    if !config_path.exists() {
        return Ok(AppConfig::default());
    }
    
    let content = tokio::fs::read_to_string(&config_path)
        .await
        .map_err(|e| format!("Failed to read config: {}", e))?;
    
    let config: AppConfig = serde_yaml::from_str(&content)
        .map_err(|e| format!("Failed to parse config: {}", e))?;
    
    Ok(config)
}

/// Format YAML config with blank lines between sections
fn format_config_yaml(config: &AppConfig) -> Result<String, String> {
    // Serialize each section separately
    let llm_yaml = serde_yaml::to_string(&config.llm)
        .map_err(|e| format!("Failed to serialize llm config: {}", e))?;
    
    let img_yaml = serde_yaml::to_string(&config.img)
        .map_err(|e| format!("Failed to serialize img config: {}", e))?;
    
    let sizes_yaml = serde_yaml::to_string(&config.image_sizes)
        .map_err(|e| format!("Failed to serialize image_sizes: {}", e))?;
    
    // Combine with proper formatting
    let mut result = String::new();
    
    result.push_str("llm:\n");
    // Indent llm content (skip the first line since we already added "llm:")
    for line in llm_yaml.lines() {
        result.push_str(&format!("  {}\n", line));
    }
    
    result.push('\n'); // Blank line between sections
    
    result.push_str("img:\n");
    for line in img_yaml.lines() {
        result.push_str(&format!("  {}\n", line));
    }
    
    result.push('\n'); // Blank line between sections
    
    result.push_str("image_sizes:\n");
    for line in sizes_yaml.lines() {
        result.push_str(&format!("  {}\n", line));
    }
    
    Ok(result)
}

#[tauri::command]
pub async fn save_config(cwd: State<'_, Arc<PathBuf>>, config: AppConfig) -> Result<(), String> {
    let config_path = get_config_path(&cwd);
    
    let content = format_config_yaml(&config)?;
    
    tokio::fs::write(&config_path, content)
        .await
        .map_err(|e| format!("Failed to write config: {}", e))?;
    
    Ok(())
}

#[tauri::command]
pub async fn test_llm_connection(
    cwd: State<'_, Arc<PathBuf>>,
    config: ApiConfig,
) -> Result<bool, String> {
    let client = reqwest::Client::new();
    
    let mut request = client
        .get(format!("{}/models", config.endpoint))
        .header("Authorization", format!("Bearer {}", config.api_key));
    
    // Add extra headers
    for header in &config.extra_headers {
        request = request.header(&header.key, &header.value);
    }
    
    let response = request
        .send()
        .await
        .map_err(|e| format!("Connection failed: {}", e))?;
    
    let success = response.status().is_success() || response.status().as_u16() == 404 || response.status().as_u16() == 403;
    
    // If connection successful, save the config
    if success {
        let mut current_config = load_config(cwd.clone()).await?;
        current_config.llm = config;
        save_config(cwd, current_config).await?;
    }
    
    Ok(success)
}

/// 同步加载配置（供其他模块使用）
pub fn load_config_sync(cwd: &PathBuf) -> Result<AppConfig, String> {
    let config_path = get_config_path(cwd);
    
    if !config_path.exists() {
        return Ok(AppConfig::default());
    }
    
    let content = std::fs::read_to_string(&config_path)
        .map_err(|e| format!("Failed to read config: {}", e))?;
    
    let config: AppConfig = serde_yaml::from_str(&content)
        .map_err(|e| format!("Failed to parse config: {}", e))?;
    
    Ok(config)
}

#[tauri::command]
pub async fn test_img_connection(
    cwd: State<'_, Arc<PathBuf>>,
    config: ImageConfig,
) -> Result<bool, String> {
    let client = reqwest::Client::new();
    
    // For image generation APIs, we can't easily test without generating an image
    // So we'll just try to connect to the base endpoint
    let base_url = config.endpoint
        .replace("/generations", "")
        .trim_end_matches('/')
        .to_string();
    
    // Try to make a simple request to verify the endpoint is reachable
    let response = client
        .get(&format!("{}/models", base_url))
        .header("Authorization", format!("Bearer {}", config.api_key))
        .send()
        .await
        .map_err(|e| format!("Connection failed: {}", e))?;
    
    // Some APIs might not have a /models endpoint, so we just check if we got any response
    // For agnes-ai and similar, even a 404 means the endpoint is reachable
    let status = response.status();
    let success = status.is_success() || status.as_u16() == 404 || status.as_u16() == 403;
    
    // If connection successful, save the config
    if success {
        let mut current_config = load_config(cwd.clone()).await?;
        current_config.img = config;
        save_config(cwd, current_config).await?;
    }
    
    Ok(success)
}
