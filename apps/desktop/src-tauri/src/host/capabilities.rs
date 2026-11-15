use serde::Serialize;

#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum StorageKind {
    Folder,
    AppPrivate,
}

#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HostCapabilities {
    pub storage_kind: StorageKind,
    pub choose_folder: bool,
    pub open_manager: bool,
    pub foreground_only: bool,
}

impl Default for HostCapabilities {
    fn default() -> Self {
        Self {
            storage_kind: StorageKind::Folder,
            choose_folder: true,
            open_manager: true,
            foreground_only: false,
        }
    }
}

impl HostCapabilities {
    pub(crate) fn app_private() -> Self {
        Self {
            storage_kind: StorageKind::AppPrivate,
            choose_folder: false,
            open_manager: false,
            foreground_only: true,
        }
    }
}
