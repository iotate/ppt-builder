import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'

export interface ProjectInfo {
  name: string
  topic: string
  created_at: string
  updated_at: string
  template: string
  status: ProjectStatus
  page_count: number
  image_count: number
  has_brainstorm: boolean
  has_outline: boolean
  has_images: boolean
  export_path?: string
  dir: string
}

export type ProjectStatus = 
  | 'draft'
  | 'outline_draft'
  | 'outline_confirmed'
  | 'pages_split'
  | 'images_generating'
  | 'images_done'
  | 'exported'

export const useProjectStore = defineStore('project', () => {
  const projects = ref<ProjectInfo[]>([])
  const currentProject = ref<ProjectInfo | null>(null)
  const loading = ref(false)
  const error = ref<string | null>(null)

  const sortedProjects = computed(() => {
    return [...projects.value].sort((a, b) => 
      new Date(b.updated_at).getTime() - new Date(a.updated_at).getTime()
    )
  })

  async function loadProjects() {
    loading.value = true
    error.value = null
    try {
      projects.value = await invoke<ProjectInfo[]>('list_projects')
    } catch (e) {
      error.value = String(e)
      projects.value = []
    } finally {
      loading.value = false
    }
  }

  async function createProject(topic: string, template: string) {
    loading.value = true
    error.value = null
    try {
      const project = await invoke<ProjectInfo>('create_project', { topic, template })
      projects.value.push(project)
      return project
    } catch (e) {
      error.value = String(e)
      throw e
    } finally {
      loading.value = false
    }
  }

  async function openProject(name: string) {
    loading.value = true
    error.value = null
    try {
      currentProject.value = await invoke<ProjectInfo>('open_project', { name })
      return currentProject.value
    } catch (e) {
      error.value = String(e)
      throw e
    } finally {
      loading.value = false
    }
  }

  async function deleteProject(name: string) {
    loading.value = true
    error.value = null
    try {
      await invoke('delete_project', { name })
      projects.value = projects.value.filter(p => p.name !== name)
    } catch (e) {
      error.value = String(e)
      throw e
    } finally {
      loading.value = false
    }
  }

  return {
    projects,
    currentProject,
    loading,
    error,
    sortedProjects,
    loadProjects,
    createProject,
    openProject,
    deleteProject
  }
})
