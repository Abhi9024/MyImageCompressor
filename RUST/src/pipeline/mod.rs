//! Compression pipeline module.
//!
//! This module orchestrates the compression workflow, handling single files
//! and batch operations with progress reporting.

use std::path::{Path, PathBuf};
use std::time::Instant;

use crate::codec::{Codec, CodecFactory};
use crate::config::{CompressionConfig, CompressionMode};
use crate::dicom::{DicomFile, DicomMetadata};
use crate::error::{MedImgError, Result};
use crate::ImageData;

/// Result of a compression operation.
#[derive(Debug)]
pub struct CompressionResult {
    /// Original file path.
    pub source_path: PathBuf,
    /// Output file path (if written).
    pub output_path: Option<PathBuf>,
    /// Original size in bytes.
    pub original_size: usize,
    /// Compressed size in bytes.
    pub compressed_size: usize,
    /// Compression ratio.
    pub compression_ratio: f64,
    /// Time taken for compression in milliseconds.
    pub compression_time_ms: u64,
    /// Whether compression was lossless.
    pub is_lossless: bool,
    /// Codec used.
    pub codec_name: String,
    /// Any warnings generated.
    pub warnings: Vec<String>,
}

impl CompressionResult {
    /// Calculate space savings as percentage.
    pub fn space_savings_percent(&self) -> f64 {
        if self.original_size == 0 {
            0.0
        } else {
            (1.0 - (self.compressed_size as f64 / self.original_size as f64)) * 100.0
        }
    }
}

/// Statistics for batch compression operations.
#[derive(Debug, Default)]
pub struct BatchStats {
    /// Total files processed.
    pub total_files: usize,
    /// Successfully compressed files.
    pub successful: usize,
    /// Failed files.
    pub failed: usize,
    /// Skipped files.
    pub skipped: usize,
    /// Total original size.
    pub total_original_bytes: usize,
    /// Total compressed size.
    pub total_compressed_bytes: usize,
    /// Total processing time in milliseconds.
    pub total_time_ms: u64,
}

impl BatchStats {
    /// Calculate overall compression ratio.
    pub fn overall_ratio(&self) -> f64 {
        if self.total_compressed_bytes == 0 {
            0.0
        } else {
            self.total_original_bytes as f64 / self.total_compressed_bytes as f64
        }
    }

    /// Calculate overall space savings.
    pub fn overall_savings_percent(&self) -> f64 {
        if self.total_original_bytes == 0 {
            0.0
        } else {
            (1.0 - (self.total_compressed_bytes as f64 / self.total_original_bytes as f64)) * 100.0
        }
    }
}

/// Compression pipeline for processing DICOM files.
pub struct CompressionPipeline {
    /// Compression configuration.
    config: CompressionConfig,
    /// Whether to perform dry-run (no actual file writing).
    dry_run: bool,
}

impl CompressionPipeline {
    /// Create a new compression pipeline with the given configuration.
    pub fn new(config: CompressionConfig) -> Self {
        Self {
            config,
            dry_run: false,
        }
    }

    /// Set dry-run mode.
    pub fn dry_run(mut self, dry_run: bool) -> Self {
        self.dry_run = dry_run;
        self
    }

