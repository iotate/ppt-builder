use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::path::PathBuf;
use tauri::State;
use crate::config::ApiConfig;
use crate::error_log;

/// 风格遵循强度
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AdherenceLevel {
    /// 宽松 - 优先保持系列感，允许重新组织
    Loose,
    /// 适度 - 框架统一、细节鲜活
    Balanced,
    /// 严格 - 锁定骨架与色彩节奏
    Strict,
}

impl Default for AdherenceLevel {
    fn default() -> Self {
        Self::Balanced
    }
}

impl AdherenceLevel {
    #[allow(dead_code)]
    pub fn label(&self) -> &'static str {
        match self {
            Self::Loose => "宽松",
            Self::Balanced => "适度",
            Self::Strict => "严格",
        }
    }

    #[allow(dead_code)]
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "loose" | "宽松" => Self::Loose,
            "strict" | "严格" => Self::Strict,
            _ => Self::Balanced,
        }
    }

    /// 获取风格遵循提示词
    #[allow(dead_code)]
    pub fn prompt_lines(&self, has_reference_images: bool) -> Vec<String> {
        if !has_reference_images {
            return vec![];
        }

        match self {
            Self::Loose => vec![
                "优先学习原稿图的版芯比例、留白、背景纹理、线条样式、卡片层级和色彩节奏，再围绕本页内容重新设计。".to_string(),
                "不要照搬某一张原稿图的具体版式，只需要保持同系列视觉一致性。".to_string(),
            ],
            Self::Balanced => vec![
                "优先解析原稿图的版芯比例、留白、背景纹理、线条样式、卡片层级与色彩节奏，将其作为设计的基准约束。".to_string(),
                "保留适度发挥空间，做到框架统一、细节鲜活，避免刻板套用。".to_string(),
            ],
            Self::Strict => vec![
                "严格锁定原稿图的版芯比例、留白、背景纹理、线条样式、卡片层级和色彩节奏，然后按照这些规则将本页内容填入。".to_string(),
                "允许按本页信息重新映射模块内容，但不要跳出这套模板的版式语法、视觉节奏与卡片组织方式。".to_string(),
            ],
        }
    }
}

/// 风格核心定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StyleCore {
    /// 背景基调（如"浅灰底色"）
    #[serde(default)]
    pub background_tone: String,
    /// 配色方案（主色、辅助色等）
    #[serde(default)]
    pub palette: Vec<String>,
    /// 标题样式
    #[serde(default)]
    pub title_style: String,
    /// 卡片样式
    #[serde(default)]
    pub card_style: String,
    /// 图标样式
    #[serde(default)]
    pub icon_style: String,
    /// 线条样式
    #[serde(default)]
    pub line_style: String,
}

impl Default for StyleCore {
    fn default() -> Self {
        Self {
            background_tone: "按主题选择浅色或中性底色，保证正文可读".to_string(),
            palette: vec!["主题主色".to_string(), "辅助强调色".to_string(), "中性色".to_string()],
            title_style: "标题层级清楚，关键词强调方式与主题气质一致".to_string(),
            card_style: "信息分组方式随内容语义选择，需要容器时保持边界可辨".to_string(),
            icon_style: "图标风格与内容领域一致，保持统一和可识别".to_string(),
            line_style: "连接线、箭头和编号关系清晰，样式随版式语义适配".to_string(),
        }
    }
}

/// 变化策略
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VariationPolicy {
    /// 同一版式最大连续重复次数
    #[serde(default = "default_same_layout_max_repeat")]
    pub same_layout_max_repeat: u32,
    /// 整套最少覆盖版式种类
    #[serde(default = "default_min_distinct_layout_families")]
    pub min_distinct_layout_families: u32,
    /// 是否允许局部重组
    #[serde(default = "default_allow_local_recomposition")]
    pub allow_local_recomposition: bool,
}

fn default_same_layout_max_repeat() -> u32 { 1 }
fn default_min_distinct_layout_families() -> u32 { 3 }
fn default_allow_local_recomposition() -> bool { true }

impl Default for VariationPolicy {
    fn default() -> Self {
        Self {
            same_layout_max_repeat: 1,
            min_distinct_layout_families: 3,
            allow_local_recomposition: true,
        }
    }
}

/// 版式家族定义
pub const DEFAULT_LAYOUT_FAMILIES: &[(&str, &str)] = &[
    ("grid_n_x_m", "宫格卡片"),
    ("timeline_horizontal", "横向时间线"),
    ("timeline_vertical", "纵向时间线"),
    ("hub_and_spoke", "中心辐射"),
    ("split_left_right", "左右分栏"),
    ("split_top_bottom", "上下分区"),
    ("compare_dual_axis", "双轴对比"),
    ("process_horizontal", "横向流程"),
    ("process_vertical", "纵向流程"),
    ("hero_with_supporting_cards", "主视觉卡片"),
];

/// 获取版式家族标签
#[allow(dead_code)]
pub fn get_layout_family_label(name: &str) -> &str {
    DEFAULT_LAYOUT_FAMILIES
        .iter()
        .find(|(key, _)| *key == name)
        .map(|(_, label)| *label)
        .unwrap_or(name)
}

