//! Progress reporting for compression operations.
//!
//! This module provides a flexible progress reporting API that supports:
//! - Callback-based progress reporting
//! - Channel-based progress for async workflows
//! - Cancellation support
//!
//! # Example
//!
//! ```rust,ignore
//! use medimg_compress::progress::{CallbackProgress, ProgressEvent, ProgressHandler};
//!
//! // Create a callback-based progress handler
//! let progress = CallbackProgress::new(|event| {
//!     println!("Progress: {:.1}%", event.overall_progress * 100.0);
//! });
//!
//! // Use with batch processing
//! let processor = BatchProcessor::new(config, progress);
//! ```

mod handler;
mod callback;
mod channel;

pub use handler::{ProgressEvent, ProgressHandler, ProgressPhase, NullProgress};
pub use callback::{CallbackProgress, CallbackProgressBuilder, BuiltCallbackProgress};
pub use channel::{ChannelProgress, ProgressReceiver};

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    #[test]
    fn test_null_progress() {
        let progress = NullProgress;

        let event = ProgressEvent {
            phase: ProgressPhase::Encoding,
            current_file: None,
            total_files: Some(10),
            completed_files: 5,
            file_progress: 0.5,
            overall_progress: 0.5,
            bytes_processed: 1024,
            total_bytes: Some(2048),
            throughput_bps: 100.0,
            eta_seconds: Some(10.0),
            message: "Processing...".into(),
        };

        // Should not panic
        progress.on_progress(&event);
        assert!(!progress.is_cancelled());
    }

    #[test]
    fn test_callback_progress_receives_events() {
        let count = Arc::new(AtomicUsize::new(0));
        let count_clone = count.clone();

        let progress = CallbackProgress::new(move |_event| {
            count_clone.fetch_add(1, Ordering::SeqCst);
        });

        let event = ProgressEvent::default();
        progress.on_progress(&event);
        progress.on_progress(&event);

        assert_eq!(count.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn test_callback_progress_cancellation() {
        let progress = CallbackProgress::new(|_| {});

        assert!(!progress.is_cancelled());
        progress.cancel();
        assert!(progress.is_cancelled());
    }

    #[test]
    fn test_channel_progress() {
        let (progress, receiver) = ChannelProgress::new();

        let event = ProgressEvent {
            phase: ProgressPhase::Reading,
            overall_progress: 0.5,
            message: "Test".into(),
            ..Default::default()
        };

        progress.on_progress(&event);

        let received = receiver.try_recv().unwrap();
        assert_eq!(received.overall_progress, 0.5);
        assert_eq!(received.message, "Test");
    }
}
