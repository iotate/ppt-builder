//! 图像处理模块
//! 
//! 实现 ai-ppt-maker 的元素图生成和图像分割功能：
//! 1. 元素图生成：通过 Images Edits API 去除文字
//! 2. 图像分割：基于 alpha 通道的连通域检测，分割出独立元素

#[allow(dead_code)]
pub mod alpha;
#[allow(dead_code)]
pub mod components;
#[allow(dead_code)]
pub mod splitter;
#[allow(dead_code)]
pub mod elements_gen;

// 内部使用，暂不导出
