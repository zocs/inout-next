mod host;
mod native;

pub use host::{resolve_web_dir, HostController, HostStatus, StartOptions};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    native::run();
}
