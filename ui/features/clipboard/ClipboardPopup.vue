<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from 'vue';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { appState } from '../../app/store';
import { client, errorMessage } from '../../bindings/client';
import AppIcon from '../../components/ui/AppIcon.vue';
import { formatDate, useHistory } from './history';

const { query, items, total, loading, error, selected, load } = useHistory(false);
const searchInput = ref<HTMLInputElement>();
const busy = ref(false);
const visible = ref(false);
const feedback = ref('');
const thumbnails = ref<Record<string, string>>({});
const selectedItem = computed(() => items.value.find((item) => item.id === selected.value));
let thumbnailGeneration = 0;
const currentIndex = computed(() =>
  Math.max(
    0,
    items.value.findIndex((item) => item.id === selected.value),
  ),
);
let unlisten: UnlistenFn | undefined;
let unlistenClosed: UnlistenFn | undefined;
let previewGeneration = 0;

async function close(): Promise<void> {
  visible.value = false;
  await client.closeClipboard();
}
function drag(event: MouseEvent): void {
  if (event.button !== 0) return;
  void client.dragClipboard().catch((reason: unknown) => {
    feedback.value = errorMessage(reason);
  });
}
async function use(id: string): Promise<void> {
  if (busy.value || loading.value) return;
  busy.value = true;
  feedback.value = '';
  try {
    await client.useHistory(id);
    await close();
  } catch (reason) {
    feedback.value = errorMessage(reason);
  } finally {
    busy.value = false;
  }
}

function keyboard(event: KeyboardEvent): void {
  if (event.isComposing) return;
  if (event.key === 'Escape') {
    event.preventDefault();
    void close();
  }
  if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
    event.preventDefault();
    const index = Math.max(
      0,
      Math.min(items.value.length - 1, currentIndex.value + (event.key === 'ArrowDown' ? 1 : -1)),
    );
    selected.value = items.value[index]?.id ?? null;
  }
  if (event.key === 'Enter' && selected.value) {
    event.preventDefault();
    void use(selected.value);
  }
}
watch([items, visible], async ([page, shown]) => {
  const generation = ++thumbnailGeneration;
  if (!shown) return;
  const images = page.filter((item) => item.kind === 'image');
  thumbnails.value = Object.fromEntries(
    images.flatMap((item) =>
      thumbnails.value[item.id] ? [[item.id, thumbnails.value[item.id]]] : [],
    ),
  );
  const pending = images.filter((item) => !thumbnails.value[item.id]);
  let index = 0;
  async function worker(): Promise<void> {
    while (generation === thumbnailGeneration && index < pending.length) {
      const item = pending[index++];
      for (let attempt = 0; attempt < 3 && generation === thumbnailGeneration; attempt += 1) {
        try {
          const thumbnail = await client.thumbnail(item.id);
          if (generation === thumbnailGeneration) thumbnails.value[item.id] = thumbnail;
          break;
        } catch (reason) {
          const occupied =
            typeof reason === 'object' &&
            reason !== null &&
            'code' in reason &&
            reason.code === 'busy';
          if (!occupied || attempt === 2) break;
          await new Promise((resolve) => setTimeout(resolve, 100));
        }
      }
    }
  }
  await Promise.all([worker(), worker()]);
});
watch(selected, () => {
  void nextTick(() =>
    document.querySelector('.popup-history-row.selected')?.scrollIntoView({ block: 'nearest' }),
  );
});
watch([selected, visible], async ([id, shown]) => {
  const generation = ++previewGeneration;
  try {
    await client.setClipboardPreview(shown && selectedItem.value?.kind === 'image' ? id : null);
  } catch (reason) {
    if (generation === previewGeneration) feedback.value = errorMessage(reason);
  }
});
onMounted(async () => {
  searchInput.value?.focus();
  window.addEventListener('keydown', keyboard);
  unlisten = await listen('clipboard-opened', () => {
    visible.value = true;
    query.value = '';
    feedback.value = '';
    void load();
    void nextTick(() => searchInput.value?.focus());
  });
  unlistenClosed = await listen('clipboard-closed', () => {
    visible.value = false;
  });
  visible.value = await client.clipboardVisible();
});
onUnmounted(() => {
  thumbnailGeneration += 1;
  unlisten?.();
  unlistenClosed?.();
  window.removeEventListener('keydown', keyboard);
});
</script>

<template>
  <div class="clipboard-popup">
    <button
      class="popup-drag-handle"
      aria-label="拖动浮窗"
      title="拖动浮窗"
      @mousedown.prevent="drag"
    >
      <span></span>
    </button>
    <header class="popup-search">
      <label class="popup-search-field"
        ><AppIcon name="search" :size="20" /><input
          ref="searchInput"
          v-model="query"
          aria-label="搜索剪贴板历史"
          placeholder="搜索复制过的内容…"
          maxlength="512"
          autocomplete="off" /></label
      ><button class="icon-button" aria-label="关闭剪贴板浮窗" @click="close">
        <AppIcon name="x" :size="17" />
      </button>
    </header>
    <div
      v-if="feedback || error || appState.status?.storageError"
      class="inline-notice error popup-notice"
      role="alert"
    >
      {{ feedback || error || appState.status?.storageError }}
    </div>
    <div class="popup-content">
      <div class="popup-history">
        <div class="popup-subheading" @mousedown.prevent="drag">
          <span>剪贴板历史</span><span>{{ total }} 条记录</span>
        </div>
        <div class="popup-results">
          <button
            v-for="item in items"
            :key="item.id"
            class="popup-history-row"
            :class="{ selected: selected === item.id }"
            :disabled="busy"
            @pointermove="selected = item.id"
            @click="use(item.id)"
          >
            <span class="popup-item-icon"
              ><img v-if="thumbnails[item.id]" :src="thumbnails[item.id]" alt="" decoding="async" />
              <AppIcon v-else :name="item.kind === 'image' ? 'image' : 'text'" :size="20" /></span
            ><span class="history-row-copy"
              ><strong>{{ item.preview }}</strong
              ><span
                >{{ item.sourceApp ?? '来源未知' }} · {{ formatDate(item.updatedAt) }}</span
              ></span
            ><AppIcon v-if="item.pinned" name="pin" :size="13" /><kbd v-if="selected === item.id"
              ><AppIcon name="return" :size="12"
            /></kbd>
          </button>
          <div v-if="!items.length" class="empty-state">
            <AppIcon name="clipboard" :size="30" />
            <p>{{ loading ? '正在查找…' : '还没有匹配的历史记录' }}</p>
          </div>
        </div>
      </div>
    </div>
    <footer class="popup-footer">
      <span class="popup-shortcut"
        ><kbd><AppIcon name="arrow-up" :size="12" /></kbd
        ><kbd><AppIcon name="arrow-down" :size="12" /></kbd><span>选择</span></span
      >
      <span class="popup-shortcut"
        ><kbd><AppIcon name="return" :size="12" /></kbd><span>复制并关闭</span></span
      >
      <span class="popup-shortcut"><kbd>Esc</kbd><span>关闭</span></span>
    </footer>
  </div>
</template>
