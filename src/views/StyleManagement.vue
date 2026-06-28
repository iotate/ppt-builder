<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-dialog'
import { useI18n } from 'vue-i18n'

const { t } = useI18n()

interface StyleInfo {
  name: string
  path: string
  content?: string
  colors?: string[]
}

interface TemplateInfo {
  name: string
  path: string
  front_cover_path?: string
  content_path?: string
  back_cover_path?: string
  has_front_cover: boolean
  has_content: boolean
  has_back_cover: boolean
}

// 默认风格模板
const DEFAULT_STYLE_TEMPLATE = `# 风格名称

简要描述这个风格的特点：

## 风格锚点
统一使用XX为主色调，XX作为强调色，保持XX的视觉风格

## 配色
- 主色：颜色名 (#RRGGBB)
- 强调色：颜色名 (#RRGGBB)
- 背景色：颜色名 (#RRGGBB)
- 卡片色：颜色名 (#RRGGBB)
- 文本色：颜色名 (#RRGGBB)
- 辅助文本：颜色名 (#RRGGBB)

## 整体布局
- 顶部10%：左侧放置标题（字体、字重、颜色），右侧放置Logo（尺寸、位置）
- 底部5%：横线（颜色、粗细），横线下方左侧放置"XX"文字（字体、颜色），中间放置"XX"文字，右侧放置页码（格式、字体）
- 内容区域：占剩余85%，边距XXpx，圆角XXpx

## 背景基调
描述背景的底色和层次感

## 标题样式
描述标题的字体、字重、层级

## 卡片样式
描述卡片的圆角、阴影、质感

## 图标样式
描述图标的风格和颜色

## 线条样式
描述连接线的风格

## 禁止规则
- 不要出现无关的公司名称、Logo 或文字
- 不要使用干扰阅读的复杂背景
- 不要让页面之间的视觉语言突然断裂
`

// 必需的风格结构字段
const REQUIRED_SECTIONS = [
  '风格锚点',
  '配色',
  '整体布局',
  '背景基调',
  '标题样式',
  '卡片样式',
  '图标样式',
  '线条样式',
  '禁止规则'
]

const styles = ref<StyleInfo[]>([])
const loading = ref(false)

// 编辑弹窗
const showEditor = ref(false)
const editingName = ref('')
const editingContent = ref('')
const isNewStyle = ref(false)
const saving = ref(false)

// 提取风格
const extracting = ref(false)
const showExtractModal = ref(false)
const extractMode = ref<'file' | 'template'>('file')  // 提取模式
const extractName = ref('')
const extractFilePath = ref('')
const templates = ref<TemplateInfo[]>([])
const selectedTemplate = ref<string>('')

onMounted(async () => {
  await loadStyles()
})

async function loadStyles() {
  loading.value = true
  try {
    const files = await invoke<StyleInfo[]>('list_styles')
    // 加载每个风格的内容和提取颜色
    const stylesWithColors = await Promise.all(
      files.map(async (style) => {
        try {
          const content = await invoke<string>('get_style_content', { name: style.name })
          const colors = extractColors(content)
          return { ...style, content, colors }
        } catch {
          return { ...style, colors: [] }
        }
      })
    )
    styles.value = stylesWithColors
  } catch (e) {
    console.error(t('style.loadFailed'), e)
    styles.value = []
  } finally {
    loading.value = false
  }
}

// 从风格内容中提取颜色
function extractColors(content: string): string[] {
  const colors: string[] = []
  // 匹配 HEX 颜色 (#RRGGBB 或 #RGB)
  const hexPattern = /#([0-9A-Fa-f]{6}|[0-9A-Fa-f]{3})\b/g
  let match
  while ((match = hexPattern.exec(content)) !== null) {
    const color = match[0].toUpperCase()
    if (!colors.includes(color)) {
      colors.push(color)
    }
    if (colors.length >= 6) break // 最多显示6个颜色
  }
  return colors
}

async function createNewStyle() {
  isNewStyle.value = true
  editingName.value = ''
  editingContent.value = DEFAULT_STYLE_TEMPLATE
  showEditor.value = true
}

