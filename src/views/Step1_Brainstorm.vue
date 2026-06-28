<script setup lang="ts">
import { ref, onMounted, nextTick, computed } from 'vue'
import { useRouter, useRoute } from 'vue-router'
import { invoke } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-dialog'
import { useI18n } from 'vue-i18n'

interface Message {
  id: number
  role: 'user' | 'assistant'
  content: string
  attachments?: string[]
  timestamp: string
}

interface Attachment {
  name: string
  path: string
  type: string
}

const { t } = useI18n()
const router = useRouter()
const route = useRoute()

const messages = ref<Message[]>([])
const inputMessage = ref('')
const attachments = ref<Attachment[]>([])
const savedAttachments = ref<Attachment[]>([])
const loading = ref(false)
const generating = ref(false)
const messagesContainer = ref<HTMLElement | null>(null)
const hasRequirements = ref(false)

const projectId = computed(() => route.params.id as string)

const systemPromptZh = `你是一位资深的演示设计和信息传达专家。你的任务是通过对话帮助用户厘清他们想要通过PPT表达什么、起到什么作用。

请引导用户思考以下关键问题：

1. **目标与目的**
   - 这次演示的核心目标是什么？
   - 希望听众在演示结束后做什么或想什么？
   - 这是为了说服、教育、汇报还是激励？

2. **受众分析**
   - 听众是谁？他们的背景、知识水平和关注点？
   - 他们已经知道什么？需要知道什么？
   - 他们可能的疑虑或反对意见是什么？

3. **核心信息**
   - 如果只能记住一件事，你希望听众记住什么？
   - 有哪些关键数据、案例或证据支撑你的观点？
   - 有哪些可以省略的内容？

4. **场景与约束**
   - 演示的场合和形式（会议、演讲、远程演示）？
   - 可用的时间是多少？
   - 有没有品牌或视觉要求？

请以友好、专业的方式引导对话，不要一次问完所有问题，而是根据用户的回答逐步深入。如果用户上传了附件，请先阅读附件内容，然后基于附件内容进行对话。`

const systemPromptEn = `You are a senior presentation design and communication expert. Your task is to help users clarify what they want to express through their PPT and what effect they want to achieve through conversation.

Please guide users to think about the following key questions:

1. **Goals and Objectives**
   - What is the core goal of this presentation?
   - What do you want the audience to do or think after the presentation?
   - Is this to persuade, educate, report, or inspire?

2. **Audience Analysis**
   - Who is the audience? What are their background, knowledge level, and concerns?
   - What do they already know? What do they need to know?
   - What are their potential doubts or objections?

3. **Core Message**
   - If the audience can only remember one thing, what do you want them to remember?
   - What key data, cases, or evidence support your point?
   - What content can be omitted?

4. **Context and Constraints**
   - What is the occasion and format of the presentation (meeting, speech, remote)?
   - How much time is available?
   - Are there any brand or visual requirements?

Please guide the conversation in a friendly and professional way. Don't ask all questions at once, but gradually deepen based on the user's responses. If the user uploads attachments, please read the attachment content first, and then have a conversation based on the attachment content.`

const systemPrompt = computed(() => {
  return localStorage.getItem('locale') === 'en-US' ? systemPromptEn : systemPromptZh
})

onMounted(async () => {
  await Promise.all([
    loadConversation(),
    loadAttachments(),
    checkRequirements()
  ])
})

async function loadConversation() {
  try {
    const data = await invoke<Message[]>('load_conversation', {
      projectName: projectId.value
    })
    messages.value = data || []
    await scrollToBottom()
  } catch (e) {
    console.error('加载对话失败', e)
    messages.value = []
  }
}

async function saveConversation() {
  try {
    await invoke('save_conversation', {
      projectName: projectId.value,
      messages: messages.value
    })
  } catch (e) {
    console.error('保存对话失败', e)
  }
}

async function scrollToBottom() {
  await nextTick()
  if (messagesContainer.value) {
    messagesContainer.value.scrollTop = messagesContainer.value.scrollHeight
  }
}