/// 风格指南
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StyleGuide {
    /// 风格核心定义
    #[serde(default)]
    pub style_core: StyleCore,
    /// 可用版式家族列表
    #[serde(default = "default_layout_families")]
    pub layout_families: Vec<String>,
    /// 元素原语（语义分组、重点标记等）
    #[serde(default = "default_element_primitives")]
    pub element_primitives: Vec<String>,
    /// 变化策略
    #[serde(default)]
    pub variation_policy: VariationPolicy,
    /// 禁止规则
    #[serde(default = "default_negative_rules")]
    pub negative_rules: Vec<String>,
    /// 风格锚点描述（注入到每页提示词）
    #[serde(default)]
    pub prompt_anchor: String,
    /// 风格遵循强度
    #[serde(default)]
    pub adherence_level: AdherenceLevel,
}

fn default_layout_families() -> Vec<String> {
    DEFAULT_LAYOUT_FAMILIES.iter().map(|(key, _)| key.to_string()).collect()
}

fn default_element_primitives() -> Vec<String> {
    vec![
        "语义分组".to_string(),
        "重点标记".to_string(),
        "步骤或指标标记".to_string(),
        "内容匹配图标".to_string(),
        "关系连接".to_string(),
    ]
}

fn default_negative_rules() -> Vec<String> {
    vec![
        "不要套用与主题无关的固定行业模板".to_string(),
        "不要使用干扰阅读的复杂背景".to_string(),
        "不要让页面之间的视觉语言突然断裂".to_string(),
    ]
}

impl Default for StyleGuide {
    fn default() -> Self {
        Self {
            style_core: StyleCore::default(),
            layout_families: default_layout_families(),
            element_primitives: default_element_primitives(),
            variation_policy: VariationPolicy::default(),
            negative_rules: default_negative_rules(),
            prompt_anchor: String::new(),
            adherence_level: AdherenceLevel::default(),
        }
    }
}

impl StyleGuide {
    /// 构建压缩的风格提示词
    #[allow(dead_code)]
    pub fn build_style_prompt(&self, layout_family: Option<&str>) -> String {
        let mut lines = Vec::new();

        // 风格锚点
        if !self.prompt_anchor.is_empty() {
            lines.push(format!("统一视觉锚点：{}", self.prompt_anchor));
        }

        // 背景与配色
        if !self.style_core.background_tone.is_empty() {
            lines.push(format!("背景明度与底色要求：{}", self.style_core.background_tone));
        }
        if !self.style_core.palette.is_empty() {
            lines.push(format!("建议配色：{}", self.style_core.palette.join("、")));
        }

        // 版式约束
        if let Some(layout) = layout_family {
            let label = get_layout_family_label(layout);
            lines.push(format!("组织方式可参考 {}（{}）", layout, label));
        }

        // 元素语言
        if !self.element_primitives.is_empty() {
            lines.push(format!(
                "元素语言要求：使用 {}，按本页内容重新生成具体图形",
                self.element_primitives.join("、")
            ));
        }

        // 禁止规则
        if !self.negative_rules.is_empty() {
            lines.push(format!("禁止事项：{}", self.negative_rules[..3.min(self.negative_rules.len())].join("；")));
        }

        lines.join("\n")
    }
}

/// 版式分配器
pub struct LayoutAssigner {
    layout_families: Vec<String>,
    policy: VariationPolicy,
}

impl LayoutAssigner {
    pub fn new(style_guide: &StyleGuide) -> Self {
        Self {
            layout_families: style_guide.layout_families.clone(),
            policy: style_guide.variation_policy.clone(),
        }
    }

    /// 为页面分配版式家族
    pub fn assign(&self, page_count: u32, existing_assignments: Option<&HashMap<u32, String>>) -> HashMap<u32, String> {
        let mut assignments = existing_assignments.cloned().unwrap_or_default();
        let mut used_families: Vec<String> = Vec::new();
        let mut consecutive_count: u32 = 0;
        let mut last_family: Option<String> = None;

        for page_num in 1..=page_count {
            // 如果已有分配，检查是否需要重新分配
            if let Some(existing) = assignments.get(&page_num) {
                // 检查是否符合约束
                if last_family.as_ref() == Some(existing) {
                    consecutive_count += 1;
                    if consecutive_count > self.policy.same_layout_max_repeat {
                        // 需要重新分配
                        if let Some(new_family) = self.select_different_family(&last_family, &used_families) {
                            assignments.insert(page_num, new_family.clone());
                            last_family = Some(new_family.clone());
                            used_families.push(new_family);
                            consecutive_count = 1;
                        }
                        continue;
                    }
                } else {
                    consecutive_count = 1;
                }
                last_family = Some(existing.clone());
                used_families.push(existing.clone());
                continue;
            }

            // 新分配
            let family = self.select_family(&last_family, &used_families, page_count - page_num + 1);
            assignments.insert(page_num, family.clone());
            
            if last_family.as_ref() == Some(&family) {
                consecutive_count += 1;
            } else {
                consecutive_count = 1;
            }
            last_family = Some(family.clone());
            used_families.push(family);
        }

        // 检查是否满足最小多样性
        let distinct_count = assignments.values().collect::<std::collections::HashSet<_>>().len() as u32;
        if distinct_count < self.policy.min_distinct_layout_families {
            // 尝试增加多样性
            self.increase_diversity(&mut assignments, page_count);
        }

        assignments
    }

