<script setup lang="ts">
import { ref, onMounted, onUnmounted, computed, watch } from 'vue'
import { useRouter, useRoute } from 'vue-router'
import { invoke } from '@tauri-apps/api/core'
import { convertFileSrc } from '@tauri-apps/api/core'
import { useI18n } from 'vue-i18n'

const { t } = useI18n()


interface PageInfo {
  page_num: number
  title: string
  markdown: string
  image_path?: string
  image_status: 'pending' | 'generating' | 'done' | 'failed'
}

interface PageFileInfo {
  page_num: number
  title: string
  md_path: string
  png_path?: string
  status: string
}

interface TemplateInfo {
  name: string
  path: string
  has_front_cover: boolean
  has_content: boolean
  has_back_cover: boolean
}

interface StyleInfo {
  name: string
  path: string
}

// 图片预览弹窗
const previewModalVisible = ref(false)
const previewImageUrl = ref('')
const previewPageIndex = ref(0)

function openImagePreview(imagePath: string) {
  previewImageUrl.value = imagePath
  previewPageIndex.value = selectedPageIndex.value
  previewModalVisible.value = true
}

function closeImagePreview() {
  previewModalVisible.value = false
}

// 打开文件夹
async function openImageFolder(imagePath: string) {
  try {
    // 获取文件夹路径
    const lastSep = imagePath.lastIndexOf('\\')
    const folderPath = lastSep > 0 ? imagePath.substring(0, lastSep) : imagePath.substring(0, imagePath.lastIndexOf('/'))
    await invoke('open_folder', { path: folderPath })
  } catch (e) {
    console.error('打开文件夹失败', e)
    alert('打开文件夹失败：' + e)
  }
}



// Convert file path to asset URL for display (synchronous)
// 添加时间戳参数防止浏览器缓存
function getImageUrl(imagePath: string | undefined): string {
  if (!imagePath) return '';
  // Normalize Windows backslashes to forward slashes
  const normalizedPath = imagePath.replace(/\\/g, '/');
  // convertFileSrc with 'asset' protocol explicitly specified for Tauri v2
  const baseUrl = convertFileSrc(normalizedPath, 'asset');
  // 添加时间戳参数强制刷新缓存
  return `${baseUrl}?t=${Date.now()}`;
}

const router = useRouter()
const route = useRoute()

const pages = ref<PageInfo[]>([])
const selectedPageIndex = ref(0)
const loading = ref(false)
const saving = ref(false)

// 批量生成
const generatingAll = ref(false)
const generatingProgress = ref(0)

// 导出 PDF
const exporting = ref(false)

// 导出 PPTX
const exportingSimplePptx = ref(false)
const exportingEditablePptx = ref(false)
const exportingTemplatePptx = ref(false)
const exportProgress = ref('')
const exportPercent = ref(0)

// 监听导出进度事件
import { listen } from '@tauri-apps/api/event'

let unlistenProgress: (() => void) | null = null

onMounted(async () => {
  // 监听导出进度
  unlistenProgress = await listen<{ stage: string; current: number; total: number; message: string; percent: number }>('export-progress', (event) => {
    exportProgress.value = event.payload.message
    exportPercent.value = event.payload.percent
    console.log(`Export progress: ${event.payload.stage} - ${event.payload.message}`)
  })
})

onUnmounted(() => {
  if (unlistenProgress) {
    unlistenProgress()
  }
})

// 配置选项 - 从后端加载
const templates = ref<TemplateInfo[]>([])
const styles = ref<StyleInfo[]>([])
const selectedTemplate = ref('none')
const selectedStyle = ref('')

// 默认使用 16:9 横屏尺寸
const DEFAULT_SIZE = { name: '16:9 横屏', width: 1920, height: 1072 }

// 版式分配和风格遵循强度
const layoutAssignments = ref<Record<number, string>>({})
const adherenceLevel = ref<'loose' | 'balanced' | 'strict'>('strict')

// 图片微调
const showRefineModal = ref(false)
const refinePrompt = ref('')
const refining = ref(false)

const projectId = route.params.id as string

const selectedPage = computed(() => pages.value[selectedPageIndex.value] || null)

