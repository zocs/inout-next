use crate::{AppWindow, Labels, TransferRow};
use share_host::{Interrupt, ProblemAction, SessionSnapshot};
use share_http::{TransferKind, TransferRecord, TransferState};
use share_native_slint::{
    bridge::BridgeUpdate,
    presentation::{text, Locale, TextKey},
    updates::CommandCompletion,
};
use slint::{Model, ModelRc, VecModel};
use std::{path::PathBuf, rc::Rc};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Pending {
    Start,
    RequestStop,
    RequestClose,
    Confirm,
    Cancel,
}

pub struct ViewState {
    pub locale: Locale,
    pub snapshot: Option<SessionSnapshot>,
    pub native_folder: Option<PathBuf>,
    pub picker_ticket: u64,
    pub pending: CommandCompletion<Pending>,
    pub local_problem: Option<TextKey>,
    pub notice: Option<TextKey>,
    pub resources_ready: bool,
    rows: Rc<VecModel<TransferRow>>,
    row_ids: Vec<u64>,
    records: Vec<TransferRecord>,
    addresses: Vec<String>,
}

impl ViewState {
    pub fn new(locale: Locale, resources_ready: bool, ui: &AppWindow) -> Self {
        let rows = Rc::new(VecModel::default());
        ui.set_transfers(ModelRc::from(rows.clone()));
        Self {
            locale,
            snapshot: None,
            native_folder: None,
            picker_ticket: 0,
            pending: CommandCompletion::default(),
            local_problem: None,
            notice: None,
            resources_ready,
            rows,
            row_ids: Vec::new(),
            records: Vec::new(),
            addresses: Vec::new(),
        }
    }

    pub fn apply(&mut self, ui: &AppWindow, update: BridgeUpdate) {
        let snapshot = update.snapshot;
        let previous_confirmation = self.snapshot.as_ref().and_then(|state| state.confirmation);
        let pending = self.pending.finish_through(update.completed_command);
        ui.set_running(snapshot.host.running);
        ui.set_busy(self.pending.is_pending());
        ui.set_confirming(snapshot.confirmation.is_some());
        ui.set_closing(snapshot.confirmation == Some(Interrupt::Close));
        if snapshot.confirmation.is_some() && previous_confirmation.is_none() {
            ui.invoke_show_confirmation();
        } else if snapshot.confirmation.is_none() && previous_confirmation.is_some() {
            ui.invoke_hide_confirmation();
        }
        self.snapshot = Some(snapshot);
        self.render(ui, false);
        // Only the completion of the user's action owns a focus handoff.
        // Passive progress and locale/theme changes preserve the current focus.
        match pending {
            Some(Pending::Start) if ui.get_running() => ui.invoke_focus_stop(),
            Some(Pending::Start | Pending::Confirm) if !ui.get_running() => ui.invoke_focus_start(),
            Some(Pending::Cancel) if ui.get_running() => ui.invoke_focus_stop(),
            _ => {}
        }
    }

    pub fn render(&mut self, ui: &AppWindow, force_locale: bool) {
        if force_locale {
            ui.set_labels(labels(self.locale));
        }
        ui.set_locale_index(self.locale.index());
        ui.set_resources_ready(self.resources_ready);
        let problem = if !self.resources_ready {
            Some(TextKey::ErrorAssets)
        } else {
            self.local_problem.or_else(|| {
                self.snapshot
                    .as_ref()?
                    .problem
                    .as_ref()
                    .map(|problem| match problem.action {
                        ProblemAction::SelectRoot => TextKey::ErrorSelect,
                        ProblemAction::Start => TextKey::ErrorStart,
                        ProblemAction::Stop => TextKey::ErrorStop,
                    })
            })
        };
        ui.set_problem(
            problem
                .map(|key| text(self.locale, key))
                .unwrap_or_default()
                .into(),
        );
        ui.set_notice(
            self.notice
                .map(|key| text(self.locale, key))
                .unwrap_or_default()
                .into(),
        );
        let Some(snapshot) = &self.snapshot else {
            return;
        };
        ui.set_shared_folder(snapshot.host.root.clone().unwrap_or_default().into());
        let effective = snapshot
            .host
            .active_settings
            .as_ref()
            .map(|settings| {
                let mut parts = vec![
                    text(
                        self.locale,
                        if settings.network {
                            TextKey::Network
                        } else {
                            TextKey::LocalOnly
                        },
                    )
                    .to_owned(),
                    format!("{} {}", text(self.locale, TextKey::Port), settings.port),
                    text(
                        self.locale,
                        if settings.auth_required {
                            TextKey::Protected
                        } else {
                            TextKey::Public
                        },
                    )
                    .to_owned(),
                ];
                if settings.uploads {
                    parts.push(text(self.locale, TextKey::Upload).to_owned());
                }
                if settings.manage {
                    parts.push(text(self.locale, TextKey::Manage).to_owned());
                }
                parts.join("\n")
            })
            .unwrap_or_default();
        ui.set_effective_settings(effective.into());
        let mut addresses = snapshot.host.addresses.clone();
        if let Some(local) = &snapshot.host.manager_url {
            if !addresses.contains(local) {
                addresses.push(local.clone());
            }
        }
        if addresses != self.addresses {
            let selected = ui.get_selected_address();
            let index = addresses
                .iter()
                .position(|address| address == selected.as_str())
                .unwrap_or(0);
            ui.set_addresses(ModelRc::new(VecModel::from(
                addresses
                    .iter()
                    .map(|value| value.clone().into())
                    .collect::<Vec<_>>(),
            )));
            ui.set_address_index(index as i32);
            ui.set_selected_address(addresses.get(index).cloned().unwrap_or_default().into());
            self.addresses = addresses;
        }
        // Keep the virtualized model alive across progress, theme and locale
        // changes. Only a changed receipt or language replaces that row's data.
        let records: Vec<_> = snapshot.transfers.records.iter().rev().cloned().collect();
        let ids: Vec<_> = records.iter().map(|record| record.id).collect();
        if ids != self.row_ids {
            self.rows.set_vec(
                records
                    .iter()
                    .map(|record| transfer_row(self.locale, record))
                    .collect::<Vec<_>>(),
            );
            self.row_ids = ids;
        } else {
            for (index, record) in records.iter().enumerate() {
                if force_locale || self.records.get(index) != Some(record) {
                    self.rows
                        .set_row_data(index, transfer_row(self.locale, record));
                }
            }
        }
        self.records = records;
    }

