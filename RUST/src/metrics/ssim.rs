//! SSIM (Structural Similarity Index) calculation.
//!
//! SSIM is a perceptual metric that measures structural similarity between images.
//! It considers luminance, contrast, and structure, making it more aligned with
//! human visual perception than PSNR.
//!
//! - SSIM = 1.0: Identical images
//! - SSIM > 0.95: Excellent quality (nearly imperceptible difference)
//! - SSIM > 0.90: Good quality
//! - SSIM > 0.80: Acceptable quality

use crate::error::Result;
use crate::ImageData;

use super::{extract_pixels, max_pixel_value, validate_images};

/// Configuration for SSIM calculation.
#[derive(Debug, Clone)]
pub struct SsimConfig {
    /// Window size for local statistics (default: 11).
    /// Larger windows are more stable but less sensitive to local differences.
    pub window_size: usize,

    /// K1 constant for luminance comparison (default: 0.01).
    /// Stabilizes division when luminance is close to zero.
    pub k1: f64,

    /// K2 constant for contrast comparison (default: 0.03).
    /// Stabilizes division when contrast is close to zero.
    pub k2: f64,

    /// Whether to generate a spatial SSIM map.
    /// Useful for visualizing where quality degradation occurs.
    pub generate_map: bool,
}

impl Default for SsimConfig {
    fn default() -> Self {
        Self {
            window_size: 11,
            k1: 0.01,
            k2: 0.03,
            generate_map: false,
        }
    }
}

impl SsimConfig {
    /// Create a new configuration with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set window size.
    pub fn window_size(mut self, size: usize) -> Self {
        self.window_size = size;
        self
    }

    /// Enable SSIM map generation.
    pub fn with_map(mut self) -> Self {
        self.generate_map = true;
        self
    }
}

/// Result of SSIM calculation.
#[derive(Debug, Clone)]
pub struct SsimResult {
    /// SSIM value (0.0 to 1.0, where 1.0 = identical).
    pub ssim: f64,

    /// SSIM map showing local similarity across the image.
    /// Only populated if `config.generate_map` is true.
    pub ssim_map: Option<Vec<f64>>,

    /// Map dimensions (width, height) if map is generated.
    pub map_dimensions: Option<(usize, usize)>,

    /// Per-component SSIM for multi-channel images.
    /// None for single-channel (grayscale) images.
    pub per_component: Option<Vec<f64>>,

    /// Luminance comparison component.
    pub luminance: f64,

    /// Contrast comparison component.
    pub contrast: f64,

    /// Structure comparison component.
    pub structure: f64,
}

impl SsimResult {
    /// Check if images are structurally identical.
    pub fn is_identical(&self) -> bool {
        (self.ssim - 1.0).abs() < f64::EPSILON
    }

    /// Get a quality rating based on SSIM value.
    pub fn quality_rating(&self) -> &'static str {
        if self.ssim >= 0.999 {
            "Excellent (visually lossless)"
        } else if self.ssim >= 0.95 {
            "Very Good"
        } else if self.ssim >= 0.90 {
            "Good"
        } else if self.ssim >= 0.80 {
            "Fair"
        } else if self.ssim >= 0.60 {
            "Poor"
        } else {
            "Very Poor"
        }
    }
}

impl std::fmt::Display for SsimResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SSIM: {:.4} ({})", self.ssim, self.quality_rating())
    }
}

/// Calculate SSIM between original and compressed images.
///
/// # Arguments
///
/// * `original` - The original (reference) image
/// * `compressed` - The compressed (test) image
/// * `config` - SSIM calculation configuration
///
/// # Returns
///
/// An `SsimResult` containing the SSIM value and optional spatial map.
///
/// # Errors
///
/// Returns an error if the images have different dimensions or formats.
///
/// # Example
///
/// ```rust,ignore
/// use medimg_compress::metrics::{calculate_ssim, SsimConfig};
///
/// let config = SsimConfig::default().with_map();
/// let result = calculate_ssim(&original, &compressed, &config)?;
/// println!("SSIM: {:.4}", result.ssim);
/// ```
pub fn calculate_ssim(
    original: &ImageData,
    compressed: &ImageData,
    config: &SsimConfig,
) -> Result<SsimResult> {
    validate_images(original, compressed)?;

    let width = original.width as usize;
    let height = original.height as usize;
    let max_value = max_pixel_value(original.bits_per_sample);

    let original_pixels = extract_pixels(original);
    let compressed_pixels = extract_pixels(compressed);

    // Calculate C1 and C2 constants
    let c1 = (config.k1 * max_value).powi(2);
    let c2 = (config.k2 * max_value).powi(2);

    // Per-component SSIM for multi-channel images
    let per_component = if original.samples_per_pixel > 1 {
        let samples = original.samples_per_pixel as usize;
        let mut component_ssims = Vec::with_capacity(samples);

        for c in 0..samples {
            let orig_component: Vec<f64> = original_pixels
                .iter()
                .enumerate()
                .filter(|(i, _)| i % samples == c)
                .map(|(_, &v)| v)
                .collect();
            let comp_component: Vec<f64> = compressed_pixels
                .iter()
                .enumerate()
                .filter(|(i, _)| i % samples == c)
                .map(|(_, &v)| v)
                .collect();

            let (ssim, _, _, _) = compute_ssim_components(
                &orig_component,
                &comp_component,
                width,
                height,
                config.window_size,
                c1,
                c2,
                false,
            );
            component_ssims.push(ssim);
        }

        Some(component_ssims)
    } else {
        None
    };

    // Calculate overall SSIM
    let (ssim, luminance, contrast, structure) = compute_ssim_components(
        &original_pixels,
        &compressed_pixels,
        width,
        height,
        config.window_size,
        c1,
        c2,
        false,
    );

    // Generate SSIM map if requested
    let (ssim_map, map_dimensions) = if config.generate_map && original.samples_per_pixel == 1 {
        let map = generate_ssim_map(
            &original_pixels,
            &compressed_pixels,
            width,
            height,
            config.window_size,
            c1,
            c2,
        );
        let map_width = width.saturating_sub(config.window_size - 1);
        let map_height = height.saturating_sub(config.window_size - 1);
        (Some(map), Some((map_width, map_height)))
    } else {
        (None, None)
    };

    Ok(SsimResult {
        ssim,
        ssim_map,
        map_dimensions,
        per_component,
        luminance,
        contrast,
        structure,
    })
}