// 监听选择变化，自动保存项目设置
let saveSettingsTimer: ReturnType<typeof setTimeout> | null = null
watch([selectedTemplate], () => {
  // 延迟保存，避免频繁调用
  if (saveSettingsTimer) {
    clearTimeout(saveSettingsTimer)
  }
  saveSettingsTimer = setTimeout(() => {
    saveProjectSettings()
  }, 500)
})

// 键盘事件处理
function handleKeydown(e: KeyboardEvent) {
  // 如果正在输入文本框，不处理方向键
  const target = e.target as HTMLElement
  if (target.tagName === 'TEXTAREA' || target.tagName === 'INPUT') {
    return
  }
  
  if (e.key === 'ArrowLeft') {
    e.preventDefault()
    if (previewModalVisible.value) {
      // 弹窗中切换上一页
      if (previewPageIndex.value > 0) {
        previewPageIndex.value--
        const page = pages.value[previewPageIndex.value]
        if (page?.image_path) {
          previewImageUrl.value = page.image_path
        }
      }
    } else {
      // 正常切换上一页
      if (selectedPageIndex.value > 0) {
        selectPage(selectedPageIndex.value - 1)
      }
    }
  } else if (e.key === 'ArrowRight') {
    e.preventDefault()
    if (previewModalVisible.value) {
      // 弹窗中切换下一页
      if (previewPageIndex.value < pages.value.length - 1) {
        previewPageIndex.value++
        const page = pages.value[previewPageIndex.value]
        if (page?.image_path) {
          previewImageUrl.value = page.image_path
        }
      }
    } else {
      // 正常切换下一页
      if (selectedPageIndex.value < pages.value.length - 1) {
        selectPage(selectedPageIndex.value + 1)
      }
    }
  } else if (e.key === 'Escape' && previewModalVisible.value) {
    closeImagePreview()
  }
}

onMounted(async () => {
  // 添加键盘事件监听
  window.addEventListener('keydown', handleKeydown)
  
  await Promise.all([
    loadPages(),
    loadTemplates(),
    loadStyles()
  ])
  // 加载项目设置
  await loadPromptSize()
})

onUnmounted(() => {
  // 移除键盘事件监听
  window.removeEventListener('keydown', handleKeydown)
})

async function loadPromptSize() {
  try {
    const project = await invoke<any>('open_project', { name: projectId })
    if (project) {
      // 加载项目保存的模板和风格
      if (project.template) {
        selectedTemplate.value = project.template
      }
      if (project.style) {
        selectedStyle.value = project.style
      }
    }
  } catch (e) {
    console.error('加载项目设置失败', e)
  }
}

// 保存项目设置（模板、风格）
async function saveProjectSettings() {
  try {
    await invoke('update_project_settings', {
      name: projectId,
      template: selectedTemplate.value === 'none' ? null : selectedTemplate.value || null,
      style: selectedStyle.value || null,
      sizeIndex: 0  // 固定使用 16:9
    })
  } catch (e) {
    console.error('保存项目设置失败', e)
  }
}

async function loadTemplates() {
  try {
    templates.value = await invoke<TemplateInfo[]>('list_templates')
    // 默认保持 'none'，如果项目有保存的模板则会在 loadPromptSize 中设置
  } catch (e) {
    console.error('加载模板失败', e)
    templates.value = []
  }
}

async function loadStyles() {
  try {
    styles.value = await invoke<StyleInfo[]>('list_styles')
    if (styles.value.length > 0 && !selectedStyle.value) {
      selectedStyle.value = styles.value[0].name
    }
  } catch (e) {
    console.error('加载风格失败', e)
    styles.value = []
  }
}

async function loadPages() {
  loading.value = true
  try {
    const pageFiles = await invoke<PageFileInfo[]>('list_pages', { projectName: projectId })
    
    // 一次性加载所有页面内容
    pages.value = await Promise.all(
      pageFiles.map(async (pf) => {
        let markdown = ''
        try {
          markdown = await invoke<string>('read_page', { projectName: projectId, pageNum: pf.page_num })
        } catch (e) {
          console.error(`加载页面 ${pf.page_num} 内容失败:`, e)
        }
        return {
          page_num: pf.page_num,
          title: pf.title,
          markdown: markdown,
          image_path: pf.png_path,
          image_status: pf.png_path ? 'done' : 'pending'
        }
      })
    )
  } catch (e) {
    console.error('加载页面失败', e)
    pages.value = []
  } finally {
    loading.value = false
  }
}

