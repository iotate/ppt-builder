import { notification } from 'ant-design-vue'

export interface NotificationOptions {
  type: 'success' | 'error' | 'info' | 'warning'
  title: string
  message?: string
  duration?: number
}

export function showNotification(options: NotificationOptions) {
  const { type, title, message, duration = 3 } = options
  
  notification[type]({
    message: title,
    description: message,
    duration
  })
}

export function showSuccess(title: string, message?: string) {
  showNotification({ type: 'success', title, message })
}

export function showError(title: string, message?: string) {
  showNotification({ type: 'error', title, message, duration: 5 })
}

export function showInfo(title: string, message?: string) {
  showNotification({ type: 'info', title, message })
}

export function showWarning(title: string, message?: string) {
  showNotification({ type: 'warning', title, message })
}

// 处理 Tauri invoke 错误
export async function safeInvoke<T>(command: string, args?: Record<string, unknown>): Promise<T | null> {
  try {
    const { invoke } = await import('@tauri-apps/api/core')
    return await invoke<T>(command, args)
  } catch (error) {
    const errorMessage = error instanceof Error ? error.message : String(error)
    showError('操作失败', errorMessage)
    return null
  }
}

// 确认对话框
export async function confirm(message: string, title: string = '确认'): Promise<boolean> {
  return new Promise((resolve) => {
    const result = window.confirm(`${title}\n\n${message}`)
    resolve(result)
  })
}
