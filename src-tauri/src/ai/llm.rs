use crate::config::ApiConfig;
use serde::{Deserialize, Serialize};

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMResponse {
    pub content: String,
    pub usage: Option<TokenUsage>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

#[allow(dead_code)]
pub async fn generate_text(
    prompt: String,
    system_prompt: Option<String>,
    config: &ApiConfig,
) -> Result<String, String> {
    let client = reqwest::Client::new();
    
    let mut messages = Vec::new();
    
    if let Some(system) = system_prompt {
        messages.push(serde_json::json!({
            "role": "system",
            "content": system
        }));
    }
    
    messages.push(serde_json::json!({
        "role": "user",
        "content": prompt
    }));
    
    let response = client
        .post(format!("{}/chat/completions", config.endpoint))
        .header("Authorization", format!("Bearer {}", config.api_key))
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({
            "model": config.model,
            "messages": messages,
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

    let content = json["choices"][0]["message"]["content"]
        .as_str()
        .ok_or("Failed to extract content from LLM response")?
        .to_string();

    Ok(content)
}
