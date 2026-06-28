<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { convertFileSrc } from '@tauri-apps/api/core'
import { useI18n } from 'vue-i18n'

const { t } = useI18n()

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

const templates = ref<TemplateInfo[]>([])
const loading = ref(false)

onMounted(async () => {
  await loadTemplates()
})

async function loadTemplates() {
  loading.value = true
  try {
    templates.value = await invoke<TemplateInfo[]>('list_templates')
  } catch (e) {
    console.error(t('template.loadFailed'), e)
    templates.value = []
  } finally {
    loading.value = false
  }
}

async function deleteTemplate(template: TemplateInfo) {
  if (template.name === 'default') {
    alert(t('template.defaultCannotDelete'))
    return
  }
  
  if (!confirm(t('template.deleteConfirm'))) return
  
  try {
    // TODO: 实现删除模板命令
    alert(t('template.deleteNotImplemented'))
  } catch (e) {
    alert(t('template.deleteFailed') + '：' + e)
  }
}

function getPreviewUrl(template: TemplateInfo, type: 'front' | 'content' | 'back'): string | undefined {
  let path: string | undefined
  if (type === 'front') path = template.front_cover_path
  else if (type === 'content') path = template.content_path
  else path = template.back_cover_path
  
  if (!path) return undefined
  
  // Use Tauri's convertFileSrc to create proper asset URL
  // Normalize path separators for Tauri asset protocol
  // Windows paths use backslashes, but asset protocol expects forward slashes
  const normalizedPath = path.replace(/\\/g, '/');
  return convertFileSrc(normalizedPath, 'asset')
}
</script>

<template>
  <div class="template-management">
    <div class="page-header">
      <h1 class="page-title">{{ t('template.title') }}</h1>
      <a-button @click="loadTemplates" :loading="loading">
        <template #icon>
          <svg viewBox="0 0 24 24" width="16" height="16" fill="currentColor">
            <path d="M17.65 6.35C16.2 4.9 14.21 4 12 4c-4.42 0-7.99 3.58-7.99 8s3.57 8 7.99 8c3.73 0 6.84-2.55 7.73-6h-2.08c-.82 2.33-3.04 4-5.65 4-3.31 0-6-2.69-6-6s2.69-6 6-6c1.66 0 3.14.69 4.22 1.78L13 11h7V4l-2.35 2.35z"/>
          </svg>
        </template>
        {{ t('common.refresh') }}
      </a-button>
    </div>

    <p class="page-desc">
      {{ t('template.pageDesc') }}
    </p>

    <a-spin :spinning="loading">
      <div v-if="!loading && templates.length === 0" class="empty-state">
        <a-empty :description="t('template.noTemplates')" />
      </div>

      <div v-else class="templates-grid">
        <a-card
          v-for="template in templates"
          :key="template.name"
          class="template-card"
        >
          <template #title>
            <span class="template-name">{{ template.name }}</span>
          </template>
          <template #extra>
            <a-popconfirm
              v-if="template.name !== 'default'"
              :title="t('template.deleteConfirm')"
              :ok-text="t('common.delete')"
              :cancel-text="t('common.cancel')"
              @confirm="deleteTemplate(template)"
            >
              <a-button type="link" size="small" danger>{{ t('common.delete') }}</a-button>
            </a-popconfirm>
          </template>

          <div class="template-preview">
            <div class="preview-item" :class="{ missing: !template.has_front_cover }">
              <div class="preview-thumb">
                <span v-if="!template.has_front_cover">{{ t('template.none') }}</span>
                <img v-else :src="getPreviewUrl(template, 'front')" :alt="t('template.coverImage')" />
              </div>
              <span class="preview-label">{{ t('template.coverImage') }}</span>
              <span class="preview-filename">front-cover.png</span>
            </div>
            <div class="preview-item" :class="{ missing: !template.has_content }">
              <div class="preview-thumb">
                <span v-if="!template.has_content">{{ t('template.none') }}</span>
                <img v-else :src="getPreviewUrl(template, 'content')" :alt="t('template.contentImage')" />
              </div>
              <span class="preview-label">{{ t('template.contentImage') }}</span>
              <span class="preview-filename">content.png</span>
            </div>
            <div class="preview-item" :class="{ missing: !template.has_back_cover }">
              <div class="preview-thumb">
                <span v-if="!template.has_back_cover">{{ t('template.none') }}</span>
                <img v-else :src="getPreviewUrl(template, 'back')" :alt="t('template.backImage')" />
              </div>
              <span class="preview-label">{{ t('template.backImage') }}</span>
              <span class="preview-filename">back-cover.png</span>
            </div>
          </div>
        </a-card>
      </div>
    </a-spin>

    <!-- 说明区域 -->
    <a-card class="help-card">
      <template #title>
        <span class="help-title">{{ t('template.howToAdd') }}</span>
      </template>
      <div class="help-content">
        <p>{{ t('template.helpText1') }}</p>
        <p>{{ t('template.helpText2') }}</p>
        <ul>
          <li><code>front-cover.png</code> - {{ t('template.coverImage') }}</li>
          <li><code>content.png</code> - {{ t('template.contentImage') }}</li>
          <li><code>back-cover.png</code> - {{ t('template.backImage') }}</li>
        </ul>
        <a-alert
          type="info"
          show-icon
          :message="t('template.tip')"
          :description="t('template.tipDesc')"
        />
      </div>
    </a-card>
  </div>
</template>

<style scoped>
.template-management {
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

.templates-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(320px, 1fr));
  gap: 16px;
  margin-bottom: 24px;
}

.template-card {
  cursor: default;
}

.template-name {
  font-weight: 500;
}

.template-preview {
  display: flex;
  gap: 12px;
  justify-content: center;
}

.preview-item {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 4px;
}

.preview-thumb {
  width: 80px;
  height: 45px;
  background-color: var(--bg-color);
  border: 1px solid var(--border-color);
  border-radius: var(--radius-sm);
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 11px;
  color: var(--text-disabled);
  overflow: hidden;
}

.preview-thumb img {
  width: 100%;
  height: 100%;
  object-fit: cover;
}

.preview-item.missing .preview-thumb {
  border-style: dashed;
  background-color: transparent;
}

.preview-label {
  font-size: 12px;
  font-weight: 500;
  color: var(--text-primary);
}

.preview-filename {
  font-size: 10px;
  color: var(--text-disabled);
  font-family: monospace;
}

.help-card {
  background: var(--bg-white);
}

.help-title {
  font-size: 14px;
  font-weight: 600;
}

.help-content {
  color: var(--text-secondary);
  font-size: 13px;
  line-height: 1.8;
}

.help-content p {
  margin-bottom: 8px;
}

.help-content ul {
  margin: 0 0 12px 0;
  padding-left: 20px;
}

.help-content li {
  margin-bottom: 4px;
}

.help-content code {
  background: var(--bg-color);
  padding: 2px 6px;
  border-radius: 4px;
  font-size: 12px;
  color: var(--primary-color);
}
</style>
