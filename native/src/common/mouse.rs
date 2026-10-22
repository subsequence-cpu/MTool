#[cfg(not(target_os = "macos"))]
use mtool_core::MouseSettings;
use mtool_core::{MouseDpi, MouseStatus, Result};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager, State, WebviewWindow};

#[cfg(target_os = "macos")]
pub(crate) use crate::macos::mouse::MouseService;

#[cfg(not(target_os = "macos"))]
pub(crate) struct MouseService;

#[cfg(not(target_os = "macos"))]
impl MouseService {
    pub fn new(_settings: MouseSettings) -> Self {
        Self
    }

    pub fn configure(&self, _settings: MouseSettings) {}

    pub fn status(&self) -> MouseStatus {
        MouseStatus {
            supported: false,
            accessibility_granted: false,
            error: None,
            capturing: false,
            last_button: None,
            button_sequence: 0,
            logitech_device: None,
            logitech_buttons: Vec::new(),
            logitech_dpi: Vec::new(),
            logitech_error: None,
            input_monitoring_granted: false,
        }
    }

    pub fn shutdown(&self) {}

    pub fn set_dpi(&self, _id: String, _value: u16) -> Result<MouseDpi> {
        Err(mtool_core::AppError::new(
            "unsupported",
            "硬件 DPI 调节目前仅支持 macOS 罗技鼠标",
        ))
    }

    pub fn capture(&self, _active: bool) -> Result<()> {
        Err(mtool_core::AppError::new(
            "unsupported",
            "鼠标增强目前仅支持 macOS",
        ))
    }
}

#[tauri::command]
pub async fn mouse_set_dpi(
    app: AppHandle,
    window: WebviewWindow,
    id: String,
    value: u16,
) -> Result<MouseDpi> {
    if window.label() != "main" {
        return Err(mtool_core::AppError::new(
            "forbidden",
            "请在主窗口调整硬件 DPI",
        ));
    }
    if id.len() > 64 || value == 0 {
        return Err(mtool_core::AppError::invalid("DPI 设置无效"));
    }
    tauri::async_runtime::spawn_blocking(move || app.state::<MouseService>().set_dpi(id, value))
        .await
        .map_err(|_| mtool_core::AppError::new("task_failed", "DPI 设置失败"))?
}

#[tauri::command]
pub fn mouse_capture_button(
    window: WebviewWindow,
    service: State<'_, MouseService>,
    active: bool,
) -> Result<()> {
    if window.label() != "main" {
        return Err(mtool_core::AppError::new(
            "forbidden",
            "请在主窗口识别鼠标按键",
        ));
    }
    service.capture(active)
}

#[tauri::command]
pub fn mouse_input_monitoring_settings(window: WebviewWindow) -> Result<()> {
    if window.label() != "main" {
        return Err(mtool_core::AppError::new(
            "forbidden",
            "请在主窗口管理鼠标权限",
        ));
    }
    #[cfg(target_os = "macos")]
    return crate::macos::mouse::open_input_monitoring_settings();
    #[cfg(not(target_os = "macos"))]
    Err(mtool_core::AppError::new(
        "unsupported",
        "鼠标增强目前仅支持 macOS",
    ))
}

#[tauri::command]
pub fn mouse_status(service: State<'_, MouseService>) -> MouseStatus {
    service.status()
}

#[tauri::command]
pub async fn mouse_update_settings(
    app: AppHandle,
    window: WebviewWindow,
    state: State<'_, Arc<super::state::AppState>>,
    mouse: mtool_core::MouseSettings,
) -> Result<mtool_core::Settings> {
    if window.label() != "main" {
        return Err(mtool_core::AppError::new(
            "forbidden",
            "请在主窗口修改鼠标设置",
        ));
    }
    mouse.validate()?;
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let settings = {
            let mut persistence = super::state::lock(&state.persistence)?;
            let mut settings = persistence.settings.clone();
            if settings.mouse == mouse {
                return Ok(settings);
            }
            settings.mouse = mouse;
            persistence
                .store
                .as_mut()
                .ok_or_else(super::state::unavailable)?
                .save_settings(&settings, chrono::Utc::now().timestamp_millis())?;
            persistence.settings = settings.clone();
            app.state::<MouseService>()
                .configure(settings.mouse.clone());
            settings
        };
        let _ = app.emit("settings-changed", ());
        Ok(settings)
    })
    .await
    .map_err(|_| mtool_core::AppError::new("task_failed", "鼠标设置更新失败"))?
}

#[tauri::command]
pub fn mouse_accessibility_settings(window: WebviewWindow) -> Result<()> {
    if window.label() != "main" {
        return Err(mtool_core::AppError::new(
            "forbidden",
            "请在主窗口管理鼠标权限",
        ));
    }
    #[cfg(target_os = "macos")]
    return crate::macos::finder::open_accessibility_settings();
    #[cfg(not(target_os = "macos"))]
    Err(mtool_core::AppError::new(
        "unsupported",
        "鼠标增强目前仅支持 macOS",
    ))
}

#[cfg(target_os = "macos")]
pub(crate) mod motion;

#[cfg(target_os = "macos")]
pub(crate) mod gesture;
