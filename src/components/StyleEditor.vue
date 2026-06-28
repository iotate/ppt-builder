<script setup lang="ts">
import { ref, watch } from 'vue'

const props = defineProps<{
  modelValue: {
    name: string
    content: string
  }
}>()

const emit = defineEmits<{
  (e: 'update:modelValue', value: { name: string; content: string }): void
  (e: 'save'): void
  (e: 'cancel'): void
}>()

const localStyle = ref({ ...props.modelValue })

watch(() => props.modelValue, (newVal) => {
  localStyle.value = { ...newVal }
}, { deep: true })

function updateContent() {
  emit('update:modelValue', { ...localStyle.value })
}

function handleSave() {
  emit('save')
}

function handleCancel() {
  emit('cancel')
}
</script>

<template>
  <div class="style-editor">
    <div class="form-group">
      <label>风格名称</label>
      <input
        v-model="localStyle.name"
        type="text"
        placeholder="例如：商务风格"
        class="name-input"
      />
    </div>
    
    <div class="form-group">
      <label>风格描述（Markdown）</label>
      <div class="editor-wrapper">
        <textarea
          v-model="localStyle.content"
          class="content-textarea"
          placeholder="描述这个风格的特点，例如配色、字体、图形风格等..."
          @input="updateContent"
        ></textarea>
        
        <div class="preview-pane">
          <div class="preview-label">预览</div>
          <div class="preview-content" v-if="localStyle.content">
            {{ localStyle.content }}
          </div>
          <div class="preview-placeholder" v-else>
            输入内容后在此预览
          </div>
        </div>
      </div>
    </div>
    
    <div class="tips">
      <p>💡 风格描述会作为提示词的一部分传递给 AI，用于指导图片生成风格。</p>
    </div>
    
    <div class="editor-actions">
      <button @click="handleCancel" class="btn-cancel">取消</button>
      <button @click="handleSave" class="btn-save">保存风格</button>
    </div>
  </div>
</template>

<style scoped>
.style-editor {
  display: flex;
  flex-direction: column;
  gap: 20px;
}

.form-group {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.form-group label {
  font-size: 14px;
  font-weight: 500;
}

.name-input {
  padding: 10px;
  border: 1px solid #ddd;
  border-radius: 4px;
  font-size: 14px;
  max-width: 300px;
}

.name-input:focus {
  outline: none;
  border-color: #4a9eff;
}

.editor-wrapper {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 15px;
  min-height: 300px;
}

.content-textarea {
  padding: 15px;
  border: 1px solid #ddd;
  border-radius: 8px;
  font-family: 'Consolas', 'Monaco', monospace;
  font-size: 14px;
  line-height: 1.6;
  resize: vertical;
}

.content-textarea:focus {
  outline: none;
  border-color: #4a9eff;
}

.preview-pane {
  border: 1px solid #e0e0e0;
  border-radius: 8px;
  overflow: hidden;
  display: flex;
  flex-direction: column;
}

.preview-label {
  padding: 8px 12px;
  background-color: #f5f5f5;
  font-size: 12px;
  color: #666;
  border-bottom: 1px solid #e0e0e0;
}

.preview-content {
  flex: 1;
  padding: 15px;
  overflow-y: auto;
  font-size: 14px;
  line-height: 1.6;
  white-space: pre-wrap;
}

.preview-placeholder {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  color: #999;
}

.tips {
  padding: 12px;
  background-color: #f0f7ff;
  border-radius: 4px;
}

.tips p {
  margin: 0;
  font-size: 13px;
  color: #4a9eff;
}

.editor-actions {
  display: flex;
  justify-content: flex-end;
  gap: 10px;
}

.btn-cancel {
  padding: 10px 20px;
  background-color: #f0f0f0;
  border: 1px solid #ddd;
  border-radius: 4px;
  cursor: pointer;
}

.btn-save {
  padding: 10px 20px;
  background-color: #4a9eff;
  color: #fff;
  border: none;
  border-radius: 4px;
  cursor: pointer;
}

.btn-save:hover {
  background-color: #3a8eef;
}
</style>