async function selectPage(index: number) {
  selectedPageIndex.value = index
  // 不再需要每次切换时加载，因为 loadPages 已经加载了所有内容
}

// 删除当前页面
async function deleteCurrentPage() {
  if (!selectedPage.value) return
  
  if (pages.value.length <= 1) {
    alert('至少需要保留一个页面')
    return
  }
  
  const pageNum = selectedPage.value.page_num
  
  try {
    await invoke('delete_page', {
      projectName: projectId,
      pageNum: pageNum
    })
    
    // 重新加载页面列表
    await loadPages()
    
    // 调整选中页
    if (selectedPageIndex.value >= pages.value.length) {
      selectedPageIndex.value = pages.value.length - 1
    }
  } catch (e) {
    console.error('删除页面失败:', e)
    alert('删除页面失败：' + e)
  }
}

// 保存所有页面内容
async function saveAllPages() {
  if (pages.value.length === 0) {
    return
  }
  
  saving.value = true
  let savedCount = 0
  let failedCount = 0
  
  // 遍历所有页面保存
  for (const page of pages.value) {
    try {
      // 只保存有内容的页面（markdown 不为空）
      const content = page.markdown ?? ''
      await invoke('save_page', { 
        projectName: projectId, 
        pageNum: page.page_num, 
        content: content 
      })
      savedCount++
    } catch (e) {
      failedCount++
      console.error(`保存页面 ${page.page_num} 失败:`, e)
    }
  }
  
  saving.value = false
  
}

// 是否有已生成的图片
const hasGeneratedImages = computed(() => {
  return pages.value.some(p => p.image_status === 'done')
})

// 生成当前图片
async function generateCurrentImage() {
  if (!selectedPage.value) return

  if (!selectedStyle.value) {
    alert('请先选择风格')
    return
  }
  
  const config = await invoke<any>('load_config')
  const size = DEFAULT_SIZE
  
  // 获取当前页面的版式分配
  const currentLayout = layoutAssignments.value[selectedPage.value.page_num] || null
  
  // 清理已有图片和状态
  selectedPage.value.image_path = undefined
  selectedPage.value.image_status = 'generating'
  
  try {
    // 返回值是图片路径字符串
    const imagePath = await invoke<string>('generate_image', { 
      projectName: projectId,
      pageNum: selectedPage.value.page_num,
      template: selectedTemplate.value === 'none' ? null : selectedTemplate.value || null,
      style: selectedStyle.value || null,
      width: size.width,
      height: size.height,
      config: config.img,
      layoutFamily: currentLayout,
      adherenceLevel: adherenceLevel.value
    })
    selectedPage.value.image_status = 'done'
    // 更新图片路径以立即刷新显示
    if (imagePath) {
      selectedPage.value.image_path = imagePath
    }
    // 成功时不提示，直接展示
  } catch (e) {
    selectedPage.value.image_status = 'failed'
    const errorMsg = String(e)
    console.error('图片生成失败:', errorMsg)
    alert('图片生成失败：' + errorMsg + '\n\n详细信息请查看 错误日志')
  }
}

