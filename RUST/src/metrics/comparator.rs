//! Image comparator for comprehensive quality analysis.
//!
//! Combines multiple quality metrics (PSNR, SSIM, and error statistics)
//! into a unified quality report.

use crate::error::Result;
use crate::ImageData;

use super::{calculate_psnr, calculate_ssim, extract_pixels, PsnrResult, SsimConfig, SsimResult};

/// Comprehensive quality report combining multiple metrics.
#[derive(Debug, Clone)]
pub struct QualityReport {
    /// PSNR analysis result.
    pub psnr: PsnrResult,

    /// SSIM analysis result.
    pub ssim: SsimResult,

    /// Maximum absolute difference between any two pixels.
    pub max_error: u64,

    /// Mean absolute difference between pixels.
    pub mean_error: f64,

    /// Root Mean Square Error.
    pub rmse: f64,

    /// Percentage of pixels that differ (0-100).
    pub diff_pixels_percent: f64,

    /// Number of pixels that differ.
    pub diff_pixel_count: usize,

    /// Total number of pixels compared.
    pub total_pixels: usize,
}

impl QualityReport {
    /// Check if compression was lossless (no pixel differences).
    pub fn is_lossless(&self) -> bool {
        self.diff_pixel_count == 0
    }

    /// Get an overall quality summary.
    pub fn overall_quality(&self) -> &'static str {
        if self.is_lossless() {
            return "Lossless (identical)";
        }

        // Weight SSIM more heavily as it's more perceptually relevant
        if self.ssim.ssim >= 0.99 && self.psnr.psnr_db >= 45.0 {
            "Excellent"
        } else if self.ssim.ssim >= 0.95 && self.psnr.psnr_db >= 40.0 {
            "Very Good"
        } else if self.ssim.ssim >= 0.90 && self.psnr.psnr_db >= 35.0 {
            "Good"
        } else if self.ssim.ssim >= 0.80 && self.psnr.psnr_db >= 30.0 {
            "Acceptable"
        } else if self.ssim.ssim >= 0.60 {
            "Fair"
        } else {
            "Poor"
        }
    }

    /// Check if quality meets diagnostic requirements.
    ///
    /// For medical imaging, typical diagnostic quality requires:
    /// - SSIM > 0.98 for lossy compression
    /// - PSNR > 40 dB
    pub fn meets_diagnostic_quality(&self) -> bool {
        self.is_lossless() || (self.ssim.ssim >= 0.98 && self.psnr.psnr_db >= 40.0)
    }
}

impl std::fmt::Display for QualityReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Quality Report")?;
        writeln!(f, "==============")?;
        writeln!(f, "Overall: {}", self.overall_quality())?;
        writeln!(f)?;
        writeln!(f, "{}", self.psnr)?;
        writeln!(f, "{}", self.ssim)?;
        writeln!(f)?;
        writeln!(f, "Error Statistics:")?;
        writeln!(f, "  Max Error: {}", self.max_error)?;
        writeln!(f, "  Mean Error: {:.4}", self.mean_error)?;
        writeln!(f, "  RMSE: {:.4}", self.rmse)?;
        writeln!(
            f,
            "  Different Pixels: {} / {} ({:.2}%)",
            self.diff_pixel_count, self.total_pixels, self.diff_pixels_percent
        )?;
        if self.meets_diagnostic_quality() {
            writeln!(f)?;
            writeln!(f, "✓ Meets diagnostic quality requirements")?;
        }
        Ok(())
    }
}

/// Utility for comparing original and compressed images.
#[derive(Debug, Clone)]
pub struct ImageComparator {
    /// SSIM configuration.
    ssim_config: SsimConfig,
}

impl Default for ImageComparator {
    fn default() -> Self {
        Self::new()
    }
}

impl ImageComparator {
    /// Create a new image comparator with default settings.
    pub fn new() -> Self {
        Self {
            ssim_config: SsimConfig::default(),
        }
    }

