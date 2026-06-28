# 风格一致性机制设计规格

## 概述

引入三层风格约束机制，确保多页信息图表的视觉一致性：
1. 风格指南提取 - 从参考图或用户定义提取统一风格规范
2. 版式家族约束 - 10种抽象版式模式，强制页面多样性
3. 提示词风格锁定 - 生成图片时注入统一风格锚点

## 背景

当前 pptbuilder 的风格管理仅通过 Markdown 文件描述风格，缺乏：
- 从参考图自动提取风格的能力
- 版式多样性和约束机制
- 多页生成时的风格一致性保证

## 核心组件

### 1. 风格指南结构 (StyleGuide)

```rust
pub struct StyleGuide {
    /// 风格核心定义
    pub style_core: StyleCore,
    /// 可用版式家族列表
    pub layout_families: Vec<String>,
    /// 元素原语（语义分组、重点标记等）
    pub element_primitives: Vec<String>,
    /// 变化策略
    pub variation_policy: VariationPolicy,
    /// 禁止规则
    pub negative_rules: Vec<String>,
    /// 风格锚点描述（注入到每页提示词）
    pub prompt_anchor: String,
    /// 风格遵循强度
    pub adherence_level: AdherenceLevel,
}

pub struct StyleCore {
    /// 背景基调（如"浅灰底色"）
    pub background_tone: String,
    /// 配色方案（主色、辅助色等）
    pub palette: Vec<String>,
    /// 标题样式
    pub title_style: String,
    /// 卡片样式
    pub card_style: String,
    /// 图标样式
    pub icon_style: String,
    /// 线条样式
    pub line_style: String,
}

pub struct VariationPolicy {
    /// 同一版式最大连续重复次数
    pub same_layout_max_repeat: u32,
    /// 整套最少覆盖版式种类
    pub min_distinct_layout_families: u32,
    /// 是否允许局部重组
    pub allow_local_recomposition: bool,
}

pub enum AdherenceLevel {
    /// 宽松 - 优先保持系列感，允许重新组织
    Loose,
    /// 适度 - 框架统一、细节鲜活
    Balanced,
    /// 严格 - 锁定骨架与色彩节奏
    Strict,
}
```

### 2. 版式家族定义

```rust
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
```

### 3. 风格指南提取

从参考图提取风格规范：

```rust
pub async fn extract_style_guide_from_images(
    image_paths: &[PathBuf],
    style_notes: &str,
    config: &ApiConfig,
) -> Result<StyleGuide, String> {
    // 1. 构建多模态消息，最多3张参考图
    // 2. 调用 LLM 提取风格规范
    // 3. 返回结构化 StyleGuide
}
```

### 4. 版式分配约束

```rust
pub fn assign_layout_families(
    page_count: u32,
    style_guide: &StyleGuide,
) -> Vec<String> {
    // 规则：
    // 1. 相邻页不能重复同一版式
    // 2. 整套至少覆盖 min_distinct_layout_families 种版式
    // 3. 同一版式最多连续重复 same_layout_max_repeat 次
}
```

### 5. 提示词构建

```rust
pub fn build_image_prompt_with_style(
    page_content: &str,
    style_guide: &StyleGuide,
    layout_family: &str,
    page_type: &str,
    width: u32,
    height: u32,
) -> String {
    // 注入：
    // 1. 风格锚点 (prompt_anchor)
    // 2. 配色要求 (palette, background_tone)
    // 3. 版式约束 (layout_family)
    // 4. 风格遵循强度提示
    // 5. 禁止规则
}
```

## 数据存储

### 风格文件格式 (styles/*.md)

扩展现有 Markdown 格式，增加前置 YAML 配置：

```markdown
---
adherence: balanced
layout_constraint: true
---

# 商务风格

## 配色
- 主色：深海蓝 (#1E3A5F)
- 强调色：科技青 (#00B8D9)

## 背景基调
浅灰白底色 (#F8FAFC)

## 标题样式
深色粗体，层级清晰

## 卡片样式
圆角卡片带柔和阴影

## 图标样式
线性图标风格

## 线条样式
箭头连接线，关系清晰

## 风格锚点
统一使用深海蓝为主色调，科技青作为强调色，保持简洁现代的商务风格

## 禁止规则
- 不要使用干扰阅读的复杂背景
- 不要让页面之间的视觉语言突然断裂
```

