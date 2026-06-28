<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'

interface StyleInfo {
  name: string
  path: string
}

const props = defineProps<{
  modelValue?: string
}>()

const emit = defineEmits<{
  (e: 'update:modelValue', value: string | null): void
}>()

const styles = ref<StyleInfo[]>([])
const loading = ref(false)
const selected = ref<string | null>(props.modelValue || null)

onMounted(async () => {
  await loadStyles()
})

async function loadStyles() {
  loading.value = true
  try {
    styles.value = await invoke<StyleInfo[]>('list_styles')
  } catch (e) {
    console.error('Failed to load styles:', e)
    styles.value = []
  } finally {
    loading.value = false
  }
}

function selectStyle(name: string | null) {
  selected.value = name
  emit('update:modelValue', name)
}
</script>

<template>
  <div class="style-picker">
    <div class="picker-header">
      <label>选择风格</label>
      <router-link to="/styles" class="btn-manage">管理风格</router-link>
    </div>

    <div v-if="loading" class="loading">加载中...</div>

    <div v-else class="style-list">
      <!-- No style option -->
      <div 
        class="style-option"
        :class="{ selected: selected === null }"
        @click="selectStyle(null)"
      >
        <span class="style-icon">🎨</span>
        <span class="style-name">自由发挥</span>
      </div>

      <!-- Style options -->
      <div
        v-for="style in styles"
        :key="style.name"
        class="style-option"
        :class="{ selected: selected === style.name }"
        @click="selectStyle(style.name)"
      >
        <span class="style-icon">🎨</span>
        <span class="style-name">{{ style.name }}</span>
      </div>
    </div>
  </div>
</template>

<style scoped>
.style-picker {
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

.btn-manage {
  font-size: 12px;
  color: #4a9eff;
  text-decoration: none;
}

.btn-manage:hover {
  text-decoration: underline;
}

.loading {
  text-align: center;
  padding: 20px;
  color: #666;
}

.style-list {
  display: flex;
  flex-wrap: wrap;
  gap: 10px;
}

.style-option {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 16px;
  border: 1px solid #e0e0e0;
  border-radius: 20px;
  cursor: pointer;
  transition: all 0.2s;
}

.style-option:hover {
  border-color: #4a9eff;
}

.style-option.selected {
  border-color: #4a9eff;
  background-color: #f0f7ff;
}

.style-icon {
  font-size: 16px;
}

.style-name {
  font-size: 13px;
}
</style>
