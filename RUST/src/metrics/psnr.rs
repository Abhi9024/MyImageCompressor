//! PSNR (Peak Signal-to-Noise Ratio) calculation.
//!
//! PSNR is a common metric for measuring the quality of lossy compression.
//! Higher values indicate better quality (less distortion).
//!
//! - Lossless compression: PSNR = infinity (MSE = 0)
//! - High quality: PSNR > 40 dB
//! - Good quality: PSNR 30-40 dB
//! - Acceptable: PSNR 20-30 dB

use crate::error::Result;
use crate::ImageData;

use super::{extract_pixels, max_pixel_value, validate_images};

/// Result of PSNR calculation.
#[derive(Debug, Clone)]
pub struct PsnrResult {
    /// PSNR value in decibels (higher = better quality).
    /// Returns f64::INFINITY for identical images (lossless).
    pub psnr_db: f64,

    /// Mean Squared Error between images.
    /// 0.0 indicates identical images.
    pub mse: f64,

    /// Maximum possible pixel value (based on bit depth).
    pub max_value: f64,

    /// Per-component PSNR for multi-channel images (e.g., RGB).
    /// None for single-channel (grayscale) images.
    pub per_component: Option<Vec<f64>>,
}

impl PsnrResult {
    /// Check if the images are identical (lossless).
    pub fn is_lossless(&self) -> bool {
        self.mse == 0.0
    }

    /// Get a quality rating based on PSNR value.
    pub fn quality_rating(&self) -> &'static str {
        if self.psnr_db.is_infinite() {
            "Lossless (identical)"
        } else if self.psnr_db > 50.0 {
            "Excellent"
        } else if self.psnr_db > 40.0 {
            "Very Good"
        } else if self.psnr_db > 30.0 {
            "Good"
        } else if self.psnr_db > 20.0 {
            "Fair"
        } else {
            "Poor"
        }
    }
}

impl std::fmt::Display for PsnrResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.psnr_db.is_infinite() {
            write!(f, "PSNR: Infinity (lossless)")
        } else {
            write!(f, "PSNR: {:.2} dB ({})", self.psnr_db, self.quality_rating())
        }
    }
}

/// Calculate PSNR between original and compressed images.
///
/// # Arguments
///
/// * `original` - The original (reference) image
/// * `compressed` - The compressed (test) image
///
/// # Returns
///
/// A `PsnrResult` containing the PSNR value, MSE, and related metrics.
///
/// # Errors
///
/// Returns an error if the images have different dimensions or formats.
///
/// # Example
///
/// ```rust,ignore
/// use medimg_compress::metrics::calculate_psnr;
///
/// let result = calculate_psnr(&original_image, &compressed_image)?;
/// println!("PSNR: {:.2} dB", result.psnr_db);
/// println!("MSE: {:.6}", result.mse);
/// ```
pub fn calculate_psnr(original: &ImageData, compressed: &ImageData) -> Result<PsnrResult> {
    validate_images(original, compressed)?;

    let max_value = max_pixel_value(original.bits_per_sample);
    let original_pixels = extract_pixels(original);
    let compressed_pixels = extract_pixels(compressed);

    // Calculate per-component PSNR for multi-channel images
    let per_component = if original.samples_per_pixel > 1 {
        let samples = original.samples_per_pixel as usize;
        let mut component_psnrs = Vec::with_capacity(samples);

        for c in 0..samples {
            let mse = calculate_component_mse(&original_pixels, &compressed_pixels, samples, c);
            let psnr = if mse == 0.0 {
                f64::INFINITY
            } else {
                10.0 * (max_value * max_value / mse).log10()
            };
            component_psnrs.push(psnr);
        }

        Some(component_psnrs)
    } else {
        None
    };

    // Calculate overall MSE
    let mse = calculate_mse(&original_pixels, &compressed_pixels);

    // Calculate PSNR
    let psnr_db = if mse == 0.0 {
        f64::INFINITY
    } else {
        10.0 * (max_value * max_value / mse).log10()
    };

    Ok(PsnrResult {
        psnr_db,
        mse,
        max_value,
        per_component,
    })
}

/// Calculate Mean Squared Error between two pixel arrays.
fn calculate_mse(original: &[f64], compressed: &[f64]) -> f64 {
    if original.is_empty() {
        return 0.0;
    }

    let sum: f64 = original
        .iter()
        .zip(compressed.iter())
        .map(|(o, c)| {
            let diff = o - c;
            diff * diff
        })
        .sum();

    sum / original.len() as f64
}

