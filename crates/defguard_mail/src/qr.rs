use std::io::Cursor;

use image::ImageFormat;
use qrforge::{ErrorCorrection, Mode, QRCode, QRError, Version};

/// Construct QR with content bytes and return a buffer of PNG image.
pub(crate) fn qr_png(content: &[u8]) -> Result<Vec<u8>, QRError> {
    let qr = QRCode::builder()
        .add_segment(Some(Mode::Byte), content)
        .error_correction(ErrorCorrection::M)
        .version(Version::V(5))
        .build()?;

    let image_buffer = qr
        .image_builder()
        .set_width(200)
        .set_height(200)
        .set_border(4)
        .build_image()?;

    let mut buffer = Cursor::new(Vec::new());

    image_buffer
        .write_to(&mut buffer, ImageFormat::Png)
        .map_err(|_| QRError::new("image write error"))?;

    Ok(buffer.into_inner())
}
