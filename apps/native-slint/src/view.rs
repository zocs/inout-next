use crate::{AppWindow, CodeModule, EndpointRow, Labels, PermissionChip, Tr, TransferRow};
use share_host::{
    Interrupt, LinkKind, ProblemAction, ProblemReason, SessionSnapshot, ShareEndpoint,
};
use share_http::{TransferKind, TransferRecord, TransferState};
use share_native_slint::{
    bridge::BridgeUpdate,
    folders::CommonFolder,
    presentation::{format_bytes, text, Locale, TextKey},
    qr::Symbol,
    updates::CommandCompletion,
};
use slint::{ComponentHandle, Model, ModelRc, VecModel};
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
    /// False while the language still follows the system preference.
    pub locale_chosen: bool,
    pub snapshot: Option<SessionSnapshot>,
    pub native_folder: Option<PathBuf>,
    pub picker_ticket: u64,
    pub pending: CommandCompletion<Pending>,
    pub local_problem: Option<TextKey>,
    pub notice: Option<TextKey>,
    pub resources_ready: bool,
    pub common: Vec<(CommonFolder, PathBuf)>,
    rows: Rc<VecModel<TransferRow>>,
    row_ids: Vec<u64>,
    records: Vec<TransferRecord>,
    endpoints: Vec<ShareEndpoint>,
}

