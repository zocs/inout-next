use crate::{HostController, HostStatus, StartOptions};
use serde::Serialize;
use share_http::{TransferControl, TransferSnapshot};
use std::path::PathBuf;
use tokio::sync::watch;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Interrupt {
    Stop,
    Close,
}

pub enum SessionCommand {
    SelectRoot(PathBuf),
    Start(StartOptions),
    StartWithRoot {
        root: PathBuf,
        options: StartOptions,
    },
    RequestStop,
    RequestClose,
    CancelInterrupt,
    ConfirmInterrupt,
}

#[derive(Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ProblemAction {
    SelectRoot,
    Start,
    Stop,
}

#[derive(Clone, PartialEq, Eq, Serialize)]
pub struct SessionProblem {
    pub action: ProblemAction,
    pub detail: String,
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionSnapshot {
    pub host: HostStatus,
    pub transfers: TransferSnapshot,
    pub confirmation: Option<Interrupt>,
    pub exit_ready: bool,
    pub problem: Option<SessionProblem>,
}

/// Renderer-independent application ownership and interruption confirmation.
/// Keep this owner outside view construction so replacing a surface cannot
/// drop an admitted transfer. Receipts cover the current service generation.
pub struct HostSession {
    host: HostController,
    state: SessionSnapshot,
    transfers: Option<TransferControl>,
}

impl HostSession {
    pub fn new(host: HostController) -> Self {
        let capabilities = host.capabilities();
        Self {
            host,
            transfers: None,
            state: SessionSnapshot {
                host: HostStatus {
                    root: None,
                    running: false,
                    manager_url: None,
                    addresses: Vec::new(),
                    active_settings: None,
                    capabilities,
                },
                transfers: TransferSnapshot::default(),
                confirmation: None,
                exit_ready: false,
                problem: None,
            },
        }
    }

    pub fn snapshot(&self) -> SessionSnapshot {
        self.state.clone()
    }

    pub fn transfer_subscription(&self) -> Option<watch::Receiver<()>> {
        self.transfers.as_ref().map(TransferControl::subscribe)
    }

    pub async fn observe(&mut self) {
        self.state.host = self.host.status().await;
        if self.state.host.running && self.transfers.is_none() {
            self.transfers = self.host.transfer_control().await;
        }
        self.state.transfers = self
            .transfers
            .as_ref()
            .map(TransferControl::snapshot)
            .unwrap_or_default();
    }

    pub async fn apply(&mut self, command: SessionCommand) {
        if self.state.exit_ready {
            return;
        }
        // An existing running controller may be moved into this owner. Never
        // decide interruption from the constructor's provisional empty state.
        self.observe().await;
        if self.state.confirmation.is_some()
            && matches!(
                command,
                SessionCommand::SelectRoot(_)
                    | SessionCommand::Start(_)
                    | SessionCommand::StartWithRoot { .. }
            )
        {
            return;
        }
        let result = match command {
            SessionCommand::SelectRoot(root) => {
                Some((ProblemAction::SelectRoot, self.host.select_root(root).await))
            }
            SessionCommand::Start(options) => {
                Some((ProblemAction::Start, self.start(options).await))
            }
            SessionCommand::StartWithRoot { root, options } => {
                match self.host.select_root(root).await {
                    Ok(_) => Some((ProblemAction::Start, self.start(options).await)),
                    Err(detail) => Some((ProblemAction::SelectRoot, Err(detail))),
                }
            }
            SessionCommand::RequestStop => {
                if self.state.host.running && self.state.confirmation != Some(Interrupt::Close) {
                    self.state.confirmation = Some(Interrupt::Stop);
                    self.state.problem = None;
                }
                None
            }
            SessionCommand::RequestClose => {
                self.state.problem = None;
                if self.state.host.running {
                    self.state.confirmation = Some(Interrupt::Close);
                } else {
                    self.state.exit_ready = true;
                }
                None
            }
            SessionCommand::CancelInterrupt => {
                self.state.confirmation = None;
                None
            }
            SessionCommand::ConfirmInterrupt => {
                if let Some(interrupt) = self.state.confirmation.take() {
                    let result = self.host.stop().await;
                    self.state.exit_ready = result.is_ok() && interrupt == Interrupt::Close;
                    Some((ProblemAction::Stop, result))
                } else {
                    None
                }
            }
        };
        if let Some((action, result)) = result {
            self.state.problem = result.err().map(|detail| SessionProblem { action, detail });
        }
        self.observe().await;
    }

    async fn start(&mut self, options: StartOptions) -> Result<HostStatus, String> {
        let result = self.host.start(options).await;
        if result.is_ok() {
            self.transfers = self.host.transfer_control().await;
        }
        result
    }

    pub async fn shutdown(&mut self) {
        self.state.problem = self.host.stop().await.err().map(|detail| SessionProblem {
            action: ProblemAction::Stop,
            detail,
        });
        self.state.confirmation = None;
        self.state.exit_ready = true;
        self.observe().await;
    }
}