    pub fn select_address(&self, ui: &AppWindow, index: i32) {
        let index = usize::try_from(index)
            .unwrap_or(0)
            .min(self.addresses.len().saturating_sub(1));
        ui.set_address_index(index as i32);
        ui.set_selected_address(
            self.addresses
                .get(index)
                .cloned()
                .unwrap_or_default()
                .into(),
        );
    }

    pub fn invalidate_picker(&mut self) {
        self.picker_ticket = self.picker_ticket.wrapping_add(1);
        self.native_folder = None;
    }

    pub fn error(&mut self, ui: &AppWindow, key: TextKey) {
        self.local_problem = Some(key);
        self.notice = None;
        ui.set_busy(self.pending.is_pending());
        self.render(ui, false);
    }
}

fn transfer_row(locale: Locale, record: &TransferRecord) -> TransferRow {
    let kind = match record.kind {
        TransferKind::Upload => TextKey::UploadKind,
        TransferKind::Download => TextKey::DownloadKind,
        TransferKind::Copy => TextKey::CopyKind,
    };
    let state = match record.state {
        TransferState::Active => TextKey::Active,
        TransferState::Published => TextKey::Published,
        TransferState::Served => TextKey::Served,
        TransferState::Interrupted => TextKey::Interrupted,
        TransferState::Failed => TextKey::Failed,
    };
    let unit = text(locale, TextKey::ByteUnit);
    let accounting = if record.kind == TransferKind::Copy && record.total.is_none() {
        text(locale, TextKey::NoAccounting).to_owned()
    } else if let Some(total) = record.total {
        format!("{} / {} {unit}", record.bytes, total)
    } else {
        format!("{} {unit}", record.bytes)
    };
    TransferRow {
        path: record.path.clone().into(),
        kind: text(locale, kind).into(),
        state: text(locale, state).into(),
        accounting: accounting.into(),
        active: record.state == TransferState::Active,
        failed: matches!(
            record.state,
            TransferState::Interrupted | TransferState::Failed
        ),
        progress: record
            .total
            .filter(|total| *total != 0)
            .map(|total| (record.bytes as f64 / total as f64).clamp(0.0, 1.0) as f32)
            .unwrap_or(0.0),
        known_total: record.total.is_some(),
    }
}

fn labels(locale: Locale) -> Labels {
    macro_rules! fields {
        ($($field:ident: $key:ident),+ $(,)?) => {
            Labels { $($field: text(locale, TextKey::$key).into()),+ }
        };
    }
    fields! {
        share_folder: ShareFolder, intro: Intro, folder: Folder, choose_folder: ChooseFolder,
        folder_hint: FolderHint, start: Start, stop: Stop, open_browser: OpenBrowser,
        copy_address: CopyAddress, address: Address, no_address: NoAddress, key: Key,
        key_hint: KeyHint, upload: Upload, manage: Manage, network: Network,
        port: Port, port_hint: PortHint, running: Running, stopped: Stopped,
        busy: Busy, confirm_stop: ConfirmStop, confirm_close: ConfirmClose, confirm_message: ConfirmMessage,
        cancel: Cancel, confirm: Confirm, transfers: Transfers, transfer_hint: TransferHint,
        no_transfers: NoTransfers, about: About, close_about: CloseAbout, preview: Preview,
        language: Language, theme: Theme, evergreen: Evergreen, tangerine: Tangerine,
        ocean: Ocean, berry: Berry, reduce_motion: ReduceMotion, opening: Opening,
    }
}
