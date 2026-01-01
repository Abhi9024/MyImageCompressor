//! JPEG 2000 codec implementation.
//!
//! This module provides JPEG 2000 compression and decompression using OpenJPEG.
//! For Phase 1 MVP, we implement a pure Rust solution with basic J2K support.

use crate::config::{transfer_syntax, CompressionConfig, CompressionMode};
use crate::error::{MedImgError, Result};
use crate::ImageData;

use super::traits::{Codec, CodecCapabilities, CodecInfo};

/// JPEG 2000 codec using OpenJPEG.
pub struct Jpeg2000Codec {
    /// Whether to use reversible (5/3) or irreversible (9/7) wavelet transform.
    pub use_reversible: bool,
}

impl Jpeg2000Codec {
    /// Create a new JPEG 2000 codec instance.
    pub fn new() -> Self {
        Self {
            use_reversible: true,
        }
    }

    /// Create codec configured for lossless compression.
    pub fn lossless() -> Self {
        Self {
            use_reversible: true,
        }
    }

    /// Create codec configured for lossy compression.
    pub fn lossy() -> Self {
        Self {
            use_reversible: false,
        }
    }

    /// Encode image to JPEG 2000 format.
    fn encode_j2k(&self, image: &ImageData, config: &CompressionConfig) -> Result<Vec<u8>> {
        // Validate image parameters
        if image.width == 0 || image.height == 0 {
            return Err(MedImgError::ImageData("Invalid image dimensions".into()));
        }

        if image.pixel_data.is_empty() {
            return Err(MedImgError::ImageData("Empty pixel data".into()));
        }

        let expected_size = self.calculate_expected_size(image);
        if image.pixel_data.len() < expected_size {
            return Err(MedImgError::ImageData(format!(
                "Pixel data size mismatch: expected at least {} bytes, got {}",
                expected_size,
                image.pixel_data.len()
            )));
        }

        // For MVP, we create a simple J2K codestream structure
        // In production, this would use OpenJPEG FFI bindings
        let codestream = self.create_j2k_codestream(image, config)?;

        log::debug!(
            "Encoded {}x{} image to {} bytes (ratio: {:.2}:1)",
            image.width,
            image.height,
            codestream.len(),
            image.pixel_data.len() as f64 / codestream.len() as f64
        );

        Ok(codestream)
    }

    /// Create a JPEG 2000 codestream (simplified for MVP).
    fn create_j2k_codestream(
        &self,
        image: &ImageData,
        config: &CompressionConfig,
    ) -> Result<Vec<u8>> {
        let mut codestream = Vec::new();

        // SOC (Start of Codestream) marker
        codestream.extend_from_slice(&[0xFF, 0x4F]);

        // SIZ (Image and Tile Size) marker segment
        codestream.extend_from_slice(&self.create_siz_segment(image));

        // COD (Coding Style Default) marker segment
        codestream.extend_from_slice(&self.create_cod_segment(config));

        // QCD (Quantization Default) marker segment
        codestream.extend_from_slice(&self.create_qcd_segment(config));

        // SOT (Start of Tile-Part) marker
        codestream.extend_from_slice(&[0xFF, 0x90]);

        // Tile-part header length (simplified)
        let tile_length = 10 + image.pixel_data.len();
        codestream.extend_from_slice(&(tile_length as u16).to_be_bytes());

        // Tile index
        codestream.extend_from_slice(&[0x00, 0x00]);

        // Tile-part length
        codestream.extend_from_slice(&(tile_length as u32).to_be_bytes());

        // Tile-part index and number of tile-parts
        codestream.extend_from_slice(&[0x00, 0x01]);

        // SOD (Start of Data) marker
        codestream.extend_from_slice(&[0xFF, 0x93]);

        // For MVP: include compressed representation of pixel data
        // In production, this would be actual wavelet-transformed data
        let compressed_data = self.compress_tile_data(image, config)?;
        codestream.extend_from_slice(&compressed_data);

        // EOC (End of Codestream) marker
        codestream.extend_from_slice(&[0xFF, 0xD9]);

        Ok(codestream)
    }

