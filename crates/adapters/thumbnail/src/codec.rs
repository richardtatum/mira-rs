use image::codecs::jpeg::JpegEncoder;
use image::imageops::FilterType;
use image::{DynamicImage, ImageBuffer, Rgb};
use openh264::decoder::Decoder;
use openh264::formats::YUVSource;

pub fn decode_and_encode(h264_data: &[u8]) -> Result<Vec<u8>, crate::ThumbnailError> {
    // Decode the raw H.264 Annex B bytes into a YUV frame
    let mut decoder = Decoder::new().map_err(|e| crate::ThumbnailError::Decode(e.to_string()))?;

    let yuv = decoder
        .decode(h264_data)
        .map_err(|e| crate::ThumbnailError::Decode(e.to_string()))?
        .ok_or_else(|| crate::ThumbnailError::Decode("decoder produced no frame".into()))?;

    // Convert YUV to a packed RGB buffer
    let (width, height) = yuv.dimensions();
    let mut rgb = vec![0u8; width * height * 3];
    yuv.write_rgb8(&mut rgb);

    // Wrap the RGB bytes in an image type so we can resize and encode
    let img = ImageBuffer::<Rgb<u8>, Vec<u8>>::from_raw(width as u32, height as u32, rgb)
        .ok_or_else(|| crate::ThumbnailError::Decode("invalid frame dimensions".into()))?;

    // Downscale to max 640px wide, preserving aspect ratio
    let resized = DynamicImage::ImageRgb8(img).resize(640, u32::MAX, FilterType::Triangle);

    // Encode to JPEG at quality 75 into an in-memory buffer
    let mut buf = Vec::new();
    let mut cursor = std::io::Cursor::new(&mut buf);
    resized
        .write_with_encoder(JpegEncoder::new_with_quality(&mut cursor, 75))
        .map_err(|e| crate::ThumbnailError::Decode(e.to_string()))?;

    Ok(buf)
}

#[cfg(test)]
mod tests {
    use image::codecs::jpeg::JpegEncoder;
    use image::imageops::FilterType;
    use image::{DynamicImage, ImageBuffer, Rgb};

    #[test]
    fn resize_and_encode_produces_valid_jpeg() {
        // Build a synthetic 1920x1080 RGB image (bypasses openh264, tests resize+encode path)
        let raw = vec![128u8; 1920 * 1080 * 3];
        let img = ImageBuffer::<Rgb<u8>, Vec<u8>>::from_raw(1920, 1080, raw).unwrap();
        let dynamic = DynamicImage::ImageRgb8(img);
        let resized = dynamic.resize(640, u32::MAX, FilterType::Triangle);
        assert!(resized.width() == 640);

        let mut buf = Vec::new();
        let mut cursor = std::io::Cursor::new(&mut buf);
        resized
            .write_with_encoder(JpegEncoder::new_with_quality(&mut cursor, 75))
            .unwrap();

        // JPEG magic bytes: FF D8
        assert!(buf.starts_with(&[0xFF, 0xD8]));
        assert!(!buf.is_empty());
    }
}
