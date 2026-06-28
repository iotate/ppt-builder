<script setup lang="ts">
import { ref, watch } from 'vue'

type OutlineMode = 'simple' | 'medium' | 'detailed'

const props = defineProps<{
  modelValue?: OutlineMode
}>()

const emit = defineEmits<{
  (e: 'update:modelValue', value: OutlineMode): void
}>()

const selected = ref<OutlineMode>(props.modelValue || 'medium')

watch(() => props.modelValue, (newVal) => {
  if (newVal) {
    selected.value = newVal
  }
})

const modes: { value: OutlineMode; label: string; pages: string; icon: string }[] = [
  { value: 'simple', label: '简单', pages: '3-5 页', icon: '📄' },
  { value: 'medium', label: '中等', pages: '6-10 页', icon: '📑' },
  { value: 'detailed', label: '详细', pages: '10-15 页', icon: '📚' },
]

function selectMode(mode: OutlineMode) {
  selected.value = mode
  emit('update:modelValue', mode)
}
</script>

<template>
  <div class="outline-mode-selector">
    <label>生成模式</label>
    <div class="mode-options">
      <div
        v-for="mode in modes"
        :key="mode.value"
        class="mode-option"
        :class="{ selected: selected === mode.value }"
        @click="selectMode(mode.value)"
      >
        <span class="mode-icon">{{ mode.icon }}</span>
        <div class="mode-info">
          <span class="mode-label">{{ mode.label }}</span>
          <span class="mode-pages">{{ mode.pages }}</span>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.outline-mode-selector {
  padding: 10px 0;
}

.outline-mode-selector label {
  display: block;
  font-size: 14px;
  font-weight: 500;
  margin-bottom: 10px;
}

.mode-options {
  display: flex;
  gap: 15px;
}

.mode-option {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 12px 20px;
  border: 1px solid #e0e0e0;
  border-radius: 8px;
  cursor: pointer;
  transition: all 0.2s;
}

.mode-option:hover {
  border-color: #4a9eff;
}

.mode-option.selected {
  border-color: #4a9eff;
  background-color: #f0f7ff;
}

.mode-icon {
  font-size: 24px;
}

.mode-info {
  display: flex;
  flex-direction: column;
}

.mode-label {
  font-size: 14px;
  font-weight: 500;
}

.mode-pages {
  font-size: 12px;
  color: #999;
}
</style>