    fn select_family(&self, last_family: &Option<String>, used_families: &[String], remaining: u32) -> String {
        // 如果还有足够的页面，尽量选择不同的版式
        if remaining > 1 && last_family.is_some() {
            if let Some(family) = self.select_different_family(last_family, used_families) {
                return family;
            }
        }

        // 默认选择第一个
        self.layout_families.first().cloned().unwrap_or_else(|| "grid_n_x_m".to_string())
    }

    fn select_different_family(&self, exclude: &Option<String>, used_families: &[String]) -> Option<String> {
        // 优先选择使用较少的版式
        let mut counts: HashMap<String, u32> = HashMap::new();
        for family in used_families {
            *counts.entry(family.clone()).or_insert(0) += 1;
        }

        self.layout_families
            .iter()
            .filter(|f| Some(*f) != exclude.as_ref())
            .min_by_key(|f| counts.get(*f).unwrap_or(&0))
            .cloned()
    }

    fn increase_diversity(&self, assignments: &mut HashMap<u32, String>, page_count: u32) {
        let used: std::collections::HashSet<_> = assignments.values().cloned().collect();
        let unused: Vec<_> = self.layout_families.iter()
            .filter(|f| !used.contains(*f))
            .collect();

        if unused.is_empty() {
            return;
        }

        // 找到可以替换的页面
        for page_num in 2..page_count {
            let prev = assignments.get(&(page_num - 1));
            let curr = assignments.get(&page_num);
            let next = assignments.get(&(page_num + 1));

            if prev == curr && curr == next {
                // 连续三个相同，替换中间的
                if let Some(new_family) = unused.first() {
                    assignments.insert(page_num, (*new_family).clone());
                    break;
                }
            }
        }
    }
}

/// 从 Markdown 解析风格指南
pub fn parse_style_guide_from_markdown(content: &str) -> StyleGuide {
    let mut guide = StyleGuide::default();

    // 解析风格锚点
    guide.prompt_anchor = extract_section_content(content, "风格锚点");
    
    // 解析配色 - 提取颜色代码
    if let Some(colors) = extract_colors_from_markdown(content) {
        guide.style_core.palette = colors;
    }
    
    // 解析背景基调
    guide.style_core.background_tone = extract_section_content(content, "背景基调");
    
    // 解析其他样式
    guide.style_core.title_style = extract_section_content(content, "标题样式");
    guide.style_core.card_style = extract_section_content(content, "卡片样式");
    guide.style_core.icon_style = extract_section_content(content, "图标样式");
    guide.style_core.line_style = extract_section_content(content, "线条样式");

    // 解析禁止规则
    if let Some(rules) = extract_list_section(content, "禁止规则") {
        guide.negative_rules = rules;
    }

    guide
}

/// 从 Markdown 提取颜色列表
fn extract_colors_from_markdown(content: &str) -> Option<Vec<String>> {
    let section_header = "## 配色";
    let mut in_section = false;
    let mut colors = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim();
        
        if trimmed == section_header {
            in_section = true;
            continue;
        }
        
        if in_section {
            if trimmed.starts_with("#") {
                break;
            }
            // 提取颜色：主色：深海蓝 (#0F1B33)
            if trimmed.starts_with("- ") {
                // 提取颜色名称和代码
                if let Some(pos) = trimmed.find('(') {
                    if let Some(end) = trimmed.find(')') {
                        if end > pos {
                            let color_part = &trimmed[pos + 1..end];
                            // 提取颜色名称部分（减号后面的部分）
                            let name_part = trimmed[2..pos].trim();
                            let name = name_part.trim_end_matches(':').trim();
                            let code = color_part.trim();
                            if code.starts_with('#') {
                                colors.push(format!("{} {}", name, code));
                            }
                        }
                    }
                }
            }
        }
    }

    if colors.is_empty() {
        None
    } else {
        Some(colors)
    }
}

fn extract_section_content(content: &str, section_name: &str) -> String {
    let section_header = format!("## {}", section_name);
    let mut in_section = false;
    let mut result = String::new();

    for line in content.lines() {
        let trimmed = line.trim();
        
        if trimmed == section_header {
            in_section = true;
            continue;
        }
        
        if in_section {
            if trimmed.starts_with("#") {
                break;
            }
            if !trimmed.is_empty() {
                if !result.is_empty() {
                    result.push(' ');
                }
                result.push_str(trimmed);
            }
        }
    }

    result
}

fn extract_list_section(content: &str, section_name: &str) -> Option<Vec<String>> {
    let section_header = format!("## {}", section_name);
    let mut in_section = false;
    let mut result = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim();
        
        if trimmed == section_header {
            in_section = true;
            continue;
        }
        
        if in_section {
            if trimmed.starts_with("#") {
                break;
            }
            if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
                result.push(trimmed[2..].to_string());
            }
        }
    }

    if result.is_empty() {
        None
    } else {
        Some(result)
    }
}