### 项目配置 (project.json)

```json
{
  "name": "my-project",
  "style": "business",
  "style_guide": {
    "adherence_level": "balanced",
    "layout_assignments": {
      "1": "hero_with_supporting_cards",
      "2": "grid_n_x_m",
      "3": "timeline_horizontal"
    }
  },
  "reference_images": [
    "reference/style-1.png",
    "reference/style-2.png"
  ]
}
```

## API 设计

### 后端 Tauri 命令

```rust
/// 从参考图提取风格指南
#[tauri::command]
pub async fn extract_style_guide(
    project_name: String,
    reference_paths: Vec<String>,
    style_notes: String,
    config: ApiConfig,
) -> Result<StyleGuide, String>;

/// 获取页面版式分配
#[tauri::command]
pub async fn get_layout_assignments(
    project_name: String,
) -> Result<HashMap<u32, String>, String>;

/// 更新风格遵循强度
#[tauri::command]
pub async fn update_style_adherence(
    project_name: String,
    level: String,
) -> Result<(), String>;
```

### 前端组件

```
src/components/
├── StyleGuideEditor.vue    # 风格指南编辑器
├── LayoutFamilyPicker.vue   # 版式选择器
└── ReferenceImageUpload.vue # 参考图上传
```

## 实现任务

### 阶段一：核心数据结构 ✅

- [x] 创建 `src-tauri/src/style_guide.rs` 模块
- [x] 定义 StyleGuide、StyleCore、VariationPolicy 结构体
- [x] 定义版式家族常量和标签
- [x] 实现 AdherenceLevel 枚举

### 阶段二：风格指南提取 ✅

- [x] 实现 `extract_style_guide_from_images` 函数
- [x] 构建多模态提示词（参考图 + 分析指令）
- [x] 解析 LLM 返回的 JSON 为 StyleGuide
- [x] 创建 Tauri 命令 `extract_style_guide`

### 阶段三：版式分配 ✅

- [x] 实现 `LayoutAssigner::assign` 函数
- [x] 约束检查：相邻页不重复
- [x] 约束检查：最少覆盖种类
- [x] 存储到项目配置

### 阶段四：提示词集成 ✅

- [x] 修改 `build_image_prompt` 接受 StyleGuide
- [x] 注入风格锚点
- [x] 注入版式约束
- [x] 注入配色要求
- [x] 注入禁止规则

### 阶段五：前端界面

- [x] ~~创建 ReferenceImageUpload 组件~~ （使用模板图片作为参考，不需要用户上传）
- [x] ~~创建 StyleGuideEditor 组件~~ （已有 StyleManagement.vue 和 StyleEditor.vue）
- [x] 创建 LayoutFamilyPicker 组件
- [x] 添加风格遵循强度选择器
- [x] 集成到 Step3_Pages.vue：选择模板时自动提取风格指南

### 阶段六：测试验证

- [ ] 单元测试：版式分配约束
- [ ] 集成测试：风格提取流程
- [ ] 端到端测试：多页生成一致性

### 阶段六：测试验证

- [ ] 单元测试：版式分配约束
- [ ] 集成测试：风格提取流程
- [ ] 端到端测试：多页生成一致性

## 风险与缓解

| 风险 | 缓解措施 |
|------|---------|
| LLM 提取风格不稳定 | 提供详细示例，增加温度参数控制 |
| 版式约束过严导致生成困难 | 提供 "自动" 选项让 LLM 自由选择 |
| 参考图质量差影响提取 | 限制最多3张，增加预处理提示 |
| 风格锚点过长影响生成 | 压缩提示词，保留核心信息 |

## 验收标准

1. 可上传 1-3 张参考图，系统自动提取风格指南
2. 风格指南可在界面上查看和编辑
3. 版式分配符合约束规则
4. 生成图片时提示词包含风格锚点
5. 多页生成的视觉一致性明显提升