// 批量生成图片（调用后端并发接口，最多 3 个并发）
async function generateAllImages() {
  if (!selectedStyle.value) {
    alert('请先选择风格')
    return
  }
  
  const config = await invoke<any>('load_config')
  const size = DEFAULT_SIZE
  
  generatingAll.value = true
  generatingProgress.value = 0
  
  // 先标记所有页面为生成中
  for (let i = 0; i < pages.value.length; i++) {
    pages.value[i].image_path = undefined
    pages.value[i].image_status = 'generating'
  }
  
  try {
    // 调用后端并发接口
    const results = await invoke<Array<{
      page_num: number
      status: any
      output_path: string | null
      error: string | null
    }>>('generate_all_images', {
      projectName: projectId,
      options: {
        template: selectedTemplate.value === 'none' ? null : selectedTemplate.value || null,
        style: selectedStyle.value || null,
        width: size.width,
        height: size.height
      },
      config: config.img
    })
    
    // 更新每个页面的状态
    let successCount = 0
    let failCount = 0
    const errors: string[] = []
    
    for (const result of results) {
      const pageIndex = pages.value.findIndex(p => p.page_num === result.page_num)
      if (pageIndex === -1) continue
      
      // 检查状态 - Rust 枚举序列化为字符串 "Success" 或对象 { "Failed": "错误信息" }
      const status = result.status
      let isSuccess = false
      let errorMsg = result.error || '未知错误'
      
      if (typeof status === 'string') {
        // 字符串格式：直接比较
        isSuccess = status === 'Success'
        if (status.startsWith('Failed(')) {
          // 解析 "Failed(错误信息)" 格式
          const match = status.match(/^Failed\((.+)\)$/)
          if (match) errorMsg = match[1]
        }
      } else if (typeof status === 'object' && status !== null) {
        // 对象格式：{ "Success": null } 或 { "Failed": "错误信息" }
        if ('Success' in status) {
          isSuccess = true
        } else if ('Failed' in status) {
          errorMsg = (status as any).Failed || errorMsg
        }
      }
      
      if (isSuccess && result.output_path) {
        pages.value[pageIndex].image_status = 'done'
        pages.value[pageIndex].image_path = result.output_path
        successCount++
      } else {
        pages.value[pageIndex].image_status = 'failed'
        failCount++
        errors.push(`页面 ${result.page_num}: ${errorMsg}`)
      }
    }
    
    generatingProgress.value = 100
    
  } catch (e) {
    console.error('批量生成失败:', e)
    alert('批量生成失败：' + e)
    // 重置所有页面状态
    for (let i = 0; i < pages.value.length; i++) {
      if (pages.value[i].image_status === 'generating') {
        pages.value[i].image_status = 'pending'
      }
    }
  } finally {
    generatingAll.value = false
  }
}

// 打开微调弹窗
function openRefineModal() {
  if (!selectedPage.value?.image_path) {
    alert('请先生成图片')
    return
  }
  refinePrompt.value = ''
  showRefineModal.value = true
}

// 微调图片
async function refineImage() {
  if (!selectedPage.value) return
  
  if (!refinePrompt.value.trim()) {
    alert('请输入微调要求')
    return
  }
  
  const config = await invoke<any>('load_config')
  const size = DEFAULT_SIZE
  
  refining.value = true
  
  try {
    const imagePath = await invoke<string>('refine_image_with_reference', {
      projectName: projectId,
      pageNum: selectedPage.value.page_num,
      refinePrompt: refinePrompt.value.trim(),
      width: size.width,
      height: size.height,
      config: config.img
    })
    
    // 强制刷新图片：先清空再赋值
    if (imagePath) {
      selectedPage.value.image_path = undefined
      // 使用 nextTick 确保 Vue 完成更新
      await new Promise(resolve => setTimeout(resolve, 10))
      selectedPage.value.image_path = imagePath
    }
    
    showRefineModal.value = false
    // 成功时不提示，直接展示
  } catch (e) {
    console.error('图片微调失败:', e)
    alert('图片微调失败：' + e)
  } finally {
    refining.value = false
  }
}

// 导出 PDF
async function exportPdf() {
  // 检查是否有图片
  const hasImages = pages.value.some(p => p.image_status === 'done')
  if (!hasImages) {
    alert('请先生成图片')
    return
  }
  
  // 检查是否有未生成的图片
  const pendingPages = pages.value.filter(p => p.image_status !== 'done')
  if (pendingPages.length > 0) {
    const confirm = window.confirm(`还有 ${pendingPages.length} 页图片未生成，确定要导出吗？`)
    if (!confirm) {
      return
    }
  }
  
  exporting.value = true
  try {
    const pdfPath = await invoke<string>('export_pdf', { 
        projectName: projectId
      })
      
      // 打开导出的 PDF 文件所在文件夹
      try {
        await invoke('open_folder', { path: pdfPath })
      } catch (e) {
        console.error('打开文件失败', e)
      }
    } catch (e) {
      console.error('PDF 导出失败', e)
      alert('导出失败：' + e)
    } finally {
      exporting.value = false
    }
  }

