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
let previous = '';

async function edit(): Promise<void> {
  previous = model.value;
  editing.value = true;
  await nextTick();
  input.value?.focus({ preventScroll: true });
  input.value?.select();
}

async function finish(cancel = false): Promise<void> {
  if (cancel) model.value = previous;
  editing.value = false;
  await nextTick();
  button.value?.focus({ preventScroll: true });
}
</script>

<template>
  <span class="shortcut-field">
    <input
      v-if="editing"
      ref="input"
      v-model="model"
      class="shortcut-input"
      :aria-label="label"
      maxlength="100"
      autocomplete="off"
      spellcheck="false"
      title="使用 + 连接按键，例如 Alt+Space；Esc 取消编辑"
      @blur="editing = false"
      @keydown.enter.prevent="finish()"
      @keydown.esc.prevent="finish(true)"
    />
    <button
      v-else
      ref="button"
      type="button"
      class="shortcut-display"
      :aria-label="`${label}：${keys.join(' ')}，点击编辑`"
      title="编辑快捷键"
      @click="edit"
    >
      <span class="shortcut-keycaps" aria-hidden="true">
        <kbd v-for="(key, index) in keys" :key="index">{{ key }}</kbd>
        <span v-if="!keys.length" class="muted">设置快捷键</span>
      </span>
      <AppIcon name="pencil" :size="14" />
    </button>
  </span>
</template>
