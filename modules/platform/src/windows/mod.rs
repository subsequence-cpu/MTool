mod clipboard;
mod watcher;

pub use clipboard::{clipboard_revision, clipboard_source, is_sensitive, mark_owned};
pub use watcher::ClipboardWatcher;