// 导出简单 PPTX（仅图片）
async function exportSimplePptx() {
  const hasImages = pages.value.some(p => p.image_status === 'done')
  if (!hasImages) {
    alert('请先生成图片')
    return
  }
  
  const pendingPages = pages.value.filter(p => p.image_status !== 'done')
  if (pendingPages.length > 0) {
    const confirm = window.confirm(`还有 ${pendingPages.length} 页图片未生成，确定要导出吗？`)
    if (!confirm) {
      return
    }
  }
  
  exportingSimplePptx.value = true
  try {
    const pptxPath = await invoke<string>('export_simple_pptx', {
      projectName: projectId
    })
    
    // 打开导出的 PPTX 文件所在文件夹
    try {
      await invoke('open_folder', { path: pptxPath })
    } catch (e) {
      console.error('打开文件失败', e)
    }
  } catch (e) {
    console.error('PPTX 导出失败', e)
    alert('导出失败：' + e)
  } finally {
    exportingSimplePptx.value = false
  }
}



function getStatusColor(status: string): string {
  const colors: Record<string, string> = {
    pending: 'default',
    generating: 'processing',
    done: 'success',
    failed: 'error'
  }
  return colors[status] || 'default'
}

// 导出增强版可编辑 PPTX（元素图 + 分割 + AI 分析）
async function exportEditablePptxWithElements() {
  const hasImages = pages.value.some(p => p.image_status === 'done')
  if (!hasImages) {
    alert('请先生成图片')
    return
  }
  
  const pendingPages = pages.value.filter(p => p.image_status !== 'done')
  if (pendingPages.length > 0) {
    const confirm = window.confirm(`还有 ${pendingPages.length} 页图片未生成，确定要导出吗？`)
    if (!confirm) {
      return
    }
  }
  
  // 提示用户这将调用 AI 生成元素图并分析
  const confirm = window.confirm('增强版可编辑 PPT 导出将：\n1. 调用 AI 生成去文字的元素图\n2. 分割元素图得到独立元素\n3. 调用 AI 分析文字位置\n\n这可能需要较长时间。确定要继续吗？')
  if (!confirm) {
    return
  }
  
  exportingEditablePptx.value = true
  exportProgress.value = '正在初始化...'
  exportPercent.value = 0
  
  try {
    const pptxPath = await invoke<string>('export_editable_pptx_with_elements', {
      projectName: projectId
    })
    
    // 打开导出的 PPTX 文件所在文件夹
    try {
      await invoke('open_folder', { path: pptxPath })
    } catch (e) {
      console.error('打开文件失败', e)
    }
  } catch (e) {
    console.error('增强版可编辑 PPTX 导出失败', e)
    alert('导出失败：' + e)
  } finally {
    exportingEditablePptx.value = false
    exportProgress.value = ''
    exportPercent.value = 0
  }
}

// 基于模板导出可编辑 PPTX
async function exportTemplateBasedPptx() {
  const hasImages = pages.value.some(p => p.image_status === 'done')
  if (!hasImages) {
    alert('请先生成图片')
    return
  }
  
  if (selectedTemplate.value === 'none' || !selectedTemplate.value) {
    alert('请先选择模板')
    return
  }
  
  const pendingPages = pages.value.filter(p => p.image_status !== 'done')
  if (pendingPages.length > 0) {
    const confirm = window.confirm(`还有 ${pendingPages.length} 页图片未生成，确定要导出吗？`)
    if (!confirm) {
      return
    }
  }
  
  exportingTemplatePptx.value = true
  exportProgress.value = '正在分析模板...'
  exportPercent.value = 0
  
  try {
    const pptxPath = await invoke<string>('export_editable_pptx_from_template', {
      projectName: projectId,
      templateName: selectedTemplate.value
    })
    
    // 打开导出的 PPTX 文件所在文件夹
    try {
      await invoke('open_folder', { path: pptxPath })
    } catch (e) {
      console.error('打开文件失败', e)
    }
  } catch (e) {
    console.error('基于模板的 PPTX 导出失败', e)
    alert('导出失败：' + e)
  } finally {
    exportingTemplatePptx.value = false
    exportProgress.value = ''
    exportPercent.value = 0
  }
}

