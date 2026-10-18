<script setup lang="ts">
import { defineAsyncComponent, onMounted } from 'vue';
import { appState, initialize } from './store';
import AppLayout from '../layouts/AppLayout.vue';
import AppIcon from '../components/ui/AppIcon.vue';
import appIcon from '../assets/app-icon.svg';

const ClipboardPopup = defineAsyncComponent(
  () => import('../features/clipboard/ClipboardPopup.vue'),
);
const ClipboardPreview = defineAsyncComponent(
  () => import('../features/clipboard/ClipboardPreview.vue'),
);
const windowKind = new URLSearchParams(window.location.search).get('window');
const reload = (): void => window.location.reload();
onMounted(() => {
  if (windowKind !== 'clipboard-preview') void initialize();
});
</script>

<template>
  <ClipboardPreview v-if="windowKind === 'clipboard-preview'" />
  <div v-else-if="!appState.ready" class="boot-screen">
    <img class="brand-mark" :src="appIcon" alt="MTool" />
    <p>加载中…</p>
  </div>
  <div v-else-if="appState.initializationError" class="boot-screen">
    <h1>暂时无法启动</h1>
    <p>{{ appState.initializationError }}</p>
    <button class="button" @click="reload"><AppIcon name="reset" />重新加载</button>
  </div>
  <ClipboardPopup v-else-if="windowKind === 'clipboard'" />
  <AppLayout v-else><RouterView /></AppLayout>
  <div
    v-if="appState.toast"
    class="toast"
    :class="{ 'is-error': appState.toastError }"
    :role="appState.toastError ? 'alert' : 'status'"
  >
    {{ appState.toast }}
  </div>
</template>
