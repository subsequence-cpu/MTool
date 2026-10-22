use crate::{common::windowing, platform};
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
};

pub fn setup(app: &tauri::App) -> tauri::Result<()> {
    let show = MenuItem::with_id(app, "show", "打开工具箱", true, None::<&str>)?;
    let clipboard = MenuItem::with_id(app, "clipboard", "剪贴板历史", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "退出 MTool", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show, &clipboard, &quit])?;
    let builder = TrayIconBuilder::with_id("main-tray")
        .menu(&menu)
        .tooltip("MTool · 本地开发工具箱")
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show" => {
                let _ = windowing::show_main(app);
            }
            "clipboard" => {
                let _ = windowing::show_clipboard(app);
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if matches!(
                event,
                TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                }
            ) {
                let _ = windowing::show_main(tray.app_handle());
            }
        });
    platform::configure_tray(app, builder)?.build(app)?;
    Ok(())
}
