import { createRouter, createWebHistory } from 'vue-router';
import type { RouteRecordRaw } from 'vue-router';

import Layout from '@/layout/index.vue';

// 路由配置
const routes: RouteRecordRaw[] = [
  {
    path: '/',
    component: Layout,
    redirect: '/mail',
    children: [
      {
        path: '/mail',
        component: () => import('@/views/mail/index.vue'),
      },
      {
        // 兼容旧地址：邮件 demo 时期使用的 /index
        path: '/index',
        redirect: '/mail',
      },
      {
        path: '/weekly-report',
        component: () => import('@/views/weekly-report/index.vue'),
      },
      {
        path: '/zentao',
        component: () => import('@/views/zentao/index.vue'),
      },
      {
        path: '/tasks',
        component: () => import('@/views/tasks/index.vue'),
      },
      {
        path: '/schema-dict',
        component: () => import('@/views/schema-dict/index.vue'),
      },
      {
        path: '/collect',
        component: () => import('@/views/collect/index.vue'),
      },
      {
        path: '/api-client',
        component: () => import('@/views/api-project/index.vue'),
      },
      {
        path: '/api-client/projects/:id',
        name: 'api-client-project',
        component: () => import('@/views/api-client-workspace/index.vue'),
        props: route => ({ projectId: route.params.id }),
      },
      {
        path: '/docs',
        name: 'docs-manage',
        component: () => import('@/views/docs/manage.vue'),
      },
      {
        path: '/docs/r/:rootId',
        name: 'docs-root',
        component: () => import('@/views/docs/index.vue'),
        props: route => ({ rootId: route.params.rootId }),
      },
      {
        path: '/pm',
        component: () => import('@/views/pm/index.vue'),
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
