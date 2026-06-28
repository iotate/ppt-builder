<script setup lang="ts">
import { ref, onMounted, watch } from 'vue'
import { useRouter, useRoute } from 'vue-router'
import { invoke } from '@tauri-apps/api/core'
import { useI18n } from 'vue-i18n'

const { t } = useI18n()
const router = useRouter()
const route = useRoute()

// 左侧：提示词
const DEFAULT_PROMPT_ZH = `【目标】
简要描述这次汇报想达成的目标
例如：向管理层汇报项目进展，争取资源支持

【受众】
描述听众的背景和关注点
例如：公司高管，关注ROI和业务价值，时间有限

【场合】
描述汇报的场景和环境
例如：季度业务盘点会议，时间限制15分钟

【信息密度】
描述期望呈现的信息密度
例如：简洁 / 标准 / 密集

【主要内容】
描述汇报的内容
例如：北区业务大幅增长、南区业务小幅下滑.....`

const DEFAULT_PROMPT_EN = `【Goal】
Briefly describe the goal of this presentation
E.g., Report project progress to management, seek resource support

【Audience】
Describe the background and concerns of the audience
E.g., Company executives, focused on ROI and business value, limited time

【Context】
Describe the scenario and environment of the presentation
E.g., Quarterly business review meeting, 15-minute time limit

【Information Density】
Describe the expected information density
E.g., Concise / Standard / Dense

【Main Content】
Describe the presentation content
E.g., Significant growth in North region, slight decline in South region...`

function getDefaultPrompt() {
  return localStorage.getItem('locale') === 'en-US' ? DEFAULT_PROMPT_EN : DEFAULT_PROMPT_ZH
}

const promptContent = ref(getDefaultPrompt())
const savingPrompt = ref(false)

// 右侧：大纲
const outlineContent = ref('')
const saving = ref(false)
const splitting = ref(false)

// 风格选择
interface StyleInfo {
  name: string
  path: string
}
const styles = ref<StyleInfo[]>([])
const selectedStyle = ref('')

// 框架选择
interface SkeletonInfo {
  name: string
  path: string
}
const skeletons = ref<SkeletonInfo[]>([])
const selectedSkeleton = ref('none')

// 生成大纲状态
const generating = ref(false)
const infoDensity = ref<'simple' | 'medium' | 'detailed'>('medium')
const expectedPages = ref(6)

// 默认使用 16:9 横屏尺寸
const DEFAULT_SIZE = { name: '16:9 横屏', width: 1920, height: 1072 }
const projectId = route.params.id as string

// 监听风格选择变化，保存到项目设置
let saveSettingsTimer: ReturnType<typeof setTimeout> | null = null
watch(selectedStyle, () => {
  // 延迟保存，避免频繁调用
  if (saveSettingsTimer) {
    clearTimeout(saveSettingsTimer)
  }
  saveSettingsTimer = setTimeout(() => {
    saveProjectSettings()
  }, 500)
})

onMounted(async () => {
  await Promise.all([
    loadPrompt(),
    loadOutline(),
    loadStyles(),
    loadSkeletons(),
    loadProjectSettings()
  ])
})

async function loadPrompt() {
  try {
    // 先尝试加载需求文档
    const requirements = await invoke<string>('load_requirements', { projectName: projectId })
    if (requirements && requirements.trim()) {
      // 如果有需求文档，使用需求文档
      promptContent.value = requirements
    } else {
      // 否则加载prompt.md或使用默认模板
      const content = await invoke<string>('load_prompt', { projectName: projectId })
      promptContent.value = content || getDefaultPrompt()
    }
  } catch (e) {
    console.error('加载提示词失败', e)
    promptContent.value = getDefaultPrompt()
  }
}

async function loadOutline() {
  try {
    const content = await invoke<string>('load_outline', { projectName: projectId })
    outlineContent.value = content || ''
  } catch (e) {
    console.error('加载大纲失败', e)
  }
}

async function loadStyles() {
  try {
    const files = await invoke<StyleInfo[]>('list_styles')
    styles.value = files
    // 默认选中 default 风格
    if (files.length > 0 && !selectedStyle.value) {
      const defaultStyle = files.find(s => s.name === 'default')
      selectedStyle.value = defaultStyle ? defaultStyle.name : files[0].name
    }
  } catch (e) {
    console.error('加载风格失败', e)
    styles.value = []
  }
}

async function loadSkeletons() {
  try {
    const files = await invoke<SkeletonInfo[]>('list_skeletons')
    skeletons.value = files
  } catch (e) {
    console.error('加载框架失败', e)
    skeletons.value = []
  }
}

async function loadProjectSettings() {
  try {
    const settings = await invoke<{ template?: string; style?: string }>('open_project', { name: projectId })
    if (settings.style) {
      selectedStyle.value = settings.style
    }
  } catch (e) {
    console.error('加载项目设置失败', e)
  }
}

