mod clipboard;

pub use clipboard::{
    clipboard_revision, clipboard_source, is_sensitive, mark_owned, ClipboardWatcher,
};
