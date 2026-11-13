use crate::{resolve_web_dir, HostController, HostStatus, StartOptions};
use rand::{rngs::OsRng, TryRngCore};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use tauri::{AppHandle, Manager, State, WebviewUrl, WebviewWindow, WebviewWindowBuilder};
#[cfg(desktop)]
use tauri_plugin_dialog::DialogExt;
use tokio::sync::Mutex;

struct NativeState {
    host: HostController,
    transition: Mutex<()>,
    exiting: AtomicBool,
    shutdown_completed: AtomicBool,
}

impl NativeState {
    fn require_active(&self) -> Result<(), String> {
        if self.exiting.load(Ordering::Acquire) {
            Err("The application is shutting down".into())
        } else {
            Ok(())
        }
    }

    #[cfg(any(desktop, test))]
    async fn pick_folder<F, Fut>(&self, picker: F) -> Result<HostStatus, String>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<Option<std::path::PathBuf>, String>>,
    {
        {
            let _transition = self.transition.lock().await;
            self.require_active()?;
            if self.host.status().await.running {
                return Err("Stop the service before choosing another folder".into());
            }
        }
        // User-driven dialog completion must never hold the shutdown lock.
        let selection = picker().await?;
        let _transition = self.transition.lock().await;
        self.require_active()?;
        match selection {
            Some(path) => self.host.select_root(path).await,
            None => Ok(self.host.status().await),
        }
    }

    async fn stop_for_shutdown(&self) -> Result<HostStatus, String> {
        let _transition = self.transition.lock().await;
        self.host.stop().await
    }
}

fn require_host_label(label: &str) -> Result<(), String> {
    if label == "main" {
        Ok(())
    } else {
        Err("Native controls are restricted to the host window".into())
    }
}

fn navigation_allowed(origin: &tauri::Url, destination: &tauri::Url) -> bool {
    destination.origin() == origin.origin()
        && destination.username().is_empty()
        && destination.password().is_none()
}

fn access_key() -> Result<String, String> {
    let mut bytes = [0_u8; 32];
    OsRng
        .try_fill_bytes(&mut bytes)
        .map_err(|_| "An access key could not be generated")?;
    Ok(bytes.iter().map(|byte| format!("{byte:02x}")).collect())
}

#[tauri::command]
async fn host_status(
    window: WebviewWindow,
    state: State<'_, Arc<NativeState>>,
) -> Result<HostStatus, String> {
    require_host_label(window.label())?;
    Ok(state.host.status().await)
}

#[tauri::command]
async fn choose_folder(
    window: WebviewWindow,
    app: AppHandle,
    state: State<'_, Arc<NativeState>>,
) -> Result<HostStatus, String> {
    require_host_label(window.label())?;
    #[cfg(desktop)]
    {
        state
            .pick_folder(|| async {
                let (sender, receiver) = tokio::sync::oneshot::channel();
                app.dialog()
                    .file()
                    .set_parent(&window)
                    .pick_folder(move |folder| {
                        let _ = sender.send(folder);
                    });
                receiver
                    .await
                    .map_err(|_| "The folder dialog could not complete".to_owned())?
                    .map(|folder| {
                        folder
                            .into_path()
                            .map_err(|_| "The selected folder is unavailable".to_owned())
                    })
                    .transpose()
            })
            .await
    }
    #[cfg(not(desktop))]
    {
        let _ = app;
        Err("Native folder sharing requires desktop storage support".into())
    }
}

#[tauri::command]
async fn start_service(
    window: WebviewWindow,
    state: State<'_, Arc<NativeState>>,
    options: StartOptions,
) -> Result<HostStatus, String> {
    require_host_label(window.label())?;
    let _transition = state.transition.lock().await;
    state.require_active()?;
    state.host.start(options).await
}

fn close_manager(app: &AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("files") {
        window
            .destroy()
            .map_err(|_| "The manager window could not close")?;
    }
    Ok(())
}

#[tauri::command]
async fn stop_service(
    window: WebviewWindow,
    app: AppHandle,
    state: State<'_, Arc<NativeState>>,
) -> Result<HostStatus, String> {
    require_host_label(window.label())?;
    let _transition = state.transition.lock().await;
    state.require_active()?;
    let result = state.host.stop().await;
    close_manager(&app)?;
    result
}