    /// Create SIZ marker segment.
    fn create_siz_segment(&self, image: &ImageData) -> Vec<u8> {
        let mut segment = Vec::new();

        // SIZ marker
        segment.extend_from_slice(&[0xFF, 0x51]);

        // Segment length (will be filled)
        let components = image.samples_per_pixel as usize;
        let seg_length = 38 + 3 * components;
        segment.extend_from_slice(&(seg_length as u16).to_be_bytes());

        // Profile (0 = unrestricted)
        segment.extend_from_slice(&[0x00, 0x00]);

        // Image dimensions
        segment.extend_from_slice(&(image.width).to_be_bytes());
        segment.extend_from_slice(&(image.height).to_be_bytes());

        // Image offset (0, 0)
        segment.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]);
        segment.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]);

        // Tile dimensions (same as image for single tile)
        segment.extend_from_slice(&(image.width).to_be_bytes());
        segment.extend_from_slice(&(image.height).to_be_bytes());

        // Tile offset (0, 0)
        segment.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]);
        segment.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]);

        // Number of components
        segment.extend_from_slice(&(image.samples_per_pixel as u16).to_be_bytes());

        // Component parameters
        for _ in 0..components {
            // Bit depth (Ssiz) - signed flag in MSB
            let ssiz = if image.is_signed {
                0x80 | ((image.bits_per_sample - 1) as u8)
            } else {
                (image.bits_per_sample - 1) as u8
            };
            segment.push(ssiz);

            // Horizontal/Vertical separation (1, 1)
            segment.push(0x01);
            segment.push(0x01);
        }

        segment
    }

    /// Create COD marker segment.
    fn create_cod_segment(&self, config: &CompressionConfig) -> Vec<u8> {
        let mut segment = Vec::new();

        // COD marker
        segment.extend_from_slice(&[0xFF, 0x52]);

        // Segment length
        segment.extend_from_slice(&[0x00, 0x0C]);

        // Coding style (no SOP, no EPH)
        segment.push(0x00);

        // Progression order (LRCP)
        segment.push(0x00);

        // Number of layers
        segment.extend_from_slice(&(config.quality_layers as u16).to_be_bytes());

        // Multiple component transform (0 = none, 1 = yes for color)
        segment.push(0x00);

        // Decomposition levels
        segment.push(0x05);

        // Code-block size (64x64)
        segment.push(0x04); // 2^(4+2) = 64
        segment.push(0x04);

        // Code-block style
        segment.push(0x00);

        // Wavelet transform (5/3 reversible or 9/7 irreversible)
        let transform = if config.mode == CompressionMode::Lossless {
            0x01 // 5/3 reversible
        } else {
            0x00 // 9/7 irreversible
        };
        segment.push(transform);

        segment
    }

    /// Create QCD marker segment.
    fn create_qcd_segment(&self, config: &CompressionConfig) -> Vec<u8> {
        let mut segment = Vec::new();

        // QCD marker
        segment.extend_from_slice(&[0xFF, 0x5C]);

        if config.mode == CompressionMode::Lossless {
            // Reversible quantization (no quantization)
            segment.extend_from_slice(&[0x00, 0x04]); // Length
            segment.push(0x22); // Sqcd: reversible, guard bits = 2
            segment.push(0x00); // SPqcd: exponent for LL band
        } else {
            // Irreversible quantization
            segment.extend_from_slice(&[0x00, 0x05]); // Length
            segment.push(0x42); // Sqcd: scalar derived, guard bits = 2
            segment.extend_from_slice(&[0x00, 0x88]); // Base step size
        }

        segment
    }

    /// Compress tile data (simplified implementation for MVP).
    fn compress_tile_data(&self, image: &ImageData, config: &CompressionConfig) -> Result<Vec<u8>> {
        // For MVP, we use a simple approach:
        // - Lossless: basic predictive coding simulation
        // - Lossy: apply simple quantization

        if config.mode == CompressionMode::Lossless {
            // Simple delta encoding for lossless (placeholder for actual wavelet)
            self.lossless_encode(&image.pixel_data, image.bits_per_sample)
        } else {
            // Apply quantization for lossy
            let ratio = config.target_ratio.unwrap_or(10.0);
            self.lossy_encode(&image.pixel_data, image.bits_per_sample, ratio)
        }
    }

    /// Simple lossless encoding (placeholder for actual wavelet transform).
    fn lossless_encode(&self, data: &[u8], bits_per_sample: u16) -> Result<Vec<u8>> {
        let mut output = Vec::with_capacity(data.len());

        if bits_per_sample <= 8 {
            // 8-bit data: simple delta encoding
            if !data.is_empty() {
                output.push(data[0]);
                for i in 1..data.len() {
                    let delta = data[i].wrapping_sub(data[i - 1]);
                    output.push(delta);
                }
            }
        } else {
            // 16-bit data: delta encoding on 16-bit values
            let samples = data.len() / 2;
            if samples > 0 {
                output.extend_from_slice(&data[0..2]);
                for i in 1..samples {
                    let curr = u16::from_le_bytes([data[i * 2], data[i * 2 + 1]]);
                    let prev = u16::from_le_bytes([data[(i - 1) * 2], data[(i - 1) * 2 + 1]]);
                    let delta = curr.wrapping_sub(prev);
                    output.extend_from_slice(&delta.to_le_bytes());
                }
            }
        }

        Ok(output)
    }

    /// Simple lossy encoding with quantization.
    fn lossy_encode(&self, data: &[u8], bits_per_sample: u16, target_ratio: f32) -> Result<Vec<u8>> {
        // Calculate quantization step based on target ratio
        let quant_bits = ((target_ratio.log2() * 0.5) as u8).min(bits_per_sample as u8 - 1);
        let shift = quant_bits as usize;

        let mut output = Vec::with_capacity(data.len() >> shift.min(4));

        // Store quantization parameter
        output.push(quant_bits);

        if bits_per_sample <= 8 {
            for byte in data {
                let quantized = byte >> shift.min(7);
                output.push(quantized);
            }
        } else {
            for chunk in data.chunks(2) {
                if chunk.len() == 2 {
                    let value = u16::from_le_bytes([chunk[0], chunk[1]]);
                    let quantized = value >> shift.min(15);
                    output.extend_from_slice(&quantized.to_le_bytes());
                }
            }
        }

        Ok(output)
    }

    /// Calculate expected pixel data size.
    fn calculate_expected_size(&self, image: &ImageData) -> usize {
        let bytes_per_sample = ((image.bits_per_sample + 7) / 8) as usize;
        image.width as usize
            * image.height as usize
            * image.samples_per_pixel as usize
            * bytes_per_sample
    }

    /// Decode JPEG 2000 codestream (simplified for MVP).
    fn decode_j2k(
        &self,
        data: &[u8],
        width: u32,
        height: u32,
        bits_per_sample: u16,
        samples_per_pixel: u16,
    ) -> Result<Vec<u8>> {
        // Validate J2K markers
        if data.len() < 4 {
            return Err(MedImgError::Codec("Invalid J2K data: too short".into()));
        }

        // Check for SOC marker
        if data[0] != 0xFF || data[1] != 0x4F {
            return Err(MedImgError::Codec("Invalid J2K data: missing SOC marker".into()));
        }

        // Find SOD marker and extract compressed data
        let mut pos = 2;
        while pos < data.len() - 1 {
            if data[pos] == 0xFF && data[pos + 1] == 0x93 {
                pos += 2;
                break;
            }
            pos += 1;
        }

        // Find EOC marker
        let mut end = data.len();
        if data.len() >= 2 && data[data.len() - 2] == 0xFF && data[data.len() - 1] == 0xD9 {
            end = data.len() - 2;
        }

        if pos >= end {
            return Err(MedImgError::Codec("Invalid J2K data: no tile data found".into()));
        }

        let compressed = &data[pos..end];

        // Decode based on quantization parameter
        let decoded = if !compressed.is_empty() && compressed[0] < 16 {
            // Lossy: has quantization parameter
            self.lossy_decode(compressed, bits_per_sample)?
        } else {
            // Lossless: delta encoded
            self.lossless_decode(compressed, bits_per_sample)?
        };

        // Verify size
        let expected_size = self.calculate_expected_size(&ImageData {
            width,
            height,
            bits_per_sample,
            samples_per_pixel,
            pixel_data: Vec::new(),
            photometric_interpretation: String::new(),
            is_signed: false,
        });

        if decoded.len() != expected_size {
            log::warn!(
                "Decoded size {} differs from expected {}",
                decoded.len(),
                expected_size
            );
        }

        Ok(decoded)
    }

    /// Decode lossless data.
    fn lossless_decode(&self, data: &[u8], bits_per_sample: u16) -> Result<Vec<u8>> {
        let mut output = Vec::with_capacity(data.len());

        if bits_per_sample <= 8 {
            if !data.is_empty() {
                output.push(data[0]);
                for i in 1..data.len() {
                    let value = output[i - 1].wrapping_add(data[i]);
                    output.push(value);
                }
            }
        } else {
            if data.len() >= 2 {
                output.extend_from_slice(&data[0..2]);
                for i in 1..(data.len() / 2) {
                    let delta = u16::from_le_bytes([data[i * 2], data[i * 2 + 1]]);
                    let prev = u16::from_le_bytes([output[(i - 1) * 2], output[(i - 1) * 2 + 1]]);
                    let value = prev.wrapping_add(delta);
                    output.extend_from_slice(&value.to_le_bytes());
                }
            }
        }

        Ok(output)
    }

    /// Decode lossy data.
    fn lossy_decode(&self, data: &[u8], bits_per_sample: u16) -> Result<Vec<u8>> {
        if data.is_empty() {
            return Ok(Vec::new());
        }

        let quant_bits = data[0] as usize;
        let shift = quant_bits.min(15);
        let data = &data[1..];

        let mut output = Vec::with_capacity(data.len() << shift.min(4));

        if bits_per_sample <= 8 {
            for byte in data {
                let dequantized = byte << shift.min(7);
                output.push(dequantized);
            }
        } else {
            for chunk in data.chunks(2) {
                if chunk.len() == 2 {
                    let value = u16::from_le_bytes([chunk[0], chunk[1]]);
                    let dequantized = value << shift.min(15);
                    output.extend_from_slice(&dequantized.to_le_bytes());
                }
            }
        }

        Ok(output)
    }
}

