//! Quality metrics for medical image compression.
//!
//! This module provides tools to measure compression quality:
//! - **PSNR** (Peak Signal-to-Noise Ratio): Measures pixel-level fidelity
//! - **SSIM** (Structural Similarity Index): Measures perceptual quality
//!
//! # Example
//!
//! ```rust,ignore
//! use medimg_compress::metrics::{calculate_psnr, calculate_ssim, SsimConfig};
//!
//! let psnr_result = calculate_psnr(&original, &compressed)?;
//! println!("PSNR: {:.2} dB", psnr_result.psnr_db);
//!
//! let ssim_result = calculate_ssim(&original, &compressed, &SsimConfig::default())?;
//! println!("SSIM: {:.4}", ssim_result.ssim);
//! ```

mod psnr;
mod ssim;
mod comparator;

pub use psnr::{calculate_psnr, PsnrResult};
pub use ssim::{calculate_ssim, SsimConfig, SsimResult};
pub use comparator::{ImageComparator, QualityReport};

use crate::error::{MedImgError, Result};
use crate::ImageData;

/// Validate that two images can be compared.
pub(crate) fn validate_images(original: &ImageData, compressed: &ImageData) -> Result<()> {
    if original.width != compressed.width || original.height != compressed.height {
        return Err(MedImgError::ImageData(format!(
            "Image dimensions mismatch: {}x{} vs {}x{}",
            original.width, original.height, compressed.width, compressed.height
        )));
    }

    if original.bits_per_sample != compressed.bits_per_sample {
        return Err(MedImgError::ImageData(format!(
            "Bits per sample mismatch: {} vs {}",
            original.bits_per_sample, compressed.bits_per_sample
        )));
    }

    if original.samples_per_pixel != compressed.samples_per_pixel {
        return Err(MedImgError::ImageData(format!(
            "Samples per pixel mismatch: {} vs {}",
            original.samples_per_pixel, compressed.samples_per_pixel
        )));
    }

    if original.pixel_data.len() != compressed.pixel_data.len() {
        return Err(MedImgError::ImageData(format!(
            "Pixel data length mismatch: {} vs {}",
            original.pixel_data.len(),
            compressed.pixel_data.len()
        )));
    }

    Ok(())
}

/// Get the maximum possible pixel value for a given bit depth.
pub(crate) fn max_pixel_value(bits_per_sample: u16) -> f64 {
    ((1u64 << bits_per_sample) - 1) as f64
}

/// Extract pixel values as f64 from raw byte data.
pub(crate) fn extract_pixels(image: &ImageData) -> Vec<f64> {
    let bytes_per_sample = ((image.bits_per_sample + 7) / 8) as usize;
    let num_samples = image.pixel_data.len() / bytes_per_sample;
    let mut pixels = Vec::with_capacity(num_samples);

    if bytes_per_sample == 1 {
        for &byte in &image.pixel_data {
            pixels.push(byte as f64);
        }
    } else if bytes_per_sample == 2 {
        for i in 0..num_samples {
            let value = u16::from_le_bytes([
                image.pixel_data[i * 2],
                image.pixel_data[i * 2 + 1],
            ]);
            pixels.push(value as f64);
        }
    } else {
        // For other bit depths, handle as needed
        for i in 0..num_samples {
            let start = i * bytes_per_sample;
            let mut value: u64 = 0;
            for j in 0..bytes_per_sample {
                value |= (image.pixel_data[start + j] as u64) << (j * 8);
            }
            pixels.push(value as f64);
        }
    }

    pixels
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_image(width: u32, height: u32, bits: u16, value: u8) -> ImageData {
        let bytes_per_sample = ((bits + 7) / 8) as usize;
        let size = width as usize * height as usize * bytes_per_sample;
        let pixel_data = vec![value; size];

        ImageData {
            width,
            height,
            bits_per_sample: bits,
            samples_per_pixel: 1,
            pixel_data,
            photometric_interpretation: "MONOCHROME2".into(),
            is_signed: false,
        }
    }

    #[test]
    fn test_validate_images_matching() {
        let img1 = create_test_image(64, 64, 8, 128);
        let img2 = create_test_image(64, 64, 8, 100);
        assert!(validate_images(&img1, &img2).is_ok());
    }

    #[test]
    fn test_validate_images_dimension_mismatch() {
        let img1 = create_test_image(64, 64, 8, 128);
        let img2 = create_test_image(32, 32, 8, 128);
        assert!(validate_images(&img1, &img2).is_err());
    }

    #[test]
    fn test_max_pixel_value() {
        assert_eq!(max_pixel_value(8), 255.0);
        assert_eq!(max_pixel_value(12), 4095.0);
        assert_eq!(max_pixel_value(16), 65535.0);
    }

    #[test]
    fn test_extract_pixels_8bit() {
        let mut image = create_test_image(4, 4, 8, 0);
        for i in 0..16 {
            image.pixel_data[i] = i as u8;
        }
        let pixels = extract_pixels(&image);
        assert_eq!(pixels.len(), 16);
        for i in 0..16 {
            assert_eq!(pixels[i], i as f64);
        }
    }

    #[test]
    fn test_extract_pixels_16bit() {
        let mut image = ImageData {
            width: 2,
            height: 2,
            bits_per_sample: 16,
            samples_per_pixel: 1,
            pixel_data: vec![0; 8],
            photometric_interpretation: "MONOCHROME2".into(),
            is_signed: false,
        };
        // Set pixel values: 256, 512, 768, 1024
        image.pixel_data[0..2].copy_from_slice(&256u16.to_le_bytes());
        image.pixel_data[2..4].copy_from_slice(&512u16.to_le_bytes());
        image.pixel_data[4..6].copy_from_slice(&768u16.to_le_bytes());
        image.pixel_data[6..8].copy_from_slice(&1024u16.to_le_bytes());

        let pixels = extract_pixels(&image);
        assert_eq!(pixels, vec![256.0, 512.0, 768.0, 1024.0]);
    }
}
