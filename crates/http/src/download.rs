use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

pub const DEFAULT_DOWNLOAD_BUFFER_KIB: usize = 128;

#[derive(Debug, thiserror::Error)]
#[error("Choose a download buffer from 16 to 128 KiB in 16 KiB steps")]
pub struct DownloadBufferError;

/// Application-side tuning; no HTTP endpoint exposes this control. A request
/// snapshots its buffer once, keeping active streams independent of changes.
#[derive(Clone, Debug)]
pub struct DownloadControl {
    buffer_bytes: Arc<AtomicUsize>,
}

impl Default for DownloadControl {
    fn default() -> Self {
        Self {
            buffer_bytes: Arc::new(AtomicUsize::new(DEFAULT_DOWNLOAD_BUFFER_KIB * 1024)),
        }
    }
}

impl DownloadControl {
    pub fn buffer_kib(&self) -> usize {
        self.buffer_bytes() / 1024
    }

    pub fn set_buffer_kib(&self, kib: usize) -> Result<(), DownloadBufferError> {
        if !(16..=128).contains(&kib) || !kib.is_multiple_of(16) {
            return Err(DownloadBufferError);
        }
        self.buffer_bytes.store(kib * 1024, Ordering::Relaxed);
        Ok(())
    }

    pub fn reset(&self) {
        self.buffer_bytes
            .store(DEFAULT_DOWNLOAD_BUFFER_KIB * 1024, Ordering::Relaxed);
    }

    pub(crate) fn buffer_bytes(&self) -> usize {
        self.buffer_bytes.load(Ordering::Relaxed)
    }
}