async function selectAttachment() {
  const selected = await open({
    multiple: true,
    filters: [{
      name: 'Text Documents',
      extensions: ['txt', 'md']
    }]
  })
  
  if (selected) {
    const files = Array.isArray(selected) ? selected : [selected]
    for (const filePath of files) {
      const fileName = filePath.split(/[/\\]/).pop() || ''
      const extension = fileName.split('.').pop()?.toLowerCase() || ''
      
      try {
        // 复制文件到项目的attachs目录
        const savedPath = await invoke<string>('save_attachment', {
          projectName: projectId.value,
          filePath: filePath
        })
        
        const newAttachment = {
          name: fileName,
          path: savedPath,
          type: getFileType(extension)
        }
        
        // 添加到当前发送的附件列表
        attachments.value.push(newAttachment)
        // 同时更新左侧附件栏
        savedAttachments.value.push(newAttachment)
      } catch (e) {
        console.error('上传附件失败', e)
      }
    }
  }
}

function getFileType(extension: string): string {
  // 只支持文本文件
  const textTypes = ['txt', 'md']
  return textTypes.includes(extension) ? 'document' : 'other'
}

function removeAttachment(index: number) {
  attachments.value.splice(index, 1)
}

async function removeSavedAttachment(index: number) {
  const attachment = savedAttachments.value[index]
  
  try {
    // 删除项目文件夹中的文件
    await invoke('delete_attachment', {
      projectName: projectId.value,
      attachmentName: attachment.name
    })
    
    // 从列表中移除
    savedAttachments.value.splice(index, 1)
    
    // 如果该附件也在当前发送列表中，也移除
    const currentIndex = attachments.value.findIndex(a => a.path === attachment.path)
    if (currentIndex !== -1) {
      attachments.value.splice(currentIndex, 1)
    }
  } catch (e) {
    console.error('删除附件失败', e)
  }
}

async function loadAttachments() {
  try {
    // 直接扫描项目的 attachs 目录
    const data = await invoke<Attachment[]>('list_project_attachments', {
      projectName: projectId.value
    })
    savedAttachments.value = data || []
  } catch (e) {
    console.error('加载附件列表失败', e)
    savedAttachments.value = []
  }
}

async function checkRequirements() {
  try {
    const content = await invoke<string>('load_requirements', {
      projectName: projectId.value
    })
    hasRequirements.value = !!content && content.trim().length > 0
  } catch (e) {
    console.error('检查需求失败', e)
    hasRequirements.value = false
  }
}

async function sendMessage() {
  if (!inputMessage.value.trim() && attachments.value.length === 0) return
  
  const userMessage: Message = {
    id: Date.now(),
    role: 'user',
    content: inputMessage.value,
    attachments: attachments.value.map(a => a.path),
    timestamp: new Date().toISOString()
  }
  
  messages.value.push(userMessage)
  const currentInput = inputMessage.value
  const currentAttachments = [...attachments.value]
  
  inputMessage.value = ''
  attachments.value = []
  
  await saveConversation()
  await scrollToBottom()
  
  loading.value = true
  
  try {
    const response = await invoke<string>('chat_with_ai', {
      projectName: projectId.value,
      message: currentInput,
      attachments: currentAttachments.map(a => a.path),
      systemPrompt: systemPrompt.value
    })
    
    const assistantMessage: Message = {
      id: Date.now() + 1,
      role: 'assistant',
      content: response,
      timestamp: new Date().toISOString()
    }
    
    messages.value.push(assistantMessage)
    await saveConversation()
    await scrollToBottom()
  } catch (e) {
    console.error('AI对话失败', e)
    alert(t('brainstorm.chatFailed') + '：' + e)
  } finally {
    loading.value = false
  }
}

async function generateRequirements() {
  if (messages.value.length === 0) {
    alert(t('brainstorm.noConversation'))
    return
  }
  
  generating.value = true
  
  try {
    const requirements = await invoke<string>('generate_requirements', {
      projectName: projectId.value
    })
    
    // 保存需求到项目
    await invoke('save_requirements', {
      projectName: projectId.value,
      requirements: requirements
    })
    
    // 标记需求已存在
    hasRequirements.value = true
    
    // 跳转到思路页面
    router.push({ name: 'outline', params: { id: projectId.value } })
  } catch (e) {
    console.error('生成需求失败', e)
    alert(t('brainstorm.generateFailed') + '：' + e)
  } finally {
    generating.value = false
  }
}

function goToOutline() {
  router.push({ name: 'outline', params: { id: projectId.value } })
}

function goBack() {
  router.push({ name: 'projects' })
}

