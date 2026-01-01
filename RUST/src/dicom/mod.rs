//! DICOM file parsing and writing module.
//!
//! This module handles reading and writing DICOM files, extracting pixel data,
//! and managing DICOM metadata for compression operations.

use dicom::core::Tag;
use dicom::dictionary_std::tags;
use dicom::object::{open_file, DefaultDicomObject};

use crate::config::Modality;
use crate::error::{MedImgError, Result};
use crate::ImageData;

/// Type alias for the DICOM object returned by open_file.
type DicomObject = DefaultDicomObject;

/// DICOM file wrapper with parsed metadata.
pub struct DicomFile {
    /// The underlying DICOM object.
    object: DicomObject,
    /// Extracted image metadata.
    pub metadata: DicomMetadata,
}

/// Essential DICOM metadata for compression.
#[derive(Debug, Clone)]
pub struct DicomMetadata {
    /// Patient ID.
    pub patient_id: Option<String>,
    /// Study Instance UID.
    pub study_uid: Option<String>,
    /// Series Instance UID.
    pub series_uid: Option<String>,
    /// SOP Instance UID.
    pub sop_instance_uid: Option<String>,
    /// Image modality.
    pub modality: Modality,
    /// Original transfer syntax UID.
    pub transfer_syntax: String,
    /// Image width (columns).
    pub width: u32,
    /// Image height (rows).
    pub height: u32,
    /// Bits allocated per sample.
    pub bits_allocated: u16,
    /// Bits stored per sample.
    pub bits_stored: u16,
    /// High bit position.
    pub high_bit: u16,
    /// Samples per pixel (1 = grayscale, 3 = RGB).
    pub samples_per_pixel: u16,
    /// Photometric interpretation (e.g., MONOCHROME2, RGB).
    pub photometric_interpretation: String,
    /// Pixel representation (0 = unsigned, 1 = signed).
    pub pixel_representation: u16,
    /// Number of frames.
    pub number_of_frames: u32,
    /// Planar configuration (for color images).
    pub planar_configuration: u16,
}

impl DicomFile {
    /// Open and parse a DICOM file.
    pub fn open<P: AsRef<std::path::Path>>(path: P) -> Result<Self> {
        let object = open_file(path)
            .map_err(|e| MedImgError::Dicom(format!("Failed to read DICOM file: {}", e)))?;

        let metadata = Self::extract_metadata(&object)?;

        Ok(Self { object, metadata })
    }

    /// Extract metadata from DICOM object.
    fn extract_metadata(obj: &DicomObject) -> Result<DicomMetadata> {
        let get_string = |tag: Tag| -> Option<String> {
            obj.element(tag)
                .ok()
                .and_then(|e| e.to_str().ok())
                .map(|s| s.trim().to_string())
        };

        let get_u16 = |tag: Tag| -> Option<u16> {
            obj.element(tag).ok().and_then(|e| e.to_int::<u16>().ok())
        };

        let get_u32 = |tag: Tag| -> Option<u32> {
            obj.element(tag).ok().and_then(|e| e.to_int::<u32>().ok())
        };

        // Required image parameters
        let width = get_u16(tags::COLUMNS)
            .ok_or_else(|| MedImgError::Dicom("Missing Columns tag".into()))? as u32;

        let height = get_u16(tags::ROWS)
            .ok_or_else(|| MedImgError::Dicom("Missing Rows tag".into()))? as u32;

        let bits_allocated = get_u16(tags::BITS_ALLOCATED)
            .ok_or_else(|| MedImgError::Dicom("Missing BitsAllocated tag".into()))?;

        let bits_stored = get_u16(tags::BITS_STORED).unwrap_or(bits_allocated);

        let high_bit = get_u16(tags::HIGH_BIT).unwrap_or(bits_stored - 1);

        let samples_per_pixel = get_u16(tags::SAMPLES_PER_PIXEL).unwrap_or(1);

        let photometric_interpretation = get_string(tags::PHOTOMETRIC_INTERPRETATION)
            .unwrap_or_else(|| "MONOCHROME2".into());

        let pixel_representation = get_u16(tags::PIXEL_REPRESENTATION).unwrap_or(0);

        let number_of_frames = get_string(tags::NUMBER_OF_FRAMES)
            .and_then(|s| s.parse::<u32>().ok())
            .or_else(|| get_u32(tags::NUMBER_OF_FRAMES))
            .unwrap_or(1);

        let planar_configuration = get_u16(tags::PLANAR_CONFIGURATION).unwrap_or(0);

        // Transfer syntax from meta header
        let transfer_syntax = obj
            .meta()
            .transfer_syntax()
            .to_string();

        // Modality
        let modality_str = get_string(tags::MODALITY).unwrap_or_default();
        let modality = Modality::from_dicom_string(&modality_str);

        Ok(DicomMetadata {
            patient_id: get_string(tags::PATIENT_ID),
            study_uid: get_string(tags::STUDY_INSTANCE_UID),
            series_uid: get_string(tags::SERIES_INSTANCE_UID),
            sop_instance_uid: get_string(tags::SOP_INSTANCE_UID),
            modality,
            transfer_syntax,
            width,
            height,
            bits_allocated,
            bits_stored,
            high_bit,
            samples_per_pixel,
            photometric_interpretation,
            pixel_representation,
            number_of_frames,
            planar_configuration,
        })
    }

