import { createRouter, createWebHistory } from 'vue-router'

const router = createRouter({
  history: createWebHistory(),
  routes: [
    // 第一部分：项目相关
    {
      path: '/',
      name: 'projects',
      component: () => import('@/views/ProjectList.vue')
    },
    {
      path: '/project/:id',
      name: 'project',
      redirect: { name: 'brainstorm' },
      children: [
        {
          path: 'brainstorm',
          name: 'brainstorm',
          component: () => import('@/views/Step1_Brainstorm.vue')
        },
        {
          path: 'outline',
          name: 'outline',
          component: () => import('@/views/Step2_Outline.vue')
        },
        {
          path: 'pages',
          name: 'pages',
          component: () => import('@/views/Step3_Pages.vue')
        }
      ]
    },
    // 第二部分：设置相关
    {
      path: '/config',
      name: 'config',
      component: () => import('@/views/ApiConfig.vue')
    },
    {
      path: '/templates',
      name: 'templates',
      component: () => import('@/views/TemplateManagement.vue')
    },
    {
      path: '/styles',
      name: 'styles',
      component: () => import('@/views/StyleManagement.vue')
    },
    {
      path: '/skeletons',
      name: 'skeletons',
      component: () => import('@/views/SkeletonManagement.vue')
    },
    {
      path: '/logs',
      name: 'logs',
      component: () => import('@/views/LogView.vue')
    },
    {
      path: '/about',
      name: 'about',
      component: () => import('@/views/AboutView.vue')
    }
  ]
})

export default router