    /// Create a comparator with custom SSIM configuration.
    pub fn with_ssim_config(ssim_config: SsimConfig) -> Self {
        Self { ssim_config }
    }

    /// Set SSIM configuration.
    pub fn ssim_config(mut self, config: SsimConfig) -> Self {
        self.ssim_config = config;
        self
    }

    /// Compare two images and generate a comprehensive quality report.
    ///
    /// # Arguments
    ///
    /// * `original` - The original (reference) image
    /// * `compressed` - The compressed (test) image
    ///
    /// # Returns
    ///
    /// A `QualityReport` containing all quality metrics.
    ///
    /// # Errors
    ///
    /// Returns an error if the images have different dimensions or formats.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use medimg_compress::metrics::ImageComparator;
    ///
    /// let comparator = ImageComparator::new();
    /// let report = comparator.compare(&original, &compressed)?;
    /// println!("{}", report);
    /// ```
    pub fn compare(&self, original: &ImageData, compressed: &ImageData) -> Result<QualityReport> {
        // Calculate PSNR and SSIM
        let psnr = calculate_psnr(original, compressed)?;
        let ssim = calculate_ssim(original, compressed, &self.ssim_config)?;

        // Calculate error statistics
        let original_pixels = extract_pixels(original);
        let compressed_pixels = extract_pixels(compressed);
        let error_stats = calculate_error_statistics(&original_pixels, &compressed_pixels);

        Ok(QualityReport {
            psnr,
            ssim,
            max_error: error_stats.max_error,
            mean_error: error_stats.mean_error,
            rmse: error_stats.rmse,
            diff_pixels_percent: error_stats.diff_percent,
            diff_pixel_count: error_stats.diff_count,
            total_pixels: original_pixels.len(),
        })
    }

    /// Quick comparison that only calculates PSNR (faster than full comparison).
    pub fn quick_compare(&self, original: &ImageData, compressed: &ImageData) -> Result<PsnrResult> {
        calculate_psnr(original, compressed)
    }

    /// Check if two images are identical (lossless comparison).
    pub fn is_identical(&self, original: &ImageData, compressed: &ImageData) -> Result<bool> {
        if original.pixel_data.len() != compressed.pixel_data.len() {
            return Ok(false);
        }
        Ok(original.pixel_data == compressed.pixel_data)
    }
}

/// Error statistics calculated between two images.
struct ErrorStatistics {
    max_error: u64,
    mean_error: f64,
    rmse: f64,
    diff_count: usize,
    diff_percent: f64,
}

