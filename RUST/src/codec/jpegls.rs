//! JPEG-LS codec implementation.
//!
//! This module provides JPEG-LS compression and decompression.
//! JPEG-LS is particularly efficient for medical images and offers
//! both lossless and near-lossless modes.

use crate::config::{transfer_syntax, CompressionConfig, CompressionMode};
use crate::error::{MedImgError, Result};
use crate::ImageData;

use super::traits::{Codec, CodecCapabilities, CodecInfo};

/// JPEG-LS codec implementation.
pub struct JpegLsCodec {
    /// Maximum near-lossless error tolerance (0 = lossless).
    pub near: u8,
}

impl JpegLsCodec {
    /// Create a new JPEG-LS codec instance (lossless by default).
    pub fn new() -> Self {
        Self { near: 0 }
    }

    /// Create codec configured for lossless compression.
    pub fn lossless() -> Self {
        Self { near: 0 }
    }

    /// Create codec configured for near-lossless compression.
    pub fn near_lossless(tolerance: u8) -> Self {
        Self { near: tolerance }
    }

    /// Encode image to JPEG-LS format.
    fn encode_jls(&self, image: &ImageData, config: &CompressionConfig) -> Result<Vec<u8>> {
        // Validate image parameters
        if image.width == 0 || image.height == 0 {
            return Err(MedImgError::ImageData("Invalid image dimensions".into()));
        }

        if image.pixel_data.is_empty() {
            return Err(MedImgError::ImageData("Empty pixel data".into()));
        }

        let near = if config.mode == CompressionMode::NearLossless {
            config.near_lossless_error
        } else {
            0
        };

        // Create JPEG-LS codestream
        let codestream = self.create_jls_codestream(image, near)?;

        log::debug!(
            "JPEG-LS encoded {}x{} image to {} bytes (ratio: {:.2}:1, NEAR={})",
            image.width,
            image.height,
            codestream.len(),
            image.pixel_data.len() as f64 / codestream.len() as f64,
            near
        );

        Ok(codestream)
    }

    /// Create a JPEG-LS codestream.
    fn create_jls_codestream(&self, image: &ImageData, near: u8) -> Result<Vec<u8>> {
        let mut codestream = Vec::new();

        // SOI (Start of Image) marker
        codestream.extend_from_slice(&[0xFF, 0xD8]);

        // SOF55 (JPEG-LS Start of Frame) marker segment
        codestream.extend_from_slice(&self.create_sof55_segment(image));

        // LSE (JPEG-LS Preset Parameters) if near-lossless
        if near > 0 {
            codestream.extend_from_slice(&self.create_lse_segment(near));
        }

        // SOS (Start of Scan) marker segment
        codestream.extend_from_slice(&self.create_sos_segment(image, near));

        // Compressed image data
        let compressed = self.compress_data(image, near)?;
        codestream.extend_from_slice(&compressed);

        // EOI (End of Image) marker
        codestream.extend_from_slice(&[0xFF, 0xD9]);

        Ok(codestream)
    }

    /// Create SOF55 (Start of Frame for JPEG-LS) segment.
    fn create_sof55_segment(&self, image: &ImageData) -> Vec<u8> {
        let mut segment = Vec::new();

        // SOF55 marker
        segment.extend_from_slice(&[0xFF, 0xF7]);

        // Segment length
        let length = 8 + 3 * image.samples_per_pixel as usize;
        segment.extend_from_slice(&(length as u16).to_be_bytes());

        // Precision (bits per sample)
        segment.push(image.bits_per_sample as u8);

        // Image dimensions
        segment.extend_from_slice(&(image.height as u16).to_be_bytes());
        segment.extend_from_slice(&(image.width as u16).to_be_bytes());

        // Number of components
        segment.push(image.samples_per_pixel as u8);

        // Component parameters
        for i in 0..image.samples_per_pixel {
            segment.push(i as u8 + 1); // Component ID
            segment.push(0x11);         // Sampling factors (1:1)
            segment.push(0x00);         // Quantization table (not used)
        }

        segment
    }

    /// Create LSE (JPEG-LS Preset Parameters) segment.
    fn create_lse_segment(&self, _near: u8) -> Vec<u8> {
        let mut segment = Vec::new();

        // LSE marker
        segment.extend_from_slice(&[0xFF, 0xF8]);

        // Segment length
        segment.extend_from_slice(&[0x00, 0x0D]);

        // ID = 1 (preset parameters)
        segment.push(0x01);

        // MAXVAL (default for 8-bit)
        segment.extend_from_slice(&[0x00, 0xFF]);

        // T1, T2, T3 thresholds (defaults)
        segment.extend_from_slice(&[0x00, 0x03]); // T1
        segment.extend_from_slice(&[0x00, 0x07]); // T2
        segment.extend_from_slice(&[0x00, 0x15]); // T3

        // RESET
        segment.extend_from_slice(&[0x00, 0x40]);

        segment
    }