    /// Extract pixel data from the DICOM file.
    pub fn get_pixel_data(&self) -> Result<Vec<u8>> {
        let pixel_data_element = self
            .object
            .element(tags::PIXEL_DATA)
            .map_err(|_| MedImgError::Dicom("Missing PixelData element".into()))?;

        // Get raw bytes
        let bytes = pixel_data_element
            .to_bytes()
            .map_err(|e| MedImgError::Dicom(format!("Failed to extract pixel data: {}", e)))?;

        Ok(bytes.to_vec())
    }

    /// Convert to ImageData structure for compression.
    pub fn to_image_data(&self) -> Result<ImageData> {
        let pixel_data = self.get_pixel_data()?;

        Ok(ImageData {
            width: self.metadata.width,
            height: self.metadata.height,
            bits_per_sample: self.metadata.bits_stored,
            samples_per_pixel: self.metadata.samples_per_pixel,
            pixel_data,
            photometric_interpretation: self.metadata.photometric_interpretation.clone(),
            is_signed: self.metadata.pixel_representation == 1,
        })
    }

    /// Get the modality of the image.
    pub fn modality(&self) -> Modality {
        self.metadata.modality
    }

    /// Check if the image is already compressed.
    pub fn is_compressed(&self) -> bool {
        !matches!(
            self.metadata.transfer_syntax.as_str(),
            "1.2.840.10008.1.2" | "1.2.840.10008.1.2.1" | "1.2.840.10008.1.2.2"
        )
    }

    /// Get the underlying DICOM object for modification.
    pub fn inner(&self) -> &DicomObject {
        &self.object
    }

    /// Get mutable reference to the underlying DICOM object.
    pub fn inner_mut(&mut self) -> &mut DicomObject {
        &mut self.object
    }
}

/// Builder for creating new DICOM files with compressed pixel data.
pub struct DicomWriter {
    /// Source DICOM metadata to preserve.
    #[allow(dead_code)]
    source_metadata: DicomMetadata,
}

impl DicomWriter {
    /// Create a new DICOM writer from source metadata.
    pub fn new(source_metadata: DicomMetadata) -> Self {
        Self { source_metadata }
    }

    /// Write compressed DICOM file.
    pub fn write<P: AsRef<std::path::Path>>(
        &self,
        _source: &DicomFile,
        _compressed_data: &[u8],
        _new_transfer_syntax: &str,
        _output_path: P,
    ) -> Result<()> {
        // For MVP, we'll implement a simplified version
        // Full implementation would update transfer syntax and encapsulate pixel data

        log::info!(
            "Writing DICOM file with transfer syntax: {}",
            _new_transfer_syntax
        );

        // TODO: Implement full DICOM writing with:
        // 1. Update File Meta Information
        // 2. Update Transfer Syntax UID
        // 3. Encapsulate pixel data in fragments
        // 4. Write to file

        Err(MedImgError::Internal(
            "DICOM writing not fully implemented in MVP".into(),
        ))
    }
}

/// Utility functions for DICOM operations.
pub mod utils {
    use super::*;

    /// Calculate expected pixel data size from metadata.
    pub fn calculate_pixel_data_size(metadata: &DicomMetadata) -> usize {
        let bytes_per_sample = ((metadata.bits_allocated + 7) / 8) as usize;
        metadata.width as usize
            * metadata.height as usize
            * metadata.samples_per_pixel as usize
            * bytes_per_sample
            * metadata.number_of_frames as usize
    }

    /// Check if transfer syntax is lossless.
    pub fn is_lossless_transfer_syntax(ts: &str) -> bool {
        matches!(
            ts,
            "1.2.840.10008.1.2"      // Implicit VR Little Endian
            | "1.2.840.10008.1.2.1"  // Explicit VR Little Endian
            | "1.2.840.10008.1.2.2"  // Explicit VR Big Endian
            | "1.2.840.10008.1.2.4.70" // JPEG Lossless
            | "1.2.840.10008.1.2.4.80" // JPEG-LS Lossless
            | "1.2.840.10008.1.2.4.90" // JPEG 2000 Lossless
            | "1.2.840.10008.1.2.5"    // RLE Lossless
        )
    }

    /// Get human-readable name for transfer syntax.
    pub fn transfer_syntax_name(ts: &str) -> &'static str {
        match ts {
            "1.2.840.10008.1.2" => "Implicit VR Little Endian",
            "1.2.840.10008.1.2.1" => "Explicit VR Little Endian",
            "1.2.840.10008.1.2.2" => "Explicit VR Big Endian",
            "1.2.840.10008.1.2.4.70" => "JPEG Lossless",
            "1.2.840.10008.1.2.4.80" => "JPEG-LS Lossless",
            "1.2.840.10008.1.2.4.81" => "JPEG-LS Near-Lossless",
            "1.2.840.10008.1.2.4.90" => "JPEG 2000 Lossless",
            "1.2.840.10008.1.2.4.91" => "JPEG 2000 Lossy",
            "1.2.840.10008.1.2.5" => "RLE Lossless",
            _ => "Unknown",
        }
    }
}
