use crate::common::{
    shortcuts,
    state::{lock, unavailable, AppState},
    theme, windowing,
};
use base64::{engine::general_purpose::STANDARD, Engine};
use mtool_core::*;
use std::sync::{atomic::Ordering, Arc};
use tauri::{AppHandle, Emitter, Manager, State, WebviewWindow};
use tauri_plugin_autostart::ManagerExt;

#[tauri::command]
pub async fn bootstrap(state: State<'_, Arc<AppState>>) -> Result<Bootstrap> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        Ok(Bootstrap {
            tools: mtool_tools::catalog(),
            settings: state.settings()?,
            status: state.status()?,
        })
    })
    .await
    .map_err(|_| AppError::new("task_failed", "初始化任务失败"))?
}

#[tauri::command]
pub async fn run_tool(
    state: State<'_, Arc<AppState>>,
    request: ToolRequest,
) -> Result<ToolResponse> {
    let job = state.inner().job()?;
    tauri::async_runtime::spawn_blocking(move || {
        let _job = job;
        mtool_tools::execute(&request)
    })
    .await
    .map_err(|_| AppError::new("task_failed", "工具计算失败，请检查输入"))?
}

#[tauri::command]
pub async fn get_status(state: State<'_, Arc<AppState>>) -> Result<AppStatus> {
    state.status()
}

#[tauri::command]
pub async fn save_settings(
    app: AppHandle,
    window: WebviewWindow,
    state: State<'_, Arc<AppState>>,
    settings: Settings,
) -> Result<Settings> {
    if window.label() != "main" {
        return Err(AppError::new("forbidden", "请在主窗口修改设置"));
    }
    settings.validate()?;
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let previous = state.settings()?;
        let changed = previous.clipboard_shortcut != settings.clipboard_shortcut
            || previous.launcher_shortcut != settings.launcher_shortcut;
        if changed {
            shortcuts::register(&app, &settings, Some(&previous))?;
        }
        let autostart = app.autolaunch();
        if previous.launch_at_login != settings.launch_at_login {
            let result = if settings.launch_at_login {
                autostart.enable()
            } else {
                autostart.disable()
            };
            if result.is_err() {
                if changed {
                    let _ = shortcuts::register(&app, &previous, None);
                }
                return Err(AppError::new(
                    "autostart_failed",
                    "无法修改登录启动设置，已保留原配置",
                ));
            }
        }
        let result = (|| {
            let mut p = lock(&state.persistence)?;
            p.store
                .as_mut()
                .ok_or_else(unavailable)?
                .save_settings(&settings, chrono::Utc::now().timestamp_millis())?;
            p.settings = settings.clone();
            Ok(())
        })();
        if let Err(error) = result {
            if changed {
                let _ = shortcuts::register(&app, &previous, None);
            }
            if previous.launch_at_login != settings.launch_at_login {
                if previous.launch_at_login {
                    let _ = autostart.enable();
                } else {
                    let _ = autostart.disable();
                }
            }
            return Err(error);
        }
        if changed {
            *lock(&state.shortcut_error)? = None;
        }
        if previous.mouse != settings.mouse {
            app.state::<super::mouse::MouseService>()
                .configure(settings.mouse.clone());
        }
        if previous.theme != settings.theme {
            theme::apply(&app, settings.theme);
        }
        let _ = app.emit("settings-changed", ());
        let _ = app.emit("clipboard-changed", ());
        Ok(settings)
    })
    .await
    .map_err(|_| AppError::new("task_failed", "设置保存任务失败"))?
}

#[tauri::command]
pub async fn list_history(
    state: State<'_, Arc<AppState>>,
    query: String,
    pinned_only: bool,
    offset: u32,
) -> Result<ClipboardPage> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        lock(&state.persistence)?
            .store
            .as_ref()
            .ok_or_else(unavailable)?
            .list(&query, pinned_only, offset, 50)
    })
    .await
    .map_err(|_| AppError::new("task_failed", "历史查询任务失败"))?
}

