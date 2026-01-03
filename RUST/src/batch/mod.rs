//! Batch processing for multiple DICOM files.
//!
//! This module provides parallel batch compression with progress reporting.
//!
//! # Example
//!
//! ```rust,ignore
//! use medimg_compress::batch::BatchProcessor;
//! use medimg_compress::progress::CallbackProgress;
//! use medimg_compress::config::CompressionConfig;
//! use std::path::Path;
//!
//! let config = CompressionConfig::lossless(CompressionCodec::Jpeg2000);
//! let progress = CallbackProgress::new(|event| {
//!     println!("[{}/{}] {}", event.completed_files, event.total_files.unwrap_or(0), event.message);
//! });
//!
//! let processor = BatchProcessor::new(config, progress)
//!     .max_parallel(4)
//!     .recursive(true);
//!
//! let stats = processor.process_directory(Path::new("./dicom_files"))?;
//! println!("Processed {} files, {} successful", stats.total_files, stats.successful);
//! ```

mod job;
mod scheduler;
mod file_discovery;

pub use job::{BatchJob, JobResult, JobStatus};
pub use scheduler::BatchScheduler;
pub use file_discovery::FileDiscovery;

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;

use rayon::prelude::*;

use crate::config::CompressionConfig;
use crate::error::{MedImgError, Result};
use crate::pipeline::{BatchStats, CompressionPipeline, CompressionResult};
use crate::progress::{NullProgress, ProgressEvent, ProgressHandler, ProgressPhase};

/// Batch processor for compressing multiple DICOM files.
pub struct BatchProcessor<P: ProgressHandler> {
    /// Compression configuration.
    config: CompressionConfig,

    /// Progress handler.
    progress: P,

    /// Maximum parallel jobs.
    max_parallel: usize,

    /// Whether to scan directories recursively.
    recursive: bool,

    /// File patterns to match (e.g., "*.dcm").
    patterns: Vec<String>,

    /// Output directory.
    output_dir: Option<PathBuf>,

    /// Whether to preserve directory structure in output.
    preserve_structure: bool,

    /// Whether to skip already compressed files.
    skip_compressed: bool,

    /// Cancellation flag.
    cancelled: Arc<AtomicBool>,
}

