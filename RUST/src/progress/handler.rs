//! Progress handler trait and related types.

use crate::error::MedImgError;
use crate::pipeline::BatchStats;
use std::path::Path;

/// Phase of compression operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProgressPhase {
    /// Discovering files to process.
    Discovery,
    /// Reading DICOM file.
    Reading,
    /// Encoding/compressing image data.
    Encoding,
    /// Verifying lossless compression.
    Verification,
    /// Writing output file.
    Writing,
    /// Operation completed successfully.
    Complete,
    /// Operation failed.
    Failed,
}

impl ProgressPhase {
    /// Get a human-readable description of the phase.
    pub fn description(&self) -> &'static str {
        match self {
            Self::Discovery => "Discovering files",
            Self::Reading => "Reading DICOM",
            Self::Encoding => "Compressing",
            Self::Verification => "Verifying",
            Self::Writing => "Writing output",
            Self::Complete => "Complete",
            Self::Failed => "Failed",
        }
    }

    /// Check if this is a terminal phase.
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Complete | Self::Failed)
    }
}

impl std::fmt::Display for ProgressPhase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.description())
    }
}

/// Progress event emitted during compression operations.
#[derive(Debug, Clone)]
pub struct ProgressEvent {
    /// Current phase of operation.
    pub phase: ProgressPhase,

    /// Current file being processed (for batch operations).
    pub current_file: Option<std::path::PathBuf>,

    /// Total files in batch (if applicable).
    pub total_files: Option<usize>,

    /// Number of files completed so far.
    pub completed_files: usize,

    /// Progress within current file (0.0 to 1.0).
    pub file_progress: f64,

    /// Overall progress (0.0 to 1.0).
    pub overall_progress: f64,

    /// Bytes processed so far.
    pub bytes_processed: u64,

    /// Total bytes to process (if known).
    pub total_bytes: Option<u64>,

    /// Current throughput in bytes per second.
    pub throughput_bps: f64,

    /// Estimated time remaining in seconds.
    pub eta_seconds: Option<f64>,

    /// Status message.
    pub message: String,
}

impl Default for ProgressEvent {
    fn default() -> Self {
        Self {
            phase: ProgressPhase::Discovery,
            current_file: None,
            total_files: None,
            completed_files: 0,
            file_progress: 0.0,
            overall_progress: 0.0,
            bytes_processed: 0,
            total_bytes: None,
            throughput_bps: 0.0,
            eta_seconds: None,
            message: String::new(),
        }
    }
}

impl ProgressEvent {
    /// Create a new progress event for a specific phase.
    pub fn new(phase: ProgressPhase) -> Self {
        Self {
            phase,
            message: phase.description().into(),
            ..Default::default()
        }
    }

    /// Create a discovery phase event.
    pub fn discovery(message: impl Into<String>) -> Self {
        Self {
            phase: ProgressPhase::Discovery,
            message: message.into(),
            ..Default::default()
        }
    }

    /// Create a reading phase event.
    pub fn reading(file: &Path) -> Self {
        Self {
            phase: ProgressPhase::Reading,
            current_file: Some(file.to_path_buf()),
            message: format!("Reading {}", file.display()),
            ..Default::default()
        }
    }

    /// Create an encoding phase event.
    pub fn encoding(file: &Path, progress: f64) -> Self {
        Self {
            phase: ProgressPhase::Encoding,
            current_file: Some(file.to_path_buf()),
            file_progress: progress,
            message: format!("Compressing {}", file.display()),
            ..Default::default()
        }
    }

    /// Create a completion event.
    pub fn complete(files_processed: usize, total_bytes: u64) -> Self {
        Self {
            phase: ProgressPhase::Complete,
            completed_files: files_processed,
            overall_progress: 1.0,
            bytes_processed: total_bytes,
            message: format!("Completed {} files", files_processed),
            ..Default::default()
        }
    }

    /// Create a failure event.
    pub fn failed(message: impl Into<String>) -> Self {
        Self {
            phase: ProgressPhase::Failed,
            message: message.into(),
            ..Default::default()
        }
    }

    /// Set batch progress information.
    pub fn with_batch_progress(
        mut self,
        completed: usize,
        total: usize,
        bytes_processed: u64,
        total_bytes: Option<u64>,
    ) -> Self {
        self.completed_files = completed;
        self.total_files = Some(total);
        self.bytes_processed = bytes_processed;
        self.total_bytes = total_bytes;
        if total > 0 {
            self.overall_progress = completed as f64 / total as f64;
        }
        self
    }