#[allow(dead_code)]
fn extract_color_from_line(line: &str) -> Option<String> {
    // 提取形如 "深海蓝 (#1E3A5F)" 的颜色描述
    if let Some(start) = line.find('(') {
        if let Some(end) = line.find(')') {
            if end > start {
                let color = &line[start + 1..end];
                if color.starts_with('#') {
                    return Some(color.to_string());
                }
            }
        }
    }
    // 如果没有括号，尝试提取颜色名称
    let parts: Vec<&str> = line.splitn(3, ':').collect();
    if parts.len() >= 2 {
        let name = parts[1].trim();
        if !name.is_empty() {
            return Some(name.to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adherence_level_from_str() {
        assert_eq!(AdherenceLevel::from_str("loose"), AdherenceLevel::Loose);
        assert_eq!(AdherenceLevel::from_str("宽松"), AdherenceLevel::Loose);
        assert_eq!(AdherenceLevel::from_str("balanced"), AdherenceLevel::Balanced);
        assert_eq!(AdherenceLevel::from_str("适度"), AdherenceLevel::Balanced);
        assert_eq!(AdherenceLevel::from_str("strict"), AdherenceLevel::Strict);
        assert_eq!(AdherenceLevel::from_str("严格"), AdherenceLevel::Strict);
        assert_eq!(AdherenceLevel::from_str("unknown"), AdherenceLevel::Balanced);
    }

    #[test]
    fn test_layout_assigner() {
        let guide = StyleGuide::default();
        let assigner = LayoutAssigner::new(&guide);
        
        let assignments = assigner.assign(5, None);
        
        // 验证相邻页不重复
        for i in 2..=5 {
            let prev = assignments.get(&(i - 1));
            let curr = assignments.get(&i);
            assert_ne!(prev, curr, "Page {} and {} should have different layouts", i - 1, i);
        }
    }

    #[test]
    fn test_parse_style_guide() {
        let markdown = r#"
# 商务风格

## 配色
- 主色：深海蓝 (#1E3A5F)
- 强调色：科技青 (#00B8D9)

## 背景基调
浅灰白底色

## 风格锚点
统一使用深海蓝为主色调

## 禁止规则
- 不要使用复杂背景
- 不要突然断裂
"#;

        let guide = parse_style_guide_from_markdown(markdown);
        
        assert!(guide.style_core.background_tone.contains("浅灰白"));
        assert!(guide.prompt_anchor.contains("深海蓝"));
        assert_eq!(guide.negative_rules.len(), 2);
    }
}

// ============ Tauri Commands ============

/// 获取风格指南
#[tauri::command]
pub async fn get_style_guide(
    cwd: State<'_, Arc<PathBuf>>,
    style_name: String,
) -> Result<StyleGuide, String> {
    let style_path = cwd.join("styles").join(format!("{}.md", style_name));
    
    if !style_path.exists() {
        return Err(format!("风格文件不存在: {}", style_name));
    }
    
    let content = tokio::fs::read_to_string(&style_path)
        .await
        .map_err(|e| format!("读取风格文件失败: {}", e))?;
    
    Ok(parse_style_guide_from_markdown(&content))
}

/// 更新风格遵循强度
#[tauri::command]
pub async fn update_style_adherence(
    cwd: State<'_, Arc<PathBuf>>,
    project_name: String,
    level: String,
) -> Result<(), String> {
    let project_path = cwd.join("projects").join(&project_name).join("project.json");
    
    if !project_path.exists() {
        return Err(format!("项目不存在: {}", project_name));
    }
    
    let content = tokio::fs::read_to_string(&project_path)
        .await
        .map_err(|e| format!("读取项目配置失败: {}", e))?;
    
    let mut project: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| format!("解析项目配置失败: {}", e))?;
    
    // 更新 style_guide.adherence_level
    if let Some(style_guide) = project.get_mut("style_guide") {
        if let Some(obj) = style_guide.as_object_mut() {
            obj.insert("adherence_level".to_string(), serde_json::json!(level));
        }
    } else {
        project["style_guide"] = serde_json::json!({
            "adherence_level": level
        });
    }
    
    let updated = serde_json::to_string_pretty(&project)
        .map_err(|e| format!("序列化项目配置失败: {}", e))?;
    
    tokio::fs::write(&project_path, updated)
        .await
        .map_err(|e| format!("保存项目配置失败: {}", e))?;
    
    Ok(())
}

/// 获取页面版式分配
#[tauri::command]
pub async fn get_layout_assignments(
    cwd: State<'_, Arc<PathBuf>>,
    project_name: String,
    style_name: String,
    page_count: u32,
) -> Result<HashMap<u32, String>, String> {
    // 读取风格指南
    let style_path = cwd.join("styles").join(format!("{}.md", style_name));
    
    let style_guide = if style_path.exists() {
        let content = tokio::fs::read_to_string(&style_path)
            .await
            .map_err(|e| format!("读取风格文件失败: {}", e))?;
        parse_style_guide_from_markdown(&content)
    } else {
        StyleGuide::default()
    };
    
    // 读取现有分配
    let project_path = cwd.join("projects").join(&project_name).join("project.json");
    let existing: Option<HashMap<u32, String>> = if project_path.exists() {
        let content = tokio::fs::read_to_string(&project_path)
            .await
            .map_err(|e| format!("读取项目配置失败: {}", e))?;
        let project: serde_json::Value = serde_json::from_str(&content)
            .map_err(|e| format!("解析项目配置失败: {}", e))?;
        project.get("style_guide")
            .and_then(|sg| sg.get("layout_assignments"))
            .and_then(|la| serde_json::from_value(la.clone()).ok())
    } else {
        None
    };
    
    // 分配版式
    let assigner = LayoutAssigner::new(&style_guide);
    Ok(assigner.assign(page_count, existing.as_ref()))
}

/// 从参考图提取风格指南
#[tauri::command]
pub async fn extract_style_guide_from_images(
    cwd: State<'_, Arc<PathBuf>>,
    project_name: String,
    reference_paths: Vec<String>,
    style_notes: String,
    config: ApiConfig,
) -> Result<StyleGuide, String> {
    let cwd_path = cwd.inner().clone();
    
    if reference_paths.is_empty() {
        error_log::log_info(&cwd_path, "No reference images provided, returning default style guide");
        let mut guide = StyleGuide::default();
        if !style_notes.is_empty() {
            guide.prompt_anchor = style_notes;
        }
        return Ok(guide);
    }
    
    // 限制最多3张参考图
    let reference_paths: Vec<String> = reference_paths.into_iter().take(3).collect();
    
    // 读取参考图并转为 base64
    let mut image_data_list = Vec::new();
    for path in &reference_paths {
        let full_path = if path.starts_with("http") {
            // 如果是URL，跳过
            continue;
        } else {
            cwd.join(path)
        };
        
        if !full_path.exists() {
            error_log::log_error(&cwd_path, &format!("Reference image not found: {:?}", full_path));
            continue;
        }
        
        match tokio::fs::read(&full_path).await {
            Ok(data) => {
                let base64 = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &data);
                let extension = full_path.extension().and_then(|e| e.to_str()).unwrap_or("png");
                let mime = match extension {
                    "jpg" | "jpeg" => "image/jpeg",
                    "png" => "image/png",
                    "gif" => "image/gif",
                    "webp" => "image/webp",
                    _ => "image/png",
                };
                image_data_list.push(format!("data:{};base64,{}", mime, base64));
            }
            Err(e) => {
                error_log::log_error(&cwd_path, &format!("Failed to read reference image: {}", e));
            }
        }
    }
    
    if image_data_list.is_empty() {
        error_log::log_info(&cwd_path, "No valid reference images, returning default style guide");
        let mut guide = StyleGuide::default();
        if !style_notes.is_empty() {
            guide.prompt_anchor = style_notes;
        }
        return Ok(guide);
    }
    
    // 构建分析提示词
    let analysis_prompt = r#"请仔细分析这些原稿图的 PPT 风格，提炼出稳定的版式与视觉语言，供后续多页 PPT 统一复用。

【重要】颜色提取要求：
1. 仔细观察图片中实际使用的颜色，准确提取主色、辅助色、背景色
2. 必须提供准确的 HEX 颜色代码（如 #FF5733），不要凭空想象颜色
3. 如果图片明显是红色主基调，请如实返回红色系颜色

请返回 JSON 格式，包含以下字段：
{
  "style_core": {
    "background_tone": "背景色调描述（如'浅灰底色'、'深蓝底色'）",
    "palette": ["主色名称 (#颜色代码)", "辅助色名称 (#颜色代码)", "背景色名称 (#颜色代码)"],
    "title_style": "标题样式描述",
    "card_style": "卡片样式描述",
    "icon_style": "图标样式描述",
    "line_style": "线条样式描述"
  },
  "layout_families": ["grid_n_x_m", "timeline_horizontal", "hub_and_spoke", ...],
  "element_primitives": ["语义分组", "重点标记", ...],
  "negative_rules": ["禁止事项1", "禁止事项2"],
  "prompt_anchor": "一段简洁的风格锚点描述，用于注入每页的生图提示词，确保整套PPT视觉一致"
}

注意：
1. layout_families 从以下选项中选择：grid_n_x_m, timeline_horizontal, timeline_vertical, hub_and_spoke, split_left_right, split_top_bottom, compare_dual_axis, process_horizontal, process_vertical, hero_with_supporting_cards
2. prompt_anchor 应该是一段完整的描述，包含配色方案、整体风格、视觉语言等关键信息
3. 只返回 JSON，不要包含其他文字"#;

    // 构建消息内容
    let mut content_items: Vec<serde_json::Value> = vec![
        serde_json::json!({
            "type": "text",
            "text": analysis_prompt
        })
    ];
    
    for image_data in &image_data_list {
        content_items.push(serde_json::json!({
            "type": "image_url",
            "image_url": {
                "url": image_data
            }
        }));
    }
    
    // 调用 LLM API
    let client = reqwest::Client::new();
    
    let mut request = client
        .post(format!("{}/chat/completions", config.endpoint))
        .header("Authorization", format!("Bearer {}", config.api_key))
        .header("Content-Type", "application/json");
    
    for header in &config.extra_headers {
        request = request.header(&header.key, &header.value);
    }
    
    let response = request
        .json(&serde_json::json!({
            "model": config.model,
            "messages": [
                {
                    "role": "user",
                    "content": content_items
                }
            ],
            "temperature": 0.3
        }))
        .send()
        .await
        .map_err(|e| {
            let error = format!("LLM API 请求失败: {}", e);
            error_log::log_error(&cwd_path, &error);
            error
        })?;
    
    if !response.status().is_success() {
        let error_text = response.text().await.unwrap_or_default();
        // 检查是否是不支持多模态的错误
        if error_text.contains("unknown variant 'image_url'") || error_text.contains("image_url") {
            let error = "当前配置的 LLM 模型不支持图片输入（多模态）。请使用支持视觉能力的模型，如 GPT-4o、Qwen-VL、DeepSeek-V4 等。".to_string();
            error_log::log_error(&cwd_path, &error);
            return Err(error);
        }
        let error = format!("LLM API 错误: {}", error_text);
        error_log::log_error(&cwd_path, &error);
        return Err(error);
    }
    
    let json: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("解析 LLM 响应失败: {}", e))?;
    
    let content = json["choices"][0]["message"]["content"]
        .as_str()
        .ok_or("无法从 LLM 响应中提取内容")?;
    
    // 解析 JSON
    let guide: StyleGuide = serde_json::from_str(content)
        .map_err(|e| {
            error_log::log_error(&cwd_path, &format!("解析风格指南失败: {}, 原始内容: {}", e, content));
            format!("解析风格指南失败: {}", e)
        })?;
    
    error_log::log_info(&cwd_path, "Style guide extracted successfully");
    
    // 保存到项目
    let project_path = cwd.join("projects").join(&project_name).join("project.json");
    if project_path.exists() {
        let project_content = tokio::fs::read_to_string(&project_path)
            .await
            .unwrap_or_default();
        if let Ok(mut project) = serde_json::from_str::<serde_json::Value>(&project_content) {
            project["style_guide"] = serde_json::to_value(&guide).unwrap_or_default();
            if let Ok(updated) = serde_json::to_string_pretty(&project) {
                let _ = tokio::fs::write(&project_path, updated).await;
            }
        }
    }
    
    Ok(guide)
}

/// 从单个文件（图片或PPTX）提取风格并保存为风格文件
#[tauri::command]
pub async fn extract_style_from_file(
    cwd: State<'_, Arc<PathBuf>>,
    file_path: String,
    style_name: String,
) -> Result<String, String> {
    let cwd_path = cwd.inner().clone();
    
    // 检查文件是否存在
    let path = std::path::PathBuf::from(&file_path);
    if !path.exists() {
        let error = format!("文件不存在: {}", file_path);
        error_log::log_error(&cwd_path, &error);
        return Err(error);
    }
    
    // 读取配置 - 使用 YAML 格式
    let config_path = cwd.join("config.yaml");
    if !config_path.exists() {
        let error = "配置文件不存在，请先完成 API 配置".to_string();
        error_log::log_error(&cwd_path, &error);
        return Err(error);
    }
    
    let config_content = tokio::fs::read_to_string(&config_path)
        .await
        .map_err(|e| {
            let error = format!("读取配置失败: {}", e);
            error_log::log_error(&cwd_path, &error);
            error
        })?;
    
    let config: crate::config::AppConfig = serde_yaml::from_str(&config_content)
        .map_err(|e| {
            let error = format!("解析配置失败: {}", e);
            error_log::log_error(&cwd_path, &error);
            error
        })?;
    
    let llm_config = config.llm;
    
    if llm_config.api_key.is_empty() {
        let error = "请先配置 LLM API Key".to_string();
        error_log::log_error(&cwd_path, &error);
        return Err(error);
    }
    
    let extension = path.extension().and_then(|e| e.to_str()).unwrap_or("png").to_lowercase();
    
    // 处理 PPTX 文件
    if extension == "pptx" {
        return extract_style_from_pptx(&path, &style_name, &llm_config, &cwd_path).await;
    }
    
    // 处理图片文件
    let file_data = tokio::fs::read(&path)
        .await
        .map_err(|e| {
            let error = format!("读取文件失败: {}", e);
            error_log::log_error(&cwd_path, &error);
            error
        })?;
    
    let mime = match extension.as_str() {
        "jpg" | "jpeg" => "image/jpeg",
        "png" => "image/png",
        "gif" => "image/gif",
        "webp" => "image/webp",
        _ => "image/png",
    };
    
    let base64_image = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &file_data);
    let image_data = format!("data:{};base64,{}", mime, base64_image);
    
    // 构建分析提示词
    let analysis_prompt = r#"请仔细分析这张图片的设计风格，准确提取视觉元素，用于生成类似风格的PPT或信息图表。

【重要】颜色提取要求：
1. 仔细观察图片中实际使用的颜色，准确提取主色、辅助色、背景色
2. 必须提供准确的 HEX 颜色代码（如 #FF5733），不要凭空想象颜色
3. 如果图片明显是红色主基调，请如实返回红色系颜色

请返回 JSON 格式，包含以下字段：
{
  "style_core": {
    "background_tone": "背景色调描述（如'浅灰底色'、'深蓝底色'）",
    "palette": ["主色名称 (#颜色代码)", "辅助色名称 (#颜色代码)", "背景色名称 (#颜色代码)"],
    "title_style": "标题样式描述",
    "card_style": "卡片样式描述",
    "icon_style": "图标样式描述",
    "line_style": "线条样式描述"
  },
  "layout_families": ["适用的版式类型"],
  "element_primitives": ["视觉元素特征"],
  "negative_rules": ["禁止事项"],
  "prompt_anchor": "一段简洁的风格锚点描述，包含配色方案、整体风格、视觉语言等关键信息"
}

注意：只返回 JSON，不要包含其他文字"#;

    // 构建消息内容
    let content_items: Vec<serde_json::Value> = vec![
        serde_json::json!({
            "type": "text",
            "text": analysis_prompt
        }),
        serde_json::json!({
            "type": "image_url",
            "image_url": {
                "url": image_data
            }
        })
    ];
    
    // 调用 LLM API
    let client = reqwest::Client::new();
    
    let mut request = client
        .post(format!("{}/chat/completions", llm_config.endpoint))
        .header("Authorization", format!("Bearer {}", llm_config.api_key))
        .header("Content-Type", "application/json");
    
    for header in &llm_config.extra_headers {
        request = request.header(&header.key, &header.value);
    }
    
    let response = request
        .json(&serde_json::json!({
            "model": llm_config.model,
            "messages": [
                {
                    "role": "user",
                    "content": content_items
                }
            ],
            "temperature": 0.1
        }))
        .send()
        .await
        .map_err(|e| format!("LLM API 请求失败: {}", e))?;
    
    if !response.status().is_success() {
        let error_text = response.text().await.unwrap_or_default();
        // 检查是否是不支持多模态的错误
        if error_text.contains("unknown variant 'image_url'") || error_text.contains("image_url") {
            return Err("当前配置的 LLM 模型不支持图片输入（多模态）。请使用支持视觉能力的模型，如 GPT-4o、Qwen-VL、DeepSeek-V4 等。".to_string());
        }
        return Err(format!("LLM API 错误: {}", error_text));
    }
    
    let json: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("解析 LLM 响应失败: {}", e))?;
    
    let content = json["choices"][0]["message"]["content"]
        .as_str()
        .ok_or("无法从 LLM 响应中提取内容")?;
    
    // 解析 JSON
    let guide: StyleGuide = serde_json::from_str(content)
        .map_err(|e| format!("解析风格指南失败: {}, 原始内容: {}", e, content))?;
    
    // 生成 Markdown 内容
    let md_content = format!(r#"# {}

{}

## 风格锚点
{}

## 配色
{}

## 背景基调
{}

## 标题样式
{}

## 卡片样式
{}

## 图标样式
{}

## 线条样式
{}

## 禁止规则
{}
"#,
        style_name,
        guide.prompt_anchor,
        guide.prompt_anchor,
        guide.style_core.palette.iter().map(|c| format!("- {}", c)).collect::<Vec<_>>().join("\n"),
        guide.style_core.background_tone,
        guide.style_core.title_style,
        guide.style_core.card_style,
        guide.style_core.icon_style,
        guide.style_core.line_style,
        guide.negative_rules.iter().map(|r| format!("- {}", r)).collect::<Vec<_>>().join("\n")
    );
    
    error_log::log_info(&cwd_path, &format!("Style extracted: {}", style_name));
    
    // 返回 Markdown 内容，让前端编辑后再保存
    Ok(md_content)
}