#[tauri::command]
async fn open_manager(
    window: WebviewWindow,
    app: AppHandle,
    state: State<'_, Arc<NativeState>>,
) -> Result<(), String> {
    require_host_label(window.label())?;
    let _transition = state.transition.lock().await;
    state.require_active()?;
    let status = state.host.status().await;
    let url: tauri::Url = status
        .manager_url
        .ok_or("Start the service before opening its manager")?
        .parse()
        .map_err(|_| "The service address is unavailable")?;
    if let Some(manager) = app.get_webview_window("files") {
        if manager
            .url()
            .is_ok_and(|current| navigation_allowed(&url, &current))
        {
            manager
                .show()
                .map_err(|_| "The manager window could not open")?;
            return manager
                .set_focus()
                .map_err(|_| "The manager window could not be focused".into());
        }
        manager
            .destroy()
            .map_err(|_| "The previous manager window could not close")?;
    }
    let origin = url.clone();
    let title = share_domain::Product::default().name;
    WebviewWindowBuilder::new(&app, "files", WebviewUrl::External(url))
        .title(title)
        .inner_size(1060.0, 760.0)
        .min_inner_size(360.0, 560.0)
        .on_navigation(move |destination| navigation_allowed(&origin, destination))
        .on_new_window(|_, _| tauri::webview::NewWindowResponse::Deny)
        .build()
        .map_err(|_| "The manager window could not open")?;
    Ok(())
}

#[tauri::command]
fn create_access_key(
    window: WebviewWindow,
    state: State<'_, Arc<NativeState>>,
) -> Result<String, String> {
    require_host_label(window.label())?;
    state.require_active()?;
    access_key()
}

fn begin_shutdown(app: &AppHandle) {
    let Some(state) = app.try_state::<Arc<NativeState>>() else {
        return;
    };
    if state.exiting.swap(true, Ordering::AcqRel) {
        return;
    }
    let state = Arc::clone(&state);
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        // stop awaits the shared core's bounded request drain before exit.
        let _ = state.stop_for_shutdown().await;
        let _ = close_manager(&app);
        state.shutdown_completed.store(true, Ordering::Release);
        app.exit(0);
    });
}

pub(crate) fn run() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let resources = app
                .path()
                .resource_dir()
                .map_err(|_| std::io::Error::other("Application resources are unavailable"))?;
            #[cfg(debug_assertions)]
            let fallback =
                Some(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../web/out"));
            #[cfg(not(debug_assertions))]
            let fallback: Option<std::path::PathBuf> = None;
            let web_dir =
                resolve_web_dir(&resources, fallback.as_deref()).map_err(std::io::Error::other)?;
            app.manage(Arc::new(NativeState {
                host: HostController::new(web_dir),
                transition: Mutex::new(()),
                exiting: AtomicBool::new(false),
                shutdown_completed: AtomicBool::new(false),
            }));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            host_status,
            choose_folder,
            start_service,
            stop_service,
            open_manager,
            create_access_key
        ])
        .build(tauri::generate_context!())
        .expect("Native application initialization failed");

    app.run(|app, event| match event {
        tauri::RunEvent::WindowEvent {
            label,
            event: tauri::WindowEvent::CloseRequested { api, .. },
            ..
        } if label == "main" => {
            api.prevent_close();
            begin_shutdown(app);
        }
        tauri::RunEvent::WindowEvent {
            label,
            event: tauri::WindowEvent::Destroyed,
            ..
        } if label == "main" => begin_shutdown(app),
        tauri::RunEvent::WindowEvent {
            label,
            event: tauri::WindowEvent::CloseRequested { api, .. },
            ..
        } if label == "files" => {
            api.prevent_close();
            let state = Arc::clone(&app.state::<Arc<NativeState>>());
            let app = app.clone();
            tauri::async_runtime::spawn(async move {
                let _transition = state.transition.lock().await;
                let _ = state.host.stop().await;
                let _ = close_manager(&app);
            });
        }
        tauri::RunEvent::ExitRequested { api, .. }
            if app
                .try_state::<Arc<NativeState>>()
                .is_some_and(|state| !state.shutdown_completed.load(Ordering::Acquire)) =>
        {
            api.prevent_exit();
            begin_shutdown(app);
        }
        _ => {}
    });
}

#[cfg(test)]
mod picker_tests;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_privileges_are_denied_to_manager_and_other_webviews() {
        assert!(require_host_label("main").is_ok());
        for label in ["files", "other", "", "main-child"] {
            assert!(require_host_label(label).is_err());
        }
    }

    #[test]
    fn manager_navigation_stays_on_its_exact_service_origin() {
        let origin: tauri::Url = "http://127.0.0.1:43210".parse().unwrap();
        assert!(navigation_allowed(
            &origin,
            &"http://127.0.0.1:43210/folder?path=docs#list"
                .parse()
                .unwrap()
        ));
        for value in [
            "http://127.0.0.1:43211",
            "http://localhost:43210",
            "https://127.0.0.1:43210",
            "https://example.org",
            "tauri://localhost",
            "javascript:alert(1)",
            "http://user:pass@127.0.0.1:43210",
        ] {
            assert!(
                !navigation_allowed(&origin, &value.parse().unwrap()),
                "{value}"
            );
        }
    }

    #[test]
    fn generated_keys_have_at_least_32_random_bytes_in_printable_form() {
        let first = access_key().unwrap();
        let second = access_key().unwrap();
        assert_eq!(first.len(), 64);
        assert!(first.bytes().all(|byte| byte.is_ascii_hexdigit()));
        assert_ne!(first, second);
    }
}
