//! Alpha 通道处理
//! 
//! 参考 ai-ppt-maker 的 background_removal.py 实现：
//! 1. 检测和处理 alpha 通道
//! 2. 清理抠图后的毛刺和噪点

use std::collections::VecDeque;

/// Alpha 连通域
#[derive(Debug, Clone)]
pub struct AlphaComponent {
    pub pixels: Vec<(usize, usize)>,
    pub area: usize,
    pub width: usize,
    pub height: usize,
    pub max_alpha: u8,
    pub anchor_area: usize,
}

impl AlphaComponent {
    pub fn bbox_area(&self) -> usize {
        self.width.saturating_mul(self.height).max(1)
    }

    pub fn fill_ratio(&self) -> f64 {
        self.area as f64 / self.bbox_area() as f64
    }

    pub fn min_side(&self) -> usize {
        self.width.min(self.height)
    }

    pub fn max_side(&self) -> usize {
        self.width.max(self.height)
    }
}

/// 处理 alpha 通道，清理噪点和毛刺
pub fn process_alpha(
    rgba: &mut [u8],
    width: usize,
    height: usize,
    foreground_threshold: u8,
    anchor_threshold: u8,
) -> Vec<AlphaComponent> {
    let components = find_alpha_components(
        rgba,
        width,
        height,
        foreground_threshold,
        anchor_threshold,
    );

    let mut removed_mask = vec![false; width * height];

    for component in &components {
        if should_remove_component(component) {
            for (y, x) in &component.pixels {
                removed_mask[y * width + x] = true;
            }
        }
    }

    // 将需要移除的像素的 alpha 设为 0
    for (i, &remove) in removed_mask.iter().enumerate() {
        if remove {
            rgba[i * 4 + 3] = 0;
        }
    }

    components
}

/// 清理噪点后的 alpha 图像
pub fn prune_artifacts(
    rgba: &mut [u8],
    width: usize,
    height: usize,
) {
    process_alpha(rgba, width, height, 1, 160);
}

/// 查找 alpha 连通域
pub fn find_alpha_components(
    rgba: &[u8],
    width: usize,
    height: usize,
    foreground_threshold: u8,
    anchor_threshold: u8,
) -> Vec<AlphaComponent> {
    let mut mask = vec![false; width * height];
    let mut alpha_values = vec![0u8; width * height];

    for y in 0..height {
        for x in 0..width {
            let idx = y * width + x;
            let alpha = rgba[idx * 4 + 3];
            alpha_values[idx] = alpha;
            mask[idx] = alpha > foreground_threshold;
        }
    }

    let mut visited = vec![false; width * height];
    let mut components = Vec::new();

    // 8 邻域
    let neighbors: [(i32, i32); 8] = [
        (-1, -1), (0, -1), (1, -1),
        (-1, 0),           (1, 0),
        (-1, 1),  (0, 1),  (1, 1),
    ];

    for start_y in 0..height {
        for start_x in 0..width {
            let start_idx = start_y * width + start_x;
            if !mask[start_idx] || visited[start_idx] {
                continue;
            }

            // BFS 遍历连通域
            let mut queue: VecDeque<(usize, usize)> = VecDeque::new();
            queue.push_back((start_y, start_x));
            visited[start_idx] = true;

            let mut pixels = Vec::new();
            let mut min_x = start_x;
            let mut max_x = start_x;
            let mut min_y = start_y;
            let mut max_y = start_y;
            let mut max_alpha = alpha_values[start_idx];
            let mut anchor_area = 0;

            while let Some((current_y, current_x)) = queue.pop_front() {
                pixels.push((current_y, current_x));

                let current_idx = current_y * width + current_x;
                let current_alpha = alpha_values[current_idx];
                max_alpha = max_alpha.max(current_alpha);

                if current_alpha >= anchor_threshold {
                    anchor_area += 1;
                }

                min_x = min_x.min(current_x);
                max_x = max_x.max(current_x);
                min_y = min_y.min(current_y);
                max_y = max_y.max(current_y);

                for (dy, dx) in &neighbors {
                    let next_y = current_y as i32 + dy;
                    let next_x = current_x as i32 + dx;

                    if next_y < 0 || next_y >= height as i32 || next_x < 0 || next_x >= width as i32 {
                        continue;
                    }

                    let next_y = next_y as usize;
                    let next_x = next_x as usize;
                    let next_idx = next_y * width + next_x;

                    if mask[next_idx] && !visited[next_idx] {
                        visited[next_idx] = true;
                        queue.push_back((next_y, next_x));
                    }
                }
            }

            components.push(AlphaComponent {
                area: pixels.len(),
                width: max_x - min_x + 1,
                height: max_y - min_y + 1,
                pixels,
                max_alpha,
                anchor_area,
            });
        }
    }

    components
}

/// 判断是否应该移除该连通域
fn should_remove_component(component: &AlphaComponent) -> bool {
    // 有实心锚点的不移除
    if component.anchor_area > 0 {
        return false;
    }

    // 很小的碎块
    if component.area <= 20 && component.max_side() <= 8 {
        return true;
    }

    // 细长的毛刺
    if component.area <= 48 && component.min_side() <= 2 && component.max_side() >= 6 {
        return true;
    }

    // 低填充率的淡色区域
    if component.area <= 72 && component.fill_ratio() <= 0.3 && component.max_alpha <= 112 {
        return true;
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_component_properties() {
        let component = AlphaComponent {
            pixels: vec![(0, 0), (0, 1), (1, 0), (1, 1)],
            area: 4,
            width: 2,
            height: 2,
            max_alpha: 255,
            anchor_area: 4,
        };

        assert_eq!(component.bbox_area(), 4);
        assert!((component.fill_ratio() - 1.0).abs() < 0.001);
        assert_eq!(component.min_side(), 2);
        assert_eq!(component.max_side(), 2);
    }
}
