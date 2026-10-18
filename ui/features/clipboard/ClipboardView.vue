<script setup lang="ts">
import { ref } from 'vue';
import { useRouter } from 'vue-router';
import { appState, notify, updatePreferences } from '../../app/store';
import { client, desktopAvailable, errorMessage } from '../../bindings/client';
import AppIcon from '../../components/ui/AppIcon.vue';
import AppSelect from '../../components/ui/AppSelect.vue';
import ConfirmDialog from '../../components/ui/ConfirmDialog.vue';
import { getSession } from '../tools/sessions';
import { formatBytes, formatDate, useHistory } from './history';

const router = useRouter();
const {
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
} = useHistory();
const confirmation = ref<'delete' | 'clear' | null>(null);
const busy = ref(false);
const destination = ref('json');

async function action(callback: () => Promise<void>, message?: string): Promise<void> {
  if (busy.value) return;
  busy.value = true;
  try {
    await callback();
    if (message) notify(message);
  } catch (reason) {
    notify(errorMessage(reason), true);
  } finally {
    busy.value = false;
  }
}

async function confirm(): Promise<void> {
  const operation = confirmation.value;
  confirmation.value = null;
  if (operation === 'delete' && selected.value) {
    const id = selected.value;
    await action(() => client.delete(id), '这条历史已删除');
  } else if (operation === 'clear')
    await action(() => client.clear(false), '普通历史已清空，收藏已保留');
  await load();
}

function sendToTool(): void {
  const tool = appState.tools.find((item) => item.id === destination.value);
  if (!tool || detail.value?.text === null || detail.value?.text === undefined) return;
  getSession(tool).input = detail.value.text;
  void router.push(`/tools/${tool.id}`);
}

function toggleCapture(): void {
  void action(() => updatePreferences((current) => ({ captureEnabled: !current.captureEnabled })));
}

function copySelected(): void {
  const id = detail.value?.item.id;
  if (id) void action(() => client.useHistory(id), '内容已复制');
}

function pinSelected(): void {
  const item = detail.value?.item;
  if (item) void action(() => client.pin(item.id, !item.pinned));
}
</script>

