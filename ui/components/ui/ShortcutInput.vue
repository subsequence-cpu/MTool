<script setup lang="ts">
import { computed, nextTick, ref } from 'vue';
import AppIcon from './AppIcon.vue';
import { shortcutKeys } from './shortcut';

const props = defineProps<{ label: string; isMac: boolean }>();
const model = defineModel<string>({ required: true });
const editing = ref(false);
const input = ref<HTMLInputElement>();
const button = ref<HTMLButtonElement>();
const keys = computed(() => shortcutKeys(model.value, props.isMac));
const draft = ref('');

async function edit(): Promise<void> {
  draft.value = model.value;
  editing.value = true;
  await nextTick();
  input.value?.focus({ preventScroll: true });
  input.value?.select();
}

async function finish(cancel = false, restoreFocus = true): Promise<void> {
  if (!editing.value) return;
  if (!cancel) model.value = draft.value.trim();
  editing.value = false;
  await nextTick();
  if (restoreFocus) button.value?.focus({ preventScroll: true });
}
</script>

<template>
  <span class="shortcut-field">
    <input
      v-if="editing"
      ref="input"
      v-model="draft"
      class="shortcut-input"
      :aria-label="label"
      maxlength="100"
      autocomplete="off"
      spellcheck="false"
      title="使用 + 连接按键，例如 Alt+Space；Esc 取消编辑"
      @blur="finish(false, false)"
      @keydown.stop
      @keydown.enter.prevent="finish()"
      @keydown.esc.prevent="finish(true)"
    />
    <span v-else class="shortcut-display" :aria-label="`${label}：${keys.join(' ')}`">
      <span
        class="shortcut-keycaps inline-flex flex-wrap items-center gap-[5px]"
        aria-hidden="true"
      >
        <kbd v-for="(key, index) in keys" :key="index">{{ key }}</kbd>
        <span v-if="!keys.length" class="text-[12px] text-muted">设置快捷键</span>
      </span>
      <button
        ref="button"
        type="button"
        class="inline-flex h-6 w-6 shrink-0 items-center justify-center rounded border-0 border-none bg-transparent p-0 text-muted focus-visible:outline-offset-1 enabled:hover:bg-selection enabled:hover:text-foreground"
        :aria-label="`修改${label}：${keys.join(' ')}`"
        title="修改快捷键"
        @click="edit"
      >
        <AppIcon name="pencil" :size="14" />
      </button>
    </span>
  </span>
</template>
