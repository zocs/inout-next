use crate::{HostController, HostStatus, ShareEndpoint, StartFailure, StartOptions, StartReason};
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

/// The setting a problem points at, so each renderer can localize its advice.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ProblemReason {
    Folder,
    Resources,
    Key,
    PortInUse,
    Port,
    Other,
}

impl From<StartReason> for ProblemReason {
    fn from(reason: StartReason) -> Self {
        match reason {
            StartReason::NoFolder | StartReason::Folder => Self::Folder,
            StartReason::Resources => Self::Resources,
            StartReason::Key => Self::Key,
            StartReason::PortInUse => Self::PortInUse,
            StartReason::Port => Self::Port,
            StartReason::AlreadyRunning | StartReason::Other => Self::Other,
        }
    }
}

#[derive(Clone, PartialEq, Eq, Serialize)]
pub struct SessionProblem {
    pub action: ProblemAction,
    pub reason: ProblemReason,
    pub detail: String,
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionSnapshot {
    pub host: HostStatus,
    /// Labelled share addresses in recommended order; empty while stopped.
    pub endpoints: Vec<ShareEndpoint>,
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
                endpoints: Vec::new(),
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
        self.state.endpoints = self.host.endpoints().await;
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
        let folder = |detail: String| SessionProblem {
            action: ProblemAction::SelectRoot,
            reason: ProblemReason::Folder,
            detail,
        };
        let problem = match command {
            SessionCommand::SelectRoot(root) => {
                Some(self.host.select_root(root).await.err().map(folder))
            }
            SessionCommand::Start(options) => Some(self.start(options).await.err()),
            SessionCommand::StartWithRoot { root, options } => {
                match self.host.select_root(root).await {
                    Ok(_) => Some(self.start(options).await.err()),
                    Err(detail) => Some(Some(folder(detail))),
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
                    Some(result.err().map(stop_problem))
                } else {
                    None
                }
            }
        };
        if let Some(problem) = problem {
            self.state.problem = problem;
        }
        self.observe().await;
    }

    async fn start(&mut self, options: StartOptions) -> Result<HostStatus, SessionProblem> {
        let result = self.host.start_checked(options).await;
        if result.is_ok() {
            self.transfers = self.host.transfer_control().await;
        }
        result.map_err(|StartFailure { reason, message }| SessionProblem {
            action: ProblemAction::Start,
            reason: reason.into(),
            detail: message.to_owned(),
        })
    }

    pub async fn shutdown(&mut self) {
        self.state.problem = self.host.stop().await.err().map(stop_problem);
        self.state.confirmation = None;
        self.state.exit_ready = true;
        self.observe().await;
    }
}

fn stop_problem(detail: String) -> SessionProblem {
    SessionProblem {
        action: ProblemAction::Stop,
        reason: ProblemReason::Other,
        detail,
    }
}
