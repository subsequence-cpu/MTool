pub(crate) mod context_menu;
pub(crate) mod finder;
pub(crate) mod panels;
pub(crate) mod windowing;

pub(crate) fn configure_tray(
    _app: &tauri::App,
    builder: tauri::tray::TrayIconBuilder<tauri::Wry>,
) -> tauri::Result<tauri::tray::TrayIconBuilder<tauri::Wry>> {
    Ok(builder
        .icon(tauri::image::Image::from_bytes(include_bytes!(
            "../../resources/macos/tray/44x44.png"
        ))?)
        .icon_as_template(true))
}
