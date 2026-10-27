<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from 'vue';
import { appState, notify, updateMousePreferences } from '../../app/store';
import { client, desktopAvailable, errorMessage } from '../../bindings/client';
import { defaultSettings } from '../../bindings/types';
import type { MouseAction, MouseDpi, MouseSettings, MouseStatus } from '../../bindings/types';
import AppIcon from '../../components/ui/AppIcon.vue';
import MouseSlider from './MouseSlider.vue';
import LogitechDpiControl from './LogitechDpiControl.vue';

const form = ref<MouseSettings>({
  ...appState.settings.mouse,
  extraButtons: { ...appState.settings.mouse.extraButtons },
});
const status = ref<MouseStatus>();
const busy = ref(false);
const error = ref('');
const statusError = ref('');
const editable = computed(
  () => desktopAvailable && status.value?.supported && appState.status?.storageReady,
);
const saved = ref(JSON.stringify(form.value));
const dirty = computed(() => JSON.stringify(form.value) !== saved.value);
const actions: { value: MouseAction; label: string }[] = [
  { value: 'default', label: '系统默认' },
  { value: 'back', label: '后退（⌘ [）' },
  { value: 'forward', label: '前进（⌘ ]）' },
  { value: 'mission-control', label: '调度中心与空间手势' },
  { value: 'show-desktop', label: '显示桌面（F11）' },
  { value: 'quick-look', label: '快速查看（空格）' },
  { value: 'none', label: '禁用此按键' },
];
const buttons = computed(() =>
  [
    ...new Set([
      3,
      4,
      5,
      6,
      7,
      ...Object.keys(form.value.extraButtons).map(Number),
      ...(status.value?.logitechButtons ?? []),
    ]),
  ].sort((a, b) => a - b),
);
const capturePending = ref(false);
const captureNotice = ref('');
let captureSequence = 0;
const captureStarting = ref(false);

function buttonAction(button: number): MouseAction {
  if (button === 3) return form.value.middleButton;
  if (button === 4) return form.value.backButton;
  if (button === 5) return form.value.forwardButton;
  return form.value.extraButtons[button] ?? 'default';
}

function changeAction(button: number, event: Event): void {
  if (!(event.target instanceof HTMLSelectElement)) return;
  const value = event.target.value;
  const action = actions.find((action) => action.value === value)?.value;
  if (!action) return;
  if (button === 3) form.value.middleButton = action;
  else if (button === 4) form.value.backButton = action;
  else if (button === 5) form.value.forwardButton = action;
  else form.value.extraButtons[button] = action;
}

async function captureButton(): Promise<void> {
  if (capturePending.value) return;
  error.value = '';
  captureNotice.value = '';
  captureSequence = status.value?.buttonSequence ?? 0;
  capturePending.value = true;
  captureStarting.value = true;
  try {
    await client.mouseCaptureButton();
    captureStarting.value = false;
    await refresh();
  } catch (reason) {
    capturePending.value = false;
    error.value = errorMessage(reason);
  } finally {
    captureStarting.value = false;
  }
}

async function cancelCapture(): Promise<void> {
  if (!capturePending.value) return;
  try {
    await client.mouseCaptureButton(false);
    capturePending.value = false;
    captureNotice.value = '已取消识别';
  } catch (reason) {
    error.value = errorMessage(reason);
  }
}

let timer: ReturnType<typeof setInterval> | undefined;
let disposed = false;
let refreshing = false;
let dpiRevision = 0;

function updateDpi(updated: MouseDpi): void {
  dpiRevision += 1;
  if (status.value)
    status.value.logitechDpi = status.value.logitechDpi.map((item) =>
      item.id === updated.id ? updated : item,
    );
}

async function refresh(): Promise<void> {
  if (!desktopAvailable || refreshing || disposed) return;
  refreshing = true;
  const revision = dpiRevision;
  try {
    const result = await client.mouseStatus();
    if (!disposed && revision === dpiRevision) {
      status.value = result;
      statusError.value = '';
      if (
        capturePending.value &&
        !captureStarting.value &&
        result.lastButton &&
        result.buttonSequence !== captureSequence
      ) {
        const button = result.lastButton;
        if (button >= 6) form.value.extraButtons[button] ??= 'default';
        captureNotice.value = button === 3 ? '已识别中键' : `已识别按键 ${button}`;
        capturePending.value = false;
      } else if (capturePending.value && !captureStarting.value && !result.capturing) {
        capturePending.value = false;
        captureNotice.value = '未识别到按键，请保持鼠标连接后重试';
      }
    }
  } catch (reason) {
    if (!disposed) statusError.value = errorMessage(reason);
  } finally {
    refreshing = false;
  }
}

let saveTimer: ReturnType<typeof setTimeout> | undefined;

async function save(): Promise<void> {
  if (!editable.value || busy.value) return;
  busy.value = true;
  error.value = '';
  try {
    while (dirty.value) {
      const mouse = { ...form.value, extraButtons: { ...form.value.extraButtons } };
      await updateMousePreferences(mouse);
      saved.value = JSON.stringify(mouse);
    }
  } catch (reason) {
    error.value = `设置未能应用：${errorMessage(reason)}`;
    if (disposed) notify(error.value, true);
  } finally {
    busy.value = false;
  }
}

