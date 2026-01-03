//! Channel-based progress reporting.
//!
//! Provides a progress handler that sends events through an MPSC channel,
//! useful for async workflows or when progress events need to be processed
//! in a separate thread.

use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::sync::Arc;
use std::time::Duration;

use crate::error::MedImgError;
use crate::pipeline::BatchStats;

use super::handler::{ProgressEvent, ProgressHandler};

/// Channel-based progress handler.
///
/// Sends progress events to a channel for consumption by another thread
/// or async context.
///
/// # Example
///
/// ```rust,ignore
/// use medimg_compress::progress::ChannelProgress;
/// use std::thread;
///
/// let (progress, receiver) = ChannelProgress::new();
///
/// // Spawn a thread to process progress events
/// thread::spawn(move || {
///     while let Ok(event) = receiver.recv() {
///         println!("Progress: {:.1}%", event.overall_progress * 100.0);
///         if event.phase.is_terminal() {
///             break;
///         }
///     }
/// });
///
/// // Use progress handler with batch processor
/// let processor = BatchProcessor::new(config, progress);
/// ```
pub struct ChannelProgress {
    /// Channel sender for progress events.
    sender: Sender<ProgressEvent>,

    /// Cancellation flag.
    cancelled: Arc<AtomicBool>,
}

impl ChannelProgress {
    /// Create a new channel progress handler.
    ///
    /// Returns the progress handler and a receiver for progress events.
    pub fn new() -> (Self, ProgressReceiver) {
        let (sender, receiver) = mpsc::channel();
        let cancelled = Arc::new(AtomicBool::new(false));

        let handler = Self {
            sender,
            cancelled: cancelled.clone(),
        };

        let progress_receiver = ProgressReceiver {
            receiver,
            cancelled,
        };

        (handler, progress_receiver)
    }

    /// Create with a bounded channel.
    ///
    /// Uses an internal bridge to convert from sync_channel.
    ///
    /// # Arguments
    ///
    /// * `capacity` - Maximum number of events to buffer
    pub fn bounded(capacity: usize) -> (Self, ProgressReceiver) {
        let (sync_sender, receiver) = mpsc::sync_channel::<ProgressEvent>(capacity);
        let cancelled = Arc::new(AtomicBool::new(false));

        // Create a bridge channel for the handler
        let (bridge_sender, bridge_receiver) = mpsc::channel::<ProgressEvent>();

        // Spawn a thread to forward events from bridge to sync channel
        std::thread::spawn(move || {
            while let Ok(event) = bridge_receiver.recv() {
                if sync_sender.send(event).is_err() {
                    break;
                }
            }
        });

        let handler = Self {
            sender: bridge_sender,
            cancelled: cancelled.clone(),
        };

        let progress_receiver = ProgressReceiver {
            receiver,
            cancelled,
        };

        (handler, progress_receiver)
    }

    /// Request cancellation.
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }
}

impl ProgressHandler for ChannelProgress {
    fn on_progress(&self, event: &ProgressEvent) {
        // Ignore send errors (receiver may have been dropped)
        let _ = self.sender.send(event.clone());
    }

    fn on_error(&self, error: &MedImgError, file: Option<&Path>) {
        let mut event = ProgressEvent::failed(error.to_string());
        event.current_file = file.map(|p| p.to_path_buf());
        let _ = self.sender.send(event);
    }

    fn on_complete(&self, stats: &BatchStats) {
        let event = ProgressEvent::complete(stats.total_files, stats.total_original_bytes as u64);
        let _ = self.sender.send(event);
    }

    fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }
}

/// Receiver for progress events.
///
/// Wraps an MPSC receiver with convenience methods.
pub struct ProgressReceiver {
    /// The underlying channel receiver.
    receiver: Receiver<ProgressEvent>,

    /// Shared cancellation flag.
    cancelled: Arc<AtomicBool>,
}

