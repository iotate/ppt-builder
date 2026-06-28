<script setup lang="ts">
interface Props {
  modelValue: 'loose' | 'balanced' | 'strict'
  hasReferenceImages?: boolean
}

withDefaults(defineProps<Props>(), {
  hasReferenceImages: false
})

const emit = defineEmits<{
  (e: 'update:modelValue', value: 'loose' | 'balanced' | 'strict'): void
}>()

const levels = [
  { 
    value: 'loose', 
    label: '宽松', 
    desc: '优先保持系列感，允许重新组织',
    icon: '🎨'
  },
  { 
    value: 'balanced', 
    label: '适度', 
    desc: '框架统一、细节鲜活',
    icon: '⚖️'
  },
  { 
    value: 'strict', 
    label: '严格', 
    desc: '锁定骨架与色彩节奏',
    icon: '🔒'
  },
]

function handleChange(value: string) {
  emit('update:modelValue', value as 'loose' | 'balanced' | 'strict')
}
</script>

<template>
  <a-radio-group
    :value="modelValue"
    @update:value="handleChange"
    size="small"
    button-style="solid"
  >
    <a-radio-button 
      v-for="level in levels" 
      :key="level.value" 
      :value="level.value"
      :title="hasReferenceImages ? level.desc : '需要上传参考图才能生效'"
    >
      <span class="level-label">{{ level.icon }} {{ level.label }}</span>
    </a-radio-button>
  </a-radio-group>
</template>

<style scoped>
.level-label {
  font-size: 12px;
}
</style>
