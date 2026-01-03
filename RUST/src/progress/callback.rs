//! Callback-based progress reporting.

use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use crate::error::MedImgError;
use crate::pipeline::BatchStats;

use super::handler::{ProgressEvent, ProgressHandler};

/// A progress handler that invokes a callback function.
///
/// # Type Parameters
///
/// * `F` - The callback function type
///
/// # Example
///
/// ```rust,ignore
/// use medimg_compress::progress::CallbackProgress;
///
/// let progress = CallbackProgress::new(|event| {
///     println!("[{:.1}%] {}", event.overall_progress * 100.0, event.message);
/// });
///
/// // With cancellation support
/// let progress = CallbackProgress::new(|_| {});
/// progress.cancel();  // Will stop processing after current file
/// ```
pub struct CallbackProgress<F>
where
    F: Fn(ProgressEvent) + Send + Sync,
{
    /// The callback function to invoke on progress.
    callback: F,

    /// Error callback (optional).
    error_callback: Option<Arc<dyn Fn(&MedImgError, Option<&Path>) + Send + Sync>>,

    /// Completion callback (optional).
    complete_callback: Option<Arc<dyn Fn(&BatchStats) + Send + Sync>>,

    /// Cancellation flag.
    cancelled: AtomicBool,
}

impl<F> CallbackProgress<F>
where
    F: Fn(ProgressEvent) + Send + Sync,
{
    /// Create a new callback progress handler.
    ///
    /// # Arguments
    ///
    /// * `callback` - Function to call on each progress update
    pub fn new(callback: F) -> Self {
        Self {
            callback,
            error_callback: None,
            complete_callback: None,
            cancelled: AtomicBool::new(false),
        }
    }

    /// Set an error callback.
    pub fn on_error<E>(mut self, callback: E) -> Self
    where
        E: Fn(&MedImgError, Option<&Path>) + Send + Sync + 'static,
    {
        self.error_callback = Some(Arc::new(callback));
        self
    }

    /// Set a completion callback.
    pub fn on_complete<C>(mut self, callback: C) -> Self
    where
        C: Fn(&BatchStats) + Send + Sync + 'static,
    {
        self.complete_callback = Some(Arc::new(callback));
        self
    }

    /// Request cancellation of the current operation.
    ///
    /// The operation will stop after completing the current file.
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }

    /// Reset the cancellation flag.
    pub fn reset(&self) {
        self.cancelled.store(false, Ordering::SeqCst);
    }
}

impl<F> ProgressHandler for CallbackProgress<F>
where
    F: Fn(ProgressEvent) + Send + Sync,
{
    fn on_progress(&self, event: &ProgressEvent) {
        (self.callback)(event.clone());
    }

    fn on_error(&self, error: &MedImgError, file: Option<&Path>) {
        if let Some(ref callback) = self.error_callback {
            callback(error, file);
        }
    }

    fn on_complete(&self, stats: &BatchStats) {
        if let Some(ref callback) = self.complete_callback {
            callback(stats);
        }
    }

    fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }
}

/// A builder for creating callback progress handlers with multiple callbacks.
///
/// # Example
///
/// ```rust,ignore
/// use medimg_compress::progress::CallbackProgressBuilder;
///
/// let progress = CallbackProgressBuilder::new()
///     .on_progress(|event| println!("Progress: {:.1}%", event.overall_progress * 100.0))
///     .on_error(|err, file| eprintln!("Error: {} ({:?})", err, file))
///     .on_complete(|stats| println!("Done: {} files", stats.total_files))
///     .build();
/// ```
pub struct CallbackProgressBuilder {
    progress_callback: Option<Arc<dyn Fn(ProgressEvent) + Send + Sync>>,
    error_callback: Option<Arc<dyn Fn(&MedImgError, Option<&Path>) + Send + Sync>>,
    complete_callback: Option<Arc<dyn Fn(&BatchStats) + Send + Sync>>,
}

impl Default for CallbackProgressBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl CallbackProgressBuilder {
    /// Create a new builder.
    pub fn new() -> Self {
        Self {
            progress_callback: None,
            error_callback: None,
            complete_callback: None,
        }
    }

