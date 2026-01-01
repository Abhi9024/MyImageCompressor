//! Medical Image Compression Library
//!
//! A high-performance library for compressing medical images in DICOM format,
//! supporting JPEG 2000 and JPEG-LS codecs with full regulatory compliance.
//!
//! # Features
//!
//! - **JPEG 2000**: Lossless and lossy compression with progressive decoding
//! - **JPEG-LS**: Fast lossless and near-lossless compression
//! - **DICOM Compliance**: Full support for DICOM transfer syntaxes
//! - **Regulatory Aware**: Enforces FDA/ACR guidelines for modality-specific requirements
//! - **Memory Safe**: Built in Rust for reliability and security
//!
//! # Quick Start
//!
//! ```rust,ignore
//! use medimg_compress::{CompressionConfig, CompressionPipeline, CompressionCodec};
//!
//! // Create a lossless JPEG 2000 configuration
//! let config = CompressionConfig::lossless(CompressionCodec::Jpeg2000);
//!
//! // Create the pipeline
//! let pipeline = CompressionPipeline::new(config);
//!
//! // Compress a DICOM file
//! let result = pipeline.compress_file("input.dcm")?;
//! println!("Compression ratio: {:.2}:1", result.compression_ratio);
//! ```
//!
//! # Modality Safety
//!
//! The library enforces regulatory requirements for specific modalities:
//!
//! - **Mammography (MG)**: Only lossless compression is allowed (FDA requirement)
//! - **Other modalities**: Both lossless and lossy compression are available
//!
//! To override safety checks (not recommended for production):
//!
//! ```rust,ignore
//! let mut config = CompressionConfig::lossy(CompressionCodec::Jpeg2000, 10.0);
//! config.override_safety_checks = true;
//! ```

#![warn(missing_docs)]
#![warn(clippy::all)]

pub mod cli;
pub mod codec;
pub mod config;
pub mod dicom;
pub mod error;
pub mod pipeline;

// Re-export commonly used types
pub use codec::{Codec, CodecFactory, CodecInfo, Jpeg2000Codec, JpegLsCodec};
pub use config::{CompressionCodec, CompressionConfig, CompressionMode, Modality, QualityPreset};
pub use dicom::{DicomFile, DicomMetadata};
pub use error::{MedImgError, Result};
pub use pipeline::{CompressionPipeline, CompressionResult, PipelineBuilder};

/// Image data structure for compression.
#[derive(Debug, Clone)]
pub struct ImageData {
    /// Image width in pixels.
    pub width: u32,
    /// Image height in pixels.
    pub height: u32,
    /// Bits per sample (typically 8 or 16 for medical images).
    pub bits_per_sample: u16,
    /// Samples per pixel (1 for grayscale, 3 for RGB).
    pub samples_per_pixel: u16,
    /// Raw pixel data.
    pub pixel_data: Vec<u8>,
    /// Photometric interpretation (e.g., "MONOCHROME2", "RGB").
    pub photometric_interpretation: String,
    /// Whether pixel values are signed.
    pub is_signed: bool,
}

impl ImageData {
    /// Create a new ImageData instance.
    pub fn new(
        width: u32,
        height: u32,
        bits_per_sample: u16,
        samples_per_pixel: u16,
        pixel_data: Vec<u8>,
    ) -> Self {
        Self {
            width,
            height,
            bits_per_sample,
            samples_per_pixel,
            pixel_data,
            photometric_interpretation: String::new(),
            is_signed: false,
        }
    }

    /// Calculate the expected size of pixel data in bytes.
    pub fn expected_size(&self) -> usize {
        let bytes_per_sample = ((self.bits_per_sample + 7) / 8) as usize;
        self.width as usize
            * self.height as usize
            * self.samples_per_pixel as usize
            * bytes_per_sample
    }

    /// Validate that pixel data size matches expected size.
    pub fn validate(&self) -> Result<()> {
        let expected = self.expected_size();
        if self.pixel_data.len() != expected {
            return Err(MedImgError::ImageData(format!(
                "Pixel data size mismatch: expected {} bytes, got {}",
                expected,
                self.pixel_data.len()
            )));
        }
        Ok(())
    }
}

/// Library version information.
pub mod version {
    /// Library version string.
    pub const VERSION: &str = env!("CARGO_PKG_VERSION");

    /// Library name.
    pub const NAME: &str = env!("CARGO_PKG_NAME");

    /// Get full version string.
    pub fn full_version() -> String {
        format!("{} {}", NAME, VERSION)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_image_data_expected_size() {
        let image = ImageData::new(512, 512, 16, 1, vec![0; 512 * 512 * 2]);
        assert_eq!(image.expected_size(), 512 * 512 * 2);
    }

    #[test]
    fn test_image_data_validation() {
        let image = ImageData::new(64, 64, 8, 1, vec![0; 64 * 64]);
        assert!(image.validate().is_ok());

        let bad_image = ImageData::new(64, 64, 8, 1, vec![0; 100]);
        assert!(bad_image.validate().is_err());
    }

    #[test]
    fn test_modality_detection() {
        assert_eq!(Modality::from_dicom_string("CT"), Modality::CT);
        assert_eq!(Modality::from_dicom_string("MR"), Modality::MR);
        assert_eq!(Modality::from_dicom_string("MG"), Modality::MG);
        assert!(Modality::MG.requires_lossless());
        assert!(!Modality::CT.requires_lossless());
    }

    #[test]
    fn test_compression_config_validation() {
        let config = CompressionConfig::lossy(CompressionCodec::Jpeg2000, 10.0);
        assert!(config.validate_for_modality(Modality::MG).is_err());
        assert!(config.validate_for_modality(Modality::CT).is_ok());

        let lossless = CompressionConfig::lossless(CompressionCodec::Jpeg2000);
        assert!(lossless.validate_for_modality(Modality::MG).is_ok());
    }
}
