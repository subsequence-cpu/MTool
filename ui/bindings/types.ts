export type AppError = { code: string, message: string, };

export type ContextMenuStatus = { supported: boolean, bundled: boolean, enabled: boolean, vscodeInstalled: boolean, accessibilityGranted: boolean, error: string | null, };

export type MouseAction = "default" | "back" | "forward" | "mission-control" | "show-desktop" | "quick-look" | "none";

export type MouseSettings = { enabled: boolean, smoothScrolling: boolean, reverseVertical: boolean, reverseHorizontal: boolean, scrollSpeed: number, smoothDurationMs: number, middleButton: MouseAction, backButton: MouseAction, forwardButton: MouseAction, extraButtons: { [key in number]?: MouseAction }, };

export type MouseDpi = { id: string, name: string, current: number, values: Array<number>, };

export type MouseStatus = { supported: boolean, accessibilityGranted: boolean, error: string | null, capturing: boolean, lastButton: number | null, buttonSequence: number, logitechDevice: string | null, logitechButtons: Array<number>, logitechDpi: Array<MouseDpi>, logitechError: string | null, inputMonitoringGranted: boolean, };

export type ToolId = "json" | "base64" | "url" | "unicode" | "hex" | "timestamp" | "hash" | "jwt" | "uuid" | "text" | "diff" | "regex";

export type ToolOperation = { id: string, label: string, };

export type ToolDefinition = { id: ToolId, title: string, description: string, category: string, icon: string, keywords: Array<string>, operations: Array<ToolOperation>, sample: string, };

export type ToolRequest = { tool: ToolId, operation: string, input: string, secondary: string, options: { [key in string]?: string }, };

export type ToolResponse = { output: string, notice: string | null, };

export type ClipboardKind = "text" | "image";

export type ClipboardItem = { id: string, kind: ClipboardKind, preview: string, sourceApp: string | null, createdAt: number, updatedAt: number, pinned: boolean, byteSize: number, width: number | null, height: number | null, };

export type ClipboardDetail = { item: ClipboardItem, text: string | null, imageDataUrl: string | null, };

export type Settings = { mouse: MouseSettings, historyLimit: number, retentionDays: number, maxStorageMb: number, excludedApps: Array<string>, captureEnabled: boolean, launchAtLogin: boolean, clipboardShortcut: string, launcherShortcut: string, favorites: Array<ToolId>, recentTools: Array<ToolId>, };

export type AppStatus = { platform: string, storageReady: boolean, storageError: string | null, captureEnabled: boolean, captureError: string | null, shortcutError: string | null, version: string, };

export type Bootstrap = { tools: Array<ToolDefinition>, settings: Settings, status: AppStatus, };

export type ClipboardPage = { items: Array<ClipboardItem>, total: number, hasMore: boolean, };

export const defaultSettings: Settings = {
  "mouse": {
    "enabled": false,
    "smoothScrolling": true,
    "reverseVertical": false,
    "reverseHorizontal": false,
    "scrollSpeed": 1.0,
    "smoothDurationMs": 180,
    "middleButton": "default",
    "backButton": "back",
    "forwardButton": "forward",
    "extraButtons": {}
  },
  "historyLimit": 1000,
  "retentionDays": 30,
  "maxStorageMb": 512,
  "excludedApps": [
    "1password",
    "bitwarden",
    "keepass"
  ],
  "captureEnabled": true,
  "launchAtLogin": false,
  "clipboardShortcut": "CommandOrControl+Shift+C",
  "launcherShortcut": "Alt+Space",
  "favorites": [
    "json",
    "timestamp"
  ],
  "recentTools": []
};
