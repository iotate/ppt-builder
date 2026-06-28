import { createI18n } from 'vue-i18n'
import zhCN from './zh-CN'
import enUS from './en-US'

// 从 localStorage 获取保存的语言设置，默认中文
const savedLocale = localStorage.getItem('locale') || 'zh-CN'

const i18n = createI18n({
  legacy: false, // 使用 Composition API 模式
  locale: savedLocale,
  fallbackLocale: 'zh-CN',
  messages: {
    'zh-CN': zhCN,
    'en-US': enUS
  }
})

export default i18n

// 语言切换函数
export function setLocale(locale: string) {
  if (i18n.mode === 'legacy') {
    (i18n.global as any).locale = locale
  } else {
    (i18n.global.locale as any).value = locale
  }
  localStorage.setItem('locale', locale)
  document.documentElement.setAttribute('lang', locale)
}

// 获取当前语言
export function getLocale(): string {
  return i18n.mode === 'legacy' 
    ? (i18n.global as any).locale 
    : (i18n.global.locale as any).value
}

// 语言列表
export const languages = [
  { code: 'zh-CN', name: '中文', icon: '🇨🇳' },
  { code: 'en-US', name: 'English', icon: '🇺🇸' }
]
