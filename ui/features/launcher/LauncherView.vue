<script setup lang="ts">
import { computed, ref } from 'vue';
import { useRouter } from 'vue-router';
import { appState, notify, rememberTool, toggleFavorite } from '../../app/store';
import { client, desktopAvailable, errorMessage } from '../../bindings/client';
import type { ToolId } from '../../bindings/types';
import AppIcon from '../../components/ui/AppIcon.vue';
import { searchTools } from './search';

const router = useRouter();
const modifier = navigator.userAgent.includes('Mac') ? '⌘' : 'Ctrl';
const query = ref('');
const onlyFavorites = ref(false);
const matches = computed(() =>
  searchTools(appState.tools, query.value, appState.settings.favorites, onlyFavorites.value),
);
const recents = computed(() =>
  appState.settings.recentTools
    .map((id) => appState.tools.find((tool) => tool.id === id))
    .filter((tool) => tool !== undefined)
    .slice(0, 4),
);
function openTool(id: ToolId): void {
  rememberTool(id);
  void router.push(`/tools/${id}`);
}
async function openClipboard(): Promise<void> {
  try {
    await client.openClipboard();
  } catch (error) {
    notify(errorMessage(error), true);
  }
}
</script>

<template>
  <section class="launcher-page">
    <div class="launcher-search">
      <AppIcon name="search" :size="23" /><input
        v-model="query"
        data-tool-search
        aria-label="搜索工具"
        placeholder="搜索工具或动作…"
        autocomplete="off"
        spellcheck="false"
        @keydown.enter="matches[0] && openTool(matches[0].id)"
      /><kbd>{{ modifier }} K</kbd
      ><button v-if="query" class="icon-button" aria-label="清空搜索" @click="query = ''">
        <AppIcon name="x" :size="16" />
      </button>
    </div>
    <header
      class="page-heading mb-[30px] flex items-center justify-between gap-5 narrow:flex-wrap narrow:gap-[14px]"
    >
      <div>
        <h1>工具箱</h1>
      </div>
      <button class="button" :disabled="!desktopAvailable" @click="openClipboard">
        <AppIcon name="clipboard" :size="16" />剪贴板浮窗
      </button>
    </header>
    <section v-if="!query && !onlyFavorites" class="mb-[30px]">
      <div class="section-heading mb-3 flex items-center justify-between gap-3">
        <h2>最近使用</h2>
      </div>
      <div v-if="recents.length" class="mt-3 flex flex-wrap gap-2">
        <button
          v-for="tool in recents"
          :key="tool.id"
          class="button subtle"
          @click="openTool(tool.id)"
        >
          <AppIcon :name="tool.icon" :size="16" />{{ tool.title }}
        </button>
      </div>
      <p v-else class="py-1.5 text-[12px] text-muted">暂无记录</p>
    </section>
    <div class="section-heading mb-[5px] flex items-center justify-between gap-3">
      <div class="segmented">
        <button
          :aria-pressed="!onlyFavorites"
          :class="{ selected: !onlyFavorites }"
          @click="onlyFavorites = false"
        >
          <AppIcon name="grid" :size="13" />工具与动作</button
        ><button
          :aria-pressed="onlyFavorites"
          :class="{ selected: onlyFavorites }"
          @click="onlyFavorites = true"
        >
          <AppIcon name="star" :size="13" />我的收藏
        </button>
      </div>
      <span class="text-[12px] text-muted" aria-live="polite">{{ matches.length }} 项工具</span>
    </div>
    <div v-if="matches.length" class="tool-list">
      <article v-for="tool in matches" :key="tool.id" class="tool-row">
        <button class="tool-open" @click="openTool(tool.id)">
          <span class="tool-icon"><AppIcon :name="tool.icon" :size="21" /></span
          ><span class="tool-copy"
            ><strong>{{ tool.title }}</strong
            ><span>{{ tool.description }}</span></span
          ></button
        ><button
          class="icon-button favorite-button"
          :class="{ 'is-favorite': appState.settings.favorites.includes(tool.id) }"
          :aria-label="`${appState.settings.favorites.includes(tool.id) ? '取消收藏' : '收藏'} ${tool.title}`"
          :aria-pressed="appState.settings.favorites.includes(tool.id)"
          @click="toggleFavorite(tool.id)"
        >
          <AppIcon name="star" :size="16" />
        </button>
      </article>
    </div>
    <div v-else class="empty-state">
      <AppIcon :name="onlyFavorites ? 'star' : 'search'" :size="30" />
      <h2>{{ onlyFavorites ? '无匹配收藏' : '无匹配工具' }}</h2>
      <button
        class="button"
        @click="
          query = '';
          onlyFavorites = false;
        "
      >
        <AppIcon name="grid" />查看全部工具
      </button>
    </div>
  </section>
</template>
