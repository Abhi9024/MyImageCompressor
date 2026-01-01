//! Codec implementations for medical image compression.
//!
//! This module provides the `Codec` trait and implementations for:
//! - JPEG 2000 (via OpenJPEG)
//! - JPEG-LS (via CharLS)

mod jpeg2000;
mod jpegls;
mod traits;

pub use jpeg2000::Jpeg2000Codec;
pub use jpegls::JpegLsCodec;
pub use traits::{Codec, CodecCapabilities, CodecInfo};

use crate::config::{CompressionCodec, CompressionConfig};
use crate::error::Result;

/// Factory for creating codec instances.
pub struct CodecFactory;

impl CodecFactory {
    /// Create a codec instance based on configuration.
    pub fn create(codec_type: CompressionCodec) -> Box<dyn Codec> {
        match codec_type {
            CompressionCodec::Jpeg2000 => Box::new(Jpeg2000Codec::new()),
            CompressionCodec::JpegLs => Box::new(JpegLsCodec::new()),
            CompressionCodec::Uncompressed => Box::new(UncompressedCodec),
        }
    }

    /// Get the appropriate codec for the given configuration.
    pub fn for_config(config: &CompressionConfig) -> Box<dyn Codec> {
        Self::create(config.codec)
    }
}

/// Passthrough codec for uncompressed data.
struct UncompressedCodec;

impl Codec for UncompressedCodec {
    fn encode(
        &self,
        image: &crate::ImageData,
        _config: &CompressionConfig,
    ) -> Result<Vec<u8>> {
        Ok(image.pixel_data.clone())
    }

    fn decode(
        &self,
        data: &[u8],
        width: u32,
        height: u32,
        bits_per_sample: u16,
        samples_per_pixel: u16,
    ) -> Result<crate::ImageData> {
        Ok(crate::ImageData {
            width,
            height,
            bits_per_sample,
            samples_per_pixel,
            pixel_data: data.to_vec(),
            photometric_interpretation: String::new(),
            is_signed: false,
        })
    }

    fn info(&self) -> CodecInfo {
        CodecInfo {
            name: "Uncompressed",
            version: "1.0",
            supports_lossless: true,
            supports_lossy: false,
            supports_progressive: false,
            supports_roi: false,
            transfer_syntax_lossless: Some(crate::config::transfer_syntax::EXPLICIT_VR_LITTLE_ENDIAN),
            transfer_syntax_lossy: None,
        }
    }

    fn capabilities(&self) -> CodecCapabilities {
        CodecCapabilities {
            max_bits_per_sample: 16,
            supports_signed: true,
            supports_color: true,
            supports_multiframe: true,
        }
    }
}
