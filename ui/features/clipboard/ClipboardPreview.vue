<script setup lang="ts">
import { onMounted, onUnmounted, ref } from 'vue';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { client } from '../../bindings/client';
import type { ClipboardDetail } from '../../bindings/types';

const detail = ref<ClipboardDetail | null>(null);
let generation = 0;
let unlisten: UnlistenFn | undefined;
async function refresh(): Promise<void> {
  const current = ++generation;
  detail.value = null;
  try {
    const value = await client.clipboardPreview();
    if (current === generation) detail.value = value;
  } catch {
    if (current === generation) detail.value = null;
  }
}
function ready(event: Event): void {
  const id = (event.currentTarget as HTMLImageElement).dataset.historyId;
  if (id && id === detail.value?.item.id) void client.showClipboardPreview(id).catch(() => {});
}
onMounted(async () => {
  unlisten = await listen('clipboard-preview-changed', () => void refresh());
  await refresh();
});
onUnmounted(() => {
  generation += 1;
  unlisten?.();
});
</script>

<template>
  <img
    v-if="detail?.imageDataUrl"
    :key="detail.item.id"
    :data-history-id="detail.item.id"
    class="clipboard-preview-image"
    :src="detail.imageDataUrl"
    alt="剪贴板图片预览"
    draggable="false"
    @load="ready"
  />
</template>