    /// Compress a single DICOM file.
    pub fn compress_file<P: AsRef<Path>>(&self, input_path: P) -> Result<CompressionResult> {
        let input_path = input_path.as_ref();
        let start = Instant::now();
        let mut warnings = Vec::new();

        log::info!("Processing: {}", input_path.display());

        // Open DICOM file
        let dicom_file = DicomFile::open(input_path)?;

        // Validate against modality constraints
        if let Err(e) = self
            .config
            .validate_for_modality(dicom_file.modality())
        {
            if !self.config.override_safety_checks {
                return Err(MedImgError::Validation(e));
            }
            warnings.push(format!("Safety check overridden: {}", e));
        }

        // Check if already compressed
        if dicom_file.is_compressed() {
            warnings.push(format!(
                "Source is already compressed ({})",
                dicom_file.metadata.transfer_syntax
            ));
        }

        // Extract image data
        let image_data = dicom_file.to_image_data()?;
        let original_size = image_data.pixel_data.len();

        // Create codec and compress
        let codec = CodecFactory::for_config(&self.config);

        if !codec.can_encode(&image_data) {
            return Err(MedImgError::Codec(format!(
                "Codec {} cannot encode this image ({}x{}, {} bits)",
                codec.info().name,
                image_data.width,
                image_data.height,
                image_data.bits_per_sample
            )));
        }

        let compressed_data = codec.encode(&image_data, &self.config)?;
        let compressed_size = compressed_data.len();

        // Verify compression if enabled
        if self.config.verify_compression && self.config.mode == CompressionMode::Lossless {
            self.verify_lossless(&codec, &compressed_data, &image_data)?;
        }

        let compression_time_ms = start.elapsed().as_millis() as u64;

        Ok(CompressionResult {
            source_path: input_path.to_path_buf(),
            output_path: None, // MVP doesn't write files yet
            original_size,
            compressed_size,
            compression_ratio: original_size as f64 / compressed_size as f64,
            compression_time_ms,
            is_lossless: self.config.mode == CompressionMode::Lossless,
            codec_name: codec.info().name.to_string(),
            warnings,
        })
    }

    /// Compress an in-memory image.
    pub fn compress_image(&self, image: &ImageData) -> Result<Vec<u8>> {
        let codec = CodecFactory::for_config(&self.config);

        if !codec.can_encode(image) {
            return Err(MedImgError::Codec(format!(
                "Codec {} cannot encode this image",
                codec.info().name
            )));
        }

        let compressed = codec.encode(image, &self.config)?;

        if self.config.verify_compression && self.config.mode == CompressionMode::Lossless {
            self.verify_lossless(&codec, &compressed, image)?;
        }

        Ok(compressed)
    }

    /// Decompress data back to image.
    pub fn decompress(&self, data: &[u8], metadata: &DicomMetadata) -> Result<ImageData> {
        let codec = CodecFactory::for_config(&self.config);

        codec.decode(
            data,
            metadata.width,
            metadata.height,
            metadata.bits_stored,
            metadata.samples_per_pixel,
        )
    }

    /// Verify lossless compression by round-trip decode.
    fn verify_lossless(
        &self,
        codec: &Box<dyn Codec>,
        compressed: &[u8],
        original: &ImageData,
    ) -> Result<()> {
        let decoded = codec.decode(
            compressed,
            original.width,
            original.height,
            original.bits_per_sample,
            original.samples_per_pixel,
        )?;

        if decoded.pixel_data != original.pixel_data {
            return Err(MedImgError::Validation(
                "Lossless verification failed: decoded data differs from original".into(),
            ));
        }

        log::debug!("Lossless verification passed");
        Ok(())
    }

    /// Get compression statistics without writing files.
    pub fn analyze<P: AsRef<Path>>(&self, input_path: P) -> Result<CompressionResult> {
        self.compress_file(input_path)
    }
}

/// Builder for creating compression pipelines with custom settings.
pub struct PipelineBuilder {
    config: CompressionConfig,
    dry_run: bool,
}

impl PipelineBuilder {
    /// Create a new pipeline builder with default settings.
    pub fn new() -> Self {
        Self {
            config: CompressionConfig::default(),
            dry_run: false,
        }
    }

    /// Set the compression configuration.
    pub fn config(mut self, config: CompressionConfig) -> Self {
        self.config = config;
        self
    }

    /// Enable or disable dry-run mode.
    pub fn dry_run(mut self, dry_run: bool) -> Self {
        self.dry_run = dry_run;
        self
    }

    /// Build the compression pipeline.
    pub fn build(self) -> CompressionPipeline {
        CompressionPipeline {
            config: self.config,
            dry_run: self.dry_run,
        }
    }
}

impl Default for PipelineBuilder {
    fn default() -> Self {
        Self::new()
    }
}
