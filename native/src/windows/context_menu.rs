use mtool_core::{AppError, ContextMenuStatus, Result};

pub(crate) fn settings() -> Result<()> {
    Err(AppError::new("unsupported", "仅支持 macOS"))
}

pub(crate) fn accessibility_settings() -> Result<()> {
    Err(AppError::new("unsupported", "仅支持 macOS"))
}

pub(crate) fn status() -> ContextMenuStatus {
    ContextMenuStatus {
        supported: false,
        bundled: false,
        enabled: false,
        vscode_installed: false,
        accessibility_granted: false,
        error: None,
    }
}
