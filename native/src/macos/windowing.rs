use super::panels;
use mtool_core::{AppError, Result};
use tauri::AppHandle;

pub(crate) fn activate(app: &AppHandle) -> Result<()> {
    app.show()
        .map_err(|_| AppError::new("window_unavailable", "无法激活 MTool"))
}

pub(crate) fn show_clipboard(_app: &AppHandle) -> Result<()> {
    panels::show_clipboard()
}

pub(crate) fn hide_clipboard(_app: &AppHandle) -> Result<()> {
    panels::hide_clipboard()
}

pub(crate) fn hide_preview(_app: &AppHandle) -> Result<()> {
    panels::hide_preview()
}

pub(crate) fn clipboard_visible(_app: &AppHandle) -> Result<bool> {
    panels::is_visible()
}

pub(crate) fn clipboard_focused(_app: &AppHandle) -> Result<bool> {
    panels::is_focused()
}

pub(crate) fn drag_clipboard(_app: &AppHandle) -> Result<()> {
    panels::drag_clipboard()
}

pub(crate) fn position_preview(_app: &AppHandle, width: u32, height: u32) -> Result<bool> {
    panels::position_preview(width, height)
}

pub(crate) fn show_preview(_app: &AppHandle) -> Result<()> {
    if !panels::preview_visible()? {
        panels::show_preview()?;
    }
    Ok(())
}