/// Compute SSIM components using sliding window approach.
fn compute_ssim_components(
    original: &[f64],
    compressed: &[f64],
    width: usize,
    height: usize,
    window_size: usize,
    c1: f64,
    c2: f64,
    _return_map: bool,
) -> (f64, f64, f64, f64) {
    // For small images, use global statistics
    if width < window_size || height < window_size {
        return compute_global_ssim(original, compressed, c1, c2);
    }

    let mut total_ssim = 0.0;
    let mut total_luminance = 0.0;
    let mut total_contrast = 0.0;
    let mut total_structure = 0.0;
    let mut count = 0;

    // Sliding window
    for y in 0..=(height - window_size) {
        for x in 0..=(width - window_size) {
            let (ssim, lum, con, str) =
                compute_window_ssim(original, compressed, width, x, y, window_size, c1, c2);
            total_ssim += ssim;
            total_luminance += lum;
            total_contrast += con;
            total_structure += str;
            count += 1;
        }
    }

    if count == 0 {
        return compute_global_ssim(original, compressed, c1, c2);
    }

    (
        total_ssim / count as f64,
        total_luminance / count as f64,
        total_contrast / count as f64,
        total_structure / count as f64,
    )
}

/// Compute SSIM for a single window.
fn compute_window_ssim(
    original: &[f64],
    compressed: &[f64],
    width: usize,
    x: usize,
    y: usize,
    window_size: usize,
    c1: f64,
    c2: f64,
) -> (f64, f64, f64, f64) {
    let mut orig_sum = 0.0;
    let mut comp_sum = 0.0;
    let mut orig_sq_sum = 0.0;
    let mut comp_sq_sum = 0.0;
    let mut cross_sum = 0.0;
    let n = (window_size * window_size) as f64;

    for wy in 0..window_size {
        for wx in 0..window_size {
            let idx = (y + wy) * width + (x + wx);
            let o = original[idx];
            let c = compressed[idx];

            orig_sum += o;
            comp_sum += c;
            orig_sq_sum += o * o;
            comp_sq_sum += c * c;
            cross_sum += o * c;
        }
    }

    let mu_x = orig_sum / n;
    let mu_y = comp_sum / n;

    let sigma_x_sq = (orig_sq_sum / n) - (mu_x * mu_x);
    let sigma_y_sq = (comp_sq_sum / n) - (mu_y * mu_y);
    let sigma_xy = (cross_sum / n) - (mu_x * mu_y);

    // Ensure non-negative variance (numerical stability)
    let sigma_x_sq = sigma_x_sq.max(0.0);
    let sigma_y_sq = sigma_y_sq.max(0.0);
    let sigma_x = sigma_x_sq.sqrt();
    let sigma_y = sigma_y_sq.sqrt();

    // Luminance comparison
    let luminance = (2.0 * mu_x * mu_y + c1) / (mu_x * mu_x + mu_y * mu_y + c1);

    // Contrast comparison
    let contrast = (2.0 * sigma_x * sigma_y + c2) / (sigma_x_sq + sigma_y_sq + c2);

    // Structure comparison
    let c3 = c2 / 2.0;
    let structure = (sigma_xy + c3) / (sigma_x * sigma_y + c3);

    // Combined SSIM
    let ssim = luminance * contrast * structure;

    (ssim, luminance, contrast, structure)
}

