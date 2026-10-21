<script setup lang="ts">
import { computed, ref } from 'vue';
import { appState, notify, refreshStatus, updatePreferences } from '../../app/store';
import { client, desktopAvailable, errorMessage } from '../../bindings/client';
import AppIcon from '../../components/ui/AppIcon.vue';
import ConfirmDialog from '../../components/ui/ConfirmDialog.vue';
import ShortcutInput from '../../components/ui/ShortcutInput.vue';
import type { Settings } from '../../bindings/types';

const form = ref<Settings>(
  structuredClone({
    ...appState.settings,
    mouse: {
      ...appState.settings.mouse,
      extraButtons: { ...appState.settings.mouse.extraButtons },
    },
    excludedApps: [...appState.settings.excludedApps],
    favorites: [...appState.settings.favorites],
    recentTools: [...appState.settings.recentTools],
  }),
);
const exclusions = ref(form.value.excludedApps.join('\n'));
const busy = ref(false);
const error = ref('');
const confirmClear = ref(false);
const saved = ref(false);
const editable = computed(() => desktopAvailable && appState.status?.storageReady);
const isMac = navigator.userAgent.includes('Mac');

async function save(): Promise<void> {
  busy.value = true;
  error.value = '';
  saved.value = false;
  try {
    form.value.excludedApps = exclusions.value
      .split('\n')
      .map((name) => name.trim())
      .filter(Boolean);
    await updatePreferences((current) => ({
      ...form.value,
      mouse: current.mouse,
      favorites: [...current.favorites],
      recentTools: [...current.recentTools],
    }));
    await refreshStatus();
    saved.value = true;
    notify('设置已保存');
  } catch (reason) {
    error.value = errorMessage(reason);
  } finally {
    busy.value = false;
  }
}

async function clearAll(): Promise<void> {
  confirmClear.value = false;
  try {
    await client.clear(true);
    notify('所有历史与收藏已删除');
  } catch (reason) {
    notify(errorMessage(reason), true);
  }
}
</script>

<template>
  <section class="settings-page">
    <header class="page-heading">
      <div>
        <h1>设置</h1>
      </div>
      <button class="button primary" :disabled="!editable || busy" @click="save">
        <AppIcon :name="busy ? 'clock' : 'save'" />{{ busy ? '保存中…' : '保存设置' }}
      </button>
    </header>
    <form @submit.prevent="save">
      <fieldset :disabled="!editable || busy">
        <section class="settings-section">
          <h2>通用</h2>
          <div class="settings-card">
            <div class="setting-row">
              <div>
                <strong>主题</strong>
              </div>
              <span class="theme-name"><span class="theme-swatch" />Dracula</span>
            </div>
            <label class="setting-row"
              ><span
                ><strong>登录时启动</strong
                ><span class="setting-description">启动后驻留菜单栏</span></span
              ><input
                v-model="form.launchAtLogin"
                class="switch"
                type="checkbox"
                aria-label="登录时启动" /></label
            ><label class="setting-row"
              ><span><strong>记录剪贴板</strong></span
              ><input
                v-model="form.captureEnabled"
                class="switch"
                type="checkbox"
                aria-label="记录剪贴板"
            /></label>
          </div>
        </section>
        <section class="settings-section">
          <h2>快捷键</h2>
          <div class="settings-card">
            <label class="setting-row"
              ><span><strong>搜索启动台</strong></span
              ><ShortcutInput
                v-model="form.launcherShortcut"
                label="启动台快捷键"
                :is-mac="isMac" /></label
            ><label class="setting-row"
              ><span><strong>剪贴板浮窗</strong></span
              ><ShortcutInput v-model="form.clipboardShortcut" label="剪贴板快捷键" :is-mac="isMac"
            /></label>
            <p class="setting-footnote">修改后保存</p>
          </div>
        </section>
        <section class="settings-section">
          <h2>隐私与历史</h2>
          <div class="settings-card">
            <label class="setting-row"
              ><span
                ><strong>普通历史数量</strong
                ><span class="setting-description">50–10000 条，收藏除外</span></span
              ><input
                v-model.number="form.historyLimit"
                type="number"
                min="50"
                max="10000"
                aria-label="普通历史数量上限" /></label
            ><label class="setting-row"
              ><span
                ><strong>保留天数</strong
                ><span class="setting-description">1–365 天，收藏除外</span></span
              ><input
                v-model.number="form.retentionDays"
                type="number"
                min="1"
                max="365"
                aria-label="保留天数" /></label
            ><label class="setting-row"
              ><span
                ><strong>内容容量上限（MiB）</strong
                ><span class="setting-description">16–2048 MiB</span></span
              ><input
                v-model.number="form.maxStorageMb"
                type="number"
                min="16"
                max="2048"
                aria-label="内容容量上限" /></label
            ><label class="setting-block"
              ><strong>排除应用</strong
              ><span class="setting-description">每行一个应用名称或标识</span
              ><textarea
                v-model="exclusions"
                rows="4"
                aria-label="排除应用名单"
                placeholder="1password&#10;bitwarden"
                spellcheck="false"
              />
            </label>
          </div>
        </section>
      </fieldset>
      <p v-if="error" class="inline-notice error" role="alert">{{ error }}</p>
      <p v-if="saved" class="success-message" role="status">
        <AppIcon name="check" :size="15" />设置已保存
      </p>
    </form>
    <section class="settings-section">
      <h2>存储状态</h2>
      <div class="settings-card">
        <div class="setting-row">
          <div>
            <strong>历史数据库</strong>
          </div>
          <span class="badge" :class="{ success: appState.status?.storageReady }">{{
            appState.status?.storageReady ? '已解锁' : '不可用'
          }}</span>
        </div>
        <p v-if="appState.status?.storageError" class="inline-notice error" role="alert">
          {{ appState.status.storageError }}
        </p>
        <p v-if="appState.status?.shortcutError" class="inline-notice error" role="alert">
          {{ appState.status.shortcutError }}
        </p>
      </div>
    </section>
    <section class="settings-section">
      <h2>数据管理</h2>
      <div class="settings-card">
        <div class="setting-row">
          <div>
            <strong>删除全部历史与收藏</strong>
            <p>删除后无法恢复</p>
          </div>
          <button class="button danger" :disabled="!editable" @click="confirmClear = true">
            <AppIcon name="trash" />删除全部
          </button>
        </div>
      </div>
    </section>
    <p class="about-line">MTool {{ appState.status?.version ?? '0.1.0' }} · MIT</p>
    <ConfirmDialog
      v-if="confirmClear"
      title="删除全部历史与收藏？"
      description="删除全部历史和收藏，无法恢复。"
      confirm-label="删除全部"
      destructive
      @cancel="confirmClear = false"
      @confirm="clearAll"
    />
  </section>
</template>
