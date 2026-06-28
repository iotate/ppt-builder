//! 连通域检测
//! 
//! 参考 ai-ppt-maker 的 cv_mask_components.py 实现
//! 基于 BFS 的连通域检测算法

use std::collections::VecDeque;

/// 连通域
#[derive(Debug, Clone)]
pub struct Component {
    pub left: usize,
    pub top: usize,
    pub right: usize,
    pub bottom: usize,
    pub area: usize,
    pub mask: Vec<bool>,
    pub mask_width: usize,
    pub mask_height: usize,
}

impl Component {
    pub fn width(&self) -> usize {
        self.right - self.left
    }

    pub fn height(&self) -> usize {
        self.bottom - self.top
    }
}

/// 查找连通域
pub fn find_components(mask: &[bool], width: usize, height: usize, connectivity: u8) -> Vec<Component> {
    if mask.is_empty() || width == 0 || height == 0 {
        return Vec::new();
    }

    let mut visited = vec![false; width * height];
    let mut components = Vec::new();

    // 获取邻域偏移
    let offsets = if connectivity == 4 {
        vec![(-1, 0), (1, 0), (0, -1), (0, 1)]
    } else {
        vec![
            (-1, -1), (-1, 0), (-1, 1),
            (0, -1),           (0, 1),
            (1, -1),  (1, 0),  (1, 1),
        ]
    };

    for start_y in 0..height {
        for start_x in 0..width {
            let start_idx = start_y * width + start_x;
            if !mask[start_idx] || visited[start_idx] {
                continue;
            }

            // BFS 遍历
            let mut queue: VecDeque<(usize, usize)> = VecDeque::new();
            queue.push_back((start_y, start_x));
            visited[start_idx] = true;

            let mut pixels: Vec<(usize, usize)> = Vec::new();
            let mut min_x = start_x;
            let mut max_x = start_x;
            let mut min_y = start_y;
            let mut max_y = start_y;

            while let Some((current_y, current_x)) = queue.pop_front() {
                pixels.push((current_y, current_x));

                min_x = min_x.min(current_x);
                max_x = max_x.max(current_x);
                min_y = min_y.min(current_y);
                max_y = max_y.max(current_y);

                for (dy, dx) in &offsets {
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

            // 构建局部 mask
            let comp_left = min_x;
            let comp_top = min_y;
            let comp_right = max_x + 1;
            let comp_bottom = max_y + 1;
            let comp_width = comp_right - comp_left;
            let comp_height = comp_bottom - comp_top;

            let mut local_mask = vec![false; comp_width * comp_height];

            for (y, x) in &pixels {
                let local_y = y - comp_top;
                let local_x = x - comp_left;
                local_mask[local_y * comp_width + local_x] = true;
            }

            components.push(Component {
                left: comp_left,
                top: comp_top,
                right: comp_right,
                bottom: comp_bottom,
                area: pixels.len(),
                mask: local_mask,
                mask_width: comp_width,
                mask_height: comp_height,
            });
        }
    }

    components
}

/// 从种子点扩展 mask
pub fn grow_mask_from_seed(
    candidate: &[bool],
    seed: &[bool],
    width: usize,
    height: usize,
    connectivity: u8,
) -> Vec<bool> {
    if candidate.is_empty() || seed.is_empty() {
        return vec![false; width * height];
    }

    let offsets: Vec<(i32, i32)> = if connectivity == 4 {
        vec![(-1, 0), (1, 0), (0, -1), (0, 1)]
    } else {
        vec![
            (-1, -1), (-1, 0), (-1, 1),
            (0, -1),           (0, 1),
            (1, -1),  (1, 0),  (1, 1),
        ]
    };

    let mut grown = vec![false; width * height];
    let mut queue: VecDeque<(usize, usize)> = VecDeque::new();

    // 初始化队列
    for y in 0..height {
        for x in 0..width {
            let idx = y * width + x;
            if seed[idx] && candidate[idx] {
                grown[idx] = true;
                queue.push_back((y, x));
            }
        }
    }

    // BFS 扩展
    while let Some((current_y, current_x)) = queue.pop_front() {
        for (dy, dx) in &offsets {
            let next_y = current_y as i32 + dy;
            let next_x = current_x as i32 + dx;

            if next_y < 0 || next_y >= height as i32 || next_x < 0 || next_x >= width as i32 {
                continue;
            }

            let next_y = next_y as usize;
            let next_x = next_x as usize;
            let next_idx = next_y * width + next_x;

            if candidate[next_idx] && !grown[next_idx] {
                grown[next_idx] = true;
                queue.push_back((next_y, next_x));
            }
        }
    }

    grown
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_components_simple() {
        // 3x3 的单个连通域
        let mask = vec![
            true, true, true,
            true, true, true,
            true, true, true,
        ];

        let components = find_components(&mask, 3, 3, 4);
        assert_eq!(components.len(), 1);
        assert_eq!(components[0].area, 9);
        assert_eq!(components[0].left, 0);
        assert_eq!(components[0].top, 0);
        assert_eq!(components[0].right, 3);
        assert_eq!(components[0].bottom, 3);
    }

    #[test]
    fn test_find_components_separate() {
        // 两个独立的连通域
        let mask = vec![
            true, false, true,
            false, false, false,
            true, false, true,
        ];

        let components = find_components(&mask, 3, 3, 4);
        assert_eq!(components.len(), 4); // 4 个独立像素，使用 4 连接
    }

    #[test]
    fn test_grow_mask_from_seed() {
        let candidate = vec![
            true, true, true,
            true, true, true,
            true, true, true,
        ];

        let seed = vec![
            true, false, false,
            false, false, false,
            false, false, false,
        ];

        let grown = grow_mask_from_seed(&candidate, &seed, 3, 3, 4);
        assert!(grown.iter().all(|&v| v));
    }
}