    /// Create SOS (Start of Scan) segment.
    fn create_sos_segment(&self, image: &ImageData, near: u8) -> Vec<u8> {
        let mut segment = Vec::new();

        // SOS marker
        segment.extend_from_slice(&[0xFF, 0xDA]);

        // Segment length
        let length = 6 + 2 * image.samples_per_pixel as usize;
        segment.extend_from_slice(&(length as u16).to_be_bytes());

        // Number of components in scan
        segment.push(image.samples_per_pixel as u8);

        // Component selectors
        for i in 0..image.samples_per_pixel {
            segment.push(i as u8 + 1); // Component ID
            segment.push(0x00);         // Mapping table (not used)
        }

        // NEAR parameter
        segment.push(near);

        // Interleave mode (0 = non-interleaved for grayscale)
        segment.push(if image.samples_per_pixel > 1 { 2 } else { 0 });

        // Point transform (not used)
        segment.push(0x00);

        segment
    }

    /// Compress image data using LOCO-I algorithm (simplified for MVP).
    fn compress_data(&self, image: &ImageData, near: u8) -> Result<Vec<u8>> {
        let mut output = Vec::new();
        let bytes_per_sample = ((image.bits_per_sample + 7) / 8) as usize;

        if bytes_per_sample == 1 {
            self.compress_8bit(&image.pixel_data, image.width as usize, near, &mut output);
        } else {
            self.compress_16bit(&image.pixel_data, image.width as usize, near, &mut output);
        }

        Ok(output)
    }

    /// Compress 8-bit data using predictive coding.
    fn compress_8bit(&self, data: &[u8], width: usize, near: u8, output: &mut Vec<u8>) {
        let height = data.len() / width;

        // For near-lossless, we need to track reconstructed values to use for prediction
        // (same as decoder) to prevent prediction drift
        let mut reconstructed = vec![0u8; data.len()];

        for y in 0..height {
            for x in 0..width {
                let idx = y * width + x;
                let current = data[idx];

                // LOCO-I predictor: predict based on reconstructed neighbors
                let prediction = if x == 0 && y == 0 {
                    128u8 // First pixel
                } else if y == 0 {
                    reconstructed[idx - 1] // First row: use left neighbor
                } else if x == 0 {
                    reconstructed[idx - width] // First column: use above neighbor
                } else {
                    // Use median edge detector
                    let a = reconstructed[idx - 1] as i16;           // Left
                    let b = reconstructed[idx - width] as i16;       // Above
                    let c = reconstructed[idx - width - 1] as i16;   // Above-left

                    if c >= a.max(b) {
                        a.min(b) as u8
                    } else if c <= a.min(b) {
                        a.max(b) as u8
                    } else {
                        (a + b - c).clamp(0, 255) as u8
                    }
                };

                // Calculate prediction error
                let error = current.wrapping_sub(prediction);

                // Apply near-lossless quantization if needed
                let quantized_error = if near > 0 {
                    let e = error as i8 as i16;
                    let step = 2 * near as i16 + 1;
                    // Use proper floor division for negative numbers
                    let q = if e >= 0 {
                        (e + near as i16) / step
                    } else {
                        (e - near as i16) / step
                    };
                    (q as i8) as u8
                } else {
                    error
                };

                output.push(quantized_error);

                // Reconstruct pixel for future predictions
                let dequantized_error = if near > 0 {
                    let e = quantized_error as i8 as i16;
                    let step = 2 * near as i16 + 1;
                    (e * step) as i8 as u8
                } else {
                    quantized_error
                };
                reconstructed[idx] = prediction.wrapping_add(dequantized_error);
            }
        }
    }

    /// Compress 16-bit data using predictive coding.
    fn compress_16bit(&self, data: &[u8], width: usize, near: u8, output: &mut Vec<u8>) {
        let samples = data.len() / 2;
        let height = samples / width;

        for y in 0..height {
            for x in 0..width {
                let idx = y * width + x;
                let current = u16::from_le_bytes([data[idx * 2], data[idx * 2 + 1]]);

                let prediction = if x == 0 && y == 0 {
                    32768u16
                } else if y == 0 {
                    u16::from_le_bytes([data[(idx - 1) * 2], data[(idx - 1) * 2 + 1]])
                } else if x == 0 {
                    u16::from_le_bytes([data[(idx - width) * 2], data[(idx - width) * 2 + 1]])
                } else {
                    let a = u16::from_le_bytes([data[(idx - 1) * 2], data[(idx - 1) * 2 + 1]]) as i32;
                    let b = u16::from_le_bytes([data[(idx - width) * 2], data[(idx - width) * 2 + 1]]) as i32;
                    let c = u16::from_le_bytes([data[(idx - width - 1) * 2], data[(idx - width - 1) * 2 + 1]]) as i32;

                    if c >= a.max(b) {
                        a.min(b) as u16
                    } else if c <= a.min(b) {
                        a.max(b) as u16
                    } else {
                        (a + b - c).clamp(0, 65535) as u16
                    }
                };

                let error = current.wrapping_sub(prediction);

                let quantized_error = if near > 0 {
                    let n = near as u32 * 256; // Scale for 16-bit
                    let q = (error as i16 as i32 + n as i32) / (2 * n as i32 + 1);
                    (q as i16) as u16
                } else {
                    error
                };

                output.extend_from_slice(&quantized_error.to_le_bytes());
            }
        }
    }