    /// Set throughput and ETA.
    pub fn with_timing(mut self, throughput_bps: f64, eta_seconds: Option<f64>) -> Self {
        self.throughput_bps = throughput_bps;
        self.eta_seconds = eta_seconds;
        self
    }
}

impl std::fmt::Display for ProgressEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(total) = self.total_files {
            write!(
                f,
                "[{}/{}] {}: {}",
                self.completed_files, total, self.phase, self.message
            )
        } else {
            write!(f, "{}: {}", self.phase, self.message)
        }
    }
}

/// Trait for handling progress updates during compression.
///
/// Implement this trait to receive progress events from batch processing
/// or long-running compression operations.
///
/// # Example
///
/// ```rust,ignore
/// use medimg_compress::progress::{ProgressHandler, ProgressEvent};
/// use medimg_compress::error::MedImgError;
/// use medimg_compress::pipeline::BatchStats;
/// use std::path::Path;
///
/// struct MyProgressHandler;
///
/// impl ProgressHandler for MyProgressHandler {
///     fn on_progress(&self, event: &ProgressEvent) {
///         println!("Progress: {:.1}%", event.overall_progress * 100.0);
///     }
///
///     fn on_error(&self, error: &MedImgError, file: Option<&Path>) {
///         eprintln!("Error: {} (file: {:?})", error, file);
///     }
///
///     fn on_complete(&self, stats: &BatchStats) {
///         println!("Done! {} files processed", stats.total_files);
///     }
///
///     fn is_cancelled(&self) -> bool {
///         false
///     }
/// }
/// ```
pub trait ProgressHandler: Send + Sync {
    /// Called when progress is updated.
    fn on_progress(&self, event: &ProgressEvent);

    /// Called when an error occurs.
    ///
    /// # Arguments
    ///
    /// * `error` - The error that occurred
    /// * `file` - The file being processed when the error occurred (if applicable)
    fn on_error(&self, error: &MedImgError, file: Option<&Path>) {
        // Default implementation does nothing
        let _ = (error, file);
    }

    /// Called when processing completes.
    fn on_complete(&self, stats: &BatchStats) {
        // Default implementation does nothing
        let _ = stats;
    }

    /// Check if operation should be cancelled.
    ///
    /// Return `true` to cancel the current operation.
    /// The operation will stop after completing the current file.
    fn is_cancelled(&self) -> bool {
        false
    }
}

/// A no-op progress handler that does nothing.
///
/// Use this when you don't need progress reporting.
#[derive(Debug, Clone, Copy, Default)]
pub struct NullProgress;

impl ProgressHandler for NullProgress {
    fn on_progress(&self, _event: &ProgressEvent) {}
    fn on_error(&self, _error: &MedImgError, _file: Option<&Path>) {}
    fn on_complete(&self, _stats: &BatchStats) {}
    fn is_cancelled(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_progress_phase_display() {
        assert_eq!(ProgressPhase::Encoding.to_string(), "Compressing");
        assert_eq!(ProgressPhase::Complete.to_string(), "Complete");
    }

    #[test]
    fn test_progress_phase_is_terminal() {
        assert!(ProgressPhase::Complete.is_terminal());
        assert!(ProgressPhase::Failed.is_terminal());
        assert!(!ProgressPhase::Encoding.is_terminal());
    }

    #[test]
    fn test_progress_event_creation() {
        let event = ProgressEvent::new(ProgressPhase::Encoding);
        assert_eq!(event.phase, ProgressPhase::Encoding);
        assert_eq!(event.message, "Compressing");
    }

    #[test]
    fn test_progress_event_with_batch() {
        let event = ProgressEvent::new(ProgressPhase::Encoding)
            .with_batch_progress(5, 10, 5000, Some(10000));

        assert_eq!(event.completed_files, 5);
        assert_eq!(event.total_files, Some(10));
        assert!((event.overall_progress - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_progress_event_display() {
        let event = ProgressEvent::new(ProgressPhase::Encoding)
            .with_batch_progress(3, 10, 0, None);

        let display = format!("{}", event);
        assert!(display.contains("[3/10]"));
        assert!(display.contains("Compressing"));
    }
}
