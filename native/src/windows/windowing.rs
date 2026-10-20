use crate::common::{state::AppState, windowing::window};
use mtool_core::{AppError, Result};
use std::sync::{atomic::Ordering, Arc};
use tauri::{AppHandle, Manager, PhysicalPosition, PhysicalSize};

pub(crate) fn activate(_app: &AppHandle) -> Result<()> {
    Ok(())
}

pub(crate) fn show_clipboard(app: &AppHandle) -> Result<()> {
    let popup = window(app, "clipboard")?;
    let state = app.state::<Arc<AppState>>();
    if !state.clipboard_positioned.load(Ordering::Relaxed) {
        popup
            .center()
            .map_err(|_| AppError::new("window_unavailable", "无法定位剪贴板浮窗"))?;
        state.clipboard_positioned.store(true, Ordering::Relaxed);
    }
    popup
        .show()
        .and_then(|_| popup.unminimize())
        .and_then(|_| popup.set_focus())
        .map_err(|_| AppError::new("window_unavailable", "无法打开剪贴板浮窗"))
}

pub(crate) fn hide_clipboard(app: &AppHandle) -> Result<()> {
    window(app, "clipboard")?
        .hide()
        .map_err(|_| AppError::new("window_unavailable", "无法关闭剪贴板浮窗"))
}

pub(crate) fn hide_preview(app: &AppHandle) -> Result<()> {
    window(app, "clipboard-preview")?
        .hide()
        .map_err(|_| AppError::new("window_unavailable", "无法关闭图片预览"))
}

pub(crate) fn clipboard_visible(app: &AppHandle) -> Result<bool> {
    window(app, "clipboard")?
        .is_visible()
        .map_err(|_| AppError::new("window_unavailable", "无法检查浮窗状态"))
}

pub(crate) fn clipboard_focused(app: &AppHandle) -> Result<bool> {
    window(app, "clipboard")?
        .is_focused()
        .map_err(|_| AppError::new("window_unavailable", "无法检查浮窗焦点"))
}

pub(crate) fn drag_clipboard(app: &AppHandle) -> Result<()> {
    window(app, "clipboard")?
        .start_dragging()
        .map_err(|_| AppError::new("window_unavailable", "无法移动浮窗"))
}

pub(crate) fn position_preview(app: &AppHandle, width: u32, height: u32) -> Result<bool> {
    let popup = window(app, "clipboard")?;
    if !popup.is_visible().unwrap_or(false) {
        return Ok(false);
    }
    let place = || -> tauri::Result<bool> {
        let Some(monitor) = popup.current_monitor()? else {
            return Ok(false);
        };
        let area = monitor.work_area();
        let scale = popup.scale_factor()?;
        let gap = (10.0 * scale).round() as i32;
        let position = popup.outer_position()?;
        let size = popup.outer_size()?;
        let right = position.x + size.width as i32 + gap;
        let right_space = (area.position.x + area.size.width as i32 - gap - right).max(0);
        let left_space = (position.x - area.position.x - 2 * gap).max(0);
        let desired = (320.0 * scale).round() as i32;
        let use_right = right_space >= desired || right_space >= left_space;
        let max_width = desired.min(if use_right { right_space } else { left_space });
        let max_height = size
            .height
            .min(area.size.height.saturating_sub((2 * gap) as u32));
        if max_width < (48.0 * scale) as i32 || max_height == 0 {
            return Ok(false);
        }
        let ratio = (f64::from(max_width) / f64::from(width))
            .min(f64::from(max_height) / f64::from(height))
            .min(1.0);
        let image_size = PhysicalSize::new(
            (f64::from(width) * ratio).round().max(1.0) as u32,
            (f64::from(height) * ratio).round().max(1.0) as u32,
        );
        let top = area.position.y + gap;
        let y = (position.y + (i64::from(size.height) - i64::from(image_size.height)) as i32 / 2)
            .clamp(
                top,
                top + area
                    .size
                    .height
                    .saturating_sub(image_size.height + 2 * gap as u32)
                    as i32,
            );
        let x = if use_right {
            right
        } else {
            position.x - gap - image_size.width as i32
        };
        let preview = app
            .get_webview_window("clipboard-preview")
            .ok_or(tauri::Error::WindowNotFound)?;
        preview.set_size(image_size)?;
        preview.set_position(PhysicalPosition::new(x, y))?;
        Ok(true)
    };
    place().map_err(|_| AppError::new("window_unavailable", "无法定位图片预览"))
}

pub(crate) fn show_preview(app: &AppHandle) -> Result<()> {
    let preview = window(app, "clipboard-preview")?;
    if !preview.is_visible().unwrap_or(false) {
        preview
            .show()
            .map_err(|_| AppError::new("window_unavailable", "无法打开图片预览"))?;
    }
    Ok(())
}
