//! Screen capture / OCR.
//!
//! v0.1 returns a placeholder PNG path. Phase 2 will integrate the
//! `screenshots` crate for cross-platform capture and `tesseract` /
//! `rusty-tesseract` for OCR text extraction.

use serde::{Deserialize, Serialize};

use crate::error::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Screenshot {
    pub width: u32,
    pub height: u32,
    /// Base64-encoded PNG bytes.
    pub png_base64: String,
    /// OCR-extracted text (empty until Phase 2).
    pub ocr_text: String,
}

/// Capture the entire primary display.
pub fn screenshot() -> Result<Screenshot> {
    // Phase 2: integrate `screenshots` crate.
    // For v0.1, return a 1x1 transparent placeholder PNG so the IPC works.
    let png = tiny_placeholder_png();
    Ok(Screenshot {
        width: 1,
        height: 1,
        png_base64: base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &png),
        ocr_text: String::new(),
    })
}

/// Returns a 67-byte 1x1 transparent PNG. Used only as a placeholder
/// during v0.1; replaced with real capture in Phase 2.
fn tiny_placeholder_png() -> Vec<u8> {
    vec![
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, // PNG signature
        0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52, // IHDR chunk
        0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, // 1x1
        0x08, 0x06, 0x00, 0x00, 0x00, 0x1F, 0x15, 0xC4, // RGBA8
        0x89, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x44, 0x41, // IDAT chunk
        0x54, 0x78, 0x9C, 0x62, 0x00, 0x01, 0x00, 0x00, // zlib data
        0x05, 0x00, 0x01, 0x0D, 0x0A, 0x2D, 0xB4, 0x00,
        0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE, // IEND chunk
        0x42, 0x60, 0x82,
    ]
}