function goBack() {
  router.push({ name: 'outline', params: { id: projectId } })
}
</script>

<template>
  <div class="pages-page">
    <div class="page-header">
      <h1 class="page-title">{{ t('pages.title') }}</h1>
      <a-button @click="goBack">{{ t('pages.backToOutline') }}</a-button>
    </div>

    <!-- 工具栏 -->
    <div class="toolbar">
      <a-space>
        <span class="toolbar-label">{{ t('pages.style') }}</span>
        <a-tooltip :title="selectedStyle ? '' : t('pages.selectStyleFirst')">
          <a-select v-model:value="selectedStyle" style="width: 100px" size="small" :placeholder="t('pages.style')" disabled>
            <a-select-option v-for="s in styles" :key="s.name" :value="s.name">{{ s.name }}</a-select-option>
          </a-select>
        </a-tooltip>
        <span class="toolbar-label">{{ t('pages.template') }}</span>
        <a-select v-model:value="selectedTemplate" style="width: 120px" size="small" :placeholder="t('pages.template')">
          <a-select-option value="none">{{ t('pages.freeStyle') }}</a-select-option>
          <a-select-option v-for="t in templates" :key="t.name" :value="t.name">{{ t.name }}</a-select-option>
        </a-select>
        <a-divider type="vertical" />
        <a-popconfirm
          :title="t('common.confirm') + '?'"
          :ok-text="t('common.delete')"
          :cancel-text="t('common.cancel')"
          @confirm="deleteCurrentPage"
        >
          <a-button type="text" danger :disabled="pages.length <= 1">{{ t('pages.deleteCurrent') }}</a-button>
        </a-popconfirm>
        <a-button @click="saveAllPages" :loading="saving">{{ t('common.save') }}</a-button>
        <a-button @click="generateCurrentImage" :loading="selectedPage?.image_status === 'generating'">{{ t('pages.generateImage') }}</a-button>
        <a-button type="primary" @click="generateAllImages" :loading="generatingAll">{{ t('pages.generateAll') }}</a-button>
        <a-tooltip :title="!hasGeneratedImages ? t('pages.noImages') : ''">
          <a-button @click="exportPdf" :disabled="!hasGeneratedImages" :loading="exporting">{{ t('pages.exportPdf') }}</a-button>
        </a-tooltip>
        <a-divider type="vertical" />
        <a-tooltip :title="!hasGeneratedImages ? t('pages.noImages') : ''">
          <a-button @click="exportSimplePptx" :disabled="!hasGeneratedImages" :loading="exportingSimplePptx">{{ t('pages.exportPptx') }}</a-button>
        </a-tooltip>
        <a-tooltip :title="!hasGeneratedImages ? t('pages.noImages') : ''">
          <a-button @click="exportEditablePptxWithElements" :disabled="!hasGeneratedImages" :loading="exportingEditablePptx">{{ t('pages.exportEditablePptx') }}</a-button>
        </a-tooltip>
        <a-tooltip :title="!hasGeneratedImages || selectedTemplate === 'none' ? t('pages.selectTemplateAndGenerateFirst') : ''">
          <a-button type="primary" @click="exportTemplateBasedPptx" :disabled="!hasGeneratedImages || selectedTemplate === 'none'" :loading="exportingTemplatePptx">{{ t('pages.exportTemplatePptx') }}</a-button>
        </a-tooltip>
      </a-space>
    </div>

    <!-- 导出进度条 -->
    <div v-if="exportingEditablePptx || exportingTemplatePptx" class="progress-bar">
      <a-progress :percent="exportPercent" status="active" />
      <div class="progress-text">{{ exportProgress }}</div>
    </div>

    <!-- 生成进度条 -->
    <div v-if="generatingAll" class="progress-bar">
      <a-progress :percent="generatingProgress" status="active" />
    </div>

    <a-spin :spinning="loading">
      <div v-if="pages.length === 0" class="empty-state">
        <a-empty description="暂无页面，请先在大纲页面切分页面" />
      </div>

      <div v-else class="editor-layout">
        <!-- 左侧：页面列表 + Markdown 编辑器 -->
        <div class="left-panel">
          <div class="page-tabs">
            <div
              v-for="(page, index) in pages"
              :key="page.page_num"
              class="page-tab"
              :class="{ active: selectedPageIndex === index }"
              @click="selectPage(index)"
            >
              <span class="tab-num">{{ page.page_num }}</span>
              <a-tag v-if="page.image_status !== 'pending'" :color="getStatusColor(page.image_status)" size="small" style="margin-left: 4px;">
                {{ page.image_status === 'done' ? '✓' : page.image_status === 'generating' ? '...' : '!' }}
              </a-tag>
            </div>
          </div>

          <div class="markdown-editor">
            <div class="editor-header">
              <span>{{ selectedPage?.title || '选择页面' }}</span>
            </div>
            <a-textarea
              v-if="selectedPage"
              v-model:value="selectedPage.markdown"
              :auto-size="{ minRows: 12, maxRows: 20 }"
              placeholder="页面 Markdown 内容..."
            />
          </div>
        </div>

        <!-- 右侧：图片预览 -->
        <div class="right-panel">
          <div class="image-preview-header">
            <span>{{ t('pages.imagePreview') }}（{{ t('pages.arrowKeyHint') }}）</span>
            <a-space v-if="selectedPage?.image_path">
              <a-button size="small" @click="openRefineModal">{{ t('pages.refine') }}</a-button>
              <a-button size="small" @click="openImageFolder(selectedPage.image_path)">{{ t('pages.openFolder') }}</a-button>
            </a-space>
          </div>
          
          <div class="image-preview" :class="{ clickable: selectedPage?.image_path }">
            <div v-if="!selectedPage?.image_path" class="no-image">
              <a-spin v-if="selectedPage?.image_status === 'generating'" />
              <span v-else>{{ t('pages.noImages') }}</span>
            </div>
            <img v-else :src="getImageUrl(selectedPage.image_path)" :alt="selectedPage.title" @click="openImagePreview(selectedPage.image_path)" />
          </div>

          <div class="image-info">
            <span class="info-item">{{ selectedPage?.page_num }} - {{ selectedPage?.title }}</span>
            <a-tag v-if="selectedPage" :color="getStatusColor(selectedPage.image_status)" size="small">
              {{ selectedPage.image_status === 'done' ? '已生成' : selectedPage.image_status === 'generating' ? '生成中' : '待生成' }}
            </a-tag>
          </div>
        </div>
      </div>
    </a-spin>
  </div>


    <!-- 图片预览弹窗 -->
    <div v-if="previewModalVisible" class="image-modal-overlay" @click="closeImagePreview">
      <div class="image-modal-content" @click.stop>
        <div class="modal-nav-hint">
          <span>← → 切换页面 | ESC 关闭</span>
        </div>
        <img :src="getImageUrl(previewImageUrl)" alt="Preview" />
        <div class="modal-page-info">
          {{ previewPageIndex + 1 }} / {{ pages.length }}
        </div>
      </div>
    </div>

    <!-- 图片微调弹窗 -->
    <a-modal
      v-model:open="showRefineModal"
      :title="t('pages.refine')"
      :ok-text="t('pages.refine')"
      :cancel-text="t('common.cancel')"
      :confirm-loading="refining"
      :closable="!refining"
      :maskClosable="false"
      :keyboard="!refining"
      @ok="refineImage"
      @cancel="!refining && (showRefineModal = false)"
      width="500px"
    >
      <a-form layout="vertical">
        <a-form-item :label="t('pages.refine') + t('common.edit')">
          <a-textarea
            v-model:value="refinePrompt"
            :rows="4"
            :disabled="refining"
            :placeholder="t('pages.enterRefinePrompt')"
          />
        </a-form-item>
      </a-form>
    </a-modal>
  </template>

