use crate::host::{DiagnosticReport, DiagnosticStatus};
#[cfg(not(target_os = "android"))]
use crate::resolve_web_dir;
use crate::{HostController, HostStatus, StartOptions};
use rand::{rngs::OsRng, TryRngCore};
use std::sync::{
    atomic::{AtomicBool, AtomicUsize, Ordering},
    Arc,
};
use tauri::{AppHandle, Manager, State, WebviewWindow};
#[cfg(desktop)]
use tauri::{WebviewUrl, WebviewWindowBuilder};
#[cfg(desktop)]
use tauri_plugin_dialog::DialogExt;
use tokio::sync::Mutex;

struct NativeState {
    host: HostController,
    transition: Mutex<()>,
    exiting: AtomicBool,
    shutdown_completed: AtomicBool,
    foreground: AtomicBool,
    pending_suspensions: AtomicUsize,
}

impl NativeState {
    fn new(host: HostController) -> Self {
        Self {
            host,
            transition: Mutex::new(()),
            exiting: AtomicBool::new(false),
            shutdown_completed: AtomicBool::new(false),
            foreground: AtomicBool::new(true),
            pending_suspensions: AtomicUsize::new(0),
        }
    }

    fn require_active(&self) -> Result<(), String> {
        if self.exiting.load(Ordering::Acquire) {
            Err("The application is shutting down".into())
        } else {
            Ok(())
        }
    }

    fn require_can_share(&self) -> Result<(), String> {
        self.require_active()?;
        if self.host.capabilities().foreground_only
            && (!self.foreground.load(Ordering::Acquire)
                || self.pending_suspensions.load(Ordering::Acquire) != 0)
        {
            Err("Keep the app on screen and wait for sharing to stop before restarting".into())
        } else {
            Ok(())
        }
    }

    async fn start(&self, options: StartOptions) -> Result<HostStatus, String> {
        self.require_can_share()?;
        let _transition = self.transition.lock().await;
        self.require_can_share()?;
        let status = self.host.start(options).await?;
        // Suspension/exit can be signaled synchronously during the awaited
        // start. Never hand back a running listener after that signal.
        if let Err(error) = self.require_can_share() {
            self.host.stop().await?;
            return Err(error);
        }
        Ok(status)
    }

    #[cfg(any(target_os = "android", test))]
    fn mark_suspended(&self) {
        self.foreground.store(false, Ordering::Release);
        self.pending_suspensions.fetch_add(1, Ordering::AcqRel);
    }

    #[cfg(any(target_os = "android", test))]
    fn mark_resumed(&self) {
        self.foreground.store(true, Ordering::Release);
    }

    #[cfg(any(target_os = "android", test))]
    async fn stop_for_background(&self) -> Result<HostStatus, String> {
        let _transition = self.transition.lock().await;
        let result = self.host.stop().await;
        self.pending_suspensions
            .try_update(Ordering::AcqRel, Ordering::Acquire, |count| {
                count.checked_sub(1)
            })
            .map_err(|_| "The background sharing transition was not pending")?;
        result
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

#[cfg(any(desktop, test))]
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
        let _ = (app, state);
        Err("Choosing another folder is not supported on this device".into())
    }
}

