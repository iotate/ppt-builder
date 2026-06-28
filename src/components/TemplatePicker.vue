<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'

interface TemplateInfo {
  name: string
  path: string
  has_front_cover: boolean
  has_content: boolean
  has_back_cover: boolean
}

const props = defineProps<{
  modelValue?: string
}>()

const emit = defineEmits<{
  (e: 'update:modelValue', value: string | null): void
}>()

const templates = ref<TemplateInfo[]>([])
const loading = ref(false)
const selected = ref<string | null>(props.modelValue || null)

onMounted(async () => {
  await loadTemplates()
})

async function loadTemplates() {
  loading.value = true
  try {
    templates.value = await invoke<TemplateInfo[]>('list_templates')
  } catch (e) {
    console.error('Failed to load templates:', e)
    templates.value = []
  } finally {
    loading.value = false
  }
}

function selectTemplate(name: string | null) {
  selected.value = name
  emit('update:modelValue', name)
}
</script>

<template>
  <div class="template-picker">
    <div class="picker-header">
      <label>选择模板</label>
      <button @click="loadTemplates" :disabled="loading" class="btn-refresh">
        刷新
      </button>
    </div>

    <div v-if="loading" class="loading">加载中...</div>

    <div v-else class="template-grid">
      <!-- No template option -->
      <div 
        class="template-card"
        :class="{ selected: selected === null }"
        @click="selectTemplate(null)"
      >
        <div class="template-preview no-template">
          <span>🎨</span>
        </div>
        <div class="template-name">自由发挥</div>
        <div class="template-desc">AI 自由发挥</div>
      </div>

      <!-- Template options -->
      <div
        v-for="template in templates"
        :key="template.name"
        class="template-card"
        :class="{ selected: selected === template.name }"
        @click="selectTemplate(template.name)"
      >
        <div class="template-preview">
          <div class="preview-icons">
            <span :class="{ active: template.has_front_cover }">封面</span>
            <span :class="{ active: template.has_content }">内容</span>
            <span :class="{ active: template.has_back_cover }">封底</span>
          </div>
        </div>
        <div class="template-name">{{ template.name }}</div>
        <div class="template-desc">
          {{ template.has_front_cover && template.has_content && template.has_back_cover 
            ? '完整模板' 
            : '部分模板' }}
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.template-picker {
  padding: 10px 0;
}

.picker-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 15px;
}

.picker-header label {
  font-size: 14px;
  font-weight: 500;
}

.btn-refresh {
  padding: 4px 12px;
  font-size: 12px;
  background-color: #f0f0f0;
  border: 1px solid #ddd;
  border-radius: 4px;
  cursor: pointer;
}

.loading {
  text-align: center;
  padding: 20px;
  color: #666;
}

.template-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(120px, 1fr));
  gap: 15px;
}

.template-card {
  border: 2px solid #e0e0e0;
  border-radius: 8px;
  padding: 10px;
  cursor: pointer;
  transition: all 0.2s;
}

.template-card:hover {
  border-color: #4a9eff;
}

.template-card.selected {
  border-color: #4a9eff;
  background-color: #f0f7ff;
}

.template-preview {
  aspect-ratio: 16/9;
  background-color: #f5f5f5;
  border-radius: 4px;
  margin-bottom: 8px;
  display: flex;
  align-items: center;
  justify-content: center;
}

.template-preview.no-template {
  font-size: 24px;
}

.preview-icons {
  display: flex;
  flex-direction: column;
  gap: 2px;
  font-size: 10px;
}

.preview-icons span {
  padding: 2px 4px;
  background-color: #e0e0e0;
  border-radius: 2px;
  color: #999;
}

.preview-icons span.active {
  background-color: #4a9eff;
  color: #fff;
}

.template-name {
  font-size: 13px;
  font-weight: 500;
  margin-bottom: 2px;
}

.template-desc {
  font-size: 11px;
  color: #999;
}
</style>
