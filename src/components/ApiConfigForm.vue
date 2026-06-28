<script setup lang="ts">
import { ref, watch } from 'vue'
import type { ApiConfig } from '@/stores/config'

const props = defineProps<{
  modelValue: ApiConfig
  type: 'llm' | 'img'
}>()

const emit = defineEmits<{
  (e: 'update:modelValue', value: ApiConfig): void
  (e: 'test'): void
}>()

const localConfig = ref({ ...props.modelValue })
const testing = ref(false)

watch(() => props.modelValue, (newVal) => {
  localConfig.value = { ...newVal }
}, { deep: true })

function updateConfig() {
  emit('update:modelValue', { ...localConfig.value })
}

async function handleTest() {
  testing.value = true
  emit('test')
  setTimeout(() => {
    testing.value = false
  }, 2000)
}

const providers = props.type === 'llm' 
  ? [
      { value: 'openai', label: 'OpenAI' },
      { value: 'azure', label: 'Azure OpenAI' },
      { value: 'dashscope', label: 'DashScope (阿里云)' },
    ]
  : [
      { value: 'openai', label: 'OpenAI' },
      { value: 'azure', label: 'Azure' },
      { value: 'dashscope', label: 'DashScope (阿里云)' },
      { value: 'replicate', label: 'Replicate' },
    ]
</script>

<template>
  <div class="api-config-form">
    <div class="form-group">
      <label>提供商</label>
      <select v-model="localConfig.provider" @change="updateConfig">
        <option v-for="p in providers" :key="p.value" :value="p.value">
          {{ p.label }}
        </option>
      </select>
    </div>

    <div class="form-group">
      <label>API 端点</label>
      <input 
        v-model="localConfig.endpoint" 
        type="text" 
        placeholder="https://api.openai.com/v1"
        @change="updateConfig"
      />
    </div>

    <div class="form-group">
      <label>API Key</label>
      <input 
        v-model="localConfig.api_key" 
        type="password" 
        placeholder="sk-..."
        @change="updateConfig"
      />
    </div>

    <div class="form-group">
      <label>模型</label>
      <input 
        v-model="localConfig.model" 
        type="text" 
        :placeholder="type === 'llm' ? 'gpt-5' : 'gpt-image-2'"
        @change="updateConfig"
      />
    </div>

    <button @click="handleTest" :disabled="testing" class="btn-test">
      {{ testing ? '测试中...' : '测试连接' }}
    </button>
  </div>
</template>

<style scoped>
.api-config-form {
  display: flex;
  flex-direction: column;
  gap: 15px;
}

.form-group {
  display: flex;
  flex-direction: column;
  gap: 5px;
}

.form-group label {
  font-size: 14px;
  color: #555;
}

.form-group input,
.form-group select {
  padding: 8px 12px;
  border: 1px solid #ddd;
  border-radius: 4px;
  font-size: 14px;
}

.form-group input:focus,
.form-group select:focus {
  outline: none;
  border-color: #4a9eff;
}

.btn-test {
  align-self: flex-start;
  padding: 8px 16px;
  background-color: #f0f0f0;
  border: 1px solid #ddd;
  border-radius: 4px;
  cursor: pointer;
  font-size: 14px;
}

.btn-test:hover:not(:disabled) {
  background-color: #e0e0e0;
}

.btn-test:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}
</style>