impl ProgressReceiver {
    /// Block and wait for the next progress event.
    pub fn recv(&self) -> Result<ProgressEvent, mpsc::RecvError> {
        self.receiver.recv()
    }

    /// Try to receive a progress event without blocking.
    pub fn try_recv(&self) -> Result<ProgressEvent, TryRecvError> {
        self.receiver.try_recv()
    }

    /// Wait for an event with a timeout.
    pub fn recv_timeout(&self, timeout: Duration) -> Result<ProgressEvent, mpsc::RecvTimeoutError> {
        self.receiver.recv_timeout(timeout)
    }

    /// Iterate over all received events.
    pub fn iter(&self) -> impl Iterator<Item = ProgressEvent> + '_ {
        self.receiver.iter()
    }

    /// Non-blocking iterator over available events.
    pub fn try_iter(&self) -> impl Iterator<Item = ProgressEvent> + '_ {
        self.receiver.try_iter()
    }

    /// Request cancellation of the operation.
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }

    /// Check if cancellation was requested.
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }

    /// Collect all events until completion or error.
    ///
    /// Blocks until a terminal event is received.
    pub fn collect_until_complete(&self) -> Vec<ProgressEvent> {
        let mut events = Vec::new();
        while let Ok(event) = self.recv() {
            let is_terminal = event.phase.is_terminal();
            events.push(event);
            if is_terminal {
                break;
            }
        }
        events
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::progress::ProgressPhase;

    #[test]
    fn test_channel_progress_send_receive() {
        let (progress, receiver) = ChannelProgress::new();

        let event = ProgressEvent {
            phase: ProgressPhase::Encoding,
            overall_progress: 0.5,
            message: "Test event".into(),
            ..Default::default()
        };

        progress.on_progress(&event);

        let received = receiver.try_recv().unwrap();
        assert_eq!(received.phase, ProgressPhase::Encoding);
        assert!((received.overall_progress - 0.5).abs() < 0.001);
        assert_eq!(received.message, "Test event");
    }

    #[test]
    fn test_channel_progress_cancellation() {
        let (progress, receiver) = ChannelProgress::new();

        assert!(!progress.is_cancelled());
        assert!(!receiver.is_cancelled());

        progress.cancel();

        assert!(progress.is_cancelled());
        assert!(receiver.is_cancelled());
    }

    #[test]
    fn test_channel_progress_receiver_cancel() {
        let (progress, receiver) = ChannelProgress::new();

        receiver.cancel();

        assert!(progress.is_cancelled());
    }

    #[test]
    fn test_channel_progress_on_error() {
        let (progress, receiver) = ChannelProgress::new();

        let error = MedImgError::Internal("test error".into());
        let path = std::path::Path::new("/test/file.dcm");

        progress.on_error(&error, Some(path));

        let received = receiver.try_recv().unwrap();
        assert_eq!(received.phase, ProgressPhase::Failed);
        assert!(received.message.contains("test error"));
        assert!(received.current_file.is_some());
    }

    #[test]
    fn test_channel_progress_on_complete() {
        let (progress, receiver) = ChannelProgress::new();

        let stats = BatchStats {
            total_files: 10,
            successful: 10,
            failed: 0,
            skipped: 0,
            total_original_bytes: 1000,
            total_compressed_bytes: 500,
            total_time_ms: 100,
        };

        progress.on_complete(&stats);

        let received = receiver.try_recv().unwrap();
        assert_eq!(received.phase, ProgressPhase::Complete);
        assert_eq!(received.completed_files, 10);
    }

    #[test]
    fn test_channel_try_iter() {
        let (progress, receiver) = ChannelProgress::new();

        for i in 0..5 {
            let event = ProgressEvent {
                overall_progress: i as f64 / 5.0,
                ..Default::default()
            };
            progress.on_progress(&event);
        }

        let events: Vec<_> = receiver.try_iter().collect();
        assert_eq!(events.len(), 5);
    }
}
