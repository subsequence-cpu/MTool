import { onBeforeUnmount, ref, watch } from 'vue';
import { appState } from '../../app/store';
import { client, desktopAvailable, errorMessage } from '../../bindings/client';
import type { ClipboardDetail, ClipboardItem } from '../../bindings/types';

export function useHistory(withDetails = true) {
  const query = ref('');
  const pinnedOnly = ref(false);
  const items = ref<ClipboardItem[]>([]);
  const total = ref(0);
  const hasMore = ref(false);
  const loading = ref(false);
  const error = ref('');
  const selected = ref<string | null>(null);
  const detail = ref<ClipboardDetail | null>(null);
  const detailError = ref('');
  let generation = 0;
  let nextOffset = 0;
  let detailGeneration = 0;
  let timer: ReturnType<typeof setTimeout> | undefined;

  async function load(append = false): Promise<void> {
    if (!desktopAvailable || !appState.status?.storageReady) return;
    if (append && (loading.value || !hasMore.value)) return;
    const offset = append ? nextOffset : 0;
    const current = ++generation;
    loading.value = true;
    error.value = '';
    try {
      const page = await client.history(query.value, pinnedOnly.value, offset);
      if (current !== generation) return;
      const records = append ? [...items.value, ...page.items] : page.items;
      items.value = [...new Map(records.map((item) => [item.id, item])).values()];
      nextOffset = offset + page.items.length;
      total.value = page.total;
      hasMore.value = page.hasMore && page.items.length > 0;
      if (!items.value.some((item) => item.id === selected.value))
        selected.value = items.value[0]?.id ?? null;
      const latest = items.value.find((item) => item.id === selected.value);
      if (latest && detail.value?.item.id === latest.id) detail.value.item = latest;
    } catch (reason) {
      if (current === generation) error.value = errorMessage(reason);
    } finally {
      if (current === generation) loading.value = false;
    }
  }

  watch(
    [query, pinnedOnly, () => appState.historyVersion],
    () => {
      generation += 1;
      loading.value = desktopAvailable && Boolean(appState.status?.storageReady);
      clearTimeout(timer);
      timer = setTimeout(() => {
        void load();
      }, 120);
    },
    { immediate: true },
  );

  watch(selected, async (id) => {
    if (!withDetails) return;
    const current = ++detailGeneration;
    detail.value = null;
    detailError.value = '';
    if (!id) return;
    try {
      const value = await client.detail(id);
      if (current === detailGeneration) detail.value = value;
    } catch (reason) {
      if (current === detailGeneration) detailError.value = errorMessage(reason);
    }
  });

  onBeforeUnmount(() => {
    clearTimeout(timer);
    generation += 1;
    detailGeneration += 1;
  });
  return {
    query,
    pinnedOnly,
    items,
    total,
    hasMore,
    loading,
    error,
    selected,
    detail,
    detailError,
    load,
  };
}

export function formatDate(timestamp: number): string {
  return new Intl.DateTimeFormat('zh-CN', {
    month: '2-digit',
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit',
  }).format(timestamp);
}

export function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KiB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MiB`;
}