#[tauri::command]
pub async fn history_detail(
    state: State<'_, Arc<AppState>>,
    id: String,
) -> Result<ClipboardDetail> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        lock(&state.persistence)?
            .store
            .as_ref()
            .ok_or_else(unavailable)?
            .detail(&id)
    })
    .await
    .map_err(|_| AppError::new("task_failed", "历史读取任务失败"))?
}

#[tauri::command]
pub async fn history_thumbnail(state: State<'_, Arc<AppState>>, id: String) -> Result<String> {
    let job = state.inner().job()?;
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let _job = job;
        let content = lock(&state.persistence)?
            .store
            .as_ref()
            .ok_or_else(unavailable)?
            .content(&id)?
            .1;
        let ClipboardContent::Image { png, .. } = content else {
            return Err(AppError::invalid("此记录不是图片"));
        };
        let thumbnail = mtool_platform::image_thumbnail(&png)?;
        Ok(format!(
            "data:image/png;base64,{}",
            STANDARD.encode(thumbnail)
        ))
    })
    .await
    .map_err(|_| AppError::new("task_failed", "图片预览生成失败"))?
}

#[tauri::command]
pub async fn pin_history(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    id: String,
    pinned: bool,
) -> Result<()> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let mut p = lock(&state.persistence)?;
        let settings = p.settings.clone();
        p.store.as_mut().ok_or_else(unavailable)?.pin(
            &id,
            pinned,
            &settings,
            chrono::Utc::now().timestamp_millis(),
        )?;
        let _ = app.emit("clipboard-changed", ());
        Ok(())
    })
    .await
    .map_err(|_| AppError::new("task_failed", "收藏任务失败"))?
}

#[tauri::command]
pub async fn delete_history(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    id: String,
) -> Result<()> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        lock(&state.persistence)?
            .store
            .as_ref()
            .ok_or_else(unavailable)?
            .delete(&id)?;
        let _ = app.emit("clipboard-changed", ());
        Ok(())
    })
    .await
    .map_err(|_| AppError::new("task_failed", "删除任务失败"))?
}

#[tauri::command]
pub async fn clear_history(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    include_pinned: bool,
) -> Result<()> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        lock(&state.persistence)?
            .store
            .as_ref()
            .ok_or_else(unavailable)?
            .clear(include_pinned)?;
        let _ = app.emit("clipboard-changed", ());
        Ok(())
    })
    .await
    .map_err(|_| AppError::new("task_failed", "清空历史任务失败"))?
}

fn write(state: &AppState, content: &ClipboardContent) -> Result<()> {
    let _io = lock(&state.clipboard_io)?;
    mtool_platform::write_clipboard(content)?;
    *lock(&state.own_revision)? = Some(mtool_platform::clipboard_revision());
    Ok(())
}

#[tauri::command]
pub async fn use_history(state: State<'_, Arc<AppState>>, id: String) -> Result<()> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let content = lock(&state.persistence)?
            .store
            .as_ref()
            .ok_or_else(unavailable)?
            .content(&id)?
            .1;
        write(&state, &content)
    })
    .await
    .map_err(|_| AppError::new("task_failed", "复制任务失败"))?
}

#[tauri::command]
pub async fn copy_output(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    text: String,
) -> Result<()> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let content = ClipboardContent::Text(text);
        write(&state, &content)?;
        let mut p = lock(&state.persistence)?;
        let settings = p.settings.clone();
        if let Some(store) = p.store.as_mut() {
            mtool_clipboard::capture(
                store,
                &content,
                Some("MTool"),
                &settings,
                chrono::Utc::now().timestamp_millis(),
            )?;
        }
        let _ = app.emit("clipboard-changed", ());
        Ok(())
    })
    .await
    .map_err(|_| AppError::new("task_failed", "复制任务失败"))?
}

#[tauri::command]
pub fn open_clipboard(app: AppHandle) -> Result<()> {
    windowing::show_clipboard(&app)
}

