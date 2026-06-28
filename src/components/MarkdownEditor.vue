<script setup lang="ts">
import { ref, watch, computed } from 'vue'

const props = defineProps<{
  modelValue: string
  placeholder?: string
}>()

const emit = defineEmits<{
  (e: 'update:modelValue', value: string): void
}>()

const content = ref(props.modelValue)
const showPreview = ref(true)

watch(() => props.modelValue, (newVal) => {
  content.value = newVal
})

function updateContent() {
  emit('update:modelValue', content.value)
}

// Simple markdown to HTML conversion for preview
const previewHtml = computed(() => {
  let html = content.value
  
  // Headers
  html = html.replace(/^### (.*$)/gim, '<h3>$1</h3>')
  html = html.replace(/^## (.*$)/gim, '<h2>$1</h2>')
  html = html.replace(/^# (.*$)/gim, '<h1>$1</h1>')
  
  // Bold
  html = html.replace(/\*\*(.*?)\*\*/g, '<strong>$1</strong>')
  
  // Italic
  html = html.replace(/\*(.*?)\*/g, '<em>$1</em>')
  
  // Line breaks
  html = html.replace(/\n/g, '<br>')
  
  // Horizontal rules
  html = html.replace(/---/g, '<hr>')
  
  return html
})

function togglePreview() {
  showPreview.value = !showPreview.value
}
</script>

<template>
  <div class="markdown-editor">
    <div class="editor-toolbar">
      <button @click="togglePreview" class="btn-toggle">
        {{ showPreview ? '隐藏预览' : '显示预览' }}
      </button>
    </div>
    
    <div class="editor-container" :class="{ 'with-preview': showPreview }">
      <textarea
        v-model="content"
        class="editor-textarea"
        :placeholder="placeholder"
        @input="updateContent"
      ></textarea>
      
      <div v-if="showPreview" class="preview-pane">
        <div v-if="content" class="preview-content" v-html="previewHtml"></div>
        <div v-else class="preview-placeholder">
          预览区域
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.markdown-editor {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.editor-toolbar {
  display: flex;
  justify-content: flex-end;
}

.btn-toggle {
  padding: 6px 12px;
  font-size: 12px;
  background-color: #f0f0f0;
  border: 1px solid #ddd;
  border-radius: 4px;
  cursor: pointer;
}

.editor-container {
  display: flex;
  gap: 20px;
  min-height: 400px;
}

.editor-container.with-preview .editor-textarea {
  flex: 1;
}

.editor-container:not(.with-preview) .editor-textarea {
  width: 100%;
}

.editor-textarea {
  padding: 15px;
  border: 1px solid #ddd;
  border-radius: 8px;
  font-family: 'Consolas', 'Monaco', monospace;
  font-size: 14px;
  line-height: 1.6;
  resize: vertical;
}

.editor-textarea:focus {
  outline: none;
  border-color: #4a9eff;
}

.preview-pane {
  flex: 1;
  padding: 15px;
  background-color: #fafafa;
  border: 1px solid #e0e0e0;
  border-radius: 8px;
  overflow-y: auto;
}

.preview-content {
  font-size: 14px;
  line-height: 1.6;
}

.preview-content :deep(h1) {
  font-size: 24px;
  margin-bottom: 15px;
}

.preview-content :deep(h2) {
  font-size: 20px;
  margin-bottom: 12px;
  padding-bottom: 8px;
  border-bottom: 1px solid #e0e0e0;
}

.preview-content :deep(h3) {
  font-size: 16px;
  margin-bottom: 10px;
}

.preview-content :deep(strong) {
  color: #333;
}

.preview-content :deep(hr) {
  border: none;
  border-top: 1px dashed #ccc;
  margin: 15px 0;
}

.preview-placeholder {
  color: #999;
  display: flex;
  align-items: center;
  justify-content: center;
  height: 100%;
}
</style>
