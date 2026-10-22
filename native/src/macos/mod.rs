pub(crate) mod context_menu;
pub(crate) mod finder;
pub(crate) mod mouse;
pub(crate) mod panels;
pub(crate) mod windowing;

pub(crate) fn set_app_icon(app: &tauri::AppHandle, bytes: &'static [u8]) -> tauri::Result<()> {
    app.run_on_main_thread(move || {
        use objc2::{AllocAnyThread, MainThreadMarker};
        use objc2_app_kit::{NSApplication, NSImage};
        use objc2_foundation::NSData;

        if let Some(image) = NSImage::initWithData(NSImage::alloc(), &NSData::with_bytes(bytes)) {
            let mtm = MainThreadMarker::new().expect("Dock icon updated on the main thread");
            unsafe { NSApplication::sharedApplication(mtm).setApplicationIconImage(Some(&image)) };
        }
    })
}

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
