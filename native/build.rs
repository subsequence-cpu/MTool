#[path = "build_support/macos.rs"]
mod macos;

fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") {
        macos::build_finder_extension();
    }
    tauri_build::build();
}