async function openAttachmentFolder() {
  try {
    await invoke('open_project_attachs_folder', {
      projectName: projectId.value
    })
  } catch (e) {
    console.error('打开附件文件夹失败', e)
  }
}
</script>

<template>
  <div class="brainstorm-view">
    <div class="page-header">
      <h1 class="page-title">{{ t('brainstorm.title') }}</h1>
      <a-space>
        <a-button @click="goBack">← {{ t('brainstorm.backToProject') }}</a-button>
        <a-button type="primary" @click="generateRequirements" :loading="generating" :disabled="messages.length === 0">
          {{ t('brainstorm.generateRequirements') }}
        </a-button>
        <a-button v-if="hasRequirements" @click="goToOutline">
          {{ t('brainstorm.goToOutline') }} →
        </a-button>
      </a-space>
    </div>
    
    <p class="page-desc">
      {{ t('brainstorm.subtitle') }}
    </p>
    
    <div class="main-content">
      <!-- 左侧附件栏 -->
      <div class="attachments-sidebar">
        <div class="sidebar-header">
          <span>{{ t('brainstorm.attachments') }}</span>
          <a-button type="text" size="small" class="folder-btn" @click="openAttachmentFolder">
            📁
          </a-button>
        </div>
        <div class="sidebar-content">
          <div v-if="savedAttachments.length === 0" class="empty-sidebar">
            {{ t('brainstorm.noAttachments') }}
          </div>
          <div v-else class="attachment-list">
            <div v-for="(attachment, index) in savedAttachments" :key="index" class="attachment-item">
              <span class="attachment-icon">
                {{ attachment.type === 'image' ? '🖼️' : '📄' }}
              </span>
              <span class="attachment-name" :title="attachment.name">{{ attachment.name }}</span>
              <a-button type="text" size="small" class="delete-btn" @click="removeSavedAttachment(index)">×</a-button>
            </div>
          </div>
        </div>
      </div>
      
      <!-- 右侧聊天区域 -->
      <div class="chat-container">
        <!-- 消息列表 -->
        <div class="messages-container" ref="messagesContainer">
          <div v-if="messages.length === 0" class="empty-chat">
            <p>{{ t('brainstorm.emptyChat') }}</p>
          </div>
          
          <div
            v-for="message in messages"
            :key="message.id"
            class="message"
            :class="message.role"
          >
            <div class="message-header">
              <span class="message-role">{{ message.role === 'user' ? t('brainstorm.user') : t('brainstorm.assistant') }}</span>
              <span class="message-time">{{ new Date(message.timestamp).toLocaleString() }}</span>
            </div>
            <div class="message-content">{{ message.content }}</div>
            <div v-if="message.attachments && message.attachments.length > 0" class="message-attachments">
              <span v-for="(path, index) in message.attachments" :key="index" class="attachment-tag">
                📎 {{ path.split(/[/\\]/).pop() }}
              </span>
            </div>
          </div>
          
          <div v-if="loading" class="message assistant loading">
            <div class="message-content">{{ t('brainstorm.thinking') }}</div>
          </div>
        </div>
        
        <!-- 输入区域 -->
        <div class="input-area">
          <!-- 附件预览 -->
          <div v-if="attachments.length > 0" class="attachments-preview">
            <div v-for="(attachment, index) in attachments" :key="index" class="attachment-item">
              <span class="attachment-icon">
                {{ attachment.type === 'image' ? '🖼️' : '📄' }}
              </span>
              <span class="attachment-name">{{ attachment.name }}</span>
              <a-button type="text" size="small" @click="removeAttachment(index)">×</a-button>
            </div>
          </div>
          
          <div class="input-row">
            <a-button @click="selectAttachment" :disabled="loading">
              <template #icon>📎</template>
            </a-button>
            <a-textarea
              v-model:value="inputMessage"
              :placeholder="t('brainstorm.inputPlaceholder')"
              :auto-size="{ minRows: 1, maxRows: 4 }"
              @pressEnter="(e: KeyboardEvent) => { if (!e.shiftKey) { e.preventDefault(); sendMessage(); } }"
              :disabled="loading"
            />
            <a-button type="primary" @click="sendMessage" :loading="loading" :disabled="!inputMessage.trim() && attachments.length === 0">
              {{ t('brainstorm.send') }}
            </a-button>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.brainstorm-view {
  max-width: 1400px;
  margin: 0 auto;
  padding: 0 16px;
  height: calc(100vh - 100px);
  display: flex;
  flex-direction: column;
}