#[tauri::command]
pub fn close_clipboard(app: AppHandle, window: WebviewWindow) -> Result<()> {
    if window.label() != "clipboard" {
        return Err(AppError::new("forbidden", "仅剪贴板浮窗可关闭自身"));
    }
    windowing::hide_clipboard(&app)
}

#[tauri::command]
pub fn clipboard_visible(app: AppHandle, window: WebviewWindow) -> Result<bool> {
    if window.label() != "clipboard" {
        return Err(AppError::new("forbidden", "仅浮窗可检查自身状态"));
    }
    windowing::clipboard_visible(&app)
}

#[tauri::command]
pub fn drag_clipboard(app: AppHandle, window: WebviewWindow) -> Result<()> {
    if window.label() != "clipboard" {
        return Err(AppError::new("forbidden", "仅浮窗可移动自身"));
    }
    windowing::drag_clipboard(&app)
}

#[tauri::command]
pub async fn set_clipboard_preview(
    app: AppHandle,
    window: WebviewWindow,
    state: State<'_, Arc<AppState>>,
    id: Option<String>,
) -> Result<()> {
    if window.label() != "clipboard" {
        return Err(AppError::new("forbidden", "仅剪贴板浮窗可选择预览"));
    }
    let state = state.inner().clone();
    let generation = state.preview_generation.fetch_add(1, Ordering::Relaxed) + 1;
    let initial_state = state.clone();
    let initial_app = app.clone();
    app.run_on_main_thread(move || {
        if initial_state.preview_generation.load(Ordering::Relaxed) == generation {
            initial_state.preview_ready.store(false, Ordering::Relaxed);
            if let Ok(mut preview) = lock(&initial_state.preview) {
                *preview = None;
            }
            let _ = windowing::hide_preview(&initial_app);
        }
    })
    .map_err(|_| AppError::new("window_unavailable", "无法更新图片预览"))?;
    let Some(id) = id else { return Ok(()) };
    let reader = state.clone();
    let detail = tauri::async_runtime::spawn_blocking(move || {
        if reader.preview_generation.load(Ordering::Relaxed) != generation {
            return Ok(None);
        }
        let persistence = lock(&reader.persistence)?;
        if reader.preview_generation.load(Ordering::Relaxed) != generation {
            return Ok(None);
        }
        let detail = persistence
            .store
            .as_ref()
            .ok_or_else(unavailable)?
            .detail(&id)?;
        if detail.item.kind != ClipboardKind::Image || detail.image_data_url.is_none() {
            return Err(AppError::invalid("此记录不是图片"));
        }
        Ok(Some(detail))
    })
    .await
    .map_err(|_| AppError::new("task_failed", "图片预览读取失败"))??;
    let Some(detail) = detail else { return Ok(()) };
    let handle = app.clone();
    app.run_on_main_thread(move || {
        if state.preview_generation.load(Ordering::Relaxed) != generation
            || !windowing::clipboard_visible(&handle).unwrap_or(false)
        {
            return;
        }
        if let Ok(mut preview) = lock(&state.preview) {
            *preview = Some(detail);
        }
        let _ = handle.emit_to("clipboard-preview", "clipboard-preview-changed", ());
    })
    .map_err(|_| AppError::new("window_unavailable", "无法更新图片预览"))
}

#[tauri::command]
pub fn clipboard_preview(
    window: WebviewWindow,
    state: State<'_, Arc<AppState>>,
) -> Result<Option<ClipboardDetail>> {
    if window.label() != "clipboard-preview" {
        return Err(AppError::new("forbidden", "仅图片窗口可读取预览"));
    }
    Ok(lock(&state.preview)?.clone())
}

#[tauri::command]
pub fn show_clipboard_preview(app: AppHandle, window: WebviewWindow, id: String) -> Result<()> {
    if window.label() != "clipboard-preview" {
        return Err(AppError::new("forbidden", "仅图片窗口可显示预览"));
    }
    windowing::show_preview(&app, &id)
}