<style scoped>
.pages-page {
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

.toolbar {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 16px;
  padding: 10px 16px;
  background: var(--bg-white);
  border-radius: var(--radius-md);
  border: 1px solid var(--border-color);
}

.toolbar-label {
  font-size: 13px;
  color: var(--text-secondary);
}

.progress-bar {
  margin-bottom: 16px;
  padding: 10px 16px;
  background: var(--bg-white);
  border-radius: var(--radius-md);
  border: 1px solid var(--border-color);
}

.progress-text {
  margin-top: 8px;
  font-size: 13px;
  color: var(--text-secondary);
  text-align: center;
}

.empty-state {
  display: flex;
  justify-content: center;
  align-items: center;
  min-height: 500px;
  background: var(--bg-white);
  border-radius: var(--radius-md);
}

.editor-layout {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 16px;
  height: calc(100vh - 300px);
  min-height: 500px;
}

.left-panel {
  display: flex;
  flex-direction: column;
  gap: 8px;
  height: 100%;
}

.right-panel {
  background: var(--bg-white);
  border-radius: var(--radius-md);
  border: 1px solid var(--border-color);
  display: flex;
  flex-direction: column;
  height: 100%;
}

.page-tabs {
  display: flex;
  flex-wrap: wrap;
  gap: 2px;
  padding: 6px;
  background: var(--bg-white);
  border-radius: var(--radius-md);
  border: 1px solid var(--border-color);
}

.page-tab {
  display: flex;
  align-items: center;
  justify-content: center;
  min-width: 28px;
  height: 28px;
  border-radius: var(--radius-sm);
  cursor: pointer;
  transition: all 0.2s;
  font-size: 12px;
}

.page-tab:hover {
  background-color: var(--bg-color);
}

.page-tab.active {
  background-color: var(--primary-color);
  color: white;
}

.tab-num {
  font-weight: 500;
}

.markdown-editor {
  flex: 1;
  background: var(--bg-white);
  border-radius: var(--radius-md);
  border: 1px solid var(--border-color);
  padding: 12px;
  display: flex;
  flex-direction: column;
  min-height: 0;
  overflow: hidden;
}

.markdown-editor .ant-input {
  flex: 1;
  min-height: 200px;
}

.editor-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 8px;
  font-weight: 500;
  font-size: 13px;
}

