<script setup lang="ts">
import { onMounted, onUnmounted } from 'vue';
import { useRouter } from 'vue-router';
import AppIcon from '../components/ui/AppIcon.vue';
import { appState } from '../app/store';
import { themeIcon, themeName } from '../app/theme';
import { desktopAvailable } from '../bindings/client';

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
    class="app-shell flex min-h-dvh flex-col"
    :class="{ 'clipboard-shell': router.currentRoute.value.path === '/clipboard' }"
  >
    <header
      class="app-header flex items-center gap-6 border-0 border-b border-solid border-border bg-background-deep px-8 py-[19px] tablet:px-6 tablet:py-4 narrow:flex-col narrow:items-stretch narrow:gap-[15px] narrow:p-4"
    >
      <RouterLink
        to="/"
        class="brand flex items-center gap-2.5 text-[18px] font-semibold tracking-[-0.5px]"
        aria-label="MTool 首页"
        ><img class="block h-10 w-10 shrink-0" :src="themeIcon" alt="" /><span>MTool</span
        ><span
          class="ml-0.5 rounded border border-solid border-border px-1.5 py-px text-[10px] font-normal tracking-[0.4px] text-accent tablet:hidden"
          >{{ themeName }}</span
        ></RouterLink
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
    <main
      class="app-main mx-auto w-full max-w-[1440px] flex-1 px-12 pb-10 pt-8 tablet:px-7 tablet:pb-[35px] tablet:pt-7 narrow:px-[17px] narrow:pb-8 narrow:pt-[22px]"
    >
      <slot />
    </main>
  </div>
</template>
