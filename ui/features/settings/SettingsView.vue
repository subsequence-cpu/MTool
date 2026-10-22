<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watch } from 'vue';
import { appState, notify, updatePreferences } from '../../app/store';
import { client, desktopAvailable, errorMessage } from '../../bindings/client';
import AppIcon from '../../components/ui/AppIcon.vue';
import ConfirmDialog from '../../components/ui/ConfirmDialog.vue';
import ShortcutInput from '../../components/ui/ShortcutInput.vue';
import type { Settings } from '../../bindings/types';

type EditableSettings = Pick<
  Settings,
  | 'launchAtLogin'
  | 'captureEnabled'
  | 'launcherShortcut'
  | 'clipboardShortcut'
  | 'historyLimit'
  | 'retentionDays'
  | 'maxStorageMb'
  | 'excludedApps'
>;

function editableSettings(settings: Settings): EditableSettings {
  return {
    launchAtLogin: settings.launchAtLogin,
    captureEnabled: settings.captureEnabled,
    launcherShortcut: settings.launcherShortcut,
    clipboardShortcut: settings.clipboardShortcut,
    historyLimit: settings.historyLimit,
    retentionDays: settings.retentionDays,
    maxStorageMb: settings.maxStorageMb,
    excludedApps: [...settings.excludedApps],
  };
}

const initial = editableSettings(appState.settings);
const { excludedApps: initialExclusions, ...initialForm } = initial;
const form = ref(initialForm);
const exclusions = ref(initialExclusions.join('\n'));
const baseline = ref(initial);
const busy = ref(false);
const error = ref('');
const confirmClear = ref(false);
const editable = computed(() => desktopAvailable && appState.status?.storageReady);
const isMac = navigator.userAgent.includes('Mac');
let timer: ReturnType<typeof setTimeout> | undefined;
let disposed = false;
let syncing = false;

function snapshot(): EditableSettings {
  return {
    ...form.value,
    excludedApps: exclusions.value
      .split('\n')
      .map((name) => name.trim())
      .filter(Boolean),
  };
}

function changedSettings(): Partial<EditableSettings> {
  const values = snapshot();
  return Object.fromEntries(
    Object.entries(values).filter(
      ([key, value]) =>
        JSON.stringify(value) !== JSON.stringify(baseline.value[key as keyof EditableSettings]),
    ),
  );
}

async function applySettings(): Promise<void> {
  if (!editable.value || busy.value) return;
  busy.value = true;
  error.value = '';
  try {
    while (true) {
      const patch = changedSettings();
      const values = snapshot();
      for (const [key, min, max] of [
        ['historyLimit', 50, 10000],
        ['retentionDays', 1, 365],
        ['maxStorageMb', 16, 2048],
      ] as const) {
        if (!Number.isInteger(values[key]) || values[key] < min || values[key] > max)
          delete patch[key];
      }
      if (!Object.keys(patch).length) break;
      await updatePreferences(patch);
      baseline.value = editableSettings(appState.settings);
      if (timer !== undefined) return;
    }
  } catch (reason) {
    error.value = errorMessage(reason);
    if (disposed) notify(error.value, true);
  } finally {
    busy.value = false;
  }
}

watch(
  [form, exclusions],
  () => {
    if (syncing) return;
    error.value = '';
    clearTimeout(timer);
    const patch = changedSettings();
    const delayed =
      'excludedApps' in patch ||
      'historyLimit' in patch ||
      'retentionDays' in patch ||
      'maxStorageMb' in patch;
    timer = setTimeout(
      () => {
        timer = undefined;
        void applySettings();
      },
      delayed ? 400 : 0,
    );
  },
  { deep: true, flush: 'sync' },
);

watch(
  () => appState.settings,
  (settings) => {
    const next = editableSettings(settings);
    syncing = true;
    for (const key of Object.keys(form.value) as (keyof typeof initialForm)[]) {
      if (form.value[key] === baseline.value[key]) Object.assign(form.value, { [key]: next[key] });
    }
    if (
      JSON.stringify(next.excludedApps) !== JSON.stringify(baseline.value.excludedApps) &&
      JSON.stringify(snapshot().excludedApps) === JSON.stringify(baseline.value.excludedApps)
    )
      exclusions.value = next.excludedApps.join('\n');
    baseline.value = next;
    syncing = false;
  },
  { flush: 'sync' },
);

onBeforeUnmount(() => {
  clearTimeout(timer);
  timer = undefined;
  disposed = true;
  void applySettings();
});

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
      <button
        v-if="error"
        class="button"
        type="button"
        :disabled="!editable || busy"
        @click="applySettings"
      >
        重试
      </button>
    </header>
    <form @submit.prevent="applySettings">
      <fieldset :disabled="!editable">
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
            <div class="setting-row">
              <span><strong>搜索启动台</strong></span
              ><ShortcutInput
                v-model="form.launcherShortcut"
                label="启动台快捷键"
                :is-mac="isMac"
              />
            </div>
            <div class="setting-row">
              <span><strong>剪贴板浮窗</strong></span
              ><ShortcutInput
                v-model="form.clipboardShortcut"
                label="剪贴板快捷键"
                :is-mac="isMac"
              />
            </div>
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
