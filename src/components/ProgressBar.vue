<script setup lang="ts">
defineProps<{
  currentStep: number
  totalSteps: number
}>()
</script>

<template>
  <div class="progress-bar">
    <div class="progress-track">
      <div 
        class="progress-fill" 
        :style="{ width: `${(currentStep / totalSteps) * 100}%` }"
      ></div>
    </div>
    <div class="progress-labels">
      <span>步骤 {{ currentStep }} / {{ totalSteps }}</span>
      <span>{{ Math.round((currentStep / totalSteps) * 100) }}%</span>
    </div>
  </div>
</template>

<style scoped>
.progress-bar {
  margin-bottom: 20px;
}

.progress-track {
  height: 8px;
  background-color: #e0e0e0;
  border-radius: 4px;
  overflow: hidden;
}

.progress-fill {
  height: 100%;
  background: linear-gradient(90deg, #4a9eff, #2ecc71);
  transition: width 0.3s ease;
}

.progress-labels {
  display: flex;
  justify-content: space-between;
  margin-top: 8px;
  font-size: 13px;
  color: #666;
}
</style>