    /// Set the progress callback.
    pub fn on_progress<F>(mut self, callback: F) -> Self
    where
        F: Fn(ProgressEvent) + Send + Sync + 'static,
    {
        self.progress_callback = Some(Arc::new(callback));
        self
    }

    /// Set the error callback.
    pub fn on_error<F>(mut self, callback: F) -> Self
    where
        F: Fn(&MedImgError, Option<&Path>) + Send + Sync + 'static,
    {
        self.error_callback = Some(Arc::new(callback));
        self
    }

    /// Set the completion callback.
    pub fn on_complete<F>(mut self, callback: F) -> Self
    where
        F: Fn(&BatchStats) + Send + Sync + 'static,
    {
        self.complete_callback = Some(Arc::new(callback));
        self
    }

    /// Build the progress handler.
    pub fn build(self) -> BuiltCallbackProgress {
        BuiltCallbackProgress {
            progress_callback: self.progress_callback,
            error_callback: self.error_callback,
            complete_callback: self.complete_callback,
            cancelled: AtomicBool::new(false),
        }
    }
}

/// A progress handler built from CallbackProgressBuilder.
pub struct BuiltCallbackProgress {
    progress_callback: Option<Arc<dyn Fn(ProgressEvent) + Send + Sync>>,
    error_callback: Option<Arc<dyn Fn(&MedImgError, Option<&Path>) + Send + Sync>>,
    complete_callback: Option<Arc<dyn Fn(&BatchStats) + Send + Sync>>,
    cancelled: AtomicBool,
}

impl BuiltCallbackProgress {
    /// Request cancellation.
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }

    /// Reset the cancellation flag.
    pub fn reset(&self) {
        self.cancelled.store(false, Ordering::SeqCst);
    }
}

impl ProgressHandler for BuiltCallbackProgress {
    fn on_progress(&self, event: &ProgressEvent) {
        if let Some(ref callback) = self.progress_callback {
            callback(event.clone());
        }
    }

    fn on_error(&self, error: &MedImgError, file: Option<&Path>) {
        if let Some(ref callback) = self.error_callback {
            callback(error, file);
        }
    }

    fn on_complete(&self, stats: &BatchStats) {
        if let Some(ref callback) = self.complete_callback {
            callback(stats);
        }
    }

    fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicUsize;

    #[test]
    fn test_callback_progress_new() {
        let count = Arc::new(AtomicUsize::new(0));
        let count_clone = count.clone();

        let progress = CallbackProgress::new(move |_| {
            count_clone.fetch_add(1, Ordering::SeqCst);
        });

        progress.on_progress(&ProgressEvent::default());
        assert_eq!(count.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_callback_progress_with_error_handler() {
        let error_count = Arc::new(AtomicUsize::new(0));
        let error_count_clone = error_count.clone();

        let progress = CallbackProgress::new(|_| {})
            .on_error(move |_, _| {
                error_count_clone.fetch_add(1, Ordering::SeqCst);
            });

        let error = MedImgError::Internal("test".into());
        // Use the trait method via ProgressHandler
        ProgressHandler::on_error(&progress, &error, None);
        assert_eq!(error_count.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_callback_progress_builder() {
        let progress_count = Arc::new(AtomicUsize::new(0));
        let error_count = Arc::new(AtomicUsize::new(0));

        let progress_clone = progress_count.clone();
        let error_clone = error_count.clone();

        let handler = CallbackProgressBuilder::new()
            .on_progress(move |_| {
                progress_clone.fetch_add(1, Ordering::SeqCst);
            })
            .on_error(move |_, _| {
                error_clone.fetch_add(1, Ordering::SeqCst);
            })
            .build();

        handler.on_progress(&ProgressEvent::default());
        handler.on_error(&MedImgError::Internal("test".into()), None);

        assert_eq!(progress_count.load(Ordering::SeqCst), 1);
        assert_eq!(error_count.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_built_callback_cancellation() {
        let handler = CallbackProgressBuilder::new().build();

        assert!(!handler.is_cancelled());
        handler.cancel();
        assert!(handler.is_cancelled());
        handler.reset();
        assert!(!handler.is_cancelled());
    }
}
