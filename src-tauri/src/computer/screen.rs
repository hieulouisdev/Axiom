//! Screen capture / OCR.
//!
//! v1.1: migrated to the `screenshots` 0.8 `Screen` API. `capture()` now
//! returns a raw `RgbaImage`, so we PNG-encode via the `image` crate that
//! `screenshots` re-exports (`screenshots::image`). Region capture now uses
//! the native `capture_area()` instead of returning the full screen.
//! OCR is best-effort via `rusty_tesseract` when tesseract is installed.

use serde::{Deserialize, Serialize};

use screenshots::Screen;
use screenshots::image::{DynamicImage, ImageFormat};

use crate::error::{AegisError, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Screenshot {
    pub width: u32,
    pub height: u32,
    /// Base64-encoded PNG bytes.
    pub png_base64: String,
    /// OCR-extracted text (best-effort — empty if tesseract is unavailable).
    pub ocr_text: String,
}

/// Capture the primary display.
fn primary_screen() -> Result<Screen> {
    Screen::all()
        .map_err(|e| AegisError::Internal(format!("display enumeration: {e}")))?
        .into_iter()
        .next()
        .ok_or_else(|| AegisError::Internal("no display found".into()))
}

/// Encode a raw RGBA frame as PNG bytes.
fn encode_png(image: screenshots::image::RgbaImage) -> Result<(u32, u32, Vec<u8>)> {
    let width = image.width();
    let height = image.height();
    let mut png_bytes: Vec<u8> = Vec::new();
    DynamicImage::ImageRgba8(image)
        .write_to(&mut std::io::Cursor::new(&mut png_bytes), ImageFormat::Png)
        .map_err(|e| AegisError::Internal(format!("png encode: {e}")))?;
    Ok((width, height, png_bytes))
}

/// Build the final `Screenshot` payload from PNG bytes.
fn finalize(width: u32, height: u32, png_bytes: Vec<u8>) -> Screenshot {
    let png_base64 = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &png_bytes);

    // OCR text extraction (best-effort)
    let ocr_text = extract_ocr_text(&png_bytes).unwrap_or_default();

    tracing::debug!(
        "screenshot: {}x{}, {} bytes, OCR {} chars",
        width,
        height,
        png_bytes.len(),
        ocr_text.len()
    );

    Screenshot {
        width,
        height,
        png_base64,
        ocr_text,
    }
}

/// Capture the entire primary display.
pub fn screenshot() -> Result<Screenshot> {
    let screen = primary_screen()?;
    let image = screen
        .capture()
        .map_err(|e| AegisError::Internal(format!("screen capture: {e}")))?;
    let (width, height, png_bytes) = encode_png(image)?;
    Ok(finalize(width, height, png_bytes))
}

/// Capture a specific area of the primary display.
///
/// v1.1: uses the native `capture_area()` from `screenshots` 0.8 — no more
/// full-screen capture + software crop workaround.
pub fn screenshot_area(x: i32, y: i32, width: u32, height: u32) -> Result<Screenshot> {
    let screen = primary_screen()?;
    let image = screen
        .capture_area(x, y, width, height)
        .map_err(|e| AegisError::Internal(format!("area capture: {e}")))?;
    let (w, h, png_bytes) = encode_png(image)?;
    Ok(finalize(w, h, png_bytes))
}

/// Extract text from a PNG image using Tesseract OCR (best-effort).
fn extract_ocr_text(png_bytes: &[u8]) -> Result<String> {
    let temp_dir = std::env::temp_dir();
    let temp_path = temp_dir.join(format!("aegis_ocr_{}.png", uuid::Uuid::new_v4().simple()));

    std::fs::write(&temp_path, png_bytes)
        .map_err(|e| AegisError::Io(format!("temp image write: {e}")))?;

    // v0.3: rusty-tesseract 1.1 expects an `Image` struct, not a path string.
    let image = match rusty_tesseract::Image::from_path(&temp_path) {
        Ok(img) => img,
        Err(e) => {
            tracing::debug!("OCR image load failed: {e}");
            let _ = std::fs::remove_file(&temp_path);
            return Ok(String::new());
        }
    };

    let result = rusty_tesseract::image_to_string(&image, &rusty_tesseract::Args::default());

    let _ = std::fs::remove_file(&temp_path);

    match result {
        Ok(text) => Ok(text.trim().to_string()),
        Err(e) => {
            tracing::debug!("OCR failed (tesseract may not be installed): {e}");
            Ok(String::new())
        }
    }
}