.image-preview-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 10px 12px;
  border-bottom: 1px solid var(--border-color);
  font-weight: 500;
  font-size: 13px;
}

.image-preview {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  background: var(--bg-color);
  min-height: 200px;
}

.no-image {
  color: var(--text-disabled);
  font-size: 12px;
}

.image-preview img {
  max-width: 100%;
  max-height: 100%;
  object-fit: contain;
}

.image-preview.clickable {
  cursor: pointer;
}

.image-preview.clickable:hover {
  opacity: 0.9;
}

.image-info {
  padding: 8px 12px;
  border-top: 1px solid var(--border-color);
  display: flex;
  justify-content: space-between;
  align-items: center;
  font-size: 12px;
}

.info-item {
  color: var(--text-secondary);
}

.image-modal-overlay {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background: rgba(0, 0, 0, 0.9);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
  cursor: pointer;
}

.image-modal-content {
  max-width: 95vw;
  max-height: 95vh;
  display: flex;
  align-items: center;
  justify-content: center;
}

.image-modal-content img {
  max-width: 100%;
  max-height: 90vh;
  object-fit: contain;
}

.modal-nav-hint {
  position: absolute;
  top: 10px;
  left: 50%;
  transform: translateX(-50%);
  background: rgba(0, 0, 0, 0.6);
  color: white;
  padding: 6px 12px;
  border-radius: 4px;
  font-size: 12px;
}

.modal-page-info {
  position: absolute;
  bottom: 10px;
  left: 50%;
  transform: translateX(-50%);
  background: rgba(0, 0, 0, 0.6);
  color: white;
  padding: 6px 12px;
  border-radius: 4px;
  font-size: 14px;
}
</style>
