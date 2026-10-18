import { createRouter, createWebHashHistory } from 'vue-router';

export const router = createRouter({
  history: createWebHashHistory(),
  routes: [
    { path: '/', name: 'home', component: () => import('../features/launcher/LauncherView.vue') },
    {
      path: '/tools/:toolId',
      name: 'tool',
      component: () => import('../features/tools/ToolWorkspace.vue'),
    },
    {
      path: '/clipboard',
      name: 'clipboard',
      component: () => import('../features/clipboard/ClipboardView.vue'),
    },
    { path: '/mouse', name: 'mouse', component: () => import('../features/mouse/MouseView.vue') },
    {
      path: '/context-menu',
      name: 'context-menu',
      component: () => import('../features/context-menu/ContextMenuView.vue'),
    },
    {
      path: '/settings',
      name: 'settings',
      component: () => import('../features/settings/SettingsView.vue'),
    },
    { path: '/:pathMatch(.*)*', redirect: '/' },
  ],
});