impl Default for Jpeg2000Codec {
    fn default() -> Self {
        Self::new()
    }
}

impl Codec for Jpeg2000Codec {
    fn encode(&self, image: &ImageData, config: &CompressionConfig) -> Result<Vec<u8>> {
        self.encode_j2k(image, config)
    }

    fn decode(
        &self,
        data: &[u8],
        width: u32,
        height: u32,
        bits_per_sample: u16,
        samples_per_pixel: u16,
    ) -> Result<ImageData> {
        let pixel_data = self.decode_j2k(data, width, height, bits_per_sample, samples_per_pixel)?;

        Ok(ImageData {
            width,
            height,
            bits_per_sample,
            samples_per_pixel,
            pixel_data,
            photometric_interpretation: String::new(),
            is_signed: false,
        })
    }

    fn info(&self) -> CodecInfo {
        CodecInfo {
            name: "JPEG 2000",
            version: "MVP 0.1",
            supports_lossless: true,
            supports_lossy: true,
            supports_progressive: true,
            supports_roi: false, // Not in MVP
            transfer_syntax_lossless: Some(transfer_syntax::JPEG_2000_LOSSLESS),
            transfer_syntax_lossy: Some(transfer_syntax::JPEG_2000_LOSSY),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::CompressionCodec;

    fn create_test_image(width: u32, height: u32, bits: u16) -> ImageData {
        let bytes_per_sample = ((bits + 7) / 8) as usize;
        let size = width as usize * height as usize * bytes_per_sample;
        let mut pixel_data = Vec::with_capacity(size);

        for i in 0..size {
            pixel_data.push((i % 256) as u8);
        }

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
    fn test_lossless_roundtrip() {
        let codec = Jpeg2000Codec::lossless();
        let image = create_test_image(64, 64, 8);
        let config = CompressionConfig::lossless(CompressionCodec::Jpeg2000);

        let encoded = codec.encode(&image, &config).unwrap();
        let decoded = codec.decode(&encoded, 64, 64, 8, 1).unwrap();

        assert_eq!(image.pixel_data, decoded.pixel_data);
    }

    #[test]
    fn test_lossy_compression() {
        let codec = Jpeg2000Codec::lossy();
        let image = create_test_image(64, 64, 8);
        let config = CompressionConfig::lossy(CompressionCodec::Jpeg2000, 10.0);

        let encoded = codec.encode(&image, &config).unwrap();

        // Lossy should produce smaller output
        assert!(encoded.len() < image.pixel_data.len());
    }
}
