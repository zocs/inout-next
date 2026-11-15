use bytes::Bytes;
use futures_util::Stream;
use serde::Serialize;
use share_domain::Error;
use std::{
    collections::{BTreeMap, VecDeque},
    pin::Pin,
    sync::{
        atomic::{AtomicU64, AtomicU8, Ordering},
        Arc, Mutex, OnceLock,
    },
    task::{Context, Poll},
    time::{SystemTime, UNIX_EPOCH},
};

const HISTORY_LIMIT: usize = 200;
const ACTIVE_LIMIT: usize = 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum TransferKind {
    Upload,
    Download,
    Copy,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
#[repr(u8)]
pub enum TransferState {
    Active = 0,
    Published = 1,
    Served = 2,
    Interrupted = 3,
    Failed = 4,
}

#[derive(Clone, Debug, Serialize)]
pub struct TransferRecord {
    pub id: u64,
    pub kind: TransferKind,
    pub path: String,
    pub policy_revision: u64,
    pub total: Option<u64>,
    /// Observed wire bytes for uploads/downloads. For opaque copies this is
    /// the published size from a successful provider receipt, not live progress;
    /// a Copy with total=None has unavailable accounting (bytes remains zero).
    pub bytes: u64,
    pub state: TransferState,
    pub started_ms: u64,
    pub finished_ms: Option<u64>,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct TransferSnapshot {
    pub active: usize,
    pub records: Vec<TransferRecord>,
}

struct Observation {
    id: u64,
    kind: TransferKind,
    path: String,
    revision: u64,
    total: OnceLock<u64>,
    bytes: AtomicU64,
    state: AtomicU8,
    started: u64,
    finished: AtomicU64,
}

impl Observation {
    fn state(&self) -> TransferState {
        match self.state.load(Ordering::Acquire) {
            0 => TransferState::Active,
            1 => TransferState::Published,
            2 => TransferState::Served,
            3 => TransferState::Interrupted,
            _ => TransferState::Failed,
        }
    }
    fn record(&self) -> TransferRecord {
        let finished = self.finished.load(Ordering::Acquire);
        TransferRecord {
            id: self.id,
            kind: self.kind,
            path: self.path.clone(),
            policy_revision: self.revision,
            total: self.total.get().copied(),
            bytes: self.bytes.load(Ordering::Relaxed),
            state: self.state(),
            started_ms: self.started,
            finished_ms: (finished != 0).then_some(finished),
        }
    }
}

#[derive(Default)]
struct Ledger {
    sequence: u64,
    active: usize,
    records: BTreeMap<u64, Arc<Observation>>,
    finished: VecDeque<u64>,
}

/// Bounded application observation, with no sampler and no HTTP authority route.
/// Received/served bytes do not establish durability or recipient-side saving.
#[derive(Clone, Default)]
pub struct TransferControl(Arc<Mutex<Ledger>>);

impl TransferControl {
    pub fn snapshot(&self) -> TransferSnapshot {
        let ledger = self.0.lock().unwrap_or_else(|error| error.into_inner());
        let records: Vec<_> = ledger
            .records
            .values()
            .map(|value| value.record())
            .collect();
        TransferSnapshot {
            active: records
                .iter()
                .filter(|record| record.state == TransferState::Active)
                .count(),
            records,
        }
    }

    pub(crate) fn observe(
        &self,
        kind: TransferKind,
        path: &str,
        total: Option<u64>,
        revision: u64,
    ) -> Result<TransferLease, Error> {
        let mut ledger = self.0.lock().unwrap_or_else(|error| error.into_inner());
        if ledger.active >= ACTIVE_LIMIT {
            return Err(Error::Conflict);
        }
        let id = ledger.sequence.checked_add(1).ok_or(Error::Conflict)?;
        let known_total = OnceLock::new();
        if let Some(total) = total {
            let _ = known_total.set(total);
        }
        let observation = Arc::new(Observation {
            id,
            kind,
            path: path.to_owned(),
            revision,
            total: known_total,
            bytes: AtomicU64::new(0),
            state: AtomicU8::new(TransferState::Active as u8),
            started: now_ms(),
            finished: AtomicU64::new(0),
        });
        ledger.sequence = id;
        ledger.active += 1;
        ledger.records.insert(id, observation.clone());
        Ok(TransferLease {
            control: self.clone(),
            observation,
        })
    }
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX)
}

#[derive(Clone)]
pub(crate) struct Progress(Arc<Observation>);
impl Progress {
    pub fn add(&self, bytes: usize) {
        self.0.bytes.fetch_add(bytes as u64, Ordering::Relaxed);
    }
}

pub(crate) struct TransferLease {
    control: TransferControl,
    observation: Arc<Observation>,
}
impl TransferLease {
    pub fn progress(&self) -> Progress {
        Progress(self.observation.clone())
    }
    pub fn set_total(&self, total: u64) {
        let _ = self.observation.total.set(total);
    }
    pub fn finish_copy(&self, size: Option<u64>) {
        if let Some(size) = size {
            self.set_total(size);
            self.observation.bytes.store(size, Ordering::Relaxed);
        }
        self.finish(TransferState::Published);
    }
    fn add_bytes(&self, bytes: usize) {
        self.observation
            .bytes
            .fetch_add(bytes as u64, Ordering::Relaxed);
    }
    pub fn finish(&self, state: TransferState) {
        let mut ledger = self
            .control
            .0
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        if self.observation.state() != TransferState::Active {
            return;
        }
        self.observation.finished.store(now_ms(), Ordering::Release);
        self.observation.state.store(state as u8, Ordering::Release);
        ledger.active -= 1;
        ledger.finished.push_back(self.observation.id);
        while ledger.finished.len() > HISTORY_LIMIT {
            if let Some(id) = ledger.finished.pop_front() {
                ledger.records.remove(&id);
            }
        }
    }
    fn finish_download(&self) {
        let complete = self
            .observation
            .total
            .get()
            .is_some_and(|total| *total == self.observation.bytes.load(Ordering::Relaxed));
        self.finish(if complete {
            TransferState::Served
        } else {
            TransferState::Interrupted
        });
    }
}
impl Drop for TransferLease {
    fn drop(&mut self) {
        self.finish(TransferState::Interrupted);
    }
}

pub(crate) struct TrackedStream {
    stream: Pin<Box<dyn Stream<Item = Result<Bytes, std::io::Error>> + Send>>,
    lease: TransferLease,
}
impl TrackedStream {
    pub fn new(
        stream: impl Stream<Item = Result<Bytes, std::io::Error>> + Send + 'static,
        lease: TransferLease,
    ) -> Self {
        Self {
            stream: Box::pin(stream),
            lease,
        }
    }
}
impl Stream for TrackedStream {
    type Item = Result<Bytes, std::io::Error>;
    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();
        match this.stream.as_mut().poll_next(cx) {
            Poll::Ready(Some(Ok(bytes))) => {
                this.lease.add_bytes(bytes.len());
                Poll::Ready(Some(Ok(bytes)))
            }
            Poll::Ready(Some(Err(error))) => {
                this.lease.finish(TransferState::Failed);
                Poll::Ready(Some(Err(error)))
            }
            Poll::Ready(None) => {
                this.lease.finish_download();
                Poll::Ready(None)
            }
            Poll::Pending => Poll::Pending,
        }
    }
}
impl Drop for TrackedStream {
    fn drop(&mut self) {
        self.lease.finish_download();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_active_capacity_rejects_work_and_releases_one_slot_on_drop() {
        let control = TransferControl::default();
        let mut leases = Vec::new();
        for _ in 0..1024 {
            leases.push(
                control
                    .observe(TransferKind::Upload, "pending.bin", None, 1)
                    .unwrap(),
            );
        }
        assert!(matches!(
            control.observe(TransferKind::Download, "extra.bin", Some(1), 1),
            Err(Error::Conflict)
        ));
        assert_eq!(control.snapshot().active, 1024);
        drop(leases.pop());
        let replacement = control
            .observe(TransferKind::Download, "replacement.bin", Some(1), 1)
            .unwrap();
        assert_eq!(control.snapshot().active, 1024);
        drop(replacement);
        drop(leases);
        assert_eq!(control.snapshot().active, 0);
    }

    #[test]
    fn terminal_history_keeps_recent_receipts_without_evicting_active_work() {
        let control = TransferControl::default();
        let active = control
            .observe(TransferKind::Upload, "still-active.bin", None, 1)
            .unwrap();
        for _ in 0..500 {
            let terminal = control
                .observe(TransferKind::Download, "served.bin", Some(1), 1)
                .unwrap();
            terminal.add_bytes(1);
            terminal.finish(TransferState::Served);
        }
        let snapshot = control.snapshot();
        assert_eq!(snapshot.active, 1);
        assert_eq!(snapshot.records.len(), 201);
        assert_eq!(snapshot.records[0].id, 1);
        assert_eq!(snapshot.records[0].path, "still-active.bin");
        assert_eq!(snapshot.records[0].state, TransferState::Active);
        assert_eq!(snapshot.records[1].id, 302);
        assert_eq!(snapshot.records.last().unwrap().id, 501);
        drop(active);
        assert_eq!(control.snapshot().active, 0);
    }
}
