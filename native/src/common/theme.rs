use mtool_core::AppTheme;
use tauri::{window::Color, AppHandle, Manager, Theme};

pub fn appearance(theme: AppTheme) -> (Theme, Color) {
    match theme {
        AppTheme::Dracula => (Theme::Dark, Color(40, 42, 54, 255)),
        AppTheme::GithubLight => (Theme::Light, Color(255, 255, 255, 255)),
    }
}

fn icon_bytes(theme: AppTheme) -> &'static [u8] {
    match theme {
        AppTheme::Dracula => include_bytes!("../../resources/common/icons/icon.png"),
        AppTheme::GithubLight => include_bytes!("../../resources/common/icons/icon-light.png"),
    }
}

pub fn apply(app: &AppHandle, theme: AppTheme) {
    let (native, background) = appearance(theme);
    app.set_theme(Some(native));
    let icon = tauri::image::Image::from_bytes(icon_bytes(theme)).expect("valid bundled app icon");
    for (label, window) in app.webview_windows() {
        if let Err(error) = window
            .set_theme(Some(native))
            .and_then(|_| window.set_background_color(Some(background)))
            .and_then(|_| window.set_icon(icon.clone()))
        {
            eprintln!("Could not update theme for {label}: {error}");
        }
    }
    #[cfg(windows)]
    if let Some(tray) = app.tray_by_id("main-tray") {
        if let Err(error) = tray.set_icon(Some(icon)) {
            eprintln!("Could not update tray icon: {error}");
        }
    }
    #[cfg(target_os = "macos")]
    if let Err(error) = crate::macos::set_app_icon(app, icon_bytes(theme)) {
        eprintln!("Could not update Dock icon: {error}");
    }
}
