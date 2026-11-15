use crate::{resolve_web_dir, HostController};
use std::path::Path;

/// APK resources are prepared by MainActivity before Tauri starts this host.
pub(crate) fn initialize_private_host(data_dir: &Path) -> Result<HostController, String> {
    let resources = data_dir.join("files/runtime-assets");
    let web = resolve_web_dir(&resources, None)?;
    let shared = data_dir.join("files/shared");
    std::fs::create_dir_all(&shared).map_err(|_| "App storage could not be prepared")?;
    HostController::new_app_private(web, shared)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn android_layout_requires_packaged_resources_and_shares_only_the_private_sibling() {
        let temporary = tempfile::tempdir().unwrap();
        assert!(initialize_private_host(temporary.path()).is_err());
        assert!(!temporary.path().join("files/shared").exists());
        let web = temporary.path().join("files/runtime-assets/web");
        std::fs::create_dir_all(web.join("_next")).unwrap();
        std::fs::write(
            web.join("index.html"),
            "<!doctype html><title>APK resources</title>",
        )
        .unwrap();
        let host = initialize_private_host(temporary.path()).unwrap();
        let shared = temporary
            .path()
            .join("files/shared")
            .canonicalize()
            .unwrap();
        assert_eq!(host.status().await.root.as_deref(), shared.to_str());
        assert!(!host.capabilities().choose_folder);
        assert!(!host.capabilities().open_manager);
        assert!(host.capabilities().foreground_only);
        assert!(!web.starts_with(&shared) && !shared.starts_with(&web));
    }
}
