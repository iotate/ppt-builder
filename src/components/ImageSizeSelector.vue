<script setup lang="ts">
import { ref, watch } from 'vue'
import type { ImageSize } from '@/stores/config'

const props = defineProps<{
  sizes: ImageSize[]
  modelValue?: ImageSize
}>()

const emit = defineEmits<{
  (e: 'update:modelValue', value: ImageSize): void
}>()

const selected = ref<ImageSize>(props.modelValue || props.sizes[0] || {
  name: '16:9 横屏',
  width: 1920,
  height: 1080
})

watch(() => props.modelValue, (newVal) => {
  if (newVal) {
    selected.value = newVal
  }
})

function selectSize(size: ImageSize) {
  selected.value = size
  emit('update:modelValue', size)
}

function isSizeMatch(size: ImageSize): boolean {
  return selected.value.width === size.width && selected.value.height === size.height
}
</script>

<template>
  <div class="image-size-selector">
    <label>图片尺寸</label>
    <div class="size-options">
      <div
        v-for="size in sizes"
        :key="size.name"
        class="size-option"
        :class="{ selected: isSizeMatch(size) }"
        @click="selectSize(size)"
      >
        <div class="size-preview" :style="{
          aspectRatio: `${size.width} / ${size.height}`
        }"></div>
        <div class="size-info">
          <span class="size-name">{{ size.name }}</span>
          <span class="size-dims">{{ size.width }}×{{ size.height }}</span>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.image-size-selector {
  padding: 10px 0;
}

.image-size-selector label {
  display: block;
  font-size: 14px;
  font-weight: 500;
  margin-bottom: 10px;
}

.size-options {
  display: flex;
  flex-wrap: wrap;
  gap: 15px;
}

.size-option {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 10px 15px;
  border: 1px solid #e0e0e0;
  border-radius: 8px;
  cursor: pointer;
  transition: all 0.2s;
}

.size-option:hover {
  border-color: #4a9eff;
}

.size-option.selected {
  border-color: #4a9eff;
  background-color: #f0f7ff;
}

.size-preview {
  width: 40px;
  max-height: 30px;
  background-color: #e0e0e0;
  border-radius: 2px;
}

.size-info {
  display: flex;
  flex-direction: column;
}

.size-name {
  font-size: 13px;
  font-weight: 500;
}

.size-dims {
  font-size: 11px;
  color: #999;
}
</style>
