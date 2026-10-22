use std::{collections::BTreeMap, fmt};

use serde::{Deserialize, Serialize};
use ts_rs::TS;

pub const MAX_TEXT_BYTES: usize = 2 * 1024 * 1024;
pub const MAX_IMAGE_BYTES: usize = 10 * 1024 * 1024;
pub const MAX_IMAGE_PIXELS: u64 = 16_000_000;
pub const MAX_OUTPUT_BYTES: usize = 8 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct AppError {
    pub code: String,
    pub message: String,
}

impl AppError {
    pub fn new(code: &str, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
        }
    }

    pub fn invalid(message: impl Into<String>) -> Self {
        Self::new("invalid_input", message)
    }
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for AppError {}

pub type Result<T> = std::result::Result<T, AppError>;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ContextMenuStatus {
    pub supported: bool,
    pub bundled: bool,
    pub enabled: bool,
    pub vscode_installed: bool,
    pub accessibility_granted: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "kebab-case")]
pub enum MouseAction {
    Default,
    Back,
    Forward,
    MissionControl,
    ShowDesktop,
    QuickLook,
    None,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
pub struct MouseSettings {
    pub enabled: bool,
    pub smooth_scrolling: bool,
    pub reverse_vertical: bool,
    pub reverse_horizontal: bool,
    pub scroll_speed: f64,
    pub smooth_duration_ms: u32,
    pub middle_button: MouseAction,
    pub back_button: MouseAction,
    pub forward_button: MouseAction,
    pub extra_buttons: BTreeMap<u16, MouseAction>,
}

impl Default for MouseSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            smooth_scrolling: true,
            reverse_vertical: false,
            reverse_horizontal: false,
            scroll_speed: 1.0,
            smooth_duration_ms: 180,
            middle_button: MouseAction::Default,
            back_button: MouseAction::Back,
            forward_button: MouseAction::Forward,
            extra_buttons: BTreeMap::new(),
        }
    }
}

impl MouseSettings {
    pub fn button_action(&self, button: u16) -> MouseAction {
        match button {
            3 => self.middle_button,
            4 => self.back_button,
            5 => self.forward_button,
            _ => self
                .extra_buttons
                .get(&button)
                .copied()
                .unwrap_or(MouseAction::Default),
        }
    }

