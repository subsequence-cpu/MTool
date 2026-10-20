use crate::{common::state::lock, platform};
use mtool_core::{AppError, ContextMenuStatus, Result};
use std::sync::Mutex;
use tauri::{State, WebviewWindow};

#[derive(Default)]
pub struct ContextMenuState {
    pub(crate) error: Mutex<Option<AppError>>,
    #[cfg(target_os = "macos")]
    pub(crate) platform: platform::context_menu::State,
}

fn main_only(window: &WebviewWindow) -> Result<()> {
    if window.label() != "main" {
        return Err(AppError::new("forbidden", "请在主窗口操作右键菜单"));
    }
    Ok(())
}

#[tauri::command]
pub fn context_menu_status(
    window: WebviewWindow,
    state: State<'_, ContextMenuState>,
) -> Result<ContextMenuStatus> {
    main_only(&window)?;
    let mut status = platform::context_menu::status();
    let mut error = lock(&state.error)?;
    if status.accessibility_granted
        && error
            .as_ref()
            .is_some_and(|error| error.code == "accessibility_required")
    {
        *error = None;
    }
    status.error = error.as_ref().map(|error| error.message.clone());
    Ok(status)
}

#[tauri::command]
pub fn context_menu_settings(window: WebviewWindow) -> Result<()> {
    main_only(&window)?;
    platform::context_menu::settings()
}

#[tauri::command]
pub fn context_menu_accessibility_settings(window: WebviewWindow) -> Result<()> {
    main_only(&window)?;
    platform::context_menu::accessibility_settings()
}
