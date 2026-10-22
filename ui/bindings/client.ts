import { invoke, isTauri } from '@tauri-apps/api/core';
import type {
  AppStatus,
  Bootstrap,
  ClipboardDetail,
  ClipboardPage,
  ContextMenuStatus,
  MouseStatus,
  MouseDpi,
  MouseSettings,
  Settings,
  ToolRequest,
  ToolResponse,
} from './types';

export const desktopAvailable = typeof window !== 'undefined' && isTauri();

function native<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  if (!desktopAvailable) {
    return Promise.reject(new Error('此功能需要桌面客户端。请使用 pnpm desktop 启动。'));
  }
  return invoke<T>(command, args);
}

export const client = {
  bootstrap: (): Promise<Bootstrap> => native('bootstrap'),
  status: (): Promise<AppStatus> => native('get_status'),
  runTool: (request: ToolRequest): Promise<ToolResponse> => native('run_tool', { request }),
  saveSettings: (settings: Settings): Promise<Settings> => native('save_settings', { settings }),
  history: (query: string, pinnedOnly: boolean, offset = 0): Promise<ClipboardPage> =>
    native('list_history', { query, pinnedOnly, offset }),
  detail: (id: string): Promise<ClipboardDetail> => native('history_detail', { id }),
  thumbnail: (id: string): Promise<string> => native('history_thumbnail', { id }),
  pin: (id: string, pinned: boolean): Promise<void> => native('pin_history', { id, pinned }),
  delete: (id: string): Promise<void> => native('delete_history', { id }),
  clear: (includePinned: boolean): Promise<void> => native('clear_history', { includePinned }),
  useHistory: (id: string): Promise<void> => native('use_history', { id }),
  copyText: (text: string): Promise<void> => native('copy_output', { text }),
  openClipboard: (): Promise<void> => native('open_clipboard'),
  closeClipboard: (): Promise<void> => native('close_clipboard'),
  clipboardVisible: (): Promise<boolean> => native('clipboard_visible'),
  dragClipboard: (): Promise<void> => native('drag_clipboard'),
  setClipboardPreview: (id: string | null): Promise<void> =>
    native('set_clipboard_preview', { id }),
  clipboardPreview: (): Promise<ClipboardDetail | null> => native('clipboard_preview'),
  showClipboardPreview: (id: string): Promise<void> => native('show_clipboard_preview', { id }),
  contextMenuStatus: (): Promise<ContextMenuStatus> => native('context_menu_status'),
  contextMenuSettings: (): Promise<void> => native('context_menu_settings'),
  contextMenuAccessibilitySettings: (): Promise<void> =>
    native('context_menu_accessibility_settings'),
  mouseStatus: (): Promise<MouseStatus> => native('mouse_status'),
  mouseSetDpi: (id: string, value: number): Promise<MouseDpi> =>
    native('mouse_set_dpi', { id, value }),
  mouseUpdateSettings: (mouse: MouseSettings): Promise<Settings> =>
    native('mouse_update_settings', { mouse }),
  mouseCaptureButton: (active = true): Promise<void> => native('mouse_capture_button', { active }),
  mouseInputMonitoringSettings: (): Promise<void> => native('mouse_input_monitoring_settings'),
  mouseAccessibilitySettings: (): Promise<void> => native('mouse_accessibility_settings'),
};

export function errorMessage(error: unknown): string {
  if (error instanceof Error) return error.message;
  if (
    typeof error === 'object' &&
    error !== null &&
    'message' in error &&
    typeof error.message === 'string'
  )
    return error.message;
  return '操作失败，请重试。';
}