impl ViewState {
    pub fn new(
        locale: Locale,
        locale_chosen: bool,
        resources_ready: bool,
        common: Vec<(CommonFolder, PathBuf)>,
        ui: &AppWindow,
    ) -> Self {
        let rows = Rc::new(VecModel::default());
        ui.set_transfers(ModelRc::from(rows.clone()));
        Self {
            locale,
            locale_chosen,
            snapshot: None,
            native_folder: None,
            picker_ticket: 0,
            pending: CommandCompletion::default(),
            local_problem: None,
            notice: None,
            resources_ready,
            common,
            rows,
            row_ids: Vec::new(),
            records: Vec::new(),
            endpoints: Vec::new(),
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
        let locale = self.locale;
        if force_locale {
            ui.global::<Tr>().set_l(labels(locale));
            ui.set_quick_folders(ModelRc::new(VecModel::from(
                self.common
                    .iter()
                    .map(|(folder, _)| {
                        text(
                            locale,
                            match folder {
                                CommonFolder::Home => TextKey::Home,
                                CommonFolder::Desktop => TextKey::Desktop,
                                CommonFolder::Documents => TextKey::Documents,
                                CommonFolder::Downloads => TextKey::Downloads,
                            },
                        )
                        .into()
                    })
                    .collect::<Vec<slint::SharedString>>(),
            )));
        }
        ui.set_locale_index(locale.index());
        ui.set_resources_ready(self.resources_ready);
        let problem = if !self.resources_ready {
            Some(TextKey::ErrorAssets)
        } else {
            self.local_problem.or_else(|| {
                let problem = self.snapshot.as_ref()?.problem.as_ref()?;
                Some(match (problem.action, problem.reason) {
                    (ProblemAction::Stop, _) => TextKey::ErrorStop,
                    (ProblemAction::SelectRoot, _) | (_, ProblemReason::Folder) => {
                        TextKey::ErrorSelect
                    }
                    (_, ProblemReason::Resources) => TextKey::ErrorResources,
                    (_, ProblemReason::Key) => TextKey::ErrorKey,
                    (_, ProblemReason::PortInUse) => TextKey::ErrorPortInUse,
                    (_, ProblemReason::Port) => TextKey::ErrorPortUnavailable,
                    (_, ProblemReason::Other) => TextKey::ErrorStart,
                })
            })
        };
        ui.set_problem(
            problem
                .map(|key| text(locale, key))
                .unwrap_or_default()
                .into(),
        );
        ui.set_notice(
            self.notice
                .map(|key| text(locale, key))
                .unwrap_or_default()
                .into(),
        );
        let Some(snapshot) = &self.snapshot else {
            return;
        };
        ui.set_shared_folder(snapshot.host.root.clone().unwrap_or_default().into());
        let settings = snapshot.host.active_settings.as_ref();
        let uploads = settings.is_some_and(|settings| settings.uploads);
        let manage = settings.is_some_and(|settings| settings.manage);
        let keyed = settings.is_some_and(|settings| settings.auth_required);
        let chip = |key, on: bool| PermissionChip {
            text: text(locale, key).into(),
            state: text(
                locale,
                if on {
                    TextKey::Allowed
                } else {
                    TextKey::NotAllowed
                },
            )
            .into(),
            on,
        };
        // Published deletion additionally requires configured-key authentication.
        ui.set_permissions(ModelRc::new(VecModel::from(vec![
            chip(TextKey::PermBrowse, settings.is_some()),
            chip(TextKey::PermUpload, uploads),
            chip(TextKey::PermManage, manage),
            chip(TextKey::PermDelete, manage && keyed),
        ])));
        ui.set_access_summary(
            settings
                .map(|settings| {
                    format!(
                        "{} · {} {}",
                        text(
                            locale,
                            if settings.auth_required {
                                TextKey::Protected
                            } else {
                                TextKey::Public
                            }
                        ),
                        text(locale, TextKey::Port),
                        settings.port
                    )
                })
                .unwrap_or_default()
                .into(),
        );
        if force_locale || snapshot.endpoints != self.endpoints {
            let selected = ui.get_selected_address();
            let index = snapshot
                .endpoints
                .iter()
                .position(|endpoint| endpoint.url == selected.as_str())
                .unwrap_or(0);
            ui.set_endpoints(ModelRc::new(VecModel::from(
                snapshot
                    .endpoints
                    .iter()
                    .map(|endpoint| EndpointRow {
                        url: endpoint.url.clone().into(),
                        title: endpoint_title(locale, endpoint).into(),
                    })
                    .collect::<Vec<_>>(),
            )));
            self.endpoints = snapshot.endpoints.clone();
            self.select_endpoint(ui, index as i32);
        }
        // Keep the virtualized model alive across progress, theme and locale
        // changes. Only a changed receipt or language replaces that row's data.
        let records: Vec<_> = snapshot.transfers.records.iter().rev().cloned().collect();
        let ids: Vec<_> = records.iter().map(|record| record.id).collect();
        if ids != self.row_ids {
            self.rows.set_vec(
                records
                    .iter()
                    .map(|record| transfer_row(locale, record))
                    .collect::<Vec<_>>(),
            );
            self.row_ids = ids;
        } else {
            for (index, record) in records.iter().enumerate() {
                if force_locale || self.records.get(index) != Some(record) {
                    self.rows.set_row_data(index, transfer_row(locale, record));
                }
            }
        }
        self.records = records;
    }

    /// Show one address: its words, its connection and its scannable code.
    pub fn select_endpoint(&self, ui: &AppWindow, index: i32) {
        let index = usize::try_from(index)
            .unwrap_or(0)
            .min(self.endpoints.len().saturating_sub(1));
        let endpoint = self.endpoints.get(index);
        let url = endpoint.map(|endpoint| endpoint.url.as_str()).unwrap_or("");
        ui.set_endpoint_index(index as i32);
        ui.set_selected_address(url.into());
        ui.set_selected_title(
            endpoint
                .map(|endpoint| endpoint_title(self.locale, endpoint))
                .unwrap_or_default()
                .into(),
        );
        ui.set_local_only(endpoint.is_some_and(|endpoint| endpoint.kind == LinkKind::Loopback));
        let network = self
            .snapshot
            .as_ref()
            .and_then(|snapshot| snapshot.host.active_settings.as_ref())
            .is_some_and(|settings| settings.network);
        ui.set_lan_missing(network && self.endpoints.len() == 1);
        let symbol = endpoint.and_then(|endpoint| Symbol::encode(&endpoint.url));
        ui.set_code_side(symbol.as_ref().map(Symbol::size).unwrap_or(0));
        ui.set_code_modules(ModelRc::new(VecModel::from(
            symbol
                .map(|symbol| symbol.data_modules())
                .unwrap_or_default()
                .into_iter()
                .map(|module| CodeModule {
                    x: module.x,
                    y: module.y,
                })
                .collect::<Vec<_>>(),
        )));
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

/// The connection in words, with the platform interface name when known.
fn endpoint_title(locale: Locale, endpoint: &ShareEndpoint) -> String {
    let kind = text(
        locale,
        match endpoint.kind {
            LinkKind::Wifi => TextKey::LinkWifi,
            LinkKind::Wired => TextKey::LinkWired,
            LinkKind::Unknown => TextKey::LinkUnknown,
            LinkKind::Virtual => TextKey::LinkVirtual,
            LinkKind::Loopback => TextKey::LinkLoopback,
        },
    );
    if endpoint.interface.is_empty() {
        kind.to_owned()
    } else {
        format!("{kind} · {}", endpoint.interface)
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
    let accounting = if record.kind == TransferKind::Copy && record.total.is_none() {
        text(locale, TextKey::NoAccounting).to_owned()
    } else if let Some(total) = record.total.filter(|total| *total != record.bytes) {
        format!("{} / {}", format_bytes(record.bytes), format_bytes(total))
    } else {
        format_bytes(record.bytes)
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
        folder_placeholder: FolderPlaceholder, home: Home, desktop: Desktop, documents: Documents,
        downloads: Downloads, access: Access, readonly: ReadOnly, readonly_detail: ReadOnlyDetail,
        uploadable: Uploadable, uploadable_detail: UploadableDetail, manageable: Manageable,
        manageable_detail: ManageableDetail, key: Key, key_hint: KeyHint,
        key_placeholder: KeyPlaceholder, show_key: ShowKey, hide_key: HideKey,
        advanced: Advanced, network: Network, port: Port, port_hint: PortHint, start: Start,
        stop: Stop, busy: Busy, running: Running, stopped: Stopped,
        sharing_folder: SharingFolder, stop_hint: StopHint, scan: Scan, code: Code,
        address: Address, copy_address: CopyAddress, open_browser: OpenBrowser,
        switch_connection: SwitchConnection, local_only_note: LocalOnlyNote, no_lan: NoLan,
        current_permissions: CurrentPermissions, step_choose: StepChoose, step_start: StepStart,
        step_scan: StepScan, confirm_stop: ConfirmStop, confirm_close: ConfirmClose,
        confirm_message: ConfirmMessage, cancel: Cancel, confirm: Confirm, transfers: Transfers,
        transfer_hint: TransferHint, no_transfers: NoTransfers, about: About,
        close_about: CloseAbout, preview: Preview, language: Language, theme: Theme,
        evergreen: Evergreen, tangerine: Tangerine, ocean: Ocean, berry: Berry,
        appearance: Appearance, follow_system: FollowSystem, light: Light, dark: Dark,
        opening: Opening, cut: CutText, copy: CopyText, paste: PasteText, select_all: SelectAll,
    }
}