    pub fn validate(&self) -> Result<()> {
        if self
            .extra_buttons
            .keys()
            .any(|button| !(6..=32).contains(button))
        {
            return Err(AppError::invalid("额外鼠标按键编号需在 6–32 之间"));
        }
        if !self.scroll_speed.is_finite() || !(0.25..=4.0).contains(&self.scroll_speed) {
            return Err(AppError::invalid("滚动速度需在 0.25–4 倍之间"));
        }
        if !(80..=400).contains(&self.smooth_duration_ms) {
            return Err(AppError::invalid("平滑时长需在 80–400 毫秒之间"));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct MouseDpi {
    pub id: String,
    pub name: String,
    pub current: u16,
    pub values: Vec<u16>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct MouseStatus {
    pub supported: bool,
    pub accessibility_granted: bool,
    pub error: Option<String>,
    pub capturing: bool,
    pub last_button: Option<u16>,
    pub button_sequence: u32,
    pub logitech_device: Option<String>,
    pub logitech_buttons: Vec<u16>,
    pub logitech_dpi: Vec<MouseDpi>,
    pub logitech_error: Option<String>,
    pub input_monitoring_granted: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "kebab-case")]
pub enum ToolId {
    Json,
    Base64,
    Url,
    Unicode,
    Hex,
    Timestamp,
    Hash,
    Jwt,
    Uuid,
    Text,
    Diff,
    Regex,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ToolOperation {
    pub id: String,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ToolDefinition {
    pub id: ToolId,
    pub title: String,
    pub description: String,
    pub category: String,
    pub icon: String,
    pub keywords: Vec<String>,
    pub operations: Vec<ToolOperation>,
    pub sample: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ToolRequest {
    pub tool: ToolId,
    pub operation: String,
    pub input: String,
    #[serde(default)]
    pub secondary: String,
    #[serde(default)]
    pub options: BTreeMap<String, String>,
}

impl ToolRequest {
    pub fn option<'a>(&'a self, key: &str, default: &'a str) -> &'a str {
        self.options.get(key).map(String::as_str).unwrap_or(default)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ToolResponse {
    pub output: String,
    pub notice: Option<String>,
}

impl ToolResponse {
    pub fn text(output: impl Into<String>) -> Self {
        Self {
            output: output.into(),
            notice: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
pub enum ClipboardKind {
    Text,
    Image,
}

#[derive(Debug, Clone)]
pub enum ClipboardContent {
    Text(String),
    Image {
        png: Vec<u8>,
        width: u32,
        height: u32,
    },
}

impl ClipboardContent {
    pub fn bytes(&self) -> &[u8] {
        match self {
            Self::Text(text) => text.as_bytes(),
            Self::Image { png, .. } => png,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ClipboardItem {
    pub id: String,
    pub kind: ClipboardKind,
    pub preview: String,
    pub source_app: Option<String>,
    #[ts(type = "number")]
    pub created_at: i64,
    #[ts(type = "number")]
    pub updated_at: i64,
    pub pinned: bool,
    pub byte_size: u32,
    pub width: Option<u32>,
    pub height: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ClipboardDetail {
    pub item: ClipboardItem,
    pub text: Option<String>,
    pub image_data_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub mouse: MouseSettings,
    pub history_limit: u32,
    pub retention_days: u32,
    pub max_storage_mb: u32,
    pub excluded_apps: Vec<String>,
    pub capture_enabled: bool,
    pub launch_at_login: bool,
    pub clipboard_shortcut: String,
    pub launcher_shortcut: String,
    pub favorites: Vec<ToolId>,
    pub recent_tools: Vec<ToolId>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            mouse: MouseSettings::default(),
            history_limit: 1000,
            retention_days: 30,
            max_storage_mb: 512,
            excluded_apps: vec!["1password".into(), "bitwarden".into(), "keepass".into()],
            capture_enabled: true,
            launch_at_login: false,
            clipboard_shortcut: "CommandOrControl+Shift+C".into(),
            launcher_shortcut: "Alt+Space".into(),
            favorites: vec![ToolId::Json, ToolId::Timestamp],
            recent_tools: vec![],
        }
    }
}

impl Settings {
    pub fn validate(&self) -> Result<()> {
        self.mouse.validate()?;
        if !(50..=10_000).contains(&self.history_limit)
            || !(1..=365).contains(&self.retention_days)
            || !(16..=2048).contains(&self.max_storage_mb)
        {
            return Err(AppError::invalid("历史条数、保存天数或容量超出允许范围"));
        }
        if self.excluded_apps.len() > 100
            || self
                .excluded_apps
                .iter()
                .any(|s| s.is_empty() || s.len() > 200)
        {
            return Err(AppError::invalid(
                "排除应用最多 100 个，每个名称需为 1–200 字节",
            ));
        }
        if self.favorites.len() > 12 || self.recent_tools.len() > 8 {
            return Err(AppError::invalid("收藏或最近使用数量超出限制"));
        }
        if self.clipboard_shortcut.is_empty()
            || self.launcher_shortcut.is_empty()
            || self.clipboard_shortcut.len() > 100
            || self.launcher_shortcut.len() > 100
            || self
                .clipboard_shortcut
                .eq_ignore_ascii_case(&self.launcher_shortcut)
        {
            return Err(AppError::invalid("两个快捷键必须有效且互不相同"));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct AppStatus {
    pub platform: String,
    pub storage_ready: bool,
    pub storage_error: Option<String>,
    pub capture_enabled: bool,
    pub capture_error: Option<String>,
    pub shortcut_error: Option<String>,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct Bootstrap {
    pub tools: Vec<ToolDefinition>,
    pub settings: Settings,
    pub status: AppStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ClipboardPage {
    pub items: Vec<ClipboardItem>,
    pub total: u32,
    pub has_more: bool,
}
