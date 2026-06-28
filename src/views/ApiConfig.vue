<script setup lang="ts">
import { onMounted, ref, reactive, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { PlusOutlined, DeleteOutlined } from '@ant-design/icons-vue'
import { useI18n } from 'vue-i18n'

const { t } = useI18n()

interface ExtraHeader {
  key: string
  value: string
}

interface AppConfig {
  llm: {
    provider: string
    endpoint: string
    api_key: string
    model: string
    extra_headers: ExtraHeader[]
  }
  img: {
    provider: string
    endpoint: string
    api_key: string
    model: string
    extra_headers: ExtraHeader[]
  }
  image_sizes: Array<{ name: string; width: number; height: number }>
}

// LLM 提供商预设
const llmProviders = {
  openai: { endpoint: 'https://api.openai.com/v1', model: 'gpt-5' },
  dashscope: { endpoint: 'https://dashscope.aliyuncs.com/compatible-mode/v1', model: 'qwen-plus-3.6' },
  deepseek: { endpoint: 'https://api.deepseek.com/v1', model: 'deepseek-v4-pro' },
  agnes: { endpoint: 'https://apihub.agnes-ai.com/v1', model: 'agnes-2.0-flash' },
  custom: { endpoint: '', model: '' }
}

// 图像生成提供商预设
const imgProviders = {
  openai: { endpoint: 'https://api.openai.com/v1/images/generations', model: 'gpt-image-2' },
  google: { endpoint: 'https://generativelanguage.googleapis.com/v1beta/images', model: 'gemini-3.1-flash-image-preview' },
  dashscope: { endpoint: 'https://dashscope.aliyuncs.com/api/v1/services/aigc/text2image/image-synthesis', model: 'wanx-v1' },
  agnes: { endpoint: 'https://apihub.agnes-ai.com/v1/images/generations', model: 'agnes-image-2.0-flash' },
  custom: { endpoint: '', model: '' }
}

const llmConfig = reactive({
  provider: 'openai',
  endpoint: 'https://api.openai.com/v1',
  api_key: '',
  model: 'gpt-5',
  extra_headers: [] as ExtraHeader[]
})

const imgConfig = reactive({
  provider: 'openai',
  endpoint: 'https://api.openai.com/v1/images/generations',
  api_key: '',
  model: 'gpt-image-2',
  extra_headers: [] as ExtraHeader[]
})

const testingLlm = ref(false)
const testingImg = ref(false)
const saving = ref(false)
const llmTestResult = ref<{ success: boolean; message: string } | null>(null)
const imgTestResult = ref<{ success: boolean; message: string } | null>(null)

// 判断是否为自定义提供商
const isCustomLlmProvider = ref(false)
const isCustomImgProvider = ref(false)

// 已知的图像提供商列表
const knownImgProviders = ['openai', 'azure', 'dashscope', 'replicate', 'agnes']

onMounted(async () => {
  await loadConfig()
})

// 监听 LLM 提供商变化，自动填充预设值
watch(() => llmConfig.provider, (newProvider) => {
  const preset = llmProviders[newProvider as keyof typeof llmProviders]
  if (preset && newProvider !== 'custom') {
    llmConfig.endpoint = preset.endpoint
    llmConfig.model = preset.model
    isCustomLlmProvider.value = false
  } else {
    isCustomLlmProvider.value = true
  }
})

// 监听图像提供商变化，自动填充预设值
watch(() => imgConfig.provider, (newProvider) => {
  const preset = imgProviders[newProvider as keyof typeof imgProviders]
  if (preset && newProvider !== 'custom') {
    imgConfig.endpoint = preset.endpoint
    imgConfig.model = preset.model
    isCustomImgProvider.value = false
  } else {
    isCustomImgProvider.value = true
  }
})

async function loadConfig() {
  try {
    const config = await invoke<AppConfig>('load_config')
    if (config) {
      Object.assign(llmConfig, config.llm)
      Object.assign(imgConfig, config.img)
      
      // 确保 extra_headers 存在
      if (!llmConfig.extra_headers) {
        llmConfig.extra_headers = []
      }
      if (!imgConfig.extra_headers) {
        imgConfig.extra_headers = []
      }
      
      // 检查是否为自定义提供商
      const knownLlmProvidersList = ['openai', 'azure', 'dashscope', 'deepseek', 'agnes']
      isCustomLlmProvider.value = !knownLlmProvidersList.includes(config.llm.provider)
      
      isCustomImgProvider.value = !knownImgProviders.includes(config.img.provider)
    }
  } catch (e) {
    console.error('加载配置失败', e)
  }
}

async function testLlm() {
  if (!llmConfig.api_key) {
    llmTestResult.value = { success: false, message: t('config.enterApiKey') }
    return
  }
  
  if (!llmConfig.endpoint) {
    llmTestResult.value = { success: false, message: t('config.enterEndpoint') }
    return
  }
  
  testingLlm.value = true
  llmTestResult.value = null
  
  try {
    const result = await invoke<boolean>('test_llm_connection', { config: llmConfig })
    llmTestResult.value = { 
      success: result, 
      message: result ? t('config.llmTestSuccess') : t('config.llmTestFailed')
    }
  } catch (e) {
    llmTestResult.value = { success: false, message: `${t('config.testFailed')}: ${e}` }
  } finally {
    testingLlm.value = false
  }
}

async function testImg() {
  if (!imgConfig.api_key) {
    imgTestResult.value = { success: false, message: t('config.enterApiKey') }
    return
  }
  
  if (!imgConfig.endpoint) {
    imgTestResult.value = { success: false, message: t('config.enterEndpoint') }
    return
  }
  
  testingImg.value = true
  imgTestResult.value = null
  
  try {
    const result = await invoke<boolean>('test_img_connection', { config: imgConfig })
    imgTestResult.value = { 
      success: result, 
      message: result ? t('config.imgTestSuccess') : t('config.imgTestFailed')
    }
  } catch (e) {
    imgTestResult.value = { success: false, message: `${t('config.testFailed')}: ${e}` }
  } finally {
    testingImg.value = false
  }
}

async function saveConfig() {
  saving.value = true
  
  try {
    await invoke('save_config', { 
      config: {
        llm: { ...llmConfig },
        img: { ...imgConfig },
        image_sizes: [
          { name: '16:9 Landscape', width: 1920, height: 1072 },
          { name: '9:16 Portrait', width: 1072, height: 1920 },
          { name: '4:3 Landscape', width: 1440, height: 1072 },
          { name: '3:4 Portrait', width: 1072, height: 1440 },
          { name: '1:1 Square', width: 1072, height: 1072 }
        ]
      }
    })
  } catch (e) {
    alert(`${t('config.saveFailed')}: ${e}`)
  } finally {
    saving.value = false
  }
}

function addExtraHeader() {
  llmConfig.extra_headers.push({ key: '', value: '' })
}

function removeExtraHeader(index: number) {
  llmConfig.extra_headers.splice(index, 1)
}

function addImgExtraHeader() {
  imgConfig.extra_headers.push({ key: '', value: '' })
}

function removeImgExtraHeader(index: number) {
  imgConfig.extra_headers.splice(index, 1)
}

function goBack() {
  window.history.back()
}
</script>

<template>
  <div class="config-page">
    <div class="page-header">
      <h1 class="page-title">{{ t('config.title') }}</h1>
    </div>
    <p class="page-desc">{{ t('config.description') }}</p>

    <a-row :gutter="24">
      <!-- LLM Config -->
      <a-col :span="12">
        <a-card :title="t('config.llmConfig')" class="config-card">
          <a-form layout="vertical">
            <a-form-item :label="t('config.provider')">
              <a-select v-model:value="llmConfig.provider">
                <a-select-option value="openai">OpenAI</a-select-option>
                <a-select-option value="azure">Azure OpenAI</a-select-option>
                <a-select-option value="dashscope">DashScope (Alibaba Cloud)</a-select-option>
                <a-select-option value="deepseek">DeepSeek</a-select-option>
                <a-select-option value="agnes">Agnes AI</a-select-option>
                <a-select-option value="custom">{{ t('config.customProvider') }}</a-select-option>
              </a-select>
            </a-form-item>

            <a-form-item :label="t('config.endpoint')" :required="isCustomLlmProvider">
              <a-input
                v-model:value="llmConfig.endpoint"
                :placeholder="isCustomLlmProvider ? 'https://api.example.com/v1' : t('config.autoFillEditable')"
                :disabled="!isCustomLlmProvider && llmConfig.provider !== 'custom'"
              />
              <template #extra v-if="isCustomLlmProvider">
                <span class="field-hint">{{ t('config.customEndpointHint') }}</span>
              </template>
            </a-form-item>

            <a-form-item :label="t('config.apiKey')" required>
              <a-input-password
                v-model:value="llmConfig.api_key"
                placeholder="sk-..."
              />
            </a-form-item>

            <a-form-item :label="t('config.model')" :required="isCustomLlmProvider">
              <a-input
                v-model:value="llmConfig.model"
                :placeholder="isCustomLlmProvider ? 'gpt-4o / qwen-max / ...' : t('config.autoFillEditable')"
              />
            </a-form-item>

            <a-form-item :label="t('config.extraHeaders')">
              <div class="extra-headers">
                <div v-for="(header, index) in llmConfig.extra_headers" :key="index" class="header-row">
                  <a-input
                    v-model:value="header.key"
                    :placeholder="t('config.key')"
                    style="width: 200px"
                  />
                  <a-input
                    v-model:value="header.value"
                    :placeholder="t('config.value')"
                    style="flex: 1"
                  />
                  <a-button type="text" danger @click="removeExtraHeader(index)">
                    <template #icon><DeleteOutlined /></template>
                  </a-button>
                </div>
                <a-button type="dashed" @click="addExtraHeader">
                  <template #icon><PlusOutlined /></template>
                  {{ t('config.addHeader') }}
                </a-button>
              </div>
              <template #extra>
                <span class="field-hint">{{ t('config.customHeadersHint') }}</span>
              </template>
            </a-form-item>

            <a-form-item>
              <a-space>
                <a-button @click="testLlm" :loading="testingLlm">{{ t('config.connectionTest') }}</a-button>
                <a-tag v-if="llmTestResult" :color="llmTestResult.success ? 'success' : 'error'">
                  {{ llmTestResult.success ? t('config.llmTestSuccess') : t('config.llmTestFailed') }}
                </a-tag>
              </a-space>
            </a-form-item>
          </a-form>
        </a-card>
      </a-col>

      <!-- Image Config -->
      <a-col :span="12">
        <a-card :title="t('config.imgConfig')" class="config-card">
          <a-form layout="vertical">
            <a-form-item :label="t('config.provider')">
              <a-select v-model:value="imgConfig.provider">
                <a-select-option value="openai">OpenAI</a-select-option>
                <a-select-option value="azure">Azure</a-select-option>
                <a-select-option value="dashscope">DashScope (Alibaba Cloud)</a-select-option>
                <a-select-option value="replicate">Replicate</a-select-option>
                <a-select-option value="agnes">Agnes AI</a-select-option>
                <a-select-option value="custom">{{ t('config.customProvider') }}</a-select-option>
              </a-select>
            </a-form-item>

            <a-form-item :label="t('config.endpoint')" :required="isCustomImgProvider">
              <a-input
                v-model:value="imgConfig.endpoint"
                :placeholder="isCustomImgProvider ? 'https://api.example.com/v1/images/generations' : t('config.autoFillEditable')"
              />
              <template #extra v-if="isCustomImgProvider">
                <span class="field-hint">{{ t('config.customImgEndpointHint') }}</span>
              </template>
            </a-form-item>

            <a-form-item :label="t('config.apiKey')" required>
              <a-input-password
                v-model:value="imgConfig.api_key"
                placeholder="sk-..."
              />
            </a-form-item>

            <a-form-item :label="t('config.model')" :required="isCustomImgProvider">
              <a-input
                v-model:value="imgConfig.model"
                :placeholder="isCustomImgProvider ? 'gpt-image-2 / sdxl / ...' : t('config.autoFillEditable')"
              />
            </a-form-item>

            <a-form-item :label="t('config.extraHeaders')">
              <div class="extra-headers">
                <div v-for="(header, index) in imgConfig.extra_headers" :key="index" class="header-row">
                  <a-input
                    v-model:value="header.key"
                    :placeholder="t('config.key')"
                    style="width: 200px"
                  />
                  <a-input
                    v-model:value="header.value"
                    :placeholder="t('config.value')"
                    style="flex: 1"
                  />
                  <a-button type="text" danger @click="removeImgExtraHeader(index)">
                    <template #icon><DeleteOutlined /></template>
                  </a-button>
                </div>
                <a-button type="dashed" @click="addImgExtraHeader">
                  <template #icon><PlusOutlined /></template>
                  {{ t('config.addHeader') }}
                </a-button>
              </div>
              <template #extra>
                <span class="field-hint">{{ t('config.customHeadersHintOptional') }}</span>
              </template>
            </a-form-item>

            <a-form-item>
              <a-space>
                <a-button @click="testImg" :loading="testingImg">{{ t('config.connectionTest') }}</a-button>
                <a-tag v-if="imgTestResult" :color="imgTestResult.success ? 'success' : 'error'">
                  {{ imgTestResult.success ? t('config.imgTestSuccess') : t('config.imgTestFailed') }}
                </a-tag>
              </a-space>
            </a-form-item>
          </a-form>
        </a-card>
      </a-col>
    </a-row>

    <div class="page-actions">
      <a-space>
        <a-button @click="goBack">← {{ t('common.back') }}</a-button>
        <a-button type="primary" :loading="saving" @click="saveConfig">{{ t('common.save') }}</a-button>
      </a-space>
    </div>
  </div>
</template>

<style scoped>
.config-page {
  max-width: 1920px;
  margin: 0 auto;
  padding: 0 16px;
}

.page-header {
  margin-bottom: 8px;
}

.page-title {
  font-size: 20px;
  font-weight: 600;
  margin: 0;
  color: var(--text-primary);
}

.page-desc {
  color: var(--text-secondary);
  margin-bottom: 24px;
}

.extra-headers {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.header-row {
  display: flex;
  gap: 8px;
  align-items: center;
}

.field-hint {
  font-size: 12px;
  color: var(--text-secondary);
}

.page-actions {
  margin-top: 24px;
  display: flex;
  justify-content: flex-end;
}

.config-card {
  height: 100%;
}

.field-hint {
  font-size: 12px;
  color: var(--text-secondary);
}

.extra-headers {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.header-row {
  display: flex;
  gap: 8px;
  align-items: center;
}

.page-actions {
  margin-top: 24px;
  display: flex;
  justify-content: flex-end;
}
</style>
