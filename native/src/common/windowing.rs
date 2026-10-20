use crate::common::state::{lock, AppState};
use mtool_core::{AppError, Result};
use std::sync::{atomic::Ordering, Arc};
use tauri::{AppHandle, Emitter, Manager, WebviewWindow};

pub(crate) fn window(app: &AppHandle, label: &str) -> Result<WebviewWindow> {
    app.get_webview_window(label)
        .ok_or_else(|| AppError::new("window_unavailable", "窗口不可用"))
}

pub fn show_main(app: &AppHandle) -> Result<()> {
    crate::platform::windowing::activate(app)?;
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
    crate::platform::windowing::show_clipboard(app)?;
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
    crate::platform::windowing::hide_clipboard(app)?;
    let _ = app.emit("clipboard-closed", ());
    Ok(())
}

pub fn hide_preview(app: &AppHandle) -> Result<()> {
    hide_preview_window(app)?;
    let _ = app.emit_to("clipboard-preview", "clipboard-preview-changed", ());
    Ok(())
}

fn hide_preview_window(app: &AppHandle) -> Result<()> {
    crate::platform::windowing::hide_preview(app)
}

pub fn clipboard_visible(app: &AppHandle) -> Result<bool> {
    crate::platform::windowing::clipboard_visible(app)
}

fn clipboard_focused(app: &AppHandle) -> Result<bool> {
    crate::platform::windowing::clipboard_focused(app)
}

pub fn drag_clipboard(app: &AppHandle) -> Result<()> {
    crate::platform::windowing::drag_clipboard(app)
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
    crate::platform::windowing::position_preview(app, width, height)
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
    crate::platform::windowing::show_preview(app)?;
    Ok(())
}
