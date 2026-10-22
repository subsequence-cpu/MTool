<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watch } from 'vue';
import { notify } from '../../app/store';
import { client, errorMessage } from '../../bindings/client';
import type { MouseDpi } from '../../bindings/types';
import MouseSlider from './MouseSlider.vue';

const props = defineProps<{ device: MouseDpi; disabled: boolean }>();
const emit = defineEmits<{ updated: [device: MouseDpi] }>();
const desired = ref(props.device.current);
const confirmed = ref(props.device.current);
const busy = ref(false);
const error = ref('');
const index = computed(() => Math.max(0, props.device.values.indexOf(desired.value)));
let timer: ReturnType<typeof setTimeout> | undefined;
let disposed = false;

watch(
  () => props.device.current,
  (value) => {
    if (!busy.value && timer === undefined) desired.value = confirmed.value = value;
  },
);

function schedule(index: number): void {
  const value = props.device.values[index];
  if (value === undefined || props.disabled) return;
  desired.value = value;
  error.value = '';
  clearTimeout(timer);
  timer = setTimeout(() => {
    timer = undefined;
    void apply();
  }, 120);
}

async function apply(): Promise<void> {
  if (busy.value || props.disabled) return;
  busy.value = true;
  try {
    while (desired.value !== confirmed.value) {
      const value = desired.value;
      const result = await client.mouseSetDpi(props.device.id, value);
      confirmed.value = result.current;
      emit('updated', result);
      if (desired.value === value) desired.value = result.current;
    }
  } catch (reason) {
    desired.value = confirmed.value;
    error.value = errorMessage(reason);
    if (disposed) notify(error.value, true);
  } finally {
    busy.value = false;
  }
}

onBeforeUnmount(() => {
  clearTimeout(timer);
  void apply();
  disposed = true;
});
</script>

<template>
  <div>
    <label
      class="setting-row flex items-center justify-between gap-6 py-[17px] narrow:flex-wrap narrow:gap-3"
    >
      <strong>{{ device.name }}</strong>
      <MouseSlider
        :model-value="index"
        :min="0"
        :max="device.values.length - 1"
        :step="1"
        :value-text="`${desired} DPI`"
        :label="`${device.name} 硬件 DPI`"
        :disabled="disabled"
        @update:model-value="schedule"
      />
    </label>
    <p v-if="error" class="inline-notice error" role="alert">{{ error }}</p>
  </div>
</template>
