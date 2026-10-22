mod common;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(windows)]
mod windows;

use common::{commands, context_menu, monitor, mouse, shortcuts, state, theme, tray, windowing};
#[cfg(target_os = "macos")]
use macos as platform;
use state::{lock, AppState};
use std::sync::{atomic::Ordering, Arc};
use tauri::Manager;
#[cfg(windows)]
use windows as platform;

pub fn run() {
    #[cfg(target_os = "macos")]
    let background_launch = {
        let background =
            std::env::var_os("MTOOL_CONTEXT_LAUNCH").as_deref() == Some(std::ffi::OsStr::new("1"));
        std::env::remove_var("MTOOL_CONTEXT_LAUNCH");
        background
    };
    let context = tauri::generate_context!();
    #[cfg(target_os = "macos")]
    let context = {
        let mut context = context;
        if background_launch {
            if let Some(main) = context
                .config_mut()
                .app
                .windows
                .iter_mut()
                .find(|w| w.label == "main")
            {
                main.visible = false;
                main.focus = false;
            }
        }
        context
    };
    tauri::Builder::default()
        .manage(context_menu::ContextMenuState::default())
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            let _ = windowing::show_main(app);
        }))
        .plugin(shortcuts::plugin())
        .plugin(
            tauri_plugin_autostart::Builder::new()
                .args(["--autostart"])
                .build(),
        )
        .setup(|app| {
            let state = Arc::new(AppState::initialize(&app.path().app_data_dir()?));
            let preferences = state.settings()?;
            let (native_theme, background) = theme::appearance(preferences.theme);
            app.set_theme(Some(native_theme));
            app.manage(state.clone());
            app.manage(mouse::MouseService::new(preferences.mouse));
            for config in &app.config().app.windows {
                tauri::WebviewWindowBuilder::from_config(app.handle(), config)?
                    .theme(Some(native_theme))
                    .background_color(background)
                    .build()?;
            }
            if let Err(error) = shortcuts::register(app.handle(), &state.settings()?, None) {
                *lock(&state.shortcut_error)? = Some(error.message);
            }
            tray::setup(app)?;
            #[cfg(target_os = "macos")]
            macos::panels::prepare(app.handle())?;
            theme::apply(app.handle(), preferences.theme);
            monitor::start(app.handle().clone(), state);
            if std::env::args().any(|arg| arg == "--autostart") {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.hide();
                }
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::bootstrap,
            commands::run_tool,
            commands::get_status,
            commands::save_settings,
            commands::list_history,
            commands::history_detail,
            commands::history_thumbnail,
            commands::pin_history,
            commands::delete_history,
            commands::clear_history,
            commands::use_history,
            commands::copy_output,
            commands::open_clipboard,
            commands::close_clipboard,
            commands::clipboard_visible,
            commands::drag_clipboard,
            commands::set_clipboard_preview,
            commands::clipboard_preview,
            commands::show_clipboard_preview,
            context_menu::context_menu_status,
            context_menu::context_menu_settings,
            context_menu::context_menu_accessibility_settings,
            mouse::mouse_status,
            mouse::mouse_set_dpi,
            mouse::mouse_update_settings,
            mouse::mouse_capture_button,
            mouse::mouse_input_monitoring_settings,
            mouse::mouse_accessibility_settings
        ])
        .on_window_event(|window, event| match event {
            tauri::WindowEvent::CloseRequested { api, .. } => {
                api.prevent_close();
                if window.label() == "clipboard" {
                    let _ = windowing::hide_clipboard(window.app_handle());
                } else {
                    let _ = window.hide();
                }
            }
            #[cfg(not(target_os = "macos"))]
            tauri::WindowEvent::Focused(focused) if window.label() == "clipboard" => {
                if let Some(state) = window.try_state::<Arc<AppState>>() {
                    if *focused {
                        state.clipboard_focused.store(true, Ordering::Relaxed);
                    } else if !window.is_focused().unwrap_or(true)
                        && state.clipboard_focused.swap(false, Ordering::Relaxed)
                    {
                        let _ = windowing::hide_clipboard(window.app_handle());
                    }
                }
            }
            #[cfg(not(target_os = "macos"))]
            tauri::WindowEvent::Moved(_) | tauri::WindowEvent::Resized(_)
                if window.label() == "clipboard" =>
            {
                let _ = windowing::sync_preview(window.app_handle());
            }
            _ => {}
        })
        .build(context)
        .expect("MTool application could not be initialized")
        .run(|app, event| match event {
            #[cfg(target_os = "macos")]
            tauri::RunEvent::Opened { urls } => macos::context_menu::open_urls(app, urls),
            #[cfg(target_os = "macos")]
            tauri::RunEvent::Reopen { .. } => {
                if let Err(error) = windowing::show_main(app) {
                    eprintln!("Could not reopen the main window: {}", error.message);
                }
            }
            tauri::RunEvent::Exit => {
                if let Some(mouse) = app.try_state::<mouse::MouseService>() {
                    mouse.shutdown();
                }
                if let Some(state) = app.try_state::<Arc<AppState>>() {
                    state.stop.store(true, Ordering::Relaxed);
                }
                #[cfg(target_os = "macos")]
                macos::panels::shutdown();
            }
            _ => {}
        });
}
