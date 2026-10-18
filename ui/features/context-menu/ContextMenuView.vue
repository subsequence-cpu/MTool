<script setup lang="ts">
import { onMounted, onUnmounted, ref } from 'vue';
import { client, desktopAvailable, errorMessage } from '../../bindings/client';
import type { ContextMenuStatus } from '../../bindings/types';
import AppIcon from '../../components/ui/AppIcon.vue';

const status = ref<ContextMenuStatus>();
const busy = ref(false);
const error = ref('');

async function refresh(): Promise<void> {
  if (!desktopAvailable || busy.value) return;
  busy.value = true;
  error.value = '';
  try {
    status.value = await client.contextMenuStatus();
  } catch (reason) {
    error.value = errorMessage(reason);
  } finally {
    busy.value = false;
  }
}

async function openSettings(): Promise<void> {
  try {
    await client.contextMenuSettings();
  } catch (reason) {
    error.value = errorMessage(reason);
  }
}

async function openAccessibilitySettings(): Promise<void> {
  try {
    await client.contextMenuAccessibilitySettings();
  } catch (reason) {
    error.value = errorMessage(reason);
  }
}

onMounted(() => {
  void refresh();
  window.addEventListener('focus', refresh);
});
onUnmounted(() => window.removeEventListener('focus', refresh));
</script>

<template>
  <section class="settings-page context-menu-page">
    <header class="page-heading">
      <h1>右键增强</h1>
      <span class="badge">macOS</span>
    </header>
    <div v-if="error || status?.error" class="inline-error" role="alert">
      {{ error || status?.error }}
    </div>
    <p v-if="status && !status.supported" class="setting-description">仅支持 macOS</p>
    <section v-else class="settings-section" aria-labelledby="finder-heading">
      <div class="context-section-heading">
        <h2 id="finder-heading">Finder 右键菜单</h2>
        <button class="button" :disabled="!desktopAvailable || busy" @click="refresh">
          <AppIcon name="reset" :class="{ 'context-refreshing': busy }" />刷新
        </button>
      </div>
      <div class="settings-card">
        <div class="setting-row">
          <strong>Finder 扩展</strong>
          <div class="context-row-actions">
            <span class="badge">{{ status ? (status.enabled ? '已启用' : '未启用') : '—' }}</span>
            <button
              class="button"
              :disabled="!desktopAvailable || !status?.bundled"
              @click="openSettings"
            >
              <AppIcon name="settings" />管理扩展
            </button>
          </div>
        </div>
        <div class="setting-row">
          <span class="context-feature"><AppIcon name="file-plus" /><strong>新增文件</strong></span>
        </div>
        <div class="setting-row">
          <span class="context-feature"
            ><AppIcon name="code" /><strong>使用 VS Code 打开</strong></span
          >
        </div>
        <div class="setting-row">
          <strong>VS Code</strong>
          <span class="badge">{{
            status ? (status.vscodeInstalled ? '已安装' : '未检测到') : '—'
          }}</span>
        </div>
        <div class="setting-row">
          <strong>辅助功能</strong>
          <div class="context-row-actions">
            <span class="badge">{{
              status ? (status.accessibilityGranted ? '已授权' : '未授权') : '—'
            }}</span>
            <button class="button" :disabled="!desktopAvailable" @click="openAccessibilitySettings">
              <AppIcon name="settings" />系统设置
            </button>
          </div>
        </div>
      </div>
    </section>
  </section>
</template>