/// Calculate MSE for a specific component in multi-channel images.
fn calculate_component_mse(
    original: &[f64],
    compressed: &[f64],
    num_components: usize,
    component: usize,
) -> f64 {
    let pixels_per_component = original.len() / num_components;
    if pixels_per_component == 0 {
        return 0.0;
    }

    let sum: f64 = (0..pixels_per_component)
        .map(|i| {
            let idx = i * num_components + component;
            let diff = original[idx] - compressed[idx];
            diff * diff
        })
        .sum();

    sum / pixels_per_component as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_image(width: u32, height: u32, bits: u16, values: Vec<u8>) -> ImageData {
        ImageData {
            width,
            height,
            bits_per_sample: bits,
            samples_per_pixel: 1,
            pixel_data: values,
            photometric_interpretation: "MONOCHROME2".into(),
            is_signed: false,
        }
    }

    #[test]
    fn test_psnr_identical_images() {
        let data = vec![128u8; 64 * 64];
        let img1 = create_test_image(64, 64, 8, data.clone());
        let img2 = create_test_image(64, 64, 8, data);

        let result = calculate_psnr(&img1, &img2).unwrap();
        assert!(result.psnr_db.is_infinite());
        assert_eq!(result.mse, 0.0);
        assert!(result.is_lossless());
    }

    #[test]
    fn test_psnr_different_images() {
        let data1: Vec<u8> = (0..64 * 64).map(|_| 100).collect();
        let data2: Vec<u8> = (0..64 * 64).map(|_| 110).collect();
        let img1 = create_test_image(64, 64, 8, data1);
        let img2 = create_test_image(64, 64, 8, data2);

        let result = calculate_psnr(&img1, &img2).unwrap();
        assert!(!result.psnr_db.is_infinite());
        assert!(result.mse > 0.0);
        // MSE should be (100-110)^2 = 100
        assert!((result.mse - 100.0).abs() < 0.001);
        // PSNR = 10 * log10(255^2 / 100) = 10 * log10(650.25) ≈ 28.13 dB
        assert!(result.psnr_db > 28.0 && result.psnr_db < 29.0);
    }

    #[test]
    fn test_psnr_16bit_images() {
        let mut data1 = vec![0u8; 32 * 32 * 2];
        let mut data2 = vec![0u8; 32 * 32 * 2];

        // Set all pixels to 1000 in image 1
        for i in 0..(32 * 32) {
            let bytes = 1000u16.to_le_bytes();
            data1[i * 2] = bytes[0];
            data1[i * 2 + 1] = bytes[1];
        }

        // Set all pixels to 1100 in image 2
        for i in 0..(32 * 32) {
            let bytes = 1100u16.to_le_bytes();
            data2[i * 2] = bytes[0];
            data2[i * 2 + 1] = bytes[1];
        }

        let img1 = ImageData {
            width: 32,
            height: 32,
            bits_per_sample: 16,
            samples_per_pixel: 1,
            pixel_data: data1,
            photometric_interpretation: "MONOCHROME2".into(),
            is_signed: false,
        };

        let img2 = ImageData {
            width: 32,
            height: 32,
            bits_per_sample: 16,
            samples_per_pixel: 1,
            pixel_data: data2,
            photometric_interpretation: "MONOCHROME2".into(),
            is_signed: false,
        };

        let result = calculate_psnr(&img1, &img2).unwrap();
        assert!(!result.psnr_db.is_infinite());
        assert_eq!(result.max_value, 65535.0);
        // MSE = (1000-1100)^2 = 10000
        assert!((result.mse - 10000.0).abs() < 0.001);
    }

    #[test]
    fn test_psnr_quality_ratings() {
        let result_excellent = PsnrResult {
            psnr_db: 55.0,
            mse: 0.001,
            max_value: 255.0,
            per_component: None,
        };
        assert_eq!(result_excellent.quality_rating(), "Excellent");

        let result_poor = PsnrResult {
            psnr_db: 15.0,
            mse: 100.0,
            max_value: 255.0,
            per_component: None,
        };
        assert_eq!(result_poor.quality_rating(), "Poor");
    }

    #[test]
    fn test_calculate_mse() {
        let original = vec![100.0, 100.0, 100.0, 100.0];
        let compressed = vec![110.0, 110.0, 110.0, 110.0];
        let mse = calculate_mse(&original, &compressed);
        assert!((mse - 100.0).abs() < 0.001);
    }
}
