mod host;
#[cfg(any(target_os = "android", test))]
mod mobile;
mod native;

pub use host::{
    resolve_web_dir, ActiveSettings, HostCapabilities, HostController, HostStatus, StartOptions,
    StorageKind,
};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    native::run();
}