// 验证风格结构
function validateStyleContent(content: string): { valid: boolean; missing: string[] } {
  const missing: string[] = []
  
  for (const section of REQUIRED_SECTIONS) {
    // 检查是否存在 ## 风格锚点 这样的标题
    const pattern = new RegExp(`^##\\s+${section}`, 'm')
    if (!pattern.test(content)) {
      missing.push(section)
    }
  }
  
  return {
    valid: missing.length === 0,
    missing
  }
}

async function editStyle(style: StyleInfo) {
  isNewStyle.value = false
  editingName.value = style.name
  
  try {
    const content = await invoke<string>('get_style_content', { name: style.name })
    editingContent.value = content
    showEditor.value = true
  } catch (e) {
    alert(t('style.loadFailed') + '：' + e)
  }
}

async function saveStyle() {
  if (!editingName.value.trim()) {
    alert(t('style.enterName'))
    return
  }
  
  // 验证风格结构
  const validation = validateStyleContent(editingContent.value)
  if (!validation.valid) {
    const confirm = window.confirm(
      t('style.structureIncomplete') + '\n\n' + validation.missing.join('\n') + '\n\n' + t('style.saveAnyway')
    )
    if (!confirm) {
      return
    }
  }
  
  saving.value = true
  try {
    await invoke('save_style', { 
      name: editingName.value.trim(),
      content: editingContent.value 
    })
    
    showEditor.value = false
    await loadStyles()
  } catch (e) {
    alert(t('style.saveFailed') + '：' + e)
  } finally {
    saving.value = false
  }
}

// 打开提取弹窗时加载模板列表
async function openExtractModal() {
  showExtractModal.value = true
  extractMode.value = 'file'
  extractName.value = ''
  extractFilePath.value = ''
  selectedTemplate.value = ''
  
  // 加载模板列表
  try {
    templates.value = await invoke<TemplateInfo[]>('list_templates')
  } catch (e) {
    console.error(t('template.loadFailed'), e)
    templates.value = []
  }
}

// 打开文件选择器选择图片或PPTX
async function openFileForExtract() {
  const selected = await open({
    multiple: false,
    filters: [{
      name: 'Images & PPTX',
      extensions: ['png', 'jpg', 'jpeg', 'webp', 'pptx']
    }]
  })
  
  if (selected) {
    extractFilePath.value = selected as string
    // 从文件名提取风格名称
    const fileName = extractFilePath.value.split(/[/\\]/).pop() || ''
    extractName.value = fileName.replace(/\.(png|jpg|jpeg|webp|pptx)$/i, '')
  }
}

// 当选择模板时，自动设置风格名称
function onTemplateChange() {
  if (selectedTemplate.value) {
    extractName.value = selectedTemplate.value
  }
}

// 提取风格
async function extractStyle() {
  if (extractMode.value === 'template') {
    // 从模板提取
    if (!selectedTemplate.value) {
      alert(t('style.selectTemplateFirst'))
      return
    }
    if (!extractName.value.trim()) {
      alert(t('style.enterName'))
      return
    }
    
    extracting.value = true
    
    try {
      const result = await invoke<string>('extract_style_from_template', {
        templateName: selectedTemplate.value,
        styleName: extractName.value.trim()
      })
      
      // 关闭提取弹窗，打开编辑弹窗
      showExtractModal.value = false
      
      // 设置编辑弹窗内容
      isNewStyle.value = true
      editingName.value = extractName.value.trim()
      editingContent.value = result
      showEditor.value = true
      
      // 清空提取表单
      selectedTemplate.value = ''
      extractName.value = ''
    } catch (e) {
      alert(t('style.extractFailed') + '：' + e)
    } finally {
      extracting.value = false
    }
  } else {
    // 从文件提取
    if (!extractFilePath.value) {
      alert(t('style.selectFileFirst'))
      return
    }
    
    if (!extractName.value.trim()) {
      alert(t('style.enterName'))
      return
    }
    
    extracting.value = true
    
    try {
      const result = await invoke<string>('extract_style_from_file', {
        filePath: extractFilePath.value,
        styleName: extractName.value.trim()
      })
      
      // 关闭提取弹窗，打开编辑弹窗
      showExtractModal.value = false
      
      // 设置编辑弹窗内容
      isNewStyle.value = true
      editingName.value = extractName.value.trim()
      editingContent.value = result
      showEditor.value = true
      
      // 清空提取表单
      extractFilePath.value = ''
      extractName.value = ''
    } catch (e) {
      alert(t('style.extractFailed') + '：' + e)
    } finally {
      extracting.value = false
    }
  }
}