async function saveProjectSettings() {
  if (!selectedStyle.value) return
  try {
    await invoke('update_project_settings', {
      projectName: projectId,
      settings: {
        style: selectedStyle.value
      }
    })
  } catch (e) {
    console.error('保存项目设置失败', e)
  }
}

async function savePromptContent() {
  savingPrompt.value = true
  try {
    await invoke('save_prompt', {
      projectName: projectId,
      content: promptContent.value
    })
  } catch (e) {
    console.error('保存提示词失败', e)
  } finally {
    savingPrompt.value = false
  }
}

async function generateOutline() {
  if (!promptContent.value.trim()) {
    alert(t('outline.enterPrompt'))
    return
  }

  if (expectedPages.value < 1) {
    alert(t('outline.pagesMustBePositive'))
    return
  }

  generating.value = true
  try {
    // 先保存提示词
    await savePromptContent()

    // 保存项目设置（风格选择）
    await saveProjectSettings()

    // 获取配置
    const config = await invoke<any>('load_config')
    // 使用默认的 16:9 尺寸
    const selectedSize = DEFAULT_SIZE

    // 构建主题内容，添加前言内容设定AI角色
    const systemPrompt = `你是一位资深的演示与职场汇报专家，拥有15年的企业培训和高管汇报经验。你擅长帮助用户：
- 快速理清汇报思路，找到最核心的价值点
- 根据不同受众和场景调整表达策略
- 运用经典的汇报框架（如SCQA、STAR、PREP等）增强说服力
- 设计引人入胜的叙事节拍，让汇报更出彩

现在，请根据用户的需求，帮助用户梳理出一个清晰、有说服力的汇报大纲。`

    let topicContent = `${systemPrompt}\n\n---\n\n【用户需求】\n${promptContent.value}`

    // 如果选择了风格，加载风格内容并添加到提示词
    if (selectedStyle.value) {
      try {
        const styleContent = await invoke<string>('get_style_content', { name: selectedStyle.value })
        if (styleContent && styleContent.trim()) {
          topicContent = `${topicContent}\n\n【设计风格】\n请按照以下风格要求生成大纲：\n${styleContent}`
        }
      } catch (e) {
        console.error('加载风格内容失败', e)
      }
    }

    // 如果选择了框架，添加框架概要到提示词
    if (selectedSkeleton.value !== 'none') {
      const skeletonName = skeletons.value.find(s => s.name === selectedSkeleton.value)?.name || selectedSkeleton.value
      try {
        const skeletonContent = await invoke<string>('get_skeleton_content', { name: skeletonName })
        // 提取框架的简述和核心逻辑（取前几行）
        const lines = skeletonContent.split('\n').filter(line => line.trim())
        const summary = lines.slice(0, 5).join('\n')
        topicContent = `${topicContent}\n\n【汇报框架】请使用"${skeletonName}"框架：\n${summary}`
      } catch (e) {
        console.error('加载框架内容失败', e)
        topicContent = `${topicContent}\n\n【汇报框架】请使用"${skeletonName}"作为整体汇报结构的指导框架。`
      }
    }

    // 构建包含尺寸信息的主题
    const densityText = {
      simple: '简洁',
      medium: '标准',
      detailed: '详细'
    }
    const topicWithSize = `${topicContent}\n\n【输出要求】\n- 信息密度：${densityText[infoDensity.value]}\n- 期望页数：${expectedPages.value} 页\n- 图片尺寸：${selectedSize.name} (${selectedSize.width}×${selectedSize.height})\n- 方向：横向`

    // 调用 Tauri 命令生成大纲
    const content = await invoke<string>('generate_outline', {
      topic: topicWithSize,
      mode: infoDensity.value,
      expectedPages: expectedPages.value,
      config: config.llm
    })

    outlineContent.value = content

    // 自动保存大纲
    try {
      await invoke('save_outline', {
        projectName: projectId,
        content: content
      })
    } catch (saveError) {
      console.error('自动保存失败:', saveError)
    }
  } catch (e) {
    const errorMsg = String(e)
    console.error('大纲生成失败:', errorMsg)
    alert('生成失败：' + errorMsg)
  } finally {
    generating.value = false
  }
}

async function saveOutline() {
  if (!outlineContent.value.trim()) {
    return
  }

  saving.value = true
  try {
    await invoke('save_outline', {
      projectName: projectId,
      content: outlineContent.value
    })
  } catch (e) {
    alert(t('outline.saveFailed') + '：' + e)
  } finally {
    saving.value = false
  }
}

async function splitToPages() {
  if (!outlineContent.value.trim()) {
    alert(t('outline.generateOrEnterOutline'))
    return
  }

  if (!confirm(t('outline.confirmSplit'))) {
    return
  }

  splitting.value = true
  try {
    await invoke('split_pages', { name: projectId })
    router.push({ name: 'pages', params: { id: projectId } })
  } catch (e) {
    alert(t('outline.saveFailed') + '：' + e)
  } finally {
    splitting.value = false
  }
}

