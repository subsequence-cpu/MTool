<script setup lang="ts">
import { onMounted, ref } from 'vue';
import AppIcon from './AppIcon.vue';

defineProps<{ title: string; description: string; confirmLabel?: string; destructive?: boolean }>();
const emit = defineEmits<{ confirm: []; cancel: [] }>();
const dialog = ref<HTMLDialogElement>();
const keyboardNavigation = ref(false);
onMounted(() => dialog.value?.showModal());
</script>

<template>
  <dialog
    ref="dialog"
    class="confirm-dialog"
    :class="{ 'keyboard-navigation': keyboardNavigation }"
    @keydown.capture="keyboardNavigation = true"
    @pointerdown.capture="keyboardNavigation = false"
    @cancel.prevent="emit('cancel')"
    @click="
      (event) => {
        if (event.target === dialog) emit('cancel');
      }
    "
  >
    <div class="mb-3 flex items-center justify-between gap-3 [&>h2]:text-[17px]">
      <h2>{{ title }}</h2>
      <button class="icon-button" aria-label="关闭" @click="emit('cancel')">
        <AppIcon name="x" />
      </button>
    </div>
    <p>{{ description }}</p>
    <div class="mt-6 flex justify-end gap-[9px]">
      <button class="button" autofocus @click="emit('cancel')"><AppIcon name="x" />取消</button
      ><button class="button" :class="destructive ? 'danger' : 'primary'" @click="emit('confirm')">
        <AppIcon :name="destructive ? 'trash' : 'check'" />{{ confirmLabel ?? '确认' }}
      </button>
    </div>
  </dialog>
</template>
