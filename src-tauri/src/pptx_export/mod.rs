//! PPTX 导出模块
//!
//! 提供三种导出模式：
//! 1. 简单导出：将图片按顺序组装为一个 PPT
//! 2. 可编辑导出：基于 OCR 识别，生成可编辑的 PPT
//! 3. 基于模板导出：使用模板 PPTX，保留母版和主题，生成可编辑的 PPT

pub mod simple;
pub mod editable;
pub mod template_based;