.page-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 8px;
  flex-shrink: 0;
}

.page-title {
  font-size: 20px;
  font-weight: 600;
  margin: 0;
  color: var(--text-primary);
}

.page-desc {
  color: var(--text-secondary);
  margin-bottom: 16px;
  flex-shrink: 0;
}

.main-content {
  flex: 1;
  display: flex;
  gap: 16px;
  overflow: hidden;
}

.attachments-sidebar {
  width: 20%;
  min-width: 200px;
  max-width: 280px;
  background: var(--bg-white);
  border-radius: var(--radius-md);
  border: 1px solid var(--border-color);
  display: flex;
  flex-direction: column;
  flex-shrink: 0;
}

.sidebar-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 12px 16px;
  border-bottom: 1px solid var(--border-color);
  font-weight: 500;
  color: var(--text-primary);
}

.sidebar-header .folder-btn {
  padding: 0 4px;
  height: auto;
  line-height: 1;
  opacity: 0.6;
  transition: opacity 0.2s;
}

.sidebar-header .folder-btn:hover {
  opacity: 1;
}

.sidebar-content {
  flex: 1;
  overflow-y: auto;
  padding: 8px;
}

.empty-sidebar {
  text-align: center;
  color: var(--text-disabled);
  padding: 20px 10px;
  font-size: 13px;
}

.attachment-list {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.attachment-list .attachment-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 10px;
  background: var(--bg-color);
  border-radius: var(--radius-sm);
  font-size: 12px;
}

.attachment-list .attachment-icon {
  font-size: 16px;
}

.attachment-list .attachment-name {
  flex: 1;
  color: var(--text-primary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.attachment-list .delete-btn {
  opacity: 0;
  transition: opacity 0.2s;
}

.attachment-list .attachment-item:hover .delete-btn {
  opacity: 1;
}

.chat-container {
  flex: 1;
  display: flex;
  flex-direction: column;
  background: var(--bg-white);
  border-radius: var(--radius-md);
  border: 1px solid var(--border-color);
  overflow: hidden;
}

.messages-container {
  flex: 1;
  overflow-y: auto;
  padding: 16px;
}

.empty-chat {
  height: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--text-secondary);
}

.message {
  margin-bottom: 16px;
  max-width: 80%;
}

.message.user {
  margin-left: auto;
  max-width: 50%;
}

.message.assistant {
  margin-right: auto;
}

.message-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 4px;
  font-size: 12px;
}

.message-role {
  font-weight: 500;
  color: var(--text-primary);
}

.message-time {
  color: var(--text-disabled);
}

.message-content {
  padding: 12px 16px;
  border-radius: var(--radius-md);
  line-height: 1.6;
  white-space: pre-wrap;
}

.message.user .message-content {
  background: var(--primary-color);
  color: #ffffff;
}

.message.assistant .message-content {
  background: var(--bg-color);
  color: var(--text-primary);
}

.message.loading .message-content {
  color: var(--text-secondary);
  font-style: italic;
}

.message-attachments {
  margin-top: 8px;
  display: flex;
  gap: 8px;
  flex-wrap: wrap;
}

.attachment-tag {
  font-size: 12px;
  padding: 2px 8px;
  background: var(--bg-color);
  border-radius: var(--radius-sm);
  color: var(--text-secondary);
}

.input-area {
  border-top: 1px solid var(--border-color);
  padding: 16px;
  flex-shrink: 0;
}

.attachments-preview {
  display: flex;
  gap: 8px;
  flex-wrap: wrap;
  margin-bottom: 12px;
}

.attachments-preview .attachment-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 4px 8px;
  background: var(--bg-color);
  border-radius: var(--radius-sm);
  font-size: 12px;
}

.attachment-icon {
  font-size: 16px;
}

.attachment-name {
  color: var(--text-primary);
  max-width: 150px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.input-row {
  display: flex;
  gap: 8px;
  align-items: flex-end;
}

.input-row .ant-input {
  flex: 1;
}

/* 确保红色背景按钮上的文字是白色 */
.input-row .ant-btn-primary,
.page-header .ant-btn-primary {
  color: #ffffff !important;
}

.page-header .ant-btn-primary[disabled] {
  color: rgba(255, 255, 255, 0.65) !important;
}
</style>
