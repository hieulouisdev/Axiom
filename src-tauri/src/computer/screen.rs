//! Screen capture / OCR.
//!
//! Phase 2: Uses `screenshots` crate for cross-platform capture
//! and `rusty_tesseract` for OCR text extraction.

use serde::{Deserialize, Serialize};

use crate::error::{AegisError, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Screenshot {
    pub width: u32,
    pub height: u32,
    /// Base64-encoded PNG bytes.
    pub png_base64: String,
    /// OCR-extracted text.
    pub ocr_text: String,
}

/// Capture the entire primary display.
pub fn screenshot() -> Result<Screenshot> {
    let screens = screenshots::Screen::all()
        .map_err(|e| AegisError::Internal(format!("screen enumeration: {e}")))?;

    let screen = screens.first()
        .ok_or_else(|| AegisError::Internal("no display found".into()))?;

    let image = screen.capture()
        .map_err(|e| AegisError::Internal(format!("screen capture: {e}")))?;

    let width = image.width();
    let height = image.height();

    // Encode to PNG
    let png_bytes = image.to_png()
        .map_err(|e| AegisError::Internal(format!("png encoding: {e}")))?;

    let png_base64 = base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        &png_bytes,
    );

    // OCR text extraction
    let ocr_text = extract_ocr_text(&png_bytes).unwrap_or_default();

    tracing::debug!("screenshot: {}x{}, {} bytes, OCR {} chars",
        width, height, png_bytes.len(), ocr_text.len());

    Ok(Screenshot {
        width,
        height,
        png_base64,
        ocr_text,
    })
}

/// Capture a specific area of the primary display.
pub fn screenshot_area(x: i32, y: i32, width: u32, height: u32) -> Result<Screenshot> {
    let screens = screenshots::Screen::all()
        .map_err(|e| AegisError::Internal(format!("screen enumeration: {e}")))?;

    let screen = screens.first()
        .ok_or_else(|| AegisError::Internal("no display found".into()))?;

    let image = screen.capture_area(x, y, width, height)
        .map_err(|e| AegisError::Internal(format!("area capture: {e}")))?;

    let img_width = image.width();
    let img_height = image.height();

    let png_bytes = image.to_png()
        .map_err(|e| AegisError::Internal(format!("png encoding: {e}")))?;

    let png_base64 = base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        &png_bytes,
    );

    let ocr_text = extract_ocr_text(&png_bytes).unwrap_or_default();

    Ok(Screenshot {
        width: img_width,
        height: img_height,
        png_base64,
        ocr_text,
    })
}

/// Extract text from a PNG image using Tesseract OCR.
fn extract_ocr_text(png_bytes: &[u8]) -> Result<String> {
    // Save to a temp file for tesseract
    let temp_dir = std::env::temp_dir();
    let temp_path = temp_dir.join(format!("aegis_ocr_{}.png",
        uuid::Uuid::new_v4().simple()));

    std::fs::write(&temp_path, png_bytes)
        .map_err(|e| AegisError::Io(format!("temp image write: {e}")))?;

    let result = rusty_tesseract::image_to_string(
        &temp_path.to_string_lossy(),
        &rusty_tesseract::Args::default(),
    );

    // Clean up temp file
    let _ = std::fs::remove_file(&temp_path);

    match result {
        Ok(text) => {
            let trimmed = text.trim().to_string();
            if trimmed.is_empty() {
                Ok(String::new())
            } else {
                Ok(trimmed)
            }
        }
        Err(e) => {
            tracing::debug!("OCR failed (tesseract may not be installed): {e}");
            Ok(String::new())
        }
    }
}
