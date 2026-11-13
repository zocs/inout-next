fn main() {
    let manifest = tauri_build::AppManifest::new().commands(&[
        "host_status",
        "choose_folder",
        "start_service",
        "stop_service",
        "open_manager",
        "create_access_key",
    ]);
    tauri_build::try_build(tauri_build::Attributes::new().app_manifest(manifest))
        .expect("Native application build configuration failed");
}
