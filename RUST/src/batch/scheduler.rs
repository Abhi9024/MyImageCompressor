//! Batch job scheduler using Rayon.

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;

use rayon::prelude::*;

use super::job::{BatchJob, JobResult};

/// Batch job scheduler for parallel processing.
pub struct BatchScheduler {
    /// Number of threads to use.
    num_threads: usize,

    /// Cancellation flag.
    cancelled: Arc<AtomicBool>,

    /// Number of jobs completed.
    completed: Arc<AtomicUsize>,
}

impl BatchScheduler {
    /// Create a new scheduler with the specified number of threads.
    pub fn new(num_threads: usize) -> Self {
        Self {
            num_threads: num_threads.max(1),
            cancelled: Arc::new(AtomicBool::new(false)),
            completed: Arc::new(AtomicUsize::new(0)),
        }
    }

    /// Get the number of threads.
    pub fn num_threads(&self) -> usize {
        self.num_threads
    }

    /// Get the number of completed jobs.
    pub fn completed(&self) -> usize {
        self.completed.load(Ordering::SeqCst)
    }

    /// Request cancellation.
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }

    /// Check if cancelled.
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }

    /// Reset the scheduler state.
    pub fn reset(&self) {
        self.cancelled.store(false, Ordering::SeqCst);
        self.completed.store(0, Ordering::SeqCst);
    }

    /// Schedule jobs for parallel execution.
    ///
    /// # Arguments
    ///
    /// * `jobs` - Jobs to process
    /// * `processor` - Function to process each job
    ///
    /// # Returns
    ///
    /// Vector of job results in completion order (not necessarily input order).
    pub fn schedule<F>(&self, jobs: Vec<BatchJob>, processor: F) -> Vec<JobResult>
    where
        F: Fn(&BatchJob) -> JobResult + Send + Sync,
    {
        let cancelled = self.cancelled.clone();
        let completed = self.completed.clone();

        // Build thread pool
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(self.num_threads)
            .build()
            .expect("Failed to create thread pool");

        pool.install(|| {
            jobs.into_par_iter()
                .map(|job| {
                    // Check for cancellation
                    if cancelled.load(Ordering::SeqCst) {
                        return JobResult {
                            job: job.clone(),
                            compression_result: None,
                            error: Some(crate::error::MedImgError::Internal("Cancelled".into())),
                            duration_ms: 0,
                        };
                    }

                    // Process the job
                    let result = processor(&job);

                    // Increment completed count
                    completed.fetch_add(1, Ordering::SeqCst);

                    result
                })
                .collect()
        })
    }

    /// Schedule jobs with a progress callback.
    pub fn schedule_with_progress<F, P>(
        &self,
        jobs: Vec<BatchJob>,
        processor: F,
        progress: P,
    ) -> Vec<JobResult>
    where
        F: Fn(&BatchJob) -> JobResult + Send + Sync,
        P: Fn(usize, usize) + Send + Sync,
    {
        let cancelled = self.cancelled.clone();
        let completed = self.completed.clone();
        let total = jobs.len();

        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(self.num_threads)
            .build()
            .expect("Failed to create thread pool");

        pool.install(|| {
            jobs.into_par_iter()
                .map(|job| {
                    if cancelled.load(Ordering::SeqCst) {
                        return JobResult {
                            job: job.clone(),
                            compression_result: None,
                            error: Some(crate::error::MedImgError::Internal("Cancelled".into())),
                            duration_ms: 0,
                        };
                    }

                    let result = processor(&job);
                    let done = completed.fetch_add(1, Ordering::SeqCst) + 1;
                    progress(done, total);

                    result
                })
                .collect()
        })
    }
}

impl Default for BatchScheduler {
    fn default() -> Self {
        Self::new(num_cpus::get())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::time::Duration;

    #[test]
    fn test_scheduler_creation() {
        let scheduler = BatchScheduler::new(4);
        assert_eq!(scheduler.num_threads(), 4);
        assert!(!scheduler.is_cancelled());
        assert_eq!(scheduler.completed(), 0);
    }

    #[test]
    fn test_scheduler_default() {
        let scheduler = BatchScheduler::default();
        assert_eq!(scheduler.num_threads(), num_cpus::get());
    }

    #[test]
    fn test_scheduler_cancellation() {
        let scheduler = BatchScheduler::new(4);
        assert!(!scheduler.is_cancelled());
        scheduler.cancel();
        assert!(scheduler.is_cancelled());
        scheduler.reset();
        assert!(!scheduler.is_cancelled());
    }

    #[test]
    fn test_scheduler_schedule_empty() {
        let scheduler = BatchScheduler::new(2);
        let results = scheduler.schedule(vec![], |_| unreachable!());
        assert!(results.is_empty());
    }

    #[test]
    fn test_scheduler_schedule_jobs() {
        let scheduler = BatchScheduler::new(2);
        let jobs: Vec<BatchJob> = (0..5)
            .map(|i| BatchJob::new(i, PathBuf::from(format!("/test/{}.dcm", i))))
            .collect();

        let results = scheduler.schedule(jobs, |job| JobResult {
            job: job.clone(),
            compression_result: None,
            error: None,
            duration_ms: 10,
        });

        assert_eq!(results.len(), 5);
        assert_eq!(scheduler.completed(), 5);
    }

    #[test]
    fn test_scheduler_schedule_with_progress() {
        let scheduler = BatchScheduler::new(2);
        let jobs: Vec<BatchJob> = (0..3)
            .map(|i| BatchJob::new(i, PathBuf::from(format!("/test/{}.dcm", i))))
            .collect();

        let progress_count = Arc::new(AtomicUsize::new(0));
        let progress_clone = progress_count.clone();

        let results = scheduler.schedule_with_progress(
            jobs,
            |job| JobResult {
                job: job.clone(),
                compression_result: None,
                error: None,
                duration_ms: 10,
            },
            move |_done, _total| {
                progress_clone.fetch_add(1, Ordering::SeqCst);
            },
        );

        assert_eq!(results.len(), 3);
        assert_eq!(progress_count.load(Ordering::SeqCst), 3);
    }

    #[test]
    fn test_scheduler_cancel_during_execution() {
        let scheduler = BatchScheduler::new(1);
        let jobs: Vec<BatchJob> = (0..10)
            .map(|i| BatchJob::new(i, PathBuf::from(format!("/test/{}.dcm", i))))
            .collect();

        // Cancel immediately
        scheduler.cancel();

        let results = scheduler.schedule(jobs, |job| {
            JobResult {
                job: job.clone(),
                compression_result: None,
                error: None,
                duration_ms: 0,
            }
        });

        // All jobs should be marked as cancelled
        for result in &results {
            assert!(result.error.is_some());
        }
    }
}
