use std::io::Cursor;

use base64::{Engine, prelude::BASE64_STANDARD};
use image::{ImageFormat, Luma};
use qrcode::{EcLevel, QrCode};
use serde::Serialize;

#[derive(Debug, thiserror::Error)]
pub(crate) enum QrError {
    #[error(transparent)]
    Encode(#[from] qrcode::types::QrError),

    #[error(transparent)]
    Image(#[from] image::ImageError),

    #[error(transparent)]
    Serialize(#[from] serde_json::Error),
}

#[derive(Serialize)]
struct MobileActivation<'a> {
    url: &'a str,
    token: &'a str,
}

/// Construct QR with content bytes and return a buffer of PNG image.
pub(crate) fn qr_png(content: &[u8]) -> Result<Vec<u8>, QrError> {
    let code = QrCode::with_error_correction_level(content, EcLevel::M)?;

    let image_buffer = code
        .render::<Luma<u8>>()
        .min_dimensions(400, 400)
        .quiet_zone(true)
        .build();

    let mut buffer = Cursor::new(Vec::new());

    image_buffer.write_to(&mut buffer, ImageFormat::Png)?;

    Ok(buffer.into_inner())
}

pub(crate) fn mobile_activation_qr_data(url: &str, token: &str) -> Result<String, QrError> {
    let payload = serde_json::to_string(&MobileActivation { url, token })?;

    Ok(BASE64_STANDARD.encode(payload))
}
