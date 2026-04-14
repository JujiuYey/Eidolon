import { createRouter, createWebHistory } from 'vue-router';
import Layout from '@/layout/index.vue';

// 路由配置
const routes = [
  {
    path: '/',
    component: Layout,
    redirect: '/agent',
    children: [
      {
        path: '/conversation',
        component: () => import('@/views/conversation/index.vue'),
      },
      {
        path: '/agent',
        component: () => import('@/views/agent/index.vue'),
      },
      {
        path: '/index',
        component: () => import('@/views/mail/index.vue'),
      },
      {
        path: '/codegen',
        component: () => import('@/views/codegen/index.vue'),
      },
      {
        path: '/app-setting',
        component: () => import('@/views/app-setting/index.vue'),
      },
    ],
  },
  {
    path: '/:pathMatch(.*)*',
    name: 'NotFound',
    component: () => import('@/pages/errors/404.vue'),
  },
];

// 创建路由实例
const router = createRouter({
  history: createWebHistory(),
  routes,
});

export default router;