/// 从PPTX文件提取风格
async fn extract_style_from_pptx(
    pptx_path: &std::path::Path,
    style_name: &str,
    llm_config: &crate::config::ApiConfig,
    cwd_path: &std::path::PathBuf,
) -> Result<String, String> {
    use crate::pptx;
    
    error_log::log_info(cwd_path, &format!("Extracting style from PPTX: {:?}", pptx_path));
    
    // 提取配色方案
    let color_scheme = pptx::extract_color_scheme(pptx_path)
        .map_err(|e| {
            error_log::log_error(cwd_path, &e);
            e
        })?;
    
    // 提取幻灯片数量
    let slide_count = pptx::count_slides(pptx_path).unwrap_or(0);
    
    // 提取幻灯片布局信息
    let layouts = pptx::extract_slide_layouts(pptx_path).unwrap_or_default();
    
    error_log::log_info(cwd_path, &format!("Color scheme: {:?}", color_scheme));
    error_log::log_info(cwd_path, &format!("Slide count: {}", slide_count));
    
    // 构建配色列表
    let mut color_list = Vec::new();
    for (name, hex) in &color_scheme.colors {
        color_list.push(format!("{} {}", name, hex));
    }
    
    // 如果没有提取到颜色，使用默认颜色
    if color_list.is_empty() {
        color_list.push("主色 #4472C4".to_string());
        color_list.push("辅助色 #ED7D31".to_string());
    }
    
    // 构建提示词，让LLM生成完整的风格描述
    let prompt = format!(r#"请根据以下从PPT文件中提取的配色方案，生成一个完整的风格描述。

配色方案（主题名称：{}）：
{}

幻灯片数量：{}
主要布局类型：{}

请返回 JSON 格式，包含以下字段：
{{
  "style_core": {{
    "background_tone": "背景色调描述（根据配色推测合适的背景）",
    "palette": ["主色名称 (#颜色代码)", "辅助色名称 (#颜色代码)", "背景色名称 (#颜色代码)"],
    "title_style": "标题样式描述（根据PPT风格推测）",
    "card_style": "卡片样式描述（根据PPT风格推测）",
    "icon_style": "图标样式描述（根据PPT风格推测）",
    "line_style": "线条样式描述（根据PPT风格推测）"
  }},
  "layout_families": ["适用的版式类型"],
  "element_primitives": ["视觉元素特征"],
  "negative_rules": ["禁止事项"],
  "prompt_anchor": "一段简洁的风格锚点描述，包含配色方案、整体风格、视觉语言等关键信息"
}}

注意：
1. palette 中必须使用上面提供的实际颜色
2. 只返回 JSON，不要包含其他文字"#,
        color_scheme.name,
        color_list.join("\n"),
        slide_count,
        layouts.first().unwrap_or(&"未知".to_string())
    );
    
    // 调用 LLM API
    let client = reqwest::Client::new();
    
    let mut request = client
        .post(format!("{}/chat/completions", llm_config.endpoint))
        .header("Authorization", format!("Bearer {}", llm_config.api_key))
        .header("Content-Type", "application/json");
    
    for header in &llm_config.extra_headers {
        request = request.header(&header.key, &header.value);
    }
    
    let response = request
        .json(&serde_json::json!({
            "model": llm_config.model,
            "messages": [
                {
                    "role": "user",
                    "content": prompt
                }
            ],
            "temperature": 0.3
        }))
        .send()
        .await
        .map_err(|e| format!("LLM API 请求失败: {}", e))?;
    
    if !response.status().is_success() {
        let error_text = response.text().await.unwrap_or_default();
        return Err(format!("LLM API 错误: {}", error_text));
    }
    
    let json: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("解析 LLM 响应失败: {}", e))?;
    
    let content = json["choices"][0]["message"]["content"]
        .as_str()
        .ok_or("无法从 LLM 响应中提取内容")?;
    
    // 解析 JSON
    let guide: StyleGuide = serde_json::from_str(content)
        .map_err(|e| format!("解析风格指南失败: {}, 原始内容: {}", e, content))?;
    
    // 生成 Markdown 内容
    let md_content = format!(r#"# {}

{}

## 风格锚点
{}

## 配色
{}

## 背景基调
{}

## 标题样式
{}

## 卡片样式
{}

## 图标样式
{}

## 线条样式
{}

## 禁止规则
{}
"#,
        style_name,
        guide.prompt_anchor,
        guide.prompt_anchor,
        guide.style_core.palette.iter().map(|c| format!("- {}", c)).collect::<Vec<_>>().join("\n"),
        guide.style_core.background_tone,
        guide.style_core.title_style,
        guide.style_core.card_style,
        guide.style_core.icon_style,
        guide.style_core.line_style,
        guide.negative_rules.iter().map(|r| format!("- {}", r)).collect::<Vec<_>>().join("\n")
    );
    
    error_log::log_info(cwd_path, &format!("PPTX Style extracted: {}", style_name));
    
    Ok(md_content)
}