watch(
  form,
  () => {
    if (saveTimer !== undefined) return;
    saveTimer = setTimeout(() => {
      saveTimer = undefined;
      void save();
    }, 80);
  },
  { deep: true, flush: 'sync' },
);

async function openAccessibility(): Promise<void> {
  error.value = '';
  try {
    await client.mouseAccessibilitySettings();
  } catch (reason) {
    error.value = errorMessage(reason);
  }
}

function reset(): void {
  form.value = {
    ...defaultSettings.mouse,
    extraButtons: { ...defaultSettings.mouse.extraButtons },
  };
}

async function openInputMonitoring(): Promise<void> {
  try {
    await client.mouseInputMonitoringSettings();
  } catch (reason) {
    error.value = errorMessage(reason);
  }
}

onMounted(() => {
  void refresh();
  if (desktopAvailable) timer = setInterval(() => void refresh(), 1000);
  window.addEventListener('focus', refresh);
});
onUnmounted(() => {
  if (capturePending.value) {
    void client
      .mouseCaptureButton(false)
      .catch((reason: unknown) => notify(errorMessage(reason), true));
  }
  clearTimeout(saveTimer);
  void save();
  disposed = true;
  clearInterval(timer);
  window.removeEventListener('focus', refresh);
});
</script>

<template>
  <section class="mouse-page mx-auto max-w-[900px]">
    <header
      class="page-heading mb-[25px] flex items-start justify-between gap-5 narrow:flex-wrap narrow:gap-[14px]"
    >
      <h1>鼠标增强</h1>
      <div class="mouse-heading-actions">
        <button v-if="error && dirty" class="button ghost" type="button" @click="save">重试</button>
        <button class="button" type="button" :disabled="!editable" @click="reset">
          <AppIcon name="reset" />恢复默认
        </button>
      </div>
    </header>
    <p v-if="!desktopAvailable" class="inline-notice" role="status">
      请在 macOS 桌面客户端中使用鼠标增强。
    </p>
    <p v-else-if="status && !status.supported" class="inline-notice" role="status">
      鼠标增强目前仅支持 macOS。
    </p>
    <p v-if="error || statusError || status?.error" class="inline-notice error" role="alert">
      {{ error || statusError || status?.error }}
    </p>
    <template v-if="!status || status.supported">
      <section class="settings-section mt-7 [&>h2]:mb-3" aria-label="鼠标增强开关与权限">
        <div
          class="settings-card mt-[13px] rounded-panel border border-solid border-border bg-surface px-5"
        >
          <label
            class="setting-row flex items-center justify-between gap-6 py-[17px] narrow:flex-wrap narrow:gap-3"
          >
            <span>
              <strong>开启鼠标增强</strong>
              <span class="setting-description">调整后自动生效，关闭主窗口后仍在后台运行</span>
            </span>
            <input v-model="form.enabled" class="switch" type="checkbox" :disabled="!editable" />
          </label>
          <div
            class="setting-row flex items-center justify-between gap-6 py-[17px] narrow:flex-wrap narrow:gap-3"
          >
            <div>
              <strong>辅助功能权限</strong>
              <p>
                {{
                  status?.accessibilityGranted
                    ? '已授权，开启后自动运行'
                    : '在系统设置中允许 MTool 控制鼠标事件，授权后自动连接'
                }}
              </p>
            </div>
            <button class="button" :disabled="!desktopAvailable" @click="openAccessibility">
              <AppIcon name="settings" />{{
                status?.accessibilityGranted ? '系统设置' : '前往授权'
              }}
            </button>
          </div>
        </div>
      </section>
      <section
        v-if="status?.logitechDpi.length"
        class="settings-section mt-7 [&>h2]:mb-3"
        aria-labelledby="mouse-dpi-heading"
      >
        <h2 id="mouse-dpi-heading">硬件 DPI</h2>
        <div
          class="settings-card mt-[13px] rounded-panel border border-solid border-border bg-surface px-5"
        >
          <LogitechDpiControl
            v-for="device in status.logitechDpi"
            :key="device.id"
            :device="device"
            :disabled="
              !desktopAvailable || !status.inputMonitoringGranted || !appState.status?.storageReady
            "
            @updated="updateDpi"
          />
        </div>
      </section>
      <form @submit.prevent>
        <fieldset :disabled="!editable">
          <section class="settings-section mt-7 [&>h2]:mb-3" aria-labelledby="mouse-scroll-heading">
            <h2 id="mouse-scroll-heading">滚轮</h2>
            <div
              class="settings-card mt-[13px] rounded-panel border border-solid border-border bg-surface px-5"
            >
              <label
                class="setting-row flex items-center justify-between gap-6 py-[17px] narrow:flex-wrap narrow:gap-3"
              >
                <span
                  ><strong>平滑滚动</strong
                  ><span class="setting-description">将滚轮刻度转为连续的像素滚动</span></span
                >
                <input v-model="form.smoothScrolling" class="switch" type="checkbox" />
              </label>
              <label
                class="setting-row flex items-center justify-between gap-6 py-[17px] narrow:flex-wrap narrow:gap-3"
              >
                <span
                  ><strong>滚动速度</strong
                  ><span class="setting-description">相对于系统滚动距离</span></span
                >
                <MouseSlider
                  v-model="form.scrollSpeed"
                  :min="0.25"
                  :max="4"
                  :step="0.05"
                  :value-text="`${form.scrollSpeed.toFixed(2)}×`"
                  label="滚动速度"
                />
              </label>
              <label
                class="setting-row flex items-center justify-between gap-6 py-[17px] narrow:flex-wrap narrow:gap-3"
              >
                <span
                  ><strong>平滑时长</strong
                  ><span class="setting-description">较短更跟手，较长更柔和</span></span
                >
                <MouseSlider
                  v-model="form.smoothDurationMs"
                  :min="80"
                  :max="400"
                  :step="10"
                  :value-text="`${form.smoothDurationMs} ms`"
                  label="平滑时长"
                  :disabled="!form.smoothScrolling"
                />
              </label>
              <label
                class="setting-row flex items-center justify-between gap-6 py-[17px] narrow:flex-wrap narrow:gap-3"
              >
                <span
                  ><strong>反转纵向滚动</strong
                  ><span class="setting-description">相对于当前系统方向反转</span></span
                >
                <input v-model="form.reverseVertical" class="switch" type="checkbox" />
              </label>
              <label
                class="setting-row flex items-center justify-between gap-6 py-[17px] narrow:flex-wrap narrow:gap-3"
              >
                <span><strong>反转横向滚动</strong></span>
                <input v-model="form.reverseHorizontal" class="switch" type="checkbox" />
              </label>
              <p class="setting-footnote">
                仅调整普通鼠标滚轮。触控板、Magic Mouse 和 ⌘ / ⌃ / ⌥＋滚轮不受影响。
              </p>
            </div>
          </section>
          <section
            class="settings-section mt-7 [&>h2]:mb-3"
            aria-labelledby="mouse-buttons-heading"
          >
            <div
              class="context-section-heading mb-3 flex items-center justify-between gap-4 [&>h2]:text-[15px] [&>h2]:font-medium"
            >
              <h2 id="mouse-buttons-heading">按键映射</h2>
              <button
                class="button small"
                type="button"
                :disabled="captureStarting"
                @click="capturePending ? cancelCapture() : captureButton()"
              >
                <AppIcon :name="capturePending ? 'x' : 'mouse'" />{{
                  capturePending ? '取消识别' : '识别按键'
                }}
              </button>
            </div>
            <p v-if="capturePending || captureNotice" class="setting-description" role="status">
              {{
                capturePending
                  ? '请在 15 秒内按下要设置的鼠标按键（左键和右键除外）'
                  : captureNotice
              }}
            </p>
            <p v-if="status?.logitechDevice" class="setting-description">
              {{ status.logitechDevice }}
            </p>
            <p v-if="status?.logitechError" class="inline-notice" role="status">
              {{ status.logitechError }}
            </p>
            <div
              v-if="status?.supported && !status.inputMonitoringGranted"
              class="setting-row flex items-center justify-between gap-6 py-[17px] narrow:flex-wrap narrow:gap-3"
            >
              <span class="setting-description"
                >罗技额外按键需要「输入监控」权限，授权后重新启动 MTool。</span
              >
              <button class="button" type="button" @click="openInputMonitoring">
                输入监控设置
              </button>
            </div>
            <div
              class="settings-card mt-[13px] rounded-panel border border-solid border-border bg-surface px-5"
            >
              <div
                v-for="button in buttons"
                :key="button"
                class="setting-row flex items-center justify-between gap-6 py-[17px] narrow:flex-wrap narrow:gap-3"
              >
                <strong :id="`mouse-button-${button}`">{{
                  button === 3 ? '中键' : `按键 ${button}`
                }}</strong>
                <span
                  class="select-control mouse-action-select relative inline-flex min-w-0 max-w-full align-middle text-foreground"
                >
                  <select
                    :value="buttonAction(button)"
                    :aria-labelledby="`mouse-button-${button}`"
                    @change="changeAction(button, $event)"
                  >
                    <option v-for="action in actions" :key="action.value" :value="action.value">
                      {{ action.label }}
                    </option>
                  </select>
                  <AppIcon class="select-chevron" name="chevron-down" :size="14" />
                </span>
              </div>
            </div>
          </section>
        </fieldset>
      </form>
    </template>
  </section>
</template>

<style scoped>
.mouse-heading-actions {
  display: flex;
  align-items: center;
  gap: 12px;
}
.mouse-action-select {
  width: 250px;
  flex-shrink: 0;
}
.mouse-action-select select {
  background: var(--background);
}
.mouse-action-select > svg {
  color: var(--muted);
}
.mouse-action-select select:focus-visible {
  outline: none;
  border-color: var(--accent);
  box-shadow: inset 0 0 0 1px var(--accent);
}
@media (max-width: 800px) {
  .mouse-action-select {
    width: 190px;
  }
}
</style>
