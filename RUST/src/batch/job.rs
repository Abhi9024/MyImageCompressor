//! Batch job definitions.

use std::path::PathBuf;

use crate::error::MedImgError;
use crate::pipeline::CompressionResult;

/// Status of a batch job.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobStatus {
    /// Job is waiting to be processed.
    Pending,
    /// Job is currently being processed.
    Running,
    /// Job completed successfully.
    Completed,
    /// Job failed with an error.
    Failed,
    /// Job was cancelled.
    Cancelled,
    /// Job was skipped (e.g., already compressed).
    Skipped,
}

impl JobStatus {
    /// Check if the job is in a terminal state.
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            Self::Completed | Self::Failed | Self::Cancelled | Self::Skipped
        )
    }

    /// Check if the job completed successfully.
    pub fn is_success(&self) -> bool {
        matches!(self, Self::Completed | Self::Skipped)
    }
}

impl std::fmt::Display for JobStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pending => write!(f, "Pending"),
            Self::Running => write!(f, "Running"),
            Self::Completed => write!(f, "Completed"),
            Self::Failed => write!(f, "Failed"),
            Self::Cancelled => write!(f, "Cancelled"),
            Self::Skipped => write!(f, "Skipped"),
        }
    }
}

/// A batch compression job.
#[derive(Debug, Clone)]
pub struct BatchJob {
    /// Unique job ID.
    pub id: u64,

    /// Source file path.
    pub source_path: PathBuf,

    /// Output file path (if specified).
    pub output_path: Option<PathBuf>,

    /// Current job status.
    pub status: JobStatus,

    /// Priority (lower = higher priority).
    pub priority: u32,
}

impl BatchJob {
    /// Create a new batch job.
    pub fn new(id: u64, source_path: PathBuf) -> Self {
        Self {
            id,
            source_path,
            output_path: None,
            status: JobStatus::Pending,
            priority: 100,
        }
    }

    /// Set the output path.
    pub fn with_output(mut self, path: PathBuf) -> Self {
        self.output_path = Some(path);
        self
    }

    /// Set the priority.
    pub fn with_priority(mut self, priority: u32) -> Self {
        self.priority = priority;
        self
    }

    /// Get the file name.
    pub fn file_name(&self) -> String {
        self.source_path
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "unknown".to_string())
    }
}

/// Result of a batch job.
#[derive(Debug)]
pub struct JobResult {
    /// The job that was processed.
    pub job: BatchJob,

    /// Compression result (if successful).
    pub compression_result: Option<CompressionResult>,

    /// Error (if failed).
    pub error: Option<MedImgError>,

    /// Time taken in milliseconds.
    pub duration_ms: u64,
}

impl JobResult {
    /// Check if the job was successful.
    pub fn is_success(&self) -> bool {
        self.compression_result.is_some() && self.error.is_none()
    }

    /// Get the status based on the result.
    pub fn status(&self) -> JobStatus {
        if self.compression_result.is_some() {
            JobStatus::Completed
        } else if self.error.is_some() {
            JobStatus::Failed
        } else {
            JobStatus::Cancelled
        }
    }

    /// Get compression ratio if successful.
    pub fn compression_ratio(&self) -> Option<f64> {
        self.compression_result.as_ref().map(|r| r.compression_ratio)
    }

    /// Get original size if successful.
    pub fn original_size(&self) -> Option<usize> {
        self.compression_result.as_ref().map(|r| r.original_size)
    }

    /// Get compressed size if successful.
    pub fn compressed_size(&self) -> Option<usize> {
        self.compression_result.as_ref().map(|r| r.compressed_size)
    }
}

impl std::fmt::Display for JobResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(ref result) = self.compression_result {
            write!(
                f,
                "{}: {} (ratio: {:.2}:1, time: {}ms)",
                self.job.file_name(),
                self.status(),
                result.compression_ratio,
                self.duration_ms
            )
        } else if let Some(ref error) = self.error {
            write!(
                f,
                "{}: {} - {}",
                self.job.file_name(),
                self.status(),
                error
            )
        } else {
            write!(f, "{}: {}", self.job.file_name(), self.status())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_job_status_terminal() {
        assert!(!JobStatus::Pending.is_terminal());
        assert!(!JobStatus::Running.is_terminal());
        assert!(JobStatus::Completed.is_terminal());
        assert!(JobStatus::Failed.is_terminal());
        assert!(JobStatus::Cancelled.is_terminal());
        assert!(JobStatus::Skipped.is_terminal());
    }

    #[test]
    fn test_job_status_success() {
        assert!(!JobStatus::Pending.is_success());
        assert!(JobStatus::Completed.is_success());
        assert!(JobStatus::Skipped.is_success());
        assert!(!JobStatus::Failed.is_success());
    }

    #[test]
    fn test_batch_job_creation() {
        let job = BatchJob::new(1, PathBuf::from("/test/file.dcm"));
        assert_eq!(job.id, 1);
        assert_eq!(job.status, JobStatus::Pending);
        assert_eq!(job.file_name(), "file.dcm");
    }

    #[test]
    fn test_batch_job_builder() {
        let job = BatchJob::new(1, PathBuf::from("/test/file.dcm"))
            .with_output(PathBuf::from("/output/file.dcm"))
            .with_priority(50);

        assert_eq!(job.output_path, Some(PathBuf::from("/output/file.dcm")));
        assert_eq!(job.priority, 50);
    }

    #[test]
    fn test_job_result_success() {
        let job = BatchJob::new(1, PathBuf::from("/test/file.dcm"));
        let compression_result = CompressionResult {
            source_path: PathBuf::from("/test/file.dcm"),
            output_path: None,
            original_size: 1000,
            compressed_size: 500,
            compression_ratio: 2.0,
            compression_time_ms: 100,
            is_lossless: true,
            codec_name: "JPEG 2000".into(),
            warnings: vec![],
        };

        let result = JobResult {
            job,
            compression_result: Some(compression_result),
            error: None,
            duration_ms: 100,
        };

        assert!(result.is_success());
        assert_eq!(result.status(), JobStatus::Completed);
        assert_eq!(result.compression_ratio(), Some(2.0));
    }

    #[test]
    fn test_job_result_failure() {
        let job = BatchJob::new(1, PathBuf::from("/test/file.dcm"));
        let result = JobResult {
            job,
            compression_result: None,
            error: Some(MedImgError::Internal("Test error".into())),
            duration_ms: 50,
        };

        assert!(!result.is_success());
        assert_eq!(result.status(), JobStatus::Failed);
    }
}
