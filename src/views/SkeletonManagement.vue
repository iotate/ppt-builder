<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { useI18n } from 'vue-i18n'

const { t } = useI18n()

interface SkeletonInfo {
  name: string
  path: string
  content?: string
  description?: string
}

const skeletons = ref<SkeletonInfo[]>([])
const loading = ref(false)

// 编辑弹窗
const showEditor = ref(false)
const editingName = ref('')
const editingContent = ref('')
const isNewSkeleton = ref(false)
const saving = ref(false)

onMounted(async () => {
  await loadSkeletons()
})

async function loadSkeletons() {
  loading.value = true
  try {
    const files = await invoke<SkeletonInfo[]>('list_skeletons')
    // 加载每个结构的内容和提取描述
    const skeletonsWithDesc = await Promise.all(
      files.map(async (skeleton) => {
        try {
          const content = await invoke<string>('get_skeleton_content', { name: skeleton.name })
          const description = extractDescription(content)
          return { ...skeleton, content, description }
        } catch {
          return { ...skeleton, description: '' }
        }
      })
    )
    skeletons.value = skeletonsWithDesc
  } catch (e) {
    console.error('加载结构失败', e)
    skeletons.value = []
  } finally {
    loading.value = false
  }
}

// 从结构内容中提取描述（第一段非标题文本）
function extractDescription(content: string): string {
  const lines = content.split('\n')
  for (const line of lines) {
    const trimmed = line.trim()
    // 跳过标题和空行
    if (trimmed && !trimmed.startsWith('#')) {
      return trimmed.length > 100 ? trimmed.substring(0, 100) + '...' : trimmed
    }
  }
  return t('skeleton.noDescription')
}

async function createNewSkeleton() {
  isNewSkeleton.value = true
  editingName.value = ''
  editingContent.value = t('skeleton.defaultTemplate')
  showEditor.value = true
}

async function editSkeleton(skeleton: SkeletonInfo) {
  isNewSkeleton.value = false
  editingName.value = skeleton.name
  
  try {
    const content = await invoke<string>('get_skeleton_content', { name: skeleton.name })
    editingContent.value = content
    showEditor.value = true
  } catch (e) {
    alert(t('skeleton.loadFailed') + '：' + e)
  }
}

async function saveSkeleton() {
  if (!editingName.value.trim()) {
    alert(t('skeleton.enterName'))
    return
  }
  
  saving.value = true
  try {
    await invoke('save_skeleton', { 
      name: editingName.value.trim(),
      content: editingContent.value 
    })
    
    showEditor.value = false
    await loadSkeletons()
  } catch (e) {
    alert(t('skeleton.saveFailed') + '：' + e)
  } finally {
    saving.value = false
  }
}

async function deleteSkeleton(skeleton: SkeletonInfo) {
  if (!confirm(t('skeleton.deleteConfirm'))) return
  
  try {
    await invoke('delete_skeleton', { name: skeleton.name })
    await loadSkeletons()
  } catch (e) {
    alert(t('skeleton.deleteFailed') + '：' + e)
  }
}

function cancelEdit() {
  showEditor.value = false
}
</script>

<template>
  <div class="skeleton-management">
    <div class="page-header">
      <h1 class="page-title">{{ t('skeleton.title') }}</h1>
      <a-space>
        <a-button @click="loadSkeletons" :loading="loading">
          <template #icon>
            <svg viewBox="0 0 24 24" width="16" height="16" fill="currentColor">
              <path d="M17.65 6.35C16.2 4.9 14.21 4 12 4c-4.42 0-7.99 3.58-7.99 8s3.57 8 7.99 8c3.73 0 6.84-2.55 7.73-6h-2.08c-.82 2.33-3.04 4-5.65 4-3.31 0-6-2.69-6-6s2.69-6 6-6c1.66 0 3.14.69 4.22 1.78L13 11h7V4l-2.35 2.35z"/>
            </svg>
          </template>
          {{ t('common.reset') }}
        </a-button>
        <a-button type="primary" @click="createNewSkeleton">
          <template #icon><span>+</span></template>
          {{ t('skeleton.create') }}
        </a-button>
      </a-space>
    </div>

    <p class="page-desc">
      {{ t('skeleton.pageDesc') }}
    </p>

    <a-spin :spinning="loading">
      <div v-if="!loading && skeletons.length === 0" class="empty-state">
        <a-empty :description="t('skeleton.noSkeletons')">
          <a-button type="primary" @click="createNewSkeleton">{{ t('skeleton.create') }}</a-button>
        </a-empty>
      </div>

      <div v-else class="skeletons-grid">
        <a-card
          v-for="skeleton in skeletons"
          :key="skeleton.name"
          class="skeleton-card"
        >
          <template #title>
            <div class="skeleton-header">
              <span class="skeleton-name">{{ skeleton.name }}</span>
              <span class="skeleton-filename">{{ skeleton.name }}.md</span>
            </div>
          </template>
          <template #extra>
            <a-space>
              <a-button type="link" size="small" @click="editSkeleton(skeleton)">{{ t('common.edit') }}</a-button>
              <a-popconfirm
                :title="t('skeleton.deleteConfirm')"
                :ok-text="t('common.delete')"
                :cancel-text="t('common.cancel')"
                @confirm="deleteSkeleton(skeleton)"
              >
                <a-button type="link" size="small" danger>{{ t('common.delete') }}</a-button>
              </a-popconfirm>
            </a-space>
          </template>
          
          <!-- 描述 -->
          <div class="skeleton-desc">
            {{ skeleton.description }}
          </div>
        </a-card>
      </div>
    </a-spin>

    <!-- 编辑弹窗 -->
    <a-modal
      v-model:open="showEditor"
      :title="isNewSkeleton ? t('skeleton.create') : t('skeleton.edit')"
      :ok-text="t('common.save')"
      :cancel-text="t('common.cancel')"
      :confirm-loading="saving"
      @ok="saveSkeleton"
      @cancel="cancelEdit"
      width="700px"
    >
      <a-form layout="vertical">
        <a-form-item :label="t('skeleton.name')" required>
          <a-input
            v-model:value="editingName"
            :disabled="!isNewSkeleton"
          />
        </a-form-item>
        <a-form-item :label="t('skeleton.content')">
          <a-textarea
            v-model:value="editingContent"
            :rows="14"
          />
        </a-form-item>
      </a-form>
    </a-modal>
  </div>
</template>

<style scoped>
.skeleton-management {
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

.skeletons-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(300px, 1fr));
  gap: 16px;
}

.skeleton-card {
  cursor: default;
}

.skeleton-header {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.skeleton-name {
  font-weight: 500;
  font-size: 14px;
}

.skeleton-filename {
  font-size: 11px;
  color: var(--text-disabled);
  font-family: monospace;
}

.skeleton-desc {
  color: var(--text-secondary);
  font-size: 13px;
  line-height: 1.6;
  padding: 8px 0;
}

.field-hint {
  font-size: 12px;
  color: var(--text-secondary);
}
</style>