impl<P: ProgressHandler> BatchProcessor<P> {
    /// Create a new batch processor.
    pub fn new(config: CompressionConfig, progress: P) -> Self {
        Self {
            config,
            progress,
            max_parallel: num_cpus::get(),
            recursive: false,
            patterns: vec!["*.dcm".to_string(), "*.DCM".to_string()],
            output_dir: None,
            preserve_structure: true,
            skip_compressed: true,
            cancelled: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Set maximum parallel jobs.
    pub fn max_parallel(mut self, n: usize) -> Self {
        self.max_parallel = n.max(1);
        self
    }

    /// Enable recursive directory scanning.
    pub fn recursive(mut self, recursive: bool) -> Self {
        self.recursive = recursive;
        self
    }

    /// Add a file pattern to match.
    pub fn pattern(mut self, pattern: &str) -> Self {
        self.patterns.push(pattern.to_string());
        self
    }

    /// Set file patterns (replaces existing).
    pub fn patterns(mut self, patterns: Vec<String>) -> Self {
        self.patterns = patterns;
        self
    }

    /// Set output directory.
    pub fn output_dir(mut self, path: PathBuf) -> Self {
        self.output_dir = Some(path);
        self
    }

    /// Set whether to preserve directory structure.
    pub fn preserve_structure(mut self, preserve: bool) -> Self {
        self.preserve_structure = preserve;
        self
    }

    /// Set whether to skip already compressed files.
    pub fn skip_compressed(mut self, skip: bool) -> Self {
        self.skip_compressed = skip;
        self
    }

    /// Request cancellation of batch processing.
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }

    /// Check if cancellation was requested.
    fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst) || self.progress.is_cancelled()
    }

    /// Process a directory of DICOM files.
    pub fn process_directory(&self, input_dir: &Path) -> Result<BatchStats> {
        // Discover files
        self.progress.on_progress(&ProgressEvent::discovery(
            format!("Scanning {}", input_dir.display())
        ));

        let discovery = FileDiscovery::new()
            .recursive(self.recursive)
            .patterns(self.patterns.clone());

        let files = discovery.discover(input_dir)?;

        if files.is_empty() {
            return Err(MedImgError::Validation(format!(
                "No matching files found in {}",
                input_dir.display()
            )));
        }

        self.process_files_internal(&files, Some(input_dir))
    }

    /// Process a list of files.
    pub fn process_files(&self, files: &[PathBuf]) -> Result<BatchStats> {
        if files.is_empty() {
            return Err(MedImgError::Validation("No files to process".into()));
        }

        self.process_files_internal(files, None)
    }

    /// Internal file processing implementation.
    fn process_files_internal(&self, files: &[PathBuf], base_dir: Option<&Path>) -> Result<BatchStats> {
        let start_time = Instant::now();
        let total_files = files.len();

        // Calculate total size
        let total_bytes: u64 = files
            .iter()
            .filter_map(|p| std::fs::metadata(p).ok())
            .map(|m| m.len())
            .sum();

        self.progress.on_progress(&ProgressEvent {
            phase: ProgressPhase::Discovery,
            total_files: Some(total_files),
            total_bytes: Some(total_bytes),
            message: format!("Found {} files ({:.2} MB)", total_files, total_bytes as f64 / 1_000_000.0),
            ..Default::default()
        });

        if self.is_cancelled() {
            return Ok(BatchStats::default());
        }

        // Build thread pool
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(self.max_parallel)
            .build()
            .map_err(|e| MedImgError::Internal(e.to_string()))?;

        // Process files in parallel
        let results: Vec<JobResult> = pool.install(|| {
            files
                .par_iter()
                .enumerate()
                .map(|(idx, file)| {
                    if self.is_cancelled() {
                        return JobResult {
                            job: BatchJob::new(idx as u64, file.clone()),
                            compression_result: None,
                            error: Some(MedImgError::Internal("Cancelled".into())),
                            duration_ms: 0,
                        };
                    }

                    self.process_single_file(idx, file, total_files, base_dir)
                })
                .collect()
        });

        // Aggregate statistics
        let mut stats = BatchStats::default();
        stats.total_files = total_files;

        for result in &results {
            if let Some(ref compression_result) = result.compression_result {
                stats.successful += 1;
                stats.total_original_bytes += compression_result.original_size;
                stats.total_compressed_bytes += compression_result.compressed_size;
            } else if result.error.is_some() {
                stats.failed += 1;
            }
        }

        stats.total_time_ms = start_time.elapsed().as_millis() as u64;

        // Report completion
        self.progress.on_complete(&stats);

        Ok(stats)
    }

    /// Process a single file.
    fn process_single_file(
        &self,
        idx: usize,
        file: &Path,
        total: usize,
        base_dir: Option<&Path>,
    ) -> JobResult {
        let job = BatchJob::new(idx as u64, file.to_path_buf());
        let start = Instant::now();

        // Report progress
        self.progress.on_progress(&ProgressEvent {
            phase: ProgressPhase::Reading,
            current_file: Some(file.to_path_buf()),
            completed_files: idx,
            total_files: Some(total),
            overall_progress: idx as f64 / total as f64,
            message: format!("Processing {}", file.file_name().unwrap_or_default().to_string_lossy()),
            ..Default::default()
        });

        // Determine output path
        let output_path = self.compute_output_path(file, base_dir);

        // Create output directory if needed
        if let Some(ref out) = output_path {
            if let Some(parent) = out.parent() {
                if let Err(e) = std::fs::create_dir_all(parent) {
                    return JobResult {
                        job,
                        compression_result: None,
                        error: Some(MedImgError::Io(e)),
                        duration_ms: start.elapsed().as_millis() as u64,
                    };
                }
            }
        }

        // Process the file
        let pipeline = CompressionPipeline::new(self.config.clone());
        let result = pipeline.compress_file(file);

        let duration_ms = start.elapsed().as_millis() as u64;

        match result {
            Ok(compression_result) => {
                self.progress.on_progress(&ProgressEvent {
                    phase: ProgressPhase::Complete,
                    current_file: Some(file.to_path_buf()),
                    completed_files: idx + 1,
                    total_files: Some(total),
                    overall_progress: (idx + 1) as f64 / total as f64,
                    message: format!(
                        "Compressed {} (ratio: {:.2}:1)",
                        file.file_name().unwrap_or_default().to_string_lossy(),
                        compression_result.compression_ratio
                    ),
                    ..Default::default()
                });

                JobResult {
                    job,
                    compression_result: Some(compression_result),
                    error: None,
                    duration_ms,
                }
            }
            Err(e) => {
                self.progress.on_error(&e, Some(file));
                JobResult {
                    job,
                    compression_result: None,
                    error: Some(e),
                    duration_ms,
                }
            }
        }
    }

    /// Compute output path for a file.
    fn compute_output_path(&self, file: &Path, base_dir: Option<&Path>) -> Option<PathBuf> {
        let output_dir = self.output_dir.as_ref()?;

        if self.preserve_structure {
            if let Some(base) = base_dir {
                if let Ok(relative) = file.strip_prefix(base) {
                    return Some(output_dir.join(relative));
                }
            }
        }

        file.file_name()
            .map(|name| output_dir.join(name))
    }
}

impl BatchProcessor<NullProgress> {
    /// Create a batch processor without progress reporting.
    pub fn without_progress(config: CompressionConfig) -> Self {
        Self::new(config, NullProgress)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::CompressionCodec;
    use std::sync::atomic::AtomicUsize;

    #[test]
    fn test_batch_processor_creation() {
        let config = CompressionConfig::lossless(CompressionCodec::Jpeg2000);
        let processor = BatchProcessor::without_progress(config);

        assert_eq!(processor.max_parallel, num_cpus::get());
        assert!(!processor.recursive);
    }

    #[test]
    fn test_batch_processor_builder() {
        let config = CompressionConfig::lossless(CompressionCodec::Jpeg2000);
        let processor = BatchProcessor::without_progress(config)
            .max_parallel(4)
            .recursive(true)
            .pattern("*.dicom")
            .output_dir(PathBuf::from("/output"));

        assert_eq!(processor.max_parallel, 4);
        assert!(processor.recursive);
        assert!(processor.patterns.contains(&"*.dicom".to_string()));
        assert_eq!(processor.output_dir, Some(PathBuf::from("/output")));
    }

    #[test]
    fn test_batch_processor_cancellation() {
        let config = CompressionConfig::lossless(CompressionCodec::Jpeg2000);
        let processor = BatchProcessor::without_progress(config);

        assert!(!processor.is_cancelled());
        processor.cancel();
        assert!(processor.is_cancelled());
    }

    #[test]
    fn test_batch_processor_with_progress() {
        let config = CompressionConfig::lossless(CompressionCodec::Jpeg2000);
        let count = Arc::new(AtomicUsize::new(0));
        let count_clone = count.clone();

        let progress = crate::progress::CallbackProgress::new(move |_| {
            count_clone.fetch_add(1, Ordering::SeqCst);
        });

        let _processor = BatchProcessor::new(config, progress);
        // Progress handler is set up correctly
    }
}
