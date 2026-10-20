mod common;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(windows)]
mod windows;

pub use common::{database_key, image_thumbnail, read_clipboard, write_clipboard};
#[cfg(target_os = "macos")]
pub use macos::{clipboard_revision, clipboard_source, is_sensitive, mark_owned, ClipboardWatcher};
#[cfg(windows)]
pub use windows::{
    clipboard_revision, clipboard_source, is_sensitive, mark_owned, ClipboardWatcher,
};