<template>
  <section class="clipboard-page">
    <header class="page-heading">
      <div>
        <h1>剪贴板历史</h1>
      </div>
      <button class="button" :disabled="!desktopAvailable" @click="action(client.openClipboard)">
        <AppIcon name="clipboard" :size="16" />快捷浮窗
      </button>
    </header>
    <div class="history-toolbar">
      <label class="search-field"
        ><AppIcon name="search" :size="16" /><input
          v-model="query"
          aria-label="搜索剪贴板历史"
          placeholder="搜索历史内容或来源应用…"
          maxlength="512" /></label
      ><button
        class="button"
        :aria-pressed="pinnedOnly"
        :class="{ highlighted: pinnedOnly }"
        @click="pinnedOnly = !pinnedOnly"
      >
        <AppIcon name="pin" :size="15" />仅收藏</button
      ><button
        class="button"
        :disabled="!appState.status?.storageReady || busy"
        @click="toggleCapture"
      >
        <AppIcon :name="appState.settings.captureEnabled ? 'pause' : 'play'" :size="15" />{{
          appState.settings.captureEnabled ? '暂停记录' : '继续记录'
        }}</button
      ><button
        class="button danger"
        :disabled="!total || busy"
        aria-label="清空普通历史"
        @click="confirmation = 'clear'"
      >
        <AppIcon name="trash" />清空历史
      </button>
    </div>
    <p v-if="appState.status?.captureError" class="inline-notice" role="status">
      {{ appState.status.captureError }}
    </p>
    <p v-if="error" class="inline-notice error" role="alert">
      {{ error }} <button class="button small" @click="load()"><AppIcon name="reset" />重试</button>
    </p>
    <div v-if="!desktopAvailable || !appState.status?.storageReady" class="empty-state">
      <AppIcon name="lock" :size="34" />
      <h2>{{ desktopAvailable ? '历史存储不可用' : '请使用桌面客户端' }}</h2>
      <p v-if="appState.status?.storageError">{{ appState.status.storageError }}</p>
      <RouterLink v-if="desktopAvailable" class="button" to="/settings">
        <AppIcon name="settings" />查看设置
      </RouterLink>
    </div>
    <div v-else-if="!items.length && !loading && !error" class="empty-state">
      <AppIcon :name="query ? 'search' : 'clipboard'" :size="34" />
      <h2>{{ query ? '无匹配记录' : '暂无记录' }}</h2>
    </div>
    <div v-else class="history-layout">
      <section class="history-list" aria-label="剪贴板历史">
        <p class="history-count" aria-live="polite">
          {{ loading ? '正在加载…' : `${total} 条记录` }}
        </p>
        <button
          v-for="item in items"
          :key="item.id"
          class="history-row"
          :class="{ selected: selected === item.id }"
          :aria-pressed="selected === item.id"
          @click="selected = item.id"
        >
          <AppIcon :name="item.kind === 'image' ? 'image' : 'text'" :size="18" /><span
            class="history-row-copy"
            ><strong>{{ item.preview }}</strong
            ><span
              >{{ item.sourceApp ?? '来源未知' }} · {{ formatDate(item.updatedAt) }}</span
            ></span
          ><AppIcon v-if="item.pinned" name="pin" :size="14" /></button
        ><button v-if="hasMore" class="button load-more" :disabled="loading" @click="load(true)">
          <AppIcon name="chevron-down" />加载更多
        </button>
      </section>
      <section class="history-detail">
        <p v-if="detailError" class="inline-notice error" role="alert">{{ detailError }}</p>
        <template v-if="detail"
          ><header class="section-heading">
            <h2>内容预览</h2>
            <span class="badge">{{ detail.item.kind === 'image' ? '图片' : '文本' }}</span>
          </header>
          <img
            v-if="detail.imageDataUrl"
            class="history-image"
            :src="detail.imageDataUrl"
            :alt="detail.item.preview"
          />
          <pre v-else class="history-text">{{ detail.text }}</pre>
          <dl class="history-metadata">
            <dt>{{ appState.status?.platform === 'macos' ? '来源（估计）' : '来源' }}</dt>
            <dd>{{ detail.item.sourceApp ?? '未知' }}</dd>
            <dt>大小</dt>
            <dd>{{ formatBytes(detail.item.byteSize) }}</dd>
            <dt>记录时间</dt>
            <dd>{{ formatDate(detail.item.createdAt) }}</dd>
          </dl>
          <div class="history-actions">
            <button class="button primary" :disabled="busy" @click="copySelected">
              <AppIcon name="copy" :size="15" />复制</button
            ><button class="button" :disabled="busy" @click="pinSelected">
              <AppIcon name="pin" :size="15" />{{
                detail.item.pinned ? '取消收藏' : '收藏'
              }}</button
            ><button
              class="button danger"
              :disabled="busy"
              aria-label="删除当前历史"
              @click="confirmation = 'delete'"
            >
              <AppIcon name="trash" />删除
            </button>
          </div>
          <div v-if="detail.text !== null" class="send-to-tool">
            <label class="sr-only" for="destination-tool">目标工具</label
            ><AppSelect id="destination-tool" v-model="destination" icon="grid">
              <option
                v-for="tool in appState.tools.filter((item) => !['uuid'].includes(item.id))"
                :key="tool.id"
                :value="tool.id"
              >
                {{ tool.title }}
              </option></AppSelect
            ><button class="button" @click="sendToTool">
              <AppIcon name="arrow-up-right" :size="15" />发送到工具
            </button>
          </div>
        </template>
        <div v-else-if="selected && !detailError" class="empty-state"><p>正在读取内容…</p></div>
      </section>
    </div>
    <ConfirmDialog
      v-if="confirmation"
      :title="confirmation === 'clear' ? '清空普通历史？' : '删除这条历史？'"
      :description="
        confirmation === 'clear'
          ? '删除未收藏记录，保留收藏。删除后无法恢复。'
          : '删除此记录，无法恢复。'
      "
      confirm-label="删除"
      destructive
      @confirm="confirm"
      @cancel="confirmation = null"
    />
  </section>
</template>
