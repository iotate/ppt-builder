pub mod llm;
pub mod imggen;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageGenerationOptions {
    pub template: Option<String>,
    pub style: Option<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageJobResult {
    pub page_num: u32,
    pub status: ImageJobStatus,
    pub output_path: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ImageJobStatus {
    Success,
    Generating,
    Failed(String),
}
