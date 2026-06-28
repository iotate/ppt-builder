//! OCR 模块
//! 
//! 提供文字识别和文本定位功能
//! 支持区域感知的文字去除和精确的颜色分析

#[allow(dead_code)]
mod engine;
#[allow(dead_code)]
mod text_mask;
mod layout_recovery;
mod color_analysis;
mod native_shapes;
mod calibration;

pub use engine::*;
pub use text_mask::*;
pub use layout_recovery::*;
#[allow(unused_imports)]
pub use color_analysis::ColorAnalysis;
pub use native_shapes::*;
pub use calibration::*;

/// OCR 文字识别结果
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct OcrResult {
    /// 唯一标识
    pub id: String,
    /// 识别的文字
    pub text: String,
    /// 置信度 (0.0 - 1.0)
    pub confidence: f32,
    /// 文字区域的多边形顶点（像素坐标）
    pub polygon_px: Vec<(f32, f32)>,
    /// 包围盒 [x, y, width, height]（像素坐标）
    pub bbox_px: [f32; 4],
    /// 文字角色（title, subtitle, body, label 等）
    #[serde(default)]
    pub role: Option<String>,
    /// 审核状态
    #[serde(default = "default_review_status")]
    pub review_status: String,
}

fn default_review_status() -> String {
    "pending".to_string()
}

/// OCR 处理结果
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct OcrPageResult {
    /// 源图片路径
    pub source: String,
    /// 图片尺寸 [width, height]
    pub image_size_px: [u32; 2],
    /// OCR 引擎名称
    pub engine: String,
    /// 处理耗时（毫秒）
    pub elapsed_ms: u64,
    /// 识别到的文字列表
    pub records: Vec<OcrResult>,
}

/// 文本布局清单（用于 PPTX 生成）
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TextLayoutManifest {
    /// 标题
    pub title: String,
    /// 幻灯片尺寸
    pub slide_size: SlideSize,
    /// 单位
    #[serde(default = "default_units")]
    pub units: String,
    /// 幻灯片列表
    pub slides: Vec<SlideTextLayout>,
}

fn default_units() -> String {
    "inches".to_string()
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SlideSize {
    pub width: f64,
    pub height: f64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SlideTextLayout {
    /// 幻灯片编号
    pub slide: u32,
    /// 源图片路径
    pub source_image: String,
    /// 干净背景图片路径（文字已去除）
    pub background: Option<String>,
    /// 文本框列表
    pub text_boxes: Vec<TextBoxLayout>,
}

/// 文本框布局
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TextBoxLayout {
    /// 角色
    pub role: Option<String>,
    /// 名称
    pub name: Option<String>,
    /// 文本内容
    pub text: String,
    /// 位置 X（英寸）
    pub x: f64,
    /// 位置 Y（英寸）
    pub y: f64,
    /// 宽度（英寸）
    pub w: f64,
    /// 高度（英寸）
    pub h: f64,
    /// 字号
    pub font_size: Option<f64>,
    /// 字体
    pub font_face: Option<String>,
    /// 东亚字体
    pub east_asian_font: Option<String>,
    /// 是否粗体
    #[serde(default)]
    pub bold: bool,
    /// 是否斜体
    #[serde(default)]
    pub italic: bool,
    /// 颜色（HEX 格式，不含 #）
    pub color: Option<String>,
    /// 对齐方式
    pub align: Option<String>,
    /// 垂直对齐
    pub valign: Option<String>,
    /// OCR ID
    pub ocr_id: Option<String>,
    /// OCR 置信度
    pub ocr_confidence: Option<f32>,
    /// 源包围盒（像素）
    pub source_bbox_px: Option<[f32; 4]>,
    /// 跟踪级别
    #[serde(default = "default_trace_level")]
    pub trace_level: String,
    /// 是否禁止换行
    #[serde(default = "default_no_wrap")]
    pub no_wrap: bool,
    /// 审核状态
    pub review_status: Option<String>,
}

fn default_trace_level() -> String {
    "line".to_string()
}

fn default_no_wrap() -> bool {
    true
}

/// 可编辑性报告
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EditabilityReport {
    /// 幻灯片编号
    pub slide: u32,
    /// 可编辑文本框数量
    pub editable_text_bodies: u32,
    /// 接受的 OCR 行数
    pub accepted_ocr_lines: u32,
    /// 省略的 OCR 行数
    pub omitted_ocr_lines: u32,
    /// OCR 修正数量
    pub ocr_corrections: u32,
    /// 原生形状数量
    pub native_shapes: u32,
    /// 原生线条数量
    pub native_lines: u32,
    /// 图片/裁剪数量
    pub pictures: u32,
    /// 问题列表
    pub issues: Vec<String>,
    /// 已知限制
    pub limitations: Vec<String>,
}

/// 页面数据（用于 PPTX 导出）
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PageData {
    /// 页码
    pub page_num: u32,
    /// 标题
    pub title: String,
    /// Markdown 内容
    pub markdown: String,
    /// 图片路径
    pub image_path: std::path::PathBuf,
}
