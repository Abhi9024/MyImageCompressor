//! Error types for the medical image compression library.

use thiserror::Error;

/// Result type alias for the library.
pub type Result<T> = std::result::Result<T, MedImgError>;

/// Main error type for the medical image compression library.
#[derive(Error, Debug)]
pub enum MedImgError {
    /// Error reading or parsing DICOM file.
    #[error("DICOM error: {0}")]
    Dicom(String),

    /// Error during image compression/decompression.
    #[error("Codec error: {0}")]
    Codec(String),

    /// Invalid or unsupported image format.
    #[error("Invalid image format: {0}")]
    InvalidFormat(String),

    /// Unsupported transfer syntax.
    #[error("Unsupported transfer syntax: {0}")]
    UnsupportedTransferSyntax(String),

    /// Configuration error.
    #[error("Configuration error: {0}")]
    Config(String),

    /// I/O error.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Validation error (e.g., regulatory constraints violated).
    #[error("Validation error: {0}")]
    Validation(String),

    /// Image dimensions or data mismatch.
    #[error("Image data error: {0}")]
    ImageData(String),

    /// Compression ratio constraint violation.
    #[error("Compression constraint violation: {0}")]
    CompressionConstraint(String),

    /// Generic internal error.
    #[error("Internal error: {0}")]
    Internal(String),
}

impl From<dicom::object::ReadError> for MedImgError {
    fn from(err: dicom::object::ReadError) -> Self {
        MedImgError::Dicom(err.to_string())
    }
}

impl From<dicom::object::WriteError> for MedImgError {
    fn from(err: dicom::object::WriteError) -> Self {
        MedImgError::Dicom(err.to_string())
    }
}