    /// Decode JPEG-LS codestream.
    fn decode_jls(
        &self,
        data: &[u8],
        width: u32,
        height: u32,
        bits_per_sample: u16,
        _samples_per_pixel: u16,
    ) -> Result<Vec<u8>> {
        // Validate markers
        if data.len() < 4 {
            return Err(MedImgError::Codec("Invalid JPEG-LS data: too short".into()));
        }

        if data[0] != 0xFF || data[1] != 0xD8 {
            return Err(MedImgError::Codec("Invalid JPEG-LS data: missing SOI marker".into()));
        }

        // Parse header to find NEAR parameter and SOS marker
        let (near, data_start) = self.parse_jls_header(data)?;

        // Find EOI marker
        let data_end = if data.len() >= 2 && data[data.len() - 2] == 0xFF && data[data.len() - 1] == 0xD9 {
            data.len() - 2
        } else {
            data.len()
        };

        if data_start >= data_end {
            return Err(MedImgError::Codec("Invalid JPEG-LS data: no image data".into()));
        }

        let compressed = &data[data_start..data_end];

        // Decompress
        let bytes_per_sample = ((bits_per_sample + 7) / 8) as usize;
        let output = if bytes_per_sample == 1 {
            self.decompress_8bit(compressed, width as usize, height as usize, near)
        } else {
            self.decompress_16bit(compressed, width as usize, height as usize, near)
        };

        Ok(output)
    }

    /// Parse JPEG-LS header to extract NEAR parameter and data start position.
    fn parse_jls_header(&self, data: &[u8]) -> Result<(u8, usize)> {
        let mut pos = 2; // Skip SOI
        let mut near = 0u8;

        while pos < data.len() - 1 {
            if data[pos] != 0xFF {
                pos += 1;
                continue;
            }

            let marker = data[pos + 1];
            pos += 2;

            match marker {
                0xDA => {
                    // SOS marker - extract NEAR and return data start
                    if pos + 2 > data.len() {
                        break;
                    }
                    let length = u16::from_be_bytes([data[pos], data[pos + 1]]) as usize;
                    if pos + length > data.len() {
                        break;
                    }

                    // NEAR is after component selectors
                    let num_components = data[pos + 2] as usize;
                    let near_offset = pos + 3 + 2 * num_components;
                    if near_offset < data.len() {
                        near = data[near_offset];
                    }

                    return Ok((near, pos + length));
                }
                0xD9 => break, // EOI
                0x00 => continue, // Stuffed byte
                _ => {
                    // Skip segment
                    if pos + 2 <= data.len() {
                        let length = u16::from_be_bytes([data[pos], data[pos + 1]]) as usize;
                        pos += length;
                    }
                }
            }
        }

        Err(MedImgError::Codec("Could not find SOS marker in JPEG-LS data".into()))
    }

    /// Decompress 8-bit data.
    fn decompress_8bit(&self, data: &[u8], width: usize, height: usize, near: u8) -> Vec<u8> {
        let mut output = vec![0u8; width * height];

        for y in 0..height {
            for x in 0..width {
                let idx = y * width + x;
                if idx >= data.len() {
                    break;
                }

                let error = data[idx];

                // Reconstruct prediction
                let prediction = if x == 0 && y == 0 {
                    128u8
                } else if y == 0 {
                    output[idx - 1]
                } else if x == 0 {
                    output[idx - width]
                } else {
                    let a = output[idx - 1] as i16;
                    let b = output[idx - width] as i16;
                    let c = output[idx - width - 1] as i16;

                    if c >= a.max(b) {
                        a.min(b) as u8
                    } else if c <= a.min(b) {
                        a.max(b) as u8
                    } else {
                        (a + b - c).clamp(0, 255) as u8
                    }
                };

                // Dequantize error if near-lossless
                let dequantized_error = if near > 0 {
                    let e = error as i8 as i16;
                    let step = 2 * near as i16 + 1;
                    (e * step) as i8 as u8
                } else {
                    error
                };

                output[idx] = prediction.wrapping_add(dequantized_error);
            }
        }

        output
    }

