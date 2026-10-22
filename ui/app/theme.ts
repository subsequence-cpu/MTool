import { computed, watch } from 'vue';
import { appState } from './store';
import type { AppTheme } from '../bindings/types';
import darkIcon from '../assets/app-icon.svg';
import lightIcon from '../assets/app-icon-light.svg';

const storageKey = 'mtool-theme';
export const currentTheme = computed(() => appState.settings.theme);
export const themeName = computed(() =>
  currentTheme.value === 'github-light' ? 'GitHub Light' : 'Dracula',
);
export const themeIcon = computed(() =>
  currentTheme.value === 'github-light' ? lightIcon : darkIcon,
);

function validTheme(value: string | null): value is AppTheme {
  return value === 'dracula' || value === 'github-light';
}

export function initializeTheme(): void {
  try {
    const cached = localStorage.getItem(storageKey);
    if (validTheme(cached)) appState.settings.theme = cached;
  } catch {}
  watch(
    currentTheme,
    (theme) => {
      document.documentElement.dataset.theme = theme;
      const favicon = document.querySelector<HTMLLinkElement>('link[rel="icon"]');
      if (favicon) favicon.href = themeIcon.value;
      try {
        localStorage.setItem(storageKey, theme);
      } catch {}
    },
    { immediate: true, flush: 'sync' },
  );
  window.addEventListener('storage', (event) => {
    if (event.key === storageKey && validTheme(event.newValue))
      appState.settings = { ...appState.settings, theme: event.newValue };
  });
}
