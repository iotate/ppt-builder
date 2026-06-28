<script setup lang="ts">
import { openUrl } from '@tauri-apps/plugin-opener'
import { useI18n } from 'vue-i18n'

const { t } = useI18n()

async function openGithub() {
  try {
    await openUrl('https://github.com/iotate/ppt-builder')
  } catch (e) {
    // 如果在浏览器环境或 Tauri API 不可用，使用 window.open
    window.open('https://github.com/iotate/ppt-builder', '_blank')
  }
}
</script>

<template>
  <div class="about-page">
    <div class="about-header">
      <div class="header-row">
        <h1 class="about-title">{{ t('about.title') }} PPT Builder</h1>
        <a class="github-link" @click="openGithub">
          <svg viewBox="0 0 24 24" width="18" height="18" fill="currentColor">
            <path d="M12 0c-6.626 0-12 5.373-12 12 0 5.302 3.438 9.8 8.207 11.387.599.111.793-.261.793-.577v-2.234c-3.338.726-4.033-1.416-4.033-1.416-.546-1.387-1.333-1.756-1.333-1.756-1.089-.745.083-.729.083-.729 1.205.084 1.839 1.237 1.839 1.237 1.07 1.834 2.807 1.304 3.492.997.107-.775.418-1.305.762-1.604-2.665-.305-5.467-1.334-5.467-5.931 0-1.311.469-2.381 1.236-3.221-.124-.303-.535-1.524.117-3.176 0 0 1.008-.322 3.301 1.23.957-.266 1.983-.399 3.003-.404 1.02.005 2.047.138 3.006.404 2.291-1.552 3.297-1.23 3.297-1.23.653 1.653.242 2.874.118 3.176.77.84 1.235 1.911 1.235 3.221 0 4.609-2.807 5.624-5.479 5.921.43.372.823 1.102.823 2.222v3.293c0 .319.192.694.801.576 4.765-1.589 8.199-6.086 8.199-11.386 0-6.627-5.373-12-12-12z"/>
          </svg>
          <span>{{ t('about.github') }}: iotate/ppt-builder</span>
        </a>
      </div>
      <p class="about-desc">{{ t('about.description') }}</p>
    </div>

    <div class="about-sections">
      <!-- 核心价值 -->
      <section class="about-section">
        <h2 class="section-title">{{ t('about.coreValue') }}</h2>
        <p class="section-desc">{{ t('about.coreValueDesc') }}</p>
        <ul class="feature-list">
          <li><strong>{{ t('about.logicStructure') }}</strong>：{{ t('about.logicStructureDesc') }}</li>
          <li><strong>{{ t('about.brandCompliance') }}</strong>：{{ t('about.brandComplianceDesc') }}</li>
          <li><strong>{{ t('about.dataPrivacy') }}</strong>：{{ t('about.dataPrivacyDesc') }}</li>
        </ul>
      </section>

      <!-- 目标用户 -->
      <section class="about-section">
        <h2 class="section-title">{{ t('about.targetUsers') }}</h2>
        <ul class="feature-list">
          <li>{{ t('about.targetUsersList[0]') }}</li>
          <li>{{ t('about.targetUsersList[1]') }}</li>
          <li>{{ t('about.targetUsersList[2]') }}</li>
          <li>{{ t('about.targetUsersList[3]') }}</li>
        </ul>
      </section>

      <!-- 差异化优势 -->
      <section class="about-section">
        <h2 class="section-title">{{ t('about.differentiation') }}</h2>
        <ul class="feature-list">
          <li>{{ t('about.diffList[0]') }}</li>
          <li>{{ t('about.diffList[1]') }}</li>
          <li>{{ t('about.diffList[2]') }}</li>
          <li>{{ t('about.diffList[3]') }}</li>
        </ul>
      </section>

      <!-- 功能特性 -->
      <section class="about-section">
        <h2 class="section-title">{{ t('about.features') }}</h2>
        <ul class="feature-list">
          <li><strong>{{ t('about.brainstorming') }}</strong>：{{ t('about.brainstormingDesc') }}</li>
          <li><strong>{{ t('about.outlineGeneration') }}</strong>：{{ t('about.outlineGenerationDesc') }}</li>
          <li><strong>{{ t('about.pptDesign') }}</strong>：{{ t('about.pptDesignDesc') }}</li>
          <li><strong>{{ t('about.exportShare') }}</strong>：{{ t('about.exportShareDesc') }}</li>
          <li><strong>{{ t('about.customConfig') }}</strong>：{{ t('about.customConfigDesc') }}</li>
        </ul>
      </section>
    </div>
  </div>
</template>

<style scoped>
.about-page {
  max-width: 1920px;
  margin: 0 auto;
  padding: 24px 16px;
}

.about-header {
  margin-bottom: 32px;
}

.header-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 12px;
}

.about-title {
  font-size: 24px;
  font-weight: 600;
  margin: 0;
  color: var(--text-primary);
}

.github-link {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 8px 16px;
  background: var(--bg-white);
  border: 1px solid var(--border-color);
  border-radius: var(--radius-sm);
  cursor: pointer;
  transition: all 0.2s;
  color: var(--text-primary);
  font-size: 13px;
}

.github-link:hover {
  border-color: var(--primary-color);
  color: var(--primary-color);
}

.about-desc {
  font-size: 15px;
  color: var(--text-secondary);
  line-height: 1.6;
  margin: 0;
}

.about-sections {
  display: flex;
  flex-direction: column;
  gap: 24px;
  margin-bottom: 32px;
}

.about-section {
  background: var(--bg-white);
  border-radius: var(--radius-md);
  border: 1px solid var(--border-color);
  padding: 20px;
}

.section-title {
  font-size: 16px;
  font-weight: 600;
  margin: 0 0 12px 0;
  color: var(--text-primary);
  padding-bottom: 8px;
  border-bottom: 1px solid var(--border-color);
}

.feature-list {
  margin: 0;
  padding-left: 20px;
  color: var(--text-secondary);
  font-size: 14px;
  line-height: 1.8;
}

.feature-list li {
  margin-bottom: 4px;
}

.feature-list strong {
  color: var(--text-primary);
}

.section-desc {
  font-size: 14px;
  color: var(--text-secondary);
  margin: 0 0 12px 0;
  line-height: 1.6;
}
</style>
