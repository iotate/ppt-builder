<script setup lang="ts">
import { ref } from 'vue'
import { useI18n } from 'vue-i18n'

interface Props {
  pageNum: number
  title: string
  imageUrl?: string
  status: 'pending' | 'generating' | 'done' | 'failed'
}

defineProps<Props>()

defineEmits<{
  (e: 'refine'): void
  (e: 'regenerate'): void
}>()

const { t } = useI18n()
const showActions = ref(false)

function getStatusLabel(status: string): string {
  const labels: Record<string, string> = {
    pending: t('imageCard.pending'),
    generating: t('imageCard.generating'),
    done: t('imageCard.done'),
    failed: t('imageCard.failed')
  }
  return labels[status] || status
}

function getStatusClass(status: string): string {
  return status
}
</script>

<template>
  <div 
    class="image-card"
    @mouseenter="showActions = true"
    @mouseleave="showActions = false"
  >
    <div class="image-preview">
      <span class="page-number">{{ t('imageCard.pageNum', { num: pageNum }) }}</span>
      
      <div v-if="imageUrl" class="preview-image">
        <img :src="imageUrl" :alt="title" />
      </div>
      
      <div v-else class="placeholder" :class="getStatusClass(status)">
        <span v-if="status === 'generating'" class="spinner"></span>
        <span v-else>{{ getStatusLabel(status) }}</span>
      </div>
      
      <!-- Overlay actions -->
      <div v-if="showActions && status === 'done'" class="overlay-actions">
        <button @click="$emit('refine')" class="btn-action">{{ t('imageCard.refine') }}</button>
        <button @click="$emit('regenerate')" class="btn-action">{{ t('imageCard.regenerate') }}</button>
      </div>
    </div>
    
    <div class="image-info">
      <h4>{{ title }}</h4>
      <span class="status-badge" :class="getStatusClass(status)">
        {{ getStatusLabel(status) }}
      </span>
    </div>
  </div>
</template>

<style scoped>
.image-card {
  border-radius: 8px;
  overflow: hidden;
  background-color: #fff;
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.1);
}

.image-preview {
  position: relative;
  background-color: #f5f5f5;
  aspect-ratio: 16/9;
}

.page-number {
  position: absolute;
  top: 8px;
  left: 8px;
  background-color: rgba(0, 0, 0, 0.6);
  color: #fff;
  padding: 2px 8px;
  border-radius: 4px;
  font-size: 12px;
  z-index: 1;
}

.preview-image {
  width: 100%;
  height: 100%;
}

.preview-image img {
  width: 100%;
  height: 100%;
  object-fit: cover;
}

.placeholder {
  display: flex;
  align-items: center;
  justify-content: center;
  height: 100%;
  color: #999;
  font-size: 14px;
}

.placeholder.generating {
  color: #4a9eff;
}

.placeholder.failed {
  color: #ff4a4a;
}

.spinner {
  width: 24px;
  height: 24px;
  border: 3px solid #e0e0e0;
  border-top-color: #4a9eff;
  border-radius: 50%;
  animation: spin 1s linear infinite;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}

.overlay-actions {
  position: absolute;
  inset: 0;
  background-color: rgba(0, 0, 0, 0.5);
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 10px;
}

.btn-action {
  padding: 8px 16px;
  background-color: #fff;
  border: none;
  border-radius: 4px;
  cursor: pointer;
  font-size: 13px;
}

.btn-action:hover {
  background-color: #f0f0f0;
}

.image-info {
  padding: 12px;
}

.image-info h4 {
  font-size: 14px;
  margin-bottom: 8px;
}

.status-badge {
  display: inline-block;
  padding: 2px 8px;
  border-radius: 4px;
  font-size: 12px;
  background-color: #e0e0e0;
}

.status-badge.done {
  background-color: #c8e6c9;
  color: #2e7d32;
}

.status-badge.generating {
  background-color: #fff3e0;
  color: #ef6c00;
}

.status-badge.failed {
  background-color: #ffcdd2;
  color: #c62828;
}
</style>