/// Compute global SSIM (for small images or fallback).
fn compute_global_ssim(original: &[f64], compressed: &[f64], c1: f64, c2: f64) -> (f64, f64, f64, f64) {
    if original.is_empty() {
        return (1.0, 1.0, 1.0, 1.0);
    }

    let n = original.len() as f64;

    let mu_x: f64 = original.iter().sum::<f64>() / n;
    let mu_y: f64 = compressed.iter().sum::<f64>() / n;

    let sigma_x_sq: f64 = original.iter().map(|&v| (v - mu_x).powi(2)).sum::<f64>() / n;
    let sigma_y_sq: f64 = compressed.iter().map(|&v| (v - mu_y).powi(2)).sum::<f64>() / n;
    let sigma_xy: f64 = original
        .iter()
        .zip(compressed.iter())
        .map(|(&o, &c)| (o - mu_x) * (c - mu_y))
        .sum::<f64>()
        / n;

    let sigma_x = sigma_x_sq.max(0.0).sqrt();
    let sigma_y = sigma_y_sq.max(0.0).sqrt();

    let luminance = (2.0 * mu_x * mu_y + c1) / (mu_x * mu_x + mu_y * mu_y + c1);
    let contrast = (2.0 * sigma_x * sigma_y + c2) / (sigma_x_sq + sigma_y_sq + c2);
    let c3 = c2 / 2.0;
    let structure = (sigma_xy + c3) / (sigma_x * sigma_y + c3);

    let ssim = luminance * contrast * structure;

    (ssim, luminance, contrast, structure)
}

/// Generate spatial SSIM map.
fn generate_ssim_map(
    original: &[f64],
    compressed: &[f64],
    width: usize,
    height: usize,
    window_size: usize,
    c1: f64,
    c2: f64,
) -> Vec<f64> {
    let map_width = width.saturating_sub(window_size - 1);
    let map_height = height.saturating_sub(window_size - 1);
    let mut map = Vec::with_capacity(map_width * map_height);

    for y in 0..map_height {
        for x in 0..map_width {
            let (ssim, _, _, _) =
                compute_window_ssim(original, compressed, width, x, y, window_size, c1, c2);
            map.push(ssim);
        }
    }

    map
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
    fn test_ssim_identical_images() {
        let data = vec![128u8; 64 * 64];
        let img1 = create_test_image(64, 64, 8, data.clone());
        let img2 = create_test_image(64, 64, 8, data);

        let result = calculate_ssim(&img1, &img2, &SsimConfig::default()).unwrap();
        assert!((result.ssim - 1.0).abs() < 0.001);
        assert!(result.is_identical() || result.ssim > 0.999);
    }

    #[test]
    fn test_ssim_different_images() {
        // Create a gradient image
        let data1: Vec<u8> = (0..64 * 64).map(|i| ((i / 64) * 4) as u8).collect();
        // Create a slightly different gradient
        let data2: Vec<u8> = (0..64 * 64).map(|i| (((i / 64) * 4) + 5) as u8).collect();

        let img1 = create_test_image(64, 64, 8, data1);
        let img2 = create_test_image(64, 64, 8, data2);

        let result = calculate_ssim(&img1, &img2, &SsimConfig::default()).unwrap();
        assert!(result.ssim < 1.0);
        assert!(result.ssim > 0.0);
    }

    #[test]
    fn test_ssim_small_image() {
        // Test with image smaller than window size
        let data = vec![128u8; 8 * 8];
        let img1 = create_test_image(8, 8, 8, data.clone());
        let img2 = create_test_image(8, 8, 8, data);

        let config = SsimConfig::default().window_size(11);
        let result = calculate_ssim(&img1, &img2, &config).unwrap();
        assert!((result.ssim - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_ssim_with_map() {
        let data = vec![128u8; 32 * 32];
        let img1 = create_test_image(32, 32, 8, data.clone());
        let img2 = create_test_image(32, 32, 8, data);

        let config = SsimConfig::default().with_map().window_size(7);
        let result = calculate_ssim(&img1, &img2, &config).unwrap();

        assert!(result.ssim_map.is_some());
        assert!(result.map_dimensions.is_some());

        let (map_w, map_h) = result.map_dimensions.unwrap();
        assert_eq!(map_w, 32 - 7 + 1);
        assert_eq!(map_h, 32 - 7 + 1);
    }

    #[test]
    fn test_ssim_config_builder() {
        let config = SsimConfig::new().window_size(7).with_map();
        assert_eq!(config.window_size, 7);
        assert!(config.generate_map);
    }

    #[test]
    fn test_ssim_quality_ratings() {
        let result = SsimResult {
            ssim: 0.999,
            ssim_map: None,
            map_dimensions: None,
            per_component: None,
            luminance: 1.0,
            contrast: 1.0,
            structure: 1.0,
        };
        assert_eq!(result.quality_rating(), "Excellent (visually lossless)");

        let result_poor = SsimResult {
            ssim: 0.5,
            ssim_map: None,
            map_dimensions: None,
            per_component: None,
            luminance: 0.8,
            contrast: 0.7,
            structure: 0.9,
        };
        assert_eq!(result_poor.quality_rating(), "Very Poor");
    }
}
