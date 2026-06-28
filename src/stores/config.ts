import { defineStore } from 'pinia'
import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'

export interface ApiConfig {
  provider: string
  endpoint: string
  api_key: string
  model: string
}

export interface ImageSize {
  name: string
  width: number
  height: number
}

export interface AppConfig {
  llm: ApiConfig
  img: ApiConfig & {
    default_width: number
    default_height: number
    format: string
  }
  image_sizes: ImageSize[]
}

export const useConfigStore = defineStore('config', () => {
  const config = ref<AppConfig | null>(null)
  const loading = ref(false)
  const error = ref<string | null>(null)

  async function loadConfig() {
    loading.value = true
    error.value = null
    try {
      config.value = await invoke<AppConfig>('load_config')
    } catch (e) {
      error.value = String(e)
    } finally {
      loading.value = false
    }
  }

  async function saveConfig(newConfig: AppConfig) {
    loading.value = true
    error.value = null
    try {
      await invoke('save_config', { config: newConfig })
      config.value = newConfig
    } catch (e) {
      error.value = String(e)
      throw e
    } finally {
      loading.value = false
    }
  }

  async function testLlmConnection(apiConfig: ApiConfig) {
    try {
      return await invoke<boolean>('test_llm_connection', { config: apiConfig })
    } catch (e) {
      error.value = String(e)
      return false
    }
  }

  async function testImgConnection(apiConfig: ApiConfig) {
    try {
      return await invoke<boolean>('test_img_connection', { config: apiConfig })
    } catch (e) {
      error.value = String(e)
      return false
    }
  }

  return {
    config,
    loading,
    error,
    loadConfig,
    saveConfig,
    testLlmConnection,
    testImgConnection
  }
})