#[tauri::command]
async fn start_service(
    window: WebviewWindow,
    state: State<'_, Arc<NativeState>>,
    options: StartOptions,
) -> Result<HostStatus, String> {
    require_host_label(window.label())?;
    state.start(options).await
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
    if !state.host.capabilities().open_manager {
        return Err("Open the displayed network address on another device".into());
    }
    #[cfg(desktop)]
    {
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
    #[cfg(not(desktop))]
    {
        let _ = app;
        Err("A local manager is not supported on this device".into())
    }
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

#[tauri::command]
async fn diagnostic_status(
    window: WebviewWindow,
    state: State<'_, Arc<NativeState>>,
) -> Result<DiagnosticStatus, String> {
    require_host_label(window.label())?;
    state.require_active()?;
    Ok(state.host.diagnostic_status().await)
}

#[tauri::command]
async fn set_diagnostic_mode(
    window: WebviewWindow,
    state: State<'_, Arc<NativeState>>,
    enabled: bool,
) -> Result<DiagnosticStatus, String> {
    require_host_label(window.label())?;
    state.require_active()?;
    state.host.set_diagnostic_mode(enabled).await
}

#[tauri::command]
async fn set_download_buffer(
    window: WebviewWindow,
    state: State<'_, Arc<NativeState>>,
    kib: usize,
) -> Result<DiagnosticStatus, String> {
    require_host_label(window.label())?;
    state.require_active()?;
    state.host.set_download_buffer(kib).await
}

#[tauri::command]
async fn reset_diagnostic_settings(
    window: WebviewWindow,
    state: State<'_, Arc<NativeState>>,
) -> Result<DiagnosticStatus, String> {
    require_host_label(window.label())?;
    state.require_active()?;
    state.host.reset_diagnostic_settings().await
}

#[tauri::command]
async fn run_diagnostic_test(
    window: WebviewWindow,
    state: State<'_, Arc<NativeState>>,
    path: String,
    connections: usize,
) -> Result<DiagnosticReport, String> {
    require_host_label(window.label())?;
    state.require_active()?;
    // The host acquires a permit under its state lock, then releases the lock
    // before connecting. Neither native transition nor lifecycle is held here.
    state.host.run_diagnostic_test(path, connections).await
}

#[tauri::command]
async fn cancel_diagnostic_test(
    window: WebviewWindow,
    state: State<'_, Arc<NativeState>>,
) -> Result<DiagnosticStatus, String> {
    require_host_label(window.label())?;
    state.require_active()?;
    state.host.cancel_diagnostic_test().await
}

fn framework_exit_is_deferred(android_owned_exit: bool, shutdown_completed: Option<bool>) -> bool {
    // Android's worker owns _exit after service drain and native cleanup.
    // Ordinary framework exit must never race that final process termination.
    android_owned_exit || shutdown_completed.is_some_and(|completed| !completed)
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
        #[cfg(target_os = "android")]
        {
            app.cleanup_before_exit();
            unsafe extern "C" {
                fn _exit(status: std::ffi::c_int) -> !;
            }
            // SAFETY: Android's _exit terminates this process after the owned
            // service/request cleanup. Unlike exit, it does not run C/C++
            // destructors while Android's HWUI render threads still use them.
            unsafe { _exit(0) }
        }
        #[cfg(not(target_os = "android"))]
        app.exit(0);
    });
}

pub(crate) fn run() {
    let builder = tauri::Builder::default();
    #[cfg(desktop)]
    let builder = builder.plugin(tauri_plugin_dialog::init());
    let app = builder
        .setup(|app| {
            #[cfg(target_os = "android")]
            let host = {
                let data_dir = app
                    .path()
                    .data_dir()
                    .map_err(|_| std::io::Error::other("Application storage is unavailable"))?;
                crate::mobile::initialize_private_host(&data_dir).map_err(std::io::Error::other)?
            };
            #[cfg(not(target_os = "android"))]
            let host = {
                let resources = app
                    .path()
                    .resource_dir()
                    .map_err(|_| std::io::Error::other("Application resources are unavailable"))?;
                #[cfg(debug_assertions)]
                let fallback =
                    Some(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../web/out"));
                #[cfg(not(debug_assertions))]
                let fallback: Option<std::path::PathBuf> = None;
                let web_dir = resolve_web_dir(&resources, fallback.as_deref())
                    .map_err(std::io::Error::other)?;
                HostController::new(web_dir)
            };
            app.manage(Arc::new(NativeState::new(host)));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            host_status,
            choose_folder,
            start_service,
            stop_service,
            open_manager,
            create_access_key,
            diagnostic_status,
            set_diagnostic_mode,
            set_download_buffer,
            reset_diagnostic_settings,
            run_diagnostic_test,
            cancel_diagnostic_test
        ])
        .build(tauri::generate_context!())
        .expect("Native application initialization failed");

    app.run(|app, event| match event {
        #[cfg(target_os = "android")]
        tauri::RunEvent::WindowEvent {
            label,
            event: tauri::WindowEvent::Suspended,
            ..
        } if label == "main" => {
            if let Some(state) = app.try_state::<Arc<NativeState>>() {
                state.mark_suspended();
                let state = Arc::clone(&state);
                tauri::async_runtime::spawn(async move {
                    let _ = state.stop_for_background().await;
                });
            }
        }
        #[cfg(target_os = "android")]
        tauri::RunEvent::WindowEvent {
            label,
            event: tauri::WindowEvent::Resumed,
            ..
        } if label == "main" => {
            if let Some(state) = app.try_state::<Arc<NativeState>>() {
                state.mark_resumed();
            }
        }
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
            if framework_exit_is_deferred(
                cfg!(target_os = "android"),
                app.try_state::<Arc<NativeState>>()
                    .map(|state| state.shutdown_completed.load(Ordering::Acquire)),
            ) =>
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
mod mobile_lifecycle_tests;

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