/// 从模板提取风格
#[tauri::command]
pub async fn extract_style_from_template(
    cwd: State<'_, Arc<PathBuf>>,
    template_name: String,
    style_name: String,
) -> Result<String, String> {
    let cwd_path = cwd.inner().clone();
    
    error_log::log_info(&cwd_path, &format!("Extracting style from template: {}", template_name));
    
    // 查找模板文件夹
    let template_path = cwd.join("templates").join(&template_name);
    if !template_path.exists() {
        return Err(format!("模板不存在: {}", template_name));
    }
    
    // 优先使用PPTX文件
    let pptx_path = template_path.join("template.pptx");
    if pptx_path.exists() {
        // 读取配置
        let config_path = cwd.join("config.yaml");
        if !config_path.exists() {
            return Err("配置文件不存在，请先完成 API 配置".to_string());
        }
        
        let config_content = tokio::fs::read_to_string(&config_path)
            .await
            .map_err(|e| format!("读取配置失败: {}", e))?;
        
        let config: crate::config::AppConfig = serde_yaml::from_str(&config_content)
            .map_err(|e| format!("解析配置失败: {}", e))?;
        
        if config.llm.api_key.is_empty() {
            return Err("请先配置 LLM API Key".to_string());
        }
        
        return extract_style_from_pptx(&pptx_path, &style_name, &config.llm, &cwd_path).await;
    }
    
    // 如果没有PPTX文件，使用图片
    let content_path = template_path.join("content.png");
    if content_path.exists() {
        // 读取配置
        let config_path = cwd.join("config.yaml");
        if !config_path.exists() {
            return Err("配置文件不存在，请先完成 API 配置".to_string());
        }
        
        let config_content = tokio::fs::read_to_string(&config_path)
            .await
            .map_err(|e| format!("读取配置失败: {}", e))?;
        
        let config: crate::config::AppConfig = serde_yaml::from_str(&config_content)
            .map_err(|e| format!("解析配置失败: {}", e))?;
        
        if config.llm.api_key.is_empty() {
            return Err("请先配置 LLM API Key".to_string());
        }
        
        return extract_style_from_file(cwd, content_path.to_string_lossy().to_string(), style_name).await;
    }
    
    Err(format!("模板中未找到可用的文件（需要 template.pptx 或 content.png）"))
}
