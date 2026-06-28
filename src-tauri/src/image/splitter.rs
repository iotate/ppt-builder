//! 元素分割
//! 
//! 参考 ai-ppt-maker 的 splitter.py 实现
//! 将透明 PNG 分割为独立的元素资源

use std::path::Path;
use std::fs::{self, File};
use std::io::BufWriter;
use serde::{Deserialize, Serialize};
use image::{ImageBuffer, Rgba, ImageEncoder};

use super::components::{find_components, grow_mask_from_seed, Component};

/// 分割出的元素资产
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElementAsset {
    pub index: usize,
    pub file: String,
    pub left: usize,
    pub top: usize,
    pub width: usize,
    pub height: usize,
    pub area: usize,
}

/// 分割结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SplitResult {
    pub source_image: String,
    pub assets_dir: String,
    pub image_width: usize,
    pub image_height: usize,
    pub alpha_threshold: u8,
    pub alpha_core_threshold: u8,
    pub min_area: usize,
    pub count: usize,
    pub assets: Vec<ElementAsset>,
}

/// 分割透明 PNG 为独立元素
pub fn split_elements(
    image_path: &Path,
    out_dir: &Path,
    alpha_threshold: u8,
    alpha_core_threshold: u8,
    min_area: usize,
    padding: usize,
) -> Result<SplitResult, String> {
    // 读取图像
    let img = image::open(image_path)
        .map_err(|e| format!("Failed to open image: {}", e))?;
    
    let rgba_img = img.to_rgba8();
    let (width, height) = rgba_img.dimensions();
    let width = width as usize;
    let height = height as usize;

    // 提取 alpha 通道
    let alpha: Vec<u8> = rgba_img.pixels().map(|p| p[3]).collect();

    // 构建 visual mask (alpha > threshold)
    let visual_mask: Vec<bool> = alpha.iter().map(|&a| a > alpha_threshold).collect();

    // 构建 core mask (alpha > core_threshold)
    let core_threshold = (alpha_threshold + 1).max(alpha_core_threshold);
    let core_mask: Vec<bool> = alpha.iter().map(|&a| a > core_threshold).collect();

    // 查找连通域 (使用 4 连接)
    let components = find_components(&core_mask, width, height, 4);

    // 过滤并排序组件
    let mut filtered: Vec<Component> = components
        .into_iter()
        .filter(|c| c.area >= min_area)
        .collect();

    // 按位置排序：从上到下，从左到右
    filtered.sort_by(|a, b| {
        (a.top, a.left, a.width(), a.height())
            .cmp(&(b.top, b.left, b.width(), b.height()))
    });

    // 准备输出目录
    fs::create_dir_all(out_dir)
        .map_err(|e| format!("Failed to create output directory: {}", e))?;

    let mut assets = Vec::new();

    for (index, component) in filtered.iter().enumerate() {
        let index = index + 1;

        // 扩展到 visual mask
        let visual_component_mask = expand_component_to_visual_mask(
            component,
            &visual_mask,
            width,
            height,
        );

        // 计算裁剪区域
        let raw_left = component.left;
        let raw_top = component.top;
        let raw_right = component.right;
        let raw_bottom = component.bottom;

        let visual_left = component.left; // 简化处理
        let visual_top = component.top;

        let left = raw_left.saturating_sub(padding);
        let top = raw_top.saturating_sub(padding);
        let right = (raw_right + padding).min(width);
        let bottom = (raw_bottom + padding).min(height);

        let crop_width = right - left;
        let crop_height = bottom - top;

        // 创建裁剪图像
        let mut crop_img: ImageBuffer<Rgba<u8>, Vec<u8>> = 
            ImageBuffer::new(crop_width as u32, crop_height as u32);

        // 复制像素
        for y in 0..crop_height {
            for x in 0..crop_width {
                let src_x = left + x;
                let src_y = top + y;
                let _src_idx = src_y * width + src_x;
                let pixel = rgba_img.get_pixel(src_x as u32, src_y as u32);

                // 只保留当前连通域的像素
                let mask_y = src_y.saturating_sub(visual_top);
                let mask_x = src_x.saturating_sub(visual_left);
                let in_mask = mask_y < visual_component_mask.len() / component.mask_width
                    && mask_x < component.mask_width
                    && visual_component_mask.get(mask_y * component.mask_width + mask_x)
                        .map(|&v| v)
                        .unwrap_or(false);

                let mut pixel = *pixel;
                if !in_mask {
                    pixel[3] = 0; // 透明
                }
                crop_img.put_pixel(x as u32, y as u32, pixel);
            }
        }

        // 保存文件
        let filename = format!("asset_{:03}.png", index);
        let file_path = out_dir.join(&filename);

        let file = File::create(&file_path)
            .map_err(|e| format!("Failed to create asset file: {}", e))?;
        let writer = BufWriter::new(file);

        image::codecs::png::PngEncoder::new(writer)
            .write_image(
                crop_img.as_raw(),
                crop_width as u32,
                crop_height as u32,
                image::ExtendedColorType::Rgba8,
            )
            .map_err(|e| format!("Failed to encode PNG: {}", e))?;

        assets.push(ElementAsset {
            index,
            file: filename,
            left,
            top,
            width: crop_width,
            height: crop_height,
            area: component.area,
        });
    }

    let result = SplitResult {
        source_image: image_path.to_string_lossy().to_string(),
        assets_dir: out_dir.to_string_lossy().to_string(),
        image_width: width,
        image_height: height,
        alpha_threshold,
        alpha_core_threshold,
        min_area,
        count: assets.len(),
        assets,
    };

    // 保存 manifest
    let manifest_path = out_dir.join("assets.json");
    let manifest_content = serde_json::to_string_pretty(&result)
        .map_err(|e| format!("Failed to serialize manifest: {}", e))?;
    fs::write(&manifest_path, manifest_content)
        .map_err(|e| format!("Failed to write manifest: {}", e))?;

    Ok(result)
}

