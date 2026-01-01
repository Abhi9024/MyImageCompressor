//! Codec trait definitions.

use crate::config::CompressionConfig;
use crate::error::Result;
use crate::ImageData;

/// Information about a codec.
#[derive(Debug, Clone)]
pub struct CodecInfo {
    /// Human-readable codec name.
    pub name: &'static str,
    /// Codec version string.
    pub version: &'static str,
    /// Whether lossless compression is supported.
    pub supports_lossless: bool,
    /// Whether lossy compression is supported.
    pub supports_lossy: bool,
    /// Whether progressive/multi-resolution decoding is supported.
    pub supports_progressive: bool,
    /// Whether ROI (Region of Interest) encoding is supported.
    pub supports_roi: bool,
    /// DICOM Transfer Syntax UID for lossless mode.
    pub transfer_syntax_lossless: Option<&'static str>,
    /// DICOM Transfer Syntax UID for lossy mode.
    pub transfer_syntax_lossy: Option<&'static str>,
}

/// Codec capabilities for image formats.
#[derive(Debug, Clone)]
pub struct CodecCapabilities {
    /// Maximum supported bits per sample.
    pub max_bits_per_sample: u16,
    /// Whether signed pixel values are supported.
    pub supports_signed: bool,
    /// Whether color images are supported.
    pub supports_color: bool,
    /// Whether multi-frame images are supported.
    pub supports_multiframe: bool,
}

/// Trait for image compression/decompression codecs.
pub trait Codec: Send + Sync {
    /// Encode image data to compressed format.
    ///
    /// # Arguments
    /// * `image` - The image data to compress
    /// * `config` - Compression configuration
    ///
    /// # Returns
    /// Compressed data as bytes.
    fn encode(&self, image: &ImageData, config: &CompressionConfig) -> Result<Vec<u8>>;

    /// Decode compressed data to image.
    ///
    /// # Arguments
    /// * `data` - Compressed image data
    /// * `width` - Image width in pixels
    /// * `height` - Image height in pixels
    /// * `bits_per_sample` - Bits per pixel sample
    /// * `samples_per_pixel` - Number of samples per pixel (1=grayscale, 3=RGB)
    ///
    /// # Returns
    /// Decoded image data.
    fn decode(
        &self,
        data: &[u8],
        width: u32,
        height: u32,
        bits_per_sample: u16,
        samples_per_pixel: u16,
    ) -> Result<ImageData>;

    /// Get codec information.
    fn info(&self) -> CodecInfo;

    /// Get codec capabilities.
    fn capabilities(&self) -> CodecCapabilities;

    /// Verify that the codec can handle the given image.
    fn can_encode(&self, image: &ImageData) -> bool {
        let caps = self.capabilities();
        image.bits_per_sample <= caps.max_bits_per_sample
            && (image.samples_per_pixel == 1 || caps.supports_color)
            && (!image.is_signed || caps.supports_signed)
    }

    /// Get the DICOM transfer syntax UID for the given compression mode.
    fn transfer_syntax_uid(&self, lossless: bool) -> Option<&'static str> {
        let info = self.info();
        if lossless {
            info.transfer_syntax_lossless
        } else {
            info.transfer_syntax_lossy
        }
    }
}