async function deleteStyle(style: StyleInfo) {
  if (!confirm(t('style.deleteConfirm') + `\n\n${style.name}.md ` + '将被删除。')) return
  
  try {
    await invoke('delete_style', { name: style.name })
    await loadStyles()
  } catch (e) {
    alert(t('style.deleteFailed') + '：' + e)
  }
}

function cancelEdit() {
  showEditor.value = false
}
</script>

<template>
  <div class="style-management">
    <div class="page-header">
      <h1 class="page-title">{{ t('style.title') }}</h1>
      <a-space>
        <a-button @click="loadStyles" :loading="loading">
          <template #icon>
            <svg viewBox="0 0 24 24" width="16" height="16" fill="currentColor">
              <path d="M17.65 6.35C16.2 4.9 14.21 4 12 4c-4.42 0-7.99 3.58-7.99 8s3.57 8 7.99 8c3.73 0 6.84-2.55 7.73-6h-2.08c-.82 2.33-3.04 4-5.65 4-3.31 0-6-2.69-6-6s2.69-6 6-6c1.66 0 3.14.69 4.22 1.78L13 11h7V4l-2.35 2.35z"/>
            </svg>
          </template>
          {{ t('common.refresh') }}
        </a-button>
        <a-button @click="openExtractModal">
          <template #icon>
            <svg viewBox="0 0 24 24" width="16" height="16" fill="currentColor">
              <path d="M21 19V5c0-1.1-.9-2-2-2H5c-1.1 0-2 .9-2 2v14c0 1.1.9 2 2 2h14c1.1 0 2-.9 2-2zM8.5 13.5l2.5 3.01L14.5 12l4.5 6H5l3.5-4.5z"/>
            </svg>
          </template>
          {{ t('style.extract') }}
        </a-button>
        <a-button type="primary" @click="createNewStyle">
          <template #icon><span>+</span></template>
          {{ t('style.create') }}
        </a-button>
      </a-space>
    </div>

    <p class="page-desc">
      {{ t('style.pageDesc') }}
    </p>

    <a-spin :spinning="loading">
      <div v-if="!loading && styles.length === 0" class="empty-state">
        <a-empty :description="t('style.noStyles')">
          <a-button type="primary" @click="createNewStyle">{{ t('style.create') }}</a-button>
        </a-empty>
      </div>

      <div v-else class="styles-grid">
        <a-card
          v-for="style in styles"
          :key="style.name"
          class="style-card"
        >
          <template #title>
            <div class="style-header">
              <span class="style-name">{{ style.name }}</span>
              <span class="style-filename">{{ style.name }}.md</span>
            </div>
          </template>
          <template #extra>
            <a-space>
              <a-button type="link" size="small" @click="editStyle(style)">{{ t('common.edit') }}</a-button>
              <a-popconfirm
                :title="t('style.deleteConfirm')"
                :ok-text="t('common.delete')"
                :cancel-text="t('common.cancel')"
                @confirm="deleteStyle(style)"
              >
                <a-button type="link" size="small" danger>{{ t('common.delete') }}</a-button>
              </a-popconfirm>
            </a-space>
          </template>
          
          <!-- 颜色色块 -->
          <div class="color-palette" v-if="style.colors && style.colors.length > 0">
            <div 
              v-for="color in style.colors" 
              :key="color" 
              class="color-block" 
              :style="{ backgroundColor: color }"
              :title="color"
            ></div>
          </div>
          <div class="no-colors" v-else>
            {{ t('style.noColors') }}
          </div>
        </a-card>
      </div>
    </a-spin>

    <!-- 编辑弹窗 -->
    <a-modal
      v-model:open="showEditor"
      :title="isNewStyle ? t('style.create') : t('style.edit')"
      :ok-text="t('common.save')"
      :cancel-text="t('common.cancel')"
      :confirm-loading="saving"
      @ok="saveStyle"
      @cancel="cancelEdit"
      width="700px"
    >
      <a-form layout="vertical">
        <a-form-item :label="t('style.name')" required>
          <a-input
            v-model:value="editingName"
            placeholder="例如：商务风格、创意风格"
            :disabled="!isNewStyle"
          />
        </a-form-item>
        <a-form-item :label="t('style.content') + ' (Markdown)'">
          <a-textarea
            v-model:value="editingContent"
            :rows="12"
            :placeholder="t('style.contentPlaceholder')"
          />
          <template #extra>
            <span class="field-hint">{{ t('style.contentHint') }}</span>
          </template>
        </a-form-item>
      </a-form>
    </a-modal>

    <!-- 提取风格弹窗 -->
    <a-modal
      v-model:open="showExtractModal"
      :title="t('style.extract')"
      :ok-text="t('style.extract')"
      :cancel-text="t('common.cancel')"
      :confirm-loading="extracting"
      @ok="extractStyle"
      @cancel="showExtractModal = false"
      width="600px"
    >
      <a-form layout="vertical">
        <!-- 提取模式选择 -->
        <a-form-item :label="t('style.extractMode')">
          <a-radio-group v-model:value="extractMode">
            <a-radio value="file">{{ t('style.extractFromFile') }}</a-radio>
            <a-radio value="template">{{ t('style.extractFromTemplate') }}</a-radio>
          </a-radio-group>
        </a-form-item>
        
        <!-- 从文件提取 -->
        <template v-if="extractMode === 'file'">
          <a-form-item :label="t('style.selectFile')" required>
            <a-input-group compact>
              <a-input
                v-model:value="extractFilePath"
                :placeholder="t('style.selectFilePlaceholder')"
                style="width: calc(100% - 80px)"
                readonly
              />
              <a-button @click="openFileForExtract">{{ t('style.browse') }}</a-button>
            </a-input-group>
            <template #extra>
              <span class="field-hint">{{ t('style.fileHint') }}</span>
            </template>
          </a-form-item>
          <a-form-item :label="t('style.name')" required>
            <a-input
              v-model:value="extractName"
              :placeholder="t('style.enterName')"
            />
          </a-form-item>
        </template>
        
        <!-- 从模板提取 -->
        <template v-else>
          <a-form-item :label="t('style.selectTemplate')" required>
            <a-select 
              v-model:value="selectedTemplate" 
              :placeholder="t('style.selectTemplate')"
              style="width: 100%"
              @change="onTemplateChange"
            >
              <a-select-option v-for="t in templates" :key="t.name" :value="t.name">
                {{ t.name }}
              </a-select-option>
            </a-select>
            <template #extra>
              <span class="field-hint">{{ t('style.templateHint') }}</span>
            </template>
          </a-form-item>
          
          <a-form-item :label="t('style.name')" required>
            <a-input
              v-model:value="extractName"
              :placeholder="t('style.namePlaceholder')"
            />
          </a-form-item>
        </template>
        
        <a-alert 
          type="info" 
          :message="t('style.extractNote')"
          :description="extractMode === 'file' 
            ? t('style.extractNoteFileDesc') 
            : t('style.extractNoteTemplateDesc')"
          show-icon
          style="margin-bottom: 16px"
        />
      </a-form>
    </a-modal>
  </div>
