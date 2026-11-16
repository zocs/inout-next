use share_host::{HostController, HostSession, SessionCommand, SessionSnapshot};
use std::{
    future::Future,
    io,
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};
use tokio::{
    runtime,
    sync::{mpsc, watch},
    time::Instant,
};
use tokio_util::sync::CancellationToken;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommandError {
    Full,
    Closed,
}

#[derive(Clone)]
pub struct BridgeHandle {
    commands: Arc<Mutex<CommandQueue>>,
    shutdown: CancellationToken,
    runtime: runtime::Handle,
}

struct CommandQueue {
    sender: mpsc::Sender<CommandRequest>,
    last_command: u64,
}

struct CommandRequest {
    id: u64,
    command: SessionCommand,
}

#[derive(Clone)]
pub struct BridgeUpdate {
    pub snapshot: SessionSnapshot,
    /// All accepted FIFO commands up to this ID have completed. Passive
    /// snapshots carry the same watermark, never a one-off completion flag.
    pub completed_command: u64,
}

impl BridgeHandle {
    pub fn send(&self, command: SessionCommand) -> Result<u64, CommandError> {
        if self.shutdown.is_cancelled() {
            return Err(CommandError::Closed);
        }
        // Allocate and enqueue together so cloned handles cannot reorder IDs.
        // This short lock protects only bounded try_send, never service work.
        let mut queue = self
            .commands
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let id = queue
            .last_command
            .checked_add(1)
            .ok_or(CommandError::Closed)?;
        queue
            .sender
            .try_send(CommandRequest { id, command })
            .map_err(|error| match error {
                mpsc::error::TrySendError::Full(_) => CommandError::Full,
                mpsc::error::TrySendError::Closed(_) => CommandError::Closed,
            })?;
        queue.last_command = id;
        Ok(id)
    }

    pub fn spawn_background(
        &self,
        work: impl Future<Output = ()> + Send + 'static,
    ) -> Result<(), CommandError> {
        if self.shutdown.is_cancelled() {
            return Err(CommandError::Closed);
        }
        let shutdown = self.shutdown.clone();
        self.runtime.spawn(async move {
            tokio::select! {
                biased;
                _ = shutdown.cancelled() => {},
                _ = work => {},
            }
        });
        Ok(())
    }
}

pub struct Bridge {
    handle: BridgeHandle,
    worker: Option<thread::JoinHandle<()>>,
}

impl Bridge {
    pub fn spawn(
        host: HostController,
        sink: impl Fn(BridgeUpdate) + Send + 'static,
    ) -> io::Result<Self> {
        let runtime = runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()?;
        let (commands, mut receiver) = mpsc::channel(16);
        let shutdown = CancellationToken::new();
        let handle = BridgeHandle {
            commands: Arc::new(Mutex::new(CommandQueue {
                sender: commands,
                last_command: 0,
            })),
            shutdown: shutdown.clone(),
            runtime: runtime.handle().clone(),
        };
        let worker = thread::Builder::new().name("inout-host".into()).spawn(move || {
            runtime.block_on(async move {
                let mut session = HostSession::new(host);
                session.observe().await;
                let mut subscription = session.transfer_subscription();
                // Bootstrap adoption with the same subscribe-then-snapshot
                // ordering as Start, including an initially idle live host.
                session.observe().await;
                let mut last = session.snapshot();
                let mut completed_command = 0;
                sink(BridgeUpdate { snapshot: last.clone(), completed_command });
                let mut dirty = false;
                let mut next_sample = Instant::now();
                loop {
                    if last.exit_ready {
                        break;
                    }
                    let running = last.host.running;
                    let sampling = running && (dirty || last.transfers.active > 0);
                    let waiting = running && !dirty && subscription.is_some();
                    tokio::select! {
                        biased;
                        _ = shutdown.cancelled() => break,
                        request = receiver.recv() => {
                            let Some(request) = request else { break; };
                            session.apply(request.command).await;
                            let state = session.snapshot();
                            if state.host.running && (subscription.is_none() || state.host.manager_url != last.host.manager_url) {
                                subscription = session.transfer_subscription();
                                // Subscribe first, then snapshot: admission racing Start's
                                // original status cannot be missed by an idle bridge.
                                session.observe().await;
                            } else if !state.host.running {
                                subscription = None;
                            }
                            dirty = false;
                            // Every command completes, even if a retry produces
                            // the same error. Passive samples still publish only
                            // changes; otherwise a view could stay busy forever.
                            last = session.snapshot();
                            completed_command = request.id;
                            sink(BridgeUpdate { snapshot: last.clone(), completed_command });
                            next_sample = Instant::now() + Duration::from_millis(250);
                        },
                        changed = wait_for_transfer_change(&mut subscription), if waiting => {
                            dirty = true;
                            if !changed {
                                subscription = None;
                            }
                        },
                        _ = tokio::time::sleep_until(next_sample), if sampling => {
                            if let Some(changes) = &mut subscription {
                                changes.borrow_and_update();
                            }
                            session.observe().await;
                            dirty = false;
                            publish(&session, &sink, &mut last, completed_command);
                            next_sample = Instant::now() + Duration::from_millis(250);
                        },
                    }
                }
                session.shutdown().await;
                publish(&session, &sink, &mut last, completed_command);
                shutdown.cancel();
            });
        })?;
        Ok(Self {
            handle,
            worker: Some(worker),
        })
    }

    pub fn handle(&self) -> BridgeHandle {
        self.handle.clone()
    }
}

fn publish(
    session: &HostSession,
    sink: &impl Fn(BridgeUpdate),
    last: &mut SessionSnapshot,
    completed_command: u64,
) {
    let snapshot = session.snapshot();
    if snapshot != *last {
        *last = snapshot.clone();
        sink(BridgeUpdate {
            snapshot,
            completed_command,
        });
    }
}

async fn wait_for_transfer_change(subscription: &mut Option<watch::Receiver<()>>) -> bool {
    match subscription {
        Some(changes) => changes.changed().await.is_ok(),
        None => std::future::pending().await,
    }
}

impl Drop for Bridge {
    fn drop(&mut self) {
        self.handle.shutdown.cancel();
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}
