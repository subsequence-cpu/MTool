import { reactive } from 'vue';
import { listen } from '@tauri-apps/api/event';
import { client, desktopAvailable, errorMessage } from '../bindings/client';
import { catalog } from '../bindings/catalog';
import { defaultSettings } from '../bindings/types';
import type { AppStatus, Settings, ToolId } from '../bindings/types';

export const appState = reactive({
  tools: [...catalog],
  settings: structuredClone(defaultSettings),
  status: null as AppStatus | null,
  ready: false,
  initializationError: '',
  historyVersion: 0,
  toast: '',
  toastError: false,
});

let toastTimer: ReturnType<typeof setTimeout> | undefined;
let initializePromise: Promise<void> | undefined;
let preferencesQueue = Promise.resolve();

export function notify(message: string, error = false): void {
  appState.toast = message;
  appState.toastError = error;
  clearTimeout(toastTimer);
  toastTimer = setTimeout(
    () => {
      appState.toast = '';
    },
    error ? 9000 : 3500,
  );
}

export function initialize(): Promise<void> {
  initializePromise ??= (async () => {
    if (desktopAvailable) {
      try {
        const data = await client.bootstrap();
        appState.tools = data.tools;
        appState.settings = data.settings;
        appState.status = data.status;
        await listen('clipboard-changed', () => {
          appState.historyVersion += 1;
        });
        await listen('settings-changed', () => {
          void refresh().catch((e: unknown) => notify(errorMessage(e), true));
        });
        await listen('status-changed', () => {
          void refreshStatus().catch((e: unknown) => notify(errorMessage(e), true));
        });
        if (
          data.status.platform === 'macos' &&
          new URLSearchParams(window.location.search).get('window') === null
        ) {
          await listen<string>('context-menu-error', (event) => notify(event.payload, true));
          await client
            .contextMenuStatus()
            .then((status) => {
              if (status.error) notify(status.error, true);
            })
            .catch((error: unknown) => notify(errorMessage(error), true));
        }
      } catch (error) {
        appState.initializationError = errorMessage(error);
      }
    }
    appState.ready = true;
  })();
  return initializePromise;
}

async function refresh(): Promise<void> {
  const data = await client.bootstrap();
  appState.settings = data.settings;
  appState.status = data.status;
}

export async function refreshStatus(): Promise<void> {
  appState.status = await client.status();
}

export function updatePreferences(
  patch: Partial<Settings> | ((current: Settings) => Partial<Settings>),
): Promise<void> {
  const update = preferencesQueue.then(async () => {
    const settings = {
      ...appState.settings,
      ...(typeof patch === 'function' ? patch(appState.settings) : patch),
    };
    if (desktopAvailable && appState.status?.storageReady) {
      appState.settings = await client.saveSettings(settings);
    } else {
      appState.settings = settings;
    }
  });
  preferencesQueue = update.catch(() => {});
  return update;
}

export function rememberTool(id: ToolId): void {
  void updatePreferences((current) => ({
    recentTools: [id, ...current.recentTools.filter((item) => item !== id)].slice(0, 8),
  })).catch((error: unknown) => notify(errorMessage(error), true));
}

export async function toggleFavorite(id: ToolId): Promise<void> {
  try {
    await updatePreferences((current) => ({
      favorites: current.favorites.includes(id)
        ? current.favorites.filter((item) => item !== id)
        : [...current.favorites, id],
    }));
  } catch (error) {
    notify(errorMessage(error), true);
  }
}
