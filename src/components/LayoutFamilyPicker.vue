<script setup lang="ts">
interface Props {
  modelValue: string
  pageType?: 'front-cover' | 'back-cover' | 'content'
}

withDefaults(defineProps<Props>(), {
  pageType: 'content'
})

const emit = defineEmits<{
  (e: 'update:modelValue', value: string): void
}>()

const layoutFamilies = [
  { value: 'grid_n_x_m', label: '宫格卡片', desc: '适合展示多个并列信息' },
  { value: 'timeline_horizontal', label: '横向时间线', desc: '适合展示时间进程' },
  { value: 'timeline_vertical', label: '纵向时间线', desc: '适合展示时间进程' },
  { value: 'hub_and_spoke', label: '中心辐射', desc: '适合展示中心主题与分支' },
  { value: 'split_left_right', label: '左右分栏', desc: '适合对比或并列内容' },
  { value: 'split_top_bottom', label: '上下分区', desc: '适合层次结构' },
  { value: 'compare_dual_axis', label: '双轴对比', desc: '适合对比分析' },
  { value: 'process_horizontal', label: '横向流程', desc: '适合步骤流程' },
  { value: 'process_vertical', label: '纵向流程', desc: '适合步骤流程' },
  { value: 'hero_with_supporting_cards', label: '主视觉卡片', desc: '适合突出重点' },
]

function handleChange(value: string) {
  emit('update:modelValue', value)
}
</script>

<template>
  <a-select
    :value="modelValue"
    @update:value="handleChange"
    style="width: 140px"
    size="small"
  >
    <a-select-option value="">
      自动选择
    </a-select-option>
    <a-select-opt-group v-for="layout in layoutFamilies" :key="layout.value">
      <a-select-option :value="layout.value">
        {{ layout.label }}
      </a-select-option>
    </a-select-opt-group>
  </a-select>
</template>

<style scoped>
</style>
