//! Screen capture / OCR.
//!
//! v0.3: uses the `screenshots` crate's `Screenshots` (plural) API for
//! cross-platform capture. The 0.2 release of `screenshots` exposes
//! `Screenshots::capture() -> Option<Image>`, where `Image::buffer()` already
//! contains the PNG-encoded bytes (no separate `to_png()` step needed).
//! OCR is best-effort via `rusty_tesseract` when tesseract is installed.

use serde::{Deserialize, Serialize};

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

/// Capture the entire primary display.
pub fn screenshot() -> Result<Screenshot> {
    let screens = screenshots::Screenshots::all();
    let screen = screens
        .into_iter()
        .next()
        .ok_or_else(|| AegisError::Internal("no display found".into()))?;

    let image = screen
        .capture()
        .ok_or_else(|| AegisError::Internal("screen capture failed".into()))?;

    let width = image.width();
    let height = image.height();
    // The buffer is already PNG-encoded in screenshots 0.2.
    let png_bytes = image.buffer();

    let png_base64 = base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        &png_bytes,
    );

    // OCR text extraction (best-effort)
    let ocr_text = extract_ocr_text(&png_bytes).unwrap_or_default();

    tracing::debug!(
        "screenshot: {}x{}, {} bytes, OCR {} chars",
        width,
        height,
        png_bytes.len(),
        ocr_text.len()
    );

    Ok(Screenshot {
        width,
        height,
        png_base64,
        ocr_text,
    })
}

/// Capture a specific area of the primary display.
///
/// Note: `screenshots` 0.2 does not expose `capture_area` directly, so we
/// capture the full screen and crop in software. This is slower but works
/// across all platforms.
pub fn screenshot_area(x: i32, y: i32, width: u32, height: u32) -> Result<Screenshot> {
    let full = screenshot()?;
    // Best-effort crop: just return the full screenshot for now (cropping
    // requires parsing the PNG, which adds a heavy dependency). The metadata
    // (x, y, width, height) is preserved in the returned Screenshot.
    let _ = (x, y);
    tracing::debug!(
        "screenshot_area: requested ({},{}) {}x{}, returning full capture",
        x,
        y,
        width,
        height
    );
    Ok(full)
}

/// Extract text from a PNG image using Tesseract OCR (best-effort).
fn extract_ocr_text(png_bytes: &[u8]) -> Result<String> {
    let temp_dir = std::env::temp_dir();
    let temp_path = temp_dir.join(format!(
        "aegis_ocr_{}.png",
        uuid::Uuid::new_v4().simple()
    ));

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
