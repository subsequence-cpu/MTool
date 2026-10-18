use mtool_core::{AppError, ClipboardContent, Result, Settings, MAX_IMAGE_BYTES, MAX_TEXT_BYTES};
use mtool_storage::Store;
use sha2::{Digest, Sha256};

fn fingerprint(content: &ClipboardContent) -> String {
    let mut hash = Sha256::new();
    hash.update(match content {
        ClipboardContent::Text(_) => b"text".as_slice(),
        ClipboardContent::Image { .. } => b"image".as_slice(),
    });
    hash.update(content.bytes());
    hex::encode(hash.finalize())
}

pub fn excluded(source: Option<&str>, settings: &Settings) -> bool {
    let Some(source) = source else { return false };
    let source = source.to_lowercase();
    settings
        .excluded_apps
        .iter()
        .any(|app| source.contains(&app.to_lowercase()))
}

pub fn capture(
    store: &mut Store,
    content: &ClipboardContent,
    source: Option<&str>,
    settings: &Settings,
    now: i64,
) -> Result<bool> {
    if !settings.capture_enabled || excluded(source, settings) {
        return Ok(false);
    }
    let limit = match content {
        ClipboardContent::Text(_) => MAX_TEXT_BYTES,
        ClipboardContent::Image { .. } => MAX_IMAGE_BYTES,
    };
    if content.bytes().len() > limit {
        return Err(AppError::new(
            "clipboard_too_large",
            "此剪贴板内容过大，未保存（文本 2 MiB / 图片 10 MiB）",
        ));
    }
    if content.bytes().is_empty() {
        return Ok(false);
    }
    let recorded = store.insert(&fingerprint(content), content, source, settings, now)?;
    if !recorded {
        return Err(AppError::new(
            "history_full",
            "收藏内容已占满历史容量，新增内容未保存，请清理部分收藏",
        ));
    }
    Ok(true)
}