</template>

<style scoped>
.style-management {
  max-width: 1920px;
  margin: 0 auto;
  padding: 0 16px;
}

.page-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 8px;
}

.page-title {
  font-size: 20px;
  font-weight: 600;
  margin: 0;
  color: var(--text-primary);
}

.page-desc {
  color: var(--text-secondary);
  margin-bottom: 24px;
}

.empty-state {
  display: flex;
  justify-content: center;
  align-items: center;
  min-height: 200px;
  background: var(--bg-white);
  border-radius: var(--radius-md);
}

.styles-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(300px, 1fr));
  gap: 16px;
}

.style-card {
  cursor: default;
}

.style-header {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.style-name {
  font-weight: 500;
  font-size: 14px;
}

.style-filename {
  font-size: 11px;
  color: var(--text-disabled);
  font-family: monospace;
}

.color-palette {
  display: flex;
  gap: 8px;
  flex-wrap: wrap;
  padding: 8px 0;
}

.color-block {
  width: 32px;
  height: 32px;
  border-radius: 6px;
  border: 1px solid var(--border-color);
  cursor: pointer;
  transition: transform 0.2s;
}

.color-block:hover {
  transform: scale(1.1);
}

.no-colors {
  color: var(--text-disabled);
  font-size: 12px;
  padding: 8px 0;
}

.field-hint {
  font-size: 12px;
  color: var(--text-secondary);
}
</style>