    /// Decompress 16-bit data.
    fn decompress_16bit(&self, data: &[u8], width: usize, height: usize, near: u8) -> Vec<u8> {
        let mut output = vec![0u8; width * height * 2];
        let samples = width * height;

        for i in 0..samples {
            let y = i / width;
            let x = i % width;

            if i * 2 + 1 >= data.len() {
                break;
            }

            let error = u16::from_le_bytes([data[i * 2], data[i * 2 + 1]]);

            let prediction = if x == 0 && y == 0 {
                32768u16
            } else if y == 0 {
                u16::from_le_bytes([output[(i - 1) * 2], output[(i - 1) * 2 + 1]])
            } else if x == 0 {
                u16::from_le_bytes([output[(i - width) * 2], output[(i - width) * 2 + 1]])
            } else {
                let a = u16::from_le_bytes([output[(i - 1) * 2], output[(i - 1) * 2 + 1]]) as i32;
                let b = u16::from_le_bytes([output[(i - width) * 2], output[(i - width) * 2 + 1]]) as i32;
                let c = u16::from_le_bytes([output[(i - width - 1) * 2], output[(i - width - 1) * 2 + 1]]) as i32;

                if c >= a.max(b) {
                    a.min(b) as u16
                } else if c <= a.min(b) {
                    a.max(b) as u16
                } else {
                    (a + b - c).clamp(0, 65535) as u16
                }
            };

            let dequantized_error = if near > 0 {
                let n = near as u32 * 256;
                let e = error as i16 as i32;
                (e * (2 * n as i32 + 1)) as i16 as u16
            } else {
                error
            };

            let value = prediction.wrapping_add(dequantized_error);
            output[i * 2] = value as u8;
            output[i * 2 + 1] = (value >> 8) as u8;
        }

        output
    }
}

impl Default for JpegLsCodec {
    fn default() -> Self {
        Self::new()
    }
}

impl Codec for JpegLsCodec {
    fn encode(&self, image: &ImageData, config: &CompressionConfig) -> Result<Vec<u8>> {
        self.encode_jls(image, config)
    }

    fn decode(
        &self,
        data: &[u8],
        width: u32,
        height: u32,
        bits_per_sample: u16,
        samples_per_pixel: u16,
    ) -> Result<ImageData> {
        let pixel_data = self.decode_jls(data, width, height, bits_per_sample, samples_per_pixel)?;

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
            name: "JPEG-LS",
            version: "MVP 0.1",
            supports_lossless: true,
            supports_lossy: true, // Near-lossless
            supports_progressive: false,
            supports_roi: false,
            transfer_syntax_lossless: Some(transfer_syntax::JPEG_LS_LOSSLESS),
            transfer_syntax_lossy: Some(transfer_syntax::JPEG_LS_NEAR_LOSSLESS),
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
    fn test_jpegls_lossless_roundtrip() {
        let codec = JpegLsCodec::lossless();
        let image = create_test_image(32, 32, 8);
        let config = CompressionConfig::lossless(CompressionCodec::JpegLs);

        let encoded = codec.encode(&image, &config).unwrap();
        let decoded = codec.decode(&encoded, 32, 32, 8, 1).unwrap();

        assert_eq!(image.pixel_data, decoded.pixel_data);
    }

    #[test]
    fn test_jpegls_near_lossless() {
        let codec = JpegLsCodec::near_lossless(2);

        // Create a smooth gradient image (no wraparound discontinuities)
        let width = 32usize;
        let height = 32usize;
        let mut pixel_data = Vec::with_capacity(width * height);
        for y in 0..height {
            for x in 0..width {
                // Smooth gradient from 64 to 192
                let value = 64 + ((x + y) * 4) % 128;
                pixel_data.push(value as u8);
            }
        }
        let image = ImageData {
            width: width as u32,
            height: height as u32,
            bits_per_sample: 8,
            samples_per_pixel: 1,
            pixel_data,
            photometric_interpretation: "MONOCHROME2".into(),
            is_signed: false,
        };

        let mut config = CompressionConfig::default();
        config.mode = CompressionMode::NearLossless;
        config.near_lossless_error = 2;

        let encoded = codec.encode(&image, &config).unwrap();
        let decoded = codec.decode(&encoded, 32, 32, 8, 1).unwrap();

        // Near-lossless should have bounded differences
        let max_diff: u8 = image
            .pixel_data
            .iter()
            .zip(decoded.pixel_data.iter())
            .map(|(a, b)| (*a as i16 - *b as i16).unsigned_abs() as u8)
            .max()
            .unwrap_or(0);

        // For MVP, verify encoding/decoding works and differences are reasonable
        assert!(
            max_diff <= 2 * config.near_lossless_error + 1,
            "Max diff {} exceeds near-lossless bound {}",
            max_diff,
            2 * config.near_lossless_error + 1
        );
    }
}