/// Calculate error statistics between two pixel arrays.
fn calculate_error_statistics(original: &[f64], compressed: &[f64]) -> ErrorStatistics {
    if original.is_empty() {
        return ErrorStatistics {
            max_error: 0,
            mean_error: 0.0,
            rmse: 0.0,
            diff_count: 0,
            diff_percent: 0.0,
        };
    }

    let mut max_error: u64 = 0;
    let mut sum_abs_error: f64 = 0.0;
    let mut sum_sq_error: f64 = 0.0;
    let mut diff_count: usize = 0;

    for (o, c) in original.iter().zip(compressed.iter()) {
        let diff = (o - c).abs();
        let diff_u64 = diff as u64;

        if diff_u64 > 0 {
            diff_count += 1;
        }

        max_error = max_error.max(diff_u64);
        sum_abs_error += diff;
        sum_sq_error += diff * diff;
    }

    let n = original.len() as f64;
    let mean_error = sum_abs_error / n;
    let rmse = (sum_sq_error / n).sqrt();
    let diff_percent = (diff_count as f64 / n) * 100.0;

    ErrorStatistics {
        max_error,
        mean_error,
        rmse,
        diff_count,
        diff_percent,
    }
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
    fn test_comparator_identical_images() {
        let data = vec![128u8; 64 * 64];
        let img1 = create_test_image(64, 64, 8, data.clone());
        let img2 = create_test_image(64, 64, 8, data);

        let comparator = ImageComparator::new();
        let report = comparator.compare(&img1, &img2).unwrap();

        assert!(report.is_lossless());
        assert_eq!(report.max_error, 0);
        assert_eq!(report.diff_pixel_count, 0);
        assert!(report.psnr.is_lossless());
        assert!(report.meets_diagnostic_quality());
    }

    #[test]
    fn test_comparator_different_images() {
        let data1: Vec<u8> = (0..64 * 64).map(|_| 100).collect();
        let data2: Vec<u8> = (0..64 * 64).map(|_| 105).collect();
        let img1 = create_test_image(64, 64, 8, data1);
        let img2 = create_test_image(64, 64, 8, data2);

        let comparator = ImageComparator::new();
        let report = comparator.compare(&img1, &img2).unwrap();

        assert!(!report.is_lossless());
        assert_eq!(report.max_error, 5);
        assert_eq!(report.mean_error, 5.0);
        assert_eq!(report.diff_pixel_count, 64 * 64);
        assert!((report.diff_pixels_percent - 100.0).abs() < 0.001);
    }

    #[test]
    fn test_comparator_partial_difference() {
        let mut data1 = vec![100u8; 64 * 64];
        let mut data2 = vec![100u8; 64 * 64];

        // Make half the pixels different
        for i in 0..(64 * 32) {
            data2[i] = 110;
        }

        let img1 = create_test_image(64, 64, 8, data1);
        let img2 = create_test_image(64, 64, 8, data2);

        let comparator = ImageComparator::new();
        let report = comparator.compare(&img1, &img2).unwrap();

        assert!(!report.is_lossless());
        assert_eq!(report.max_error, 10);
        assert_eq!(report.diff_pixel_count, 64 * 32);
        assert!((report.diff_pixels_percent - 50.0).abs() < 0.001);
    }

    #[test]
    fn test_is_identical() {
        let data = vec![128u8; 64 * 64];
        let img1 = create_test_image(64, 64, 8, data.clone());
        let img2 = create_test_image(64, 64, 8, data.clone());
        let img3 = create_test_image(64, 64, 8, vec![100u8; 64 * 64]);

        let comparator = ImageComparator::new();
        assert!(comparator.is_identical(&img1, &img2).unwrap());
        assert!(!comparator.is_identical(&img1, &img3).unwrap());
    }

    #[test]
    fn test_quick_compare() {
        let data1: Vec<u8> = (0..64 * 64).map(|_| 100).collect();
        let data2: Vec<u8> = (0..64 * 64).map(|_| 110).collect();
        let img1 = create_test_image(64, 64, 8, data1);
        let img2 = create_test_image(64, 64, 8, data2);

        let comparator = ImageComparator::new();
        let psnr = comparator.quick_compare(&img1, &img2).unwrap();

        assert!(!psnr.is_lossless());
        assert!(psnr.psnr_db > 0.0);
    }

    #[test]
    fn test_quality_report_display() {
        let data = vec![128u8; 32 * 32];
        let img1 = create_test_image(32, 32, 8, data.clone());
        let img2 = create_test_image(32, 32, 8, data);

        let comparator = ImageComparator::new();
        let report = comparator.compare(&img1, &img2).unwrap();

        let display = format!("{}", report);
        assert!(display.contains("Quality Report"));
        assert!(display.contains("Overall:"));
        assert!(display.contains("PSNR:"));
        assert!(display.contains("SSIM:"));
    }

    #[test]
    fn test_overall_quality_ratings() {
        // Create a lossless report
        let data = vec![128u8; 32 * 32];
        let img = create_test_image(32, 32, 8, data.clone());

        let comparator = ImageComparator::new();
        let report = comparator.compare(&img, &img).unwrap();
        assert_eq!(report.overall_quality(), "Lossless (identical)");
    }
}
