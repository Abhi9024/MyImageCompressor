//! Configuration types for compression settings and modality-specific rules.

use serde::{Deserialize, Serialize};

/// Supported compression codecs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum CompressionCodec {
    /// JPEG 2000 (lossless or lossy)
    #[default]
    Jpeg2000,
    /// JPEG-LS (lossless or near-lossless)
    JpegLs,
    /// No compression (raw)
    Uncompressed,
}

/// Compression mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum CompressionMode {
    /// Lossless compression - exact reconstruction guaranteed.
    #[default]
    Lossless,
    /// Lossy compression with quality parameter.
    Lossy,
    /// Near-lossless (JPEG-LS only) with maximum error tolerance.
    NearLossless,
}

/// Medical imaging modality.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Modality {
    /// Computed Tomography
    CT,
    /// Magnetic Resonance Imaging
    MR,
    /// Computed/Digital Radiography
    CR,
    /// Digital X-Ray
    DX,
    /// Mammography - requires lossless only
    MG,
    /// Ultrasound
    US,
    /// Nuclear Medicine
    NM,
    /// Positron Emission Tomography
    PT,
    /// Whole Slide Imaging (Pathology)
    SM,
    /// Other/Unknown
    Other,
}

impl Modality {
    /// Parse modality from DICOM modality string.
    pub fn from_dicom_string(s: &str) -> Self {
        match s.trim().to_uppercase().as_str() {
            "CT" => Modality::CT,
            "MR" | "MRI" => Modality::MR,
            "CR" => Modality::CR,
            "DX" => Modality::DX,
            "MG" => Modality::MG,
            "US" => Modality::US,
            "NM" => Modality::NM,
            "PT" | "PET" => Modality::PT,
            "SM" => Modality::SM,
            _ => Modality::Other,
        }
    }

    /// Check if modality requires lossless compression (regulatory requirement).
    pub fn requires_lossless(&self) -> bool {
        matches!(self, Modality::MG)
    }

    /// Get recommended codec for this modality.
    pub fn recommended_codec(&self) -> CompressionCodec {
        match self {
            Modality::NM => CompressionCodec::JpegLs, // Lower resolution, fast
            _ => CompressionCodec::Jpeg2000,          // General recommendation
        }
    }
}

/// Quality preset for compression.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum QualityPreset {
    /// Maximum quality - lossless
    #[default]
    Diagnostic,
    /// High quality lossy - suitable for primary review
    HighQuality,
    /// Medium quality - suitable for reference viewing
    Standard,
    /// Lower quality - thumbnails and previews
    Preview,
}

impl QualityPreset {
    /// Get the compression ratio target for lossy compression.
    pub fn target_ratio(&self) -> Option<f32> {
        match self {
            QualityPreset::Diagnostic => None, // Lossless
            QualityPreset::HighQuality => Some(10.0),
            QualityPreset::Standard => Some(20.0),
            QualityPreset::Preview => Some(50.0),
        }
    }

    /// Get JPEG 2000 quality layers.
    pub fn quality_layers(&self) -> u32 {
        match self {
            QualityPreset::Diagnostic => 1,
            QualityPreset::HighQuality => 5,
            QualityPreset::Standard => 3,
            QualityPreset::Preview => 2,
        }
    }
}

/// Configuration for compression operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressionConfig {
    /// Codec to use for compression.
    pub codec: CompressionCodec,
    /// Compression mode (lossless, lossy, near-lossless).
    pub mode: CompressionMode,
    /// Quality preset.
    pub quality: QualityPreset,
    /// Target compression ratio (for lossy mode).
    pub target_ratio: Option<f32>,
    /// JPEG 2000 specific: number of quality layers.
    pub quality_layers: u32,
    /// JPEG 2000 specific: tile size (0 = no tiling).
    pub tile_size: u32,
    /// JPEG-LS specific: near-lossless tolerance (0 = lossless).
    pub near_lossless_error: u8,
    /// Preserve original DICOM metadata exactly.
    pub preserve_metadata: bool,
    /// Verify compression by round-trip decode.
    pub verify_compression: bool,
    /// Override modality safety checks (use with caution).
    pub override_safety_checks: bool,
}

impl Default for CompressionConfig {
    fn default() -> Self {
        Self {
            codec: CompressionCodec::Jpeg2000,
            mode: CompressionMode::Lossless,
            quality: QualityPreset::Diagnostic,
            target_ratio: None,
            quality_layers: 1,
            tile_size: 0,
            near_lossless_error: 0,
            preserve_metadata: true,
            verify_compression: true,
            override_safety_checks: false,
        }
    }
}

impl CompressionConfig {
    /// Create a lossless configuration with the given codec.
    pub fn lossless(codec: CompressionCodec) -> Self {
        Self {
            codec,
            mode: CompressionMode::Lossless,
            quality: QualityPreset::Diagnostic,
            ..Default::default()
        }
    }

    /// Create a lossy configuration with target ratio.
    pub fn lossy(codec: CompressionCodec, ratio: f32) -> Self {
        Self {
            codec,
            mode: CompressionMode::Lossy,
            quality: QualityPreset::Standard,
            target_ratio: Some(ratio),
            ..Default::default()
        }
    }

    /// Validate configuration against modality constraints.
    pub fn validate_for_modality(&self, modality: Modality) -> Result<(), String> {
        if modality.requires_lossless() && self.mode != CompressionMode::Lossless {
            if self.override_safety_checks {
                log::warn!(
                    "Safety check overridden: {:?} typically requires lossless compression",
                    modality
                );
            } else {
                return Err(format!(
                    "Modality {:?} requires lossless compression (FDA/ACR requirement). \
                     Set override_safety_checks=true to bypass.",
                    modality
                ));
            }
        }
        Ok(())
    }
}

/// Transfer syntax UIDs for DICOM.
pub mod transfer_syntax {
    /// JPEG 2000 Lossless
    pub const JPEG_2000_LOSSLESS: &str = "1.2.840.10008.1.2.4.90";
    /// JPEG 2000 Lossy
    pub const JPEG_2000_LOSSY: &str = "1.2.840.10008.1.2.4.91";
    /// JPEG-LS Lossless
    pub const JPEG_LS_LOSSLESS: &str = "1.2.840.10008.1.2.4.80";
    /// JPEG-LS Near-Lossless
    pub const JPEG_LS_NEAR_LOSSLESS: &str = "1.2.840.10008.1.2.4.81";
    /// Explicit VR Little Endian (uncompressed)
    pub const EXPLICIT_VR_LITTLE_ENDIAN: &str = "1.2.840.10008.1.2.1";
    /// Implicit VR Little Endian (uncompressed)
    pub const IMPLICIT_VR_LITTLE_ENDIAN: &str = "1.2.840.10008.1.2";
}
