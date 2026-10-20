use crate::common::{
    state::{lock, AppState},
    windowing,
};
use mtool_core::{AppError, Result, Settings};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

pub fn register(app: &AppHandle, settings: &Settings, previous: Option<&Settings>) -> Result<()> {
    let clipboard: Shortcut = settings
        .clipboard_shortcut
        .parse()
        .map_err(|_| AppError::invalid("剪贴板快捷键格式无效"))?;
    let launcher: Shortcut = settings
        .launcher_shortcut
        .parse()
        .map_err(|_| AppError::invalid("工具箱快捷键格式无效"))?;
    if clipboard == launcher {
        return Err(AppError::invalid("两个快捷键不能相同"));
    }
    app.global_shortcut()
        .unregister_all()
        .map_err(|_| AppError::new("shortcut_failed", "无法释放原快捷键"))?;
    let clipboard_ok = app.global_shortcut().register(clipboard).is_ok();
    let launcher_ok = app.global_shortcut().register(launcher).is_ok();
    if !clipboard_ok || !launcher_ok {
        if let Some(old) = previous {
            app.global_shortcut().unregister_all().map_err(|_| {
                AppError::new("shortcut_failed", "无法释放快捷键，请重新启动 MTool")
            })?;
            let clipboard_restored = app
                .global_shortcut()
                .register(old.clipboard_shortcut.as_str())
                .is_ok();
            let launcher_restored = app
                .global_shortcut()
                .register(old.launcher_shortcut.as_str())
                .is_ok();
            if !clipboard_restored || !launcher_restored {
                return Err(AppError::new(
                    "shortcut_failed",
                    "原快捷键恢复失败，请重新启动 MTool",
                ));
            }
        }
        let failed = match (clipboard_ok, launcher_ok) {
            (false, false) => "剪贴板和启动台快捷键",
            (false, true) => "剪贴板快捷键",
            _ => "启动台快捷键",
        };
        return Err(AppError::new(
            "shortcut_conflict",
            format!("{failed}注册失败，请检查占用或更换组合"),
        ));
    }
    Ok(())
}

fn report_error(app: &AppHandle, error: AppError) {
    if let Some(state) = app.try_state::<Arc<AppState>>() {
        if let Ok(mut message) = lock(&state.shortcut_error) {
            *message = Some(error.message);
        }
        let _ = app.emit("status-changed", ());
    }
}

pub fn plugin() -> tauri::plugin::TauriPlugin<tauri::Wry> {
    tauri_plugin_global_shortcut::Builder::new()
        .with_handler(|app, shortcut, event| {
            if event.state() != ShortcutState::Pressed {
                return;
            }
            let Some(state) = app.try_state::<Arc<AppState>>() else {
                return;
            };
            let Ok(settings) = state.settings() else {
                return;
            };
            let clipboard = settings
                .clipboard_shortcut
                .parse::<Shortcut>()
                .is_ok_and(|s| s == *shortcut);
            let launcher = settings
                .launcher_shortcut
                .parse::<Shortcut>()
                .is_ok_and(|s| s == *shortcut);
            if !clipboard && !launcher {
                return;
            }
            let handle = app.clone();
            let queued = app.run_on_main_thread(move || {
                let result = if clipboard {
                    windowing::show_clipboard(&handle)
                } else {
                    windowing::show_main(&handle)
                };
                if let Err(error) = result {
                    report_error(&handle, error);
                }
            });
            if queued.is_err() {
                report_error(
                    app,
                    AppError::new("window_unavailable", "无法唤出窗口，请重新启动 MTool"),
                );
            }
        })
        .build()
}
