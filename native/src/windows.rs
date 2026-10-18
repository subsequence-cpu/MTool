use crate::state::{lock, AppState};
use mtool_core::{AppError, Result};
use std::sync::{atomic::Ordering, Arc};
use tauri::{AppHandle, Emitter, Manager, WebviewWindow};
#[cfg(not(target_os = "macos"))]
use tauri::{PhysicalPosition, PhysicalSize};

fn window(app: &AppHandle, label: &str) -> Result<WebviewWindow> {
    app.get_webview_window(label)
        .ok_or_else(|| AppError::new("window_unavailable", "窗口不可用"))
}

pub fn show_main(app: &AppHandle) -> Result<()> {
    #[cfg(target_os = "macos")]
    app.show()
        .map_err(|_| AppError::new("window_unavailable", "无法激活 MTool"))?;
    let main = window(app, "main")?;
    main.show()
        .and_then(|_| main.unminimize())
        .and_then(|_| main.set_focus())
        .map_err(|_| AppError::new("window_unavailable", "无法打开主窗口"))
}

pub fn show_clipboard(app: &AppHandle) -> Result<()> {
    let popup = window(app, "clipboard")?;
    let state = app.state::<Arc<AppState>>();
    state
        .clipboard_focused
        .store(clipboard_focused(app).unwrap_or(false), Ordering::Relaxed);
    #[cfg(not(target_os = "macos"))]
    if !state.clipboard_positioned.load(Ordering::Relaxed) {
        popup
            .center()
            .map_err(|_| AppError::new("window_unavailable", "无法定位剪贴板浮窗"))?;
        state.clipboard_positioned.store(true, Ordering::Relaxed);
    }
    #[cfg(target_os = "macos")]
    crate::panels::show_clipboard()?;
    #[cfg(not(target_os = "macos"))]
    popup
        .show()
        .and_then(|_| popup.unminimize())
        .and_then(|_| popup.set_focus())
        .map_err(|_| AppError::new("window_unavailable", "无法打开剪贴板浮窗"))?;
    let _ = popup.emit("clipboard-opened", ());
    Ok(())
}

pub fn hide_clipboard(app: &AppHandle) -> Result<()> {
    let state = app.state::<Arc<AppState>>();
    state.clipboard_focused.store(false, Ordering::Relaxed);
    state.preview_generation.fetch_add(1, Ordering::Relaxed);
    state.preview_ready.store(false, Ordering::Relaxed);
    *lock(&state.preview)? = None;
    hide_preview(app)?;
    #[cfg(target_os = "macos")]
    crate::panels::hide_clipboard()?;
    #[cfg(not(target_os = "macos"))]
    window(app, "clipboard")?
        .hide()
        .map_err(|_| AppError::new("window_unavailable", "无法关闭剪贴板浮窗"))?;
    let _ = app.emit("clipboard-closed", ());
    Ok(())
}

pub fn hide_preview(app: &AppHandle) -> Result<()> {
    hide_preview_window(app)?;
    let _ = app.emit_to("clipboard-preview", "clipboard-preview-changed", ());
    Ok(())
}

fn hide_preview_window(_app: &AppHandle) -> Result<()> {
    #[cfg(target_os = "macos")]
    return crate::panels::hide_preview();
    #[cfg(not(target_os = "macos"))]
    window(_app, "clipboard-preview")?
        .hide()
        .map_err(|_| AppError::new("window_unavailable", "无法关闭图片预览"))
}

pub fn clipboard_visible(_app: &AppHandle) -> Result<bool> {
    #[cfg(target_os = "macos")]
    return crate::panels::is_visible();
    #[cfg(not(target_os = "macos"))]
    window(_app, "clipboard")?
        .is_visible()
        .map_err(|_| AppError::new("window_unavailable", "无法检查浮窗状态"))
}

fn clipboard_focused(_app: &AppHandle) -> Result<bool> {
    #[cfg(target_os = "macos")]
    return crate::panels::is_focused();
    #[cfg(not(target_os = "macos"))]
    window(_app, "clipboard")?
        .is_focused()
        .map_err(|_| AppError::new("window_unavailable", "无法检查浮窗焦点"))
}

pub fn drag_clipboard(_app: &AppHandle) -> Result<()> {
    #[cfg(target_os = "macos")]
    return crate::panels::drag_clipboard();
    #[cfg(not(target_os = "macos"))]
    window(_app, "clipboard")?
        .start_dragging()
        .map_err(|_| AppError::new("window_unavailable", "无法移动浮窗"))
}

pub fn position_preview(app: &AppHandle) -> Result<bool> {
    let Some(state) = app.try_state::<Arc<AppState>>() else {
        return Ok(false);
    };
    let dimensions = lock(&state.preview)?
        .as_ref()
        .and_then(|detail| Some((detail.item.width?, detail.item.height?)));
    let Some((width, height)) = dimensions.filter(|(w, h)| *w > 0 && *h > 0) else {
        return Ok(false);
    };
    #[cfg(target_os = "macos")]
    return crate::panels::position_preview(width, height);
    #[cfg(not(target_os = "macos"))]
    position_tauri_preview(app, width, height)
}

#[cfg(not(target_os = "macos"))]
fn position_tauri_preview(app: &AppHandle, width: u32, height: u32) -> Result<bool> {
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

pub fn show_preview(app: &AppHandle, id: &str) -> Result<()> {
    let state = app.state::<Arc<AppState>>();
    if lock(&state.preview)?
        .as_ref()
        .is_none_or(|value| value.item.id != id)
    {
        return Ok(());
    }
    state.preview_ready.store(true, Ordering::Relaxed);
    sync_preview(app)
}

pub fn sync_preview(app: &AppHandle) -> Result<()> {
    let Some(state) = app.try_state::<Arc<AppState>>() else {
        return Ok(());
    };
    if !state.preview_ready.load(Ordering::Relaxed) || !position_preview(app)? {
        return hide_preview_window(app);
    }
    #[cfg(target_os = "macos")]
    if !crate::panels::preview_visible()? {
        crate::panels::show_preview()?;
    }
    #[cfg(not(target_os = "macos"))]
    {
        let preview = window(app, "clipboard-preview")?;
        if !preview.is_visible().unwrap_or(false) {
            preview
                .show()
                .map_err(|_| AppError::new("window_unavailable", "无法打开图片预览"))?;
        }
    }
    Ok(())
}
