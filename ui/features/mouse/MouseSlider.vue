<script setup lang="ts">
import { computed } from 'vue';
const props = defineProps<{
  modelValue: number;
  min: number;
  max: number;
  step: number;
  valueText: string;
  label: string;
  disabled?: boolean;
}>();
const emit = defineEmits<{ 'update:modelValue': [value: number] }>();
const progress = computed(
  () =>
    `${Math.min(100, Math.max(0, ((props.modelValue - props.min) / Math.max(props.max - props.min, Number.EPSILON)) * 100))}%`,
);
function update(event: Event): void {
  if (event.target instanceof HTMLInputElement)
    emit('update:modelValue', event.target.valueAsNumber);
}
</script>
<template>
  <span class="mouse-slider">
    <input
      type="range"
      :value="modelValue"
      :min="min"
      :max="max"
      :step="step"
      :disabled="disabled"
      :aria-label="label"
      :aria-valuetext="valueText"
      :style="{ '--range-progress': progress }"
      @input="update"
    />
    <output>{{ valueText }}</output>
  </span>
</template>
<style scoped>
.mouse-slider {
  display: flex;
  align-items: center;
  gap: 14px;
  width: 250px;
  flex-shrink: 0;
}
.mouse-slider input {
  appearance: none;
  min-width: 0;
  width: 100%;
  height: 24px;
  margin: 0;
  padding: 0;
  border: 0;
  border-radius: 0;
  background: transparent;
  cursor: pointer;
}
.mouse-slider input::-webkit-slider-runnable-track {
  height: 4px;
  border-radius: 999px;
  background: linear-gradient(
    to right,
    var(--accent) var(--range-progress),
    var(--selection) var(--range-progress)
  );
}
.mouse-slider input::-webkit-slider-thumb {
  appearance: none;
  width: 14px;
  height: 14px;
  margin-top: -5px;
  border: 0;
  border-radius: 50%;
  background: var(--accent);
  box-shadow: 0 1px 4px rgb(0 0 0 / 18%);
}
.mouse-slider input::-moz-range-track {
  height: 4px;
  border-radius: 999px;
  background: linear-gradient(
    to right,
    var(--accent) var(--range-progress),
    var(--selection) var(--range-progress)
  );
}
.mouse-slider input::-moz-range-thumb {
  width: 14px;
  height: 14px;
  border: 0;
  border-radius: 50%;
  background: var(--accent);
  box-shadow: 0 1px 4px rgb(0 0 0 / 18%);
}
.mouse-slider input:disabled {
  opacity: 0.45;
  cursor: not-allowed;
}
.mouse-slider output {
  min-width: 62px;
  text-align: right;
  font-variant-numeric: tabular-nums;
  font-size: 12px;
  color: var(--muted);
}
@media (max-width: 800px) {
  .mouse-slider {
    width: 190px;
  }
}
</style>