function goToBrainstorm() {
  router.push({ name: 'brainstorm', params: { id: projectId } })
}

function goToPages() {
  router.push({ name: 'pages', params: { id: projectId } })
}
</script>

<template>
  <div class="outline-page">
    <div class="page-header">
      <h1 class="page-title">{{ t('outline.title') }}</h1>
      <a-space>
        <a-button @click="goToBrainstorm">{{ t('outline.backToBrainstorm') }}</a-button>
        <a-button type="primary" @click="goToPages">{{ t('outline.goToPages') }}</a-button>
      </a-space>
    </div>

    <!-- 工具栏 -->
    <div class="toolbar-row">
      <a-space>
        <span class="toolbar-label">{{ t('outline.style') }}</span>
        <a-select v-model:value="selectedStyle" style="width: 100px" size="small">
          <a-select-option v-for="s in styles" :key="s.name" :value="s.name">{{ s.name }}</a-select-option>
        </a-select>
        <span class="toolbar-label">{{ t('outline.framework') }}</span>
        <a-select v-model:value="selectedSkeleton" style="width: 100px" size="small">
          <a-select-option value="none">{{ t('outline.freeStyle') }}</a-select-option>
          <a-select-option v-for="s in skeletons" :key="s.name" :value="s.name">{{ s.name }}</a-select-option>
        </a-select>
        <span class="toolbar-label">{{ t('outline.infoDensity') }}</span>
        <a-select v-model:value="infoDensity" style="width: 80px" size="small">
          <a-select-option value="simple">{{ t('outline.simple') }}</a-select-option>
          <a-select-option value="medium">{{ t('outline.medium') }}</a-select-option>
          <a-select-option value="detailed">{{ t('outline.detailed') }}</a-select-option>
        </a-select>
        <span class="toolbar-label">{{ t('outline.expectedPages') }}</span>
        <a-input-number
          v-model:value="expectedPages"
          :min="1"
          :max="30"
          style="width: 60px"
          size="small"
        />
        <a-button type="primary" size="small" @click="generateOutline" :loading="generating">{{ t('outline.generateOutline') }}</a-button>
      </a-space>
    </div>

    <div class="two-column-layout">
      <!-- 左侧：提示词 -->
      <div class="left-column">
        <div class="column-header">
          <span class="column-title">{{ t('outline.promptColumn') }}</span>
        </div>
        <div class="column-content">
          <a-textarea
            v-model:value="promptContent"
            class="prompt-editor"
            :placeholder="t('outline.enterPrompt')"
            :auto-size="{ minRows: 18, maxRows: 30 }"
          />
        </div>
      </div>

      <!-- 右侧：大纲 -->
      <div class="right-column">
        <div class="column-header">
          <span class="column-title">{{ t('outline.outlineColumn') }}</span>
          <a-space>
            <a-button @click="saveOutline" :loading="saving">{{ t('outline.saveOutline') }}</a-button>
            <a-tooltip :title="!outlineContent.trim() ? t('common.warning') : ''">
              <a-button @click="splitToPages" :loading="splitting" :disabled="!outlineContent.trim()">{{ t('outline.splitPages') }}</a-button>
            </a-tooltip>
          </a-space>
        </div>
        <div class="column-content">
          <a-textarea
            v-model:value="outlineContent"
            class="outline-editor"
            :placeholder="t('outline.outlinePlaceholder')"
            :auto-size="{ minRows: 18, maxRows: 30 }"
          />
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.outline-page {
  max-width: 1920px;
  margin: 0 auto;
  padding: 0 16px;
}

.page-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 16px;
}

.page-title {
  font-size: 20px;
  font-weight: 600;
  margin: 0;
  color: var(--text-primary);
}

.toolbar-row {
  padding: 12px 16px;
  background: var(--bg-white);
  border-radius: var(--radius-md);
  border: 1px solid var(--border-color);
  margin-bottom: 16px;
}

.toolbar-label {
  font-size: 13px;
  color: var(--text-secondary);
  margin-left: 8px;
}

.two-column-layout {
  display: grid;
  grid-template-columns: 1fr 2fr;
  gap: 16px;
  margin-bottom: 16px;
}

.left-column,
.right-column {
  background: var(--bg-white);
  border-radius: var(--radius-md);
  border: 1px solid var(--border-color);
  display: flex;
  flex-direction: column;
  min-height: 600px;
}

.column-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 12px 16px;
  border-bottom: 1px solid var(--border-color);
  flex-shrink: 0;
}

.column-title {
  font-weight: 500;
  font-size: 14px;
  color: var(--text-primary);
}

.column-content {
  flex: 1;
  padding: 12px;
  display: flex;
  flex-direction: column;
}

.prompt-editor,
.outline-editor {
  font-family: 'Consolas', 'Monaco', 'Courier New', monospace;
  font-size: 14px;
  line-height: 1.6;
  flex: 1;
}
</style>
