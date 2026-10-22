<script setup lang="ts">
import AppIcon from './AppIcon.vue';

defineOptions({ inheritAttrs: false });
defineProps<{ icon?: string; disabled?: boolean }>();
const model = defineModel<string>({ required: true });
</script>

<template>
  <span
    class="select-control relative inline-flex min-w-0 max-w-full align-middle text-foreground"
    :class="{ 'has-icon': icon, 'is-disabled': disabled }"
  >
    <AppIcon v-if="icon" class="select-icon" :name="icon" :size="16" />
    <select v-model="model" v-bind="$attrs" :disabled="disabled">
      <slot />
    </select>
    <AppIcon class="select-chevron" name="chevron-down" :size="16" />
  </span>
</template>
