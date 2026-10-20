pub(crate) mod context_menu;
pub(crate) mod windowing;

pub(crate) fn configure_tray(
    app: &tauri::App,
    mut builder: tauri::tray::TrayIconBuilder<tauri::Wry>,
) -> tauri::Result<tauri::tray::TrayIconBuilder<tauri::Wry>> {
    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }
    Ok(builder)
}
