use image::{ImageFormat, ImageReader};
use mtool_core::{
    AppError, ClipboardContent, Result, MAX_IMAGE_BYTES, MAX_IMAGE_PIXELS, MAX_TEXT_BYTES,
};
use std::{borrow::Cow, io::Cursor};

use crate::{is_sensitive, mark_owned};

pub fn image_thumbnail(png: &[u8]) -> Result<Vec<u8>> {
    if png.len() > MAX_IMAGE_BYTES {
        return Err(AppError::invalid("图片超过 10 MiB"));
    }
    let (width, height) = ImageReader::with_format(Cursor::new(png), ImageFormat::Png)
        .into_dimensions()
        .map_err(|_| AppError::invalid("图片数据无效"))?;
    if u64::from(width) * u64::from(height) > MAX_IMAGE_PIXELS {
        return Err(AppError::invalid("图片超过 1,600 万像素"));
    }
    let mut reader = ImageReader::with_format(Cursor::new(png), ImageFormat::Png);
    let mut limits = image::Limits::default();
    limits.max_alloc = Some(64 * 1024 * 1024);
    reader.limits(limits);
    let image = reader
        .decode()
        .map_err(|_| AppError::invalid("图片解码失败"))?;
    let mut output = Cursor::new(Vec::new());
    image
        .thumbnail(96, 96)
        .write_to(&mut output, ImageFormat::Png)
        .map_err(|_| AppError::new("image_error", "无法生成图片预览"))?;
    Ok(output.into_inner())
}

pub fn read_clipboard() -> Result<Option<ClipboardContent>> {
    if is_sensitive() {
        return Ok(None);
    }
    let mut board = arboard::Clipboard::new()
        .map_err(|_| AppError::new("clipboard_unavailable", "无法访问系统剪贴板"))?;
    match board.get_text() {
        Ok(text) if !text.is_empty() => {
            if text.len() > MAX_TEXT_BYTES {
                return Err(AppError::new(
                    "clipboard_too_large",
                    "剪贴板文本超过 2 MiB，未记录",
                ));
            }
            return Ok(Some(ClipboardContent::Text(text)));
        }
        Err(arboard::Error::ClipboardOccupied) => {
            return Err(AppError::new(
                "clipboard_busy",
                "剪贴板正被其他应用使用，将在下一次变更时重试",
            ))
        }
        _ => {}
    }
    match board.get_image() {
        Ok(data) => {
            if data.width as u64 * data.height as u64 > MAX_IMAGE_PIXELS {
                return Err(AppError::new(
                    "clipboard_too_large",
                    "剪贴板图片超过 1,600 万像素，未记录",
                ));
            }
            let buffer = image::RgbaImage::from_raw(
                data.width as u32,
                data.height as u32,
                data.bytes.into_owned(),
            )
            .ok_or_else(|| AppError::new("image_error", "剪贴板图片尺寸与数据不一致"))?;
            let mut png = Cursor::new(Vec::new());
            buffer
                .write_to(&mut png, ImageFormat::Png)
                .map_err(|_| AppError::new("image_error", "无法编码剪贴板图片"))?;
            let png = png.into_inner();
            if png.len() > MAX_IMAGE_BYTES {
                return Err(AppError::new(
                    "clipboard_too_large",
                    "剪贴板图片超过 10 MiB，未记录",
                ));
            }
            Ok(Some(ClipboardContent::Image {
                png,
                width: data.width as u32,
                height: data.height as u32,
            }))
        }
        Err(arboard::Error::ContentNotAvailable) => Ok(None),
        Err(_) => Ok(None),
    }
}

pub fn write_clipboard(content: &ClipboardContent) -> Result<()> {
    let mut board = arboard::Clipboard::new()
        .map_err(|_| AppError::new("clipboard_unavailable", "无法访问系统剪贴板"))?;
    match content {
        ClipboardContent::Text(text) => {
            if text.len() > MAX_TEXT_BYTES {
                return Err(AppError::invalid("复制文本超过 2 MiB"));
            }
            board
                .set_text(text.clone())
                .map_err(|_| AppError::new("clipboard_write_failed", "无法写入系统剪贴板"))?;
        }
        ClipboardContent::Image { png, .. } => {
            if png.len() > MAX_IMAGE_BYTES {
                return Err(AppError::invalid("复制图片超过 10 MiB"));
            }
            let mut reader = ImageReader::new(Cursor::new(png))
                .with_guessed_format()
                .map_err(|_| AppError::invalid("图片格式无效"))?;
            let mut limits = image::Limits::default();
            limits.max_alloc = Some(64 * 1024 * 1024);
            reader.limits(limits);
            let image = reader
                .decode()
                .map_err(|_| AppError::invalid("图片无法解码"))?
                .into_rgba8();
            if u64::from(image.width()) * u64::from(image.height()) > MAX_IMAGE_PIXELS {
                return Err(AppError::invalid("图片尺寸过大"));
            }
            board
                .set_image(arboard::ImageData {
                    width: image.width() as usize,
                    height: image.height() as usize,
                    bytes: Cow::Owned(image.into_raw()),
                })
                .map_err(|_| AppError::new("clipboard_write_failed", "无法写入系统剪贴板图片"))?;
        }
    }
    mark_owned();
    Ok(())
}
