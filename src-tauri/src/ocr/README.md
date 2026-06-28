# OCR 模块

基于 rust-paddle-ocr (ocr-rs crate) 的 OCR 文字识别模块。

## 功能

### 1. OCR 文字识别 (`engine.rs`)

- 使用 PaddleOCR v6 模型
- 基于 MNN 推理框架
- 支持中文、英文等多种语言
- 高性能，本地运行，无需网络

### 2. 文本遮罩 (`text_mask.rs`)

基于 OCR 结果去除图片中的文字：
- 使用 OCR bbox 精确定位文字区域
- 使用周围像素颜色填充
- 生成干净的背景图片

### 3. 布局恢复 (`layout_recovery.rs`)

从 OCR 结果生成文本布局清单：
- 推断文字角色（title, body, label 等）
- 估算字号、颜色
- 生成 `text_layout_manifest.json`
- 生成可编辑性报告 `editability_report.json`

## 模型文件

模型文件位于 `src-tauri/models/` 目录：

```
src-tauri/models/
├── PP-OCRv6_small_det.mnn    # 文本检测模型
├── PP-OCRv6_small_rec.mnn    # 文本识别模型
└── ppocr_keys_v6_small.txt   # 字符集文件
```

### 下载模型

如果模型文件缺失，可以从以下地址下载：

- [PaddleOCR 官方模型库](https://github.com/PaddlePaddle/PaddleOCR/blob/main/doc/doc_ch/models_list.md)
- [rust-paddle-ocr 模型](https://github.com/zibo-chen/rust-paddle-ocr/tree/next/models)

## 使用流程

1. 对页面图片执行 OCR
2. 基于 OCR 结果去除文字
3. 推断文字样式
4. 生成可编辑 PPTX

## 输出文件

每个页面处理后会生成以下文件：

```
pptx/
├── page_01/
│   ├── ocr_results.json          # OCR 识别结果
│   ├── ocr_overlay_debug.png     # OCR 调试图（显示文字框）
│   ├── clean_background.png      # 去除文字后的干净背景
│   ├── page_01_mask_debug.png    # 文本遮罩调试图
│   ├── text_layout_manifest.json # 文本布局清单
│   └── editability_report.json   # 可编辑性报告
├── text_layout_manifest.json     # 合并的布局清单
├── editability_report.json       # 合并的可编辑性报告
└── {project}-editable.pptx       # 最终输出的 PPTX 文件
```

## 参考

- [rust-paddle-ocr](https://github.com/zibo-chen/rust-paddle-ocr) - Rust OCR 库
- [PaddleOCR](https://github.com/PaddlePaddle/PaddleOCR) - 百度开源 OCR 系统
- [ppt-to-editable](../../ppt-to-editable/SKILL.md) - 可编辑 PPT 方案