/// 扩展组件到 visual mask
fn expand_component_to_visual_mask(
    component: &Component,
    visual_mask: &[bool],
    width: usize,
    height: usize,
) -> Vec<bool> {
    // 构建种子 mask
    let seed_left = component.left.saturating_sub(1);
    let seed_top = component.top.saturating_sub(1);
    let seed_right = (component.right + 1).min(width);
    let seed_bottom = (component.bottom + 1).min(height);
    let seed_width = seed_right - seed_left;
    let seed_height = seed_bottom - seed_top;

    let mut local_visual = vec![false; seed_width * seed_height];
    let mut local_seed = vec![false; seed_width * seed_height];

    // 填充 local_visual
    for y in seed_top..seed_bottom {
        for x in seed_left..seed_right {
            let idx = y * width + x;
            let local_idx = (y - seed_top) * seed_width + (x - seed_left);
            local_visual[local_idx] = visual_mask[idx];
        }
    }

    // 填充 local_seed
    for y in component.top..component.bottom {
        for x in component.left..component.right {
            let comp_idx = (y - component.top) * component.mask_width + (x - component.left);
            if component.mask[comp_idx] {
                let local_idx = (y - seed_top) * seed_width + (x - seed_left);
                local_seed[local_idx] = true;
            }
        }
    }

    // 扩展
    let expanded = grow_mask_from_seed(&local_visual, &local_seed, seed_width, seed_height, 8);

    // 返回扩展后的 mask
    expanded
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_result_serialization() {
        let result = SplitResult {
            source_image: "test.png".to_string(),
            assets_dir: "assets".to_string(),
            image_width: 100,
            image_height: 100,
            alpha_threshold: 8,
            alpha_core_threshold: 48,
            min_area: 8,
            count: 0,
            assets: vec![],
        };

        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("test.png"));
    }
}
