//! PNG and JPEG screenshot encoding.
use image::ImageEncoder;
use std::error::Error;
use std::fs::File;
use std::io::BufWriter;
use std::path::Path;

pub(crate) fn write_png(path: &Path, size: [u32; 2], rgba: &[u8]) -> Result<(), Box<dyn Error>> {
    let file = File::create(path)?;
    let encoder = image::codecs::png::PngEncoder::new(BufWriter::new(file));
    encoder.write_image(rgba, size[0], size[1], image::ExtendedColorType::Rgba8)?;
    Ok(())
}

/// JPEG has no alpha channel: the encoder refuses RGBA, so the alpha is dropped first.
pub(crate) fn write_jpeg(path: &Path, size: [u32; 2], rgba: &[u8]) -> Result<(), Box<dyn Error>> {
    let rgb: Vec<u8> = rgba
        .chunks_exact(4)
        .flat_map(|pixel| [pixel[0], pixel[1], pixel[2]])
        .collect();
    let file = File::create(path)?;
    let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(BufWriter::new(file), 95);
    encoder.write_image(&rgb, size[0], size[1], image::ExtendedColorType::Rgb8)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn jpeg_screenshots_drop_the_alpha() {
        let path = std::env::temp_dir().join(format!("sjk-capture-{}.jpg", std::process::id()));
        let rgba: Vec<u8> = (0..4 * 4)
            .flat_map(|i| [i as u8 * 16, 128, 255, 255])
            .collect();
        super::write_jpeg(&path, [4, 4], &rgba).expect("writes a jpeg");
        let image = image::open(&path).expect("decodes");
        assert_eq!((image.width(), image.height()), (4, 4));
        let _ = std::fs::remove_file(&path);
    }
}
