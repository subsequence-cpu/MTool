<script setup lang="ts">
import { onMounted, onUnmounted } from 'vue';
import { useRouter } from 'vue-router';
import AppIcon from '../components/ui/AppIcon.vue';
import { appState } from '../app/store';
import { desktopAvailable } from '../bindings/client';
import appIcon from '../assets/app-icon.svg';

const router = useRouter();
const isMac = navigator.userAgent.includes('Mac');
const nav = [
  { to: '/', icon: 'grid', label: '工具箱' },
  { to: '/clipboard', icon: 'clipboard', label: '剪贴板' },
  { to: '/mouse', icon: 'mouse', label: '鼠标增强' },
  ...(isMac ? [{ to: '/context-menu', icon: 'context-menu', label: '右键增强' }] : []),
  { to: '/settings', icon: 'settings', label: '设置' },
];
function focusSearch(event: KeyboardEvent): void {
  if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === 'k') {
    event.preventDefault();
    void router.push('/').then(() => {
      requestAnimationFrame(() =>
        document.querySelector<HTMLInputElement>('[data-tool-search]')?.focus(),
      );
    });
  }
}
onMounted(() => window.addEventListener('keydown', focusSearch));
onUnmounted(() => window.removeEventListener('keydown', focusSearch));
</script>

<template>
  <div
    class="app-shell"
    :class="{ 'clipboard-shell': router.currentRoute.value.path === '/clipboard' }"
  >
    <header class="app-header">
      <RouterLink to="/" class="brand" aria-label="MTool 首页"
        ><img class="brand-mark" :src="appIcon" alt="" /><span>MTool</span
        ><span class="theme-caption">Dracula</span></RouterLink
      >
      <nav aria-label="主导航">
        <RouterLink
          v-for="item in nav"
          :key="item.to"
          :to="item.to"
          :class="{
            'is-active':
              item.to === '/'
                ? router.currentRoute.value.path === '/' ||
                  router.currentRoute.value.path.startsWith('/tools/')
                : router.currentRoute.value.path === item.to,
          }"
          ><AppIcon :name="item.icon" /><span>{{ item.label }}</span></RouterLink
        >
      </nav>
    </header>
    <div v-if="!desktopAvailable" class="environment-banner">
      <AppIcon name="info" :size="16" />浏览器预览
    </div>
    <RouterLink
      v-else-if="appState.status && !appState.status.storageReady"
      to="/settings"
      class="environment-banner warning"
      ><AppIcon name="alert" :size="16" />历史存储不可用 · 查看设置
      <AppIcon name="arrow-right" :size="14"
    /></RouterLink>
    <main class="app-main"><slot /></main>
  </div>
</template>
