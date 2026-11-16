#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod view;

use share_domain::Product;
use share_host::{HostController, SessionCommand, StartOptions};
use share_native_slint::{
    branding::wordmark,
    bridge::{Bridge, BridgeHandle, BridgeUpdate},
    presentation::{parse_port, text, Locale, TextKey},
    updates::Latest,
};
use slint::{ComponentHandle, ModelRc, VecModel};
use std::{
    cell::RefCell,
    path::{Path, PathBuf},
    rc::Rc,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};
use view::{Pending, ViewState};

slint::include_modules!();

struct PickerReply {
    ticket: u64,
    path: Option<PathBuf>,
}

fn send(
    handle: &BridgeHandle,
    ui: &AppWindow,
    state: &mut ViewState,
    command: SessionCommand,
    action: Pending,
) {
    match handle.send(command) {
        Ok(id) => {
            state.pending.begin(id, action);
            ui.set_busy(true);
        }
        Err(_) => state.error(ui, TextKey::ErrorCommand),
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let started = Instant::now();
    let trace = std::env::var_os("INOUT_NATIVE_TRACE").is_some_and(|value| value == "1");
    let product = Product::default();
    let locale = sys_locale::get_locales()
        .find_map(|tag| Locale::from_tag(&tag))
        .unwrap_or(Locale::English);
    let resources = std::env::current_exe()?
        .parent()
        .ok_or_else(|| std::io::Error::other("Executable directory unavailable"))?
        .to_path_buf();
    let fallback = Path::new(env!("CARGO_MANIFEST_DIR")).join("../web/out");
    let resolved = share_host::resolve_web_dir(&resources, Some(&fallback));
    let resources_ready = resolved.is_ok();
    let web = resolved.unwrap_or_else(|_| resources.join("web"));
    let ui = AppWindow::new()?;
    // Read before the first frame. Preferences persistence and native OS
    // motion-policy adapters are separate from this opt-in desktop trial.
    ui.set_reduce_motion(
        std::env::var_os("INOUT_NATIVE_REDUCE_MOTION").is_some_and(|value| value == "1"),
    );
    ui.set_product_name(product.name.clone().into());
    ui.set_product_version(product.version.into());
    ui.set_product_description(product.description.into());
    if let Some(mark) = wordmark(&product.name) {
        ui.set_wordmark_columns(mark.columns);
        ui.set_wordmark_dots(ModelRc::new(VecModel::from(
            mark.dots
                .into_iter()
                .map(|dot| MatrixDot { x: dot.x, y: dot.y })
                .collect::<Vec<_>>(),
        )));
    }
    let state = Rc::new(RefCell::new(ViewState::new(locale, resources_ready, &ui)));
    state.borrow_mut().render(&ui, true);
    let updates = Arc::new(Latest::<BridgeUpdate>::default());
    let ready = Arc::new(AtomicBool::new(false));
    let opening = Rc::new(slint::Timer::default());

    if trace {
        let ready_for_render = ready.clone();
        let mut first_render = false;
        let mut first_ready_render = false;
        let notifier = ui.window().set_rendering_notifier(move |stage, _| {
            if matches!(stage, slint::RenderingState::AfterRendering) {
                if !first_render {
                    first_render = true;
                    eprintln!(
                        "native-first-render-callback-before-presentation elapsed_ms={:.3}",
                        started.elapsed().as_secs_f64() * 1000.0
                    );
                }
                if ready_for_render.load(Ordering::Relaxed) && !first_ready_render {
                    first_ready_render = true;
                    eprintln!(
                        "native-ready-render-callback-before-presentation elapsed_ms={:.3}",
                        started.elapsed().as_secs_f64() * 1000.0
                    );
                }
            }
        });
        eprintln!("native-render-notifier supported={}", notifier.is_ok());
    }

    let weak = ui.as_weak();
    let sink_updates = updates.clone();
    let bridge = Bridge::spawn(HostController::new(web), move |snapshot| {
        if sink_updates.push(snapshot)
            && weak
                .upgrade_in_event_loop(|ui| ui.invoke_refresh())
                .is_err()
        {
            sink_updates.discard();
        }
    })?;
    let handle = bridge.handle();

    {
        let weak = ui.as_weak();
        let state = state.clone();
        let opening = opening.clone();
        let ready = ready.clone();
        ui.on_refresh(move || {
            let (Some(ui), Some(update)) = (weak.upgrade(), updates.take()) else {
                return;
            };
            let exit = update.snapshot.exit_ready;
            state.borrow_mut().apply(&ui, update);
            if !ready.swap(true, Ordering::Relaxed) {
                ui.set_ready(true);
                ui.set_opening_opacity(0.0);
                if trace {
                    eprintln!(
                        "native-host-ready-callback elapsed_ms={:.3}",
                        started.elapsed().as_secs_f64() * 1000.0
                    );
                }
                if ui.get_reduce_motion() {
                    ui.set_opening_visible(false);
                } else {
                    let weak = ui.as_weak();
                    opening.start(
                        slint::TimerMode::SingleShot,
                        Duration::from_millis(180),
                        move || {
                            if let Some(ui) = weak.upgrade() {
                                ui.set_opening_visible(false);
                            }
                        },
                    );
                }
            }
            if exit {
                let _ = slint::quit_event_loop();
            }
        });
    }
    {
        let weak = ui.as_weak();
        let state = state.clone();
        let handle = handle.clone();
        ui.on_start_sharing(move || {
            let Some(ui) = weak.upgrade() else {
                return;
            };
            let mut state = state.borrow_mut();
            let Ok(port) = parse_port(&ui.get_port()) else {
                state.error(&ui, TextKey::ErrorPort);
                return;
            };
            let display = ui.get_folder();
            let root = state
                .native_folder
                .as_ref()
                .filter(|path| path.to_string_lossy() == display.as_str())
                .cloned()
                .unwrap_or_else(|| PathBuf::from(display.as_str()));
            state.picker_ticket = state.picker_ticket.wrapping_add(1);
            let key = ui.get_key();
            let options = StartOptions {
                network: ui.get_network(),
                port,
                uploads: ui.get_uploads(),
                manage: ui.get_manage(),
                key: (!key.is_empty()).then(|| key.to_string()),
            };
            state.local_problem = None;
            state.notice = None;
            send(
                &handle,
                &ui,
                &mut state,
                SessionCommand::StartWithRoot { root, options },
                Pending::Start,
            );
        });
    }
    {
        let weak = ui.as_weak();
        let state = state.clone();
        let handle = handle.clone();
        ui.on_stop_sharing(move || {
            if let Some(ui) = weak.upgrade() {
                let mut state = state.borrow_mut();
                state.local_problem = None;
                state.notice = None;
                send(
                    &handle,
                    &ui,
                    &mut state,
                    SessionCommand::RequestStop,
                    Pending::RequestStop,
                );
            }
        });
    }
    {
        let weak = ui.as_weak();
        let state = state.clone();
        let handle = handle.clone();
        ui.window().on_close_requested(move || {
            if let Some(ui) = weak.upgrade() {
                ui.invoke_hide_about();
                let mut state = state.borrow_mut();
                state.picker_ticket = state.picker_ticket.wrapping_add(1);
                send(
                    &handle,
                    &ui,
                    &mut state,
                    SessionCommand::RequestClose,
                    Pending::RequestClose,
                );
            }
            slint::CloseRequestResponse::KeepWindowShown
        });
    }
    for confirm in [false, true] {
        let weak = ui.as_weak();
        let state = state.clone();
        let handle = handle.clone();
        let action = move || {
            if let Some(ui) = weak.upgrade() {
                let mut state = state.borrow_mut();
                state.local_problem = None;
                state.notice = None;
                let pending = if confirm {
                    Pending::Confirm
                } else {
                    Pending::Cancel
                };
                let command = if confirm {
                    SessionCommand::ConfirmInterrupt
                } else {
                    SessionCommand::CancelInterrupt
                };
                send(&handle, &ui, &mut state, command, pending);
            }
        };
        if confirm {
            ui.on_confirm_interrupt(action);
        } else {
            ui.on_cancel_interrupt(action);
        }
    }
    {
        let weak = ui.as_weak();
        let state = state.clone();
        ui.on_locale_selected(move |index| {
            if let Some(ui) = weak.upgrade() {
                let mut state = state.borrow_mut();
                state.locale = Locale::from_index(index);
                state.render(&ui, true);
            }
        });
    }
    {
        let weak = ui.as_weak();
        let state = state.clone();
        ui.on_address_selected(move |index| {
            if let Some(ui) = weak.upgrade() {
                state.borrow().select_address(&ui, index);
            }
        });
    }
    {
        let weak = ui.as_weak();
        let state = state.clone();
        // X11 clipboard data requires its owner to live after the copy action.
        let clipboard = Rc::new(RefCell::new(None::<arboard::Clipboard>));
        ui.on_copy_address(move || {
            let Some(ui) = weak.upgrade() else {
                return;
            };
            let mut state = state.borrow_mut();
            let result = (|| {
                let mut clipboard = clipboard.borrow_mut();
                if clipboard.is_none() {
                    *clipboard = Some(arboard::Clipboard::new()?);
                }
                clipboard
                    .as_mut()
                    .expect("clipboard initialized")
                    .set_text(ui.get_selected_address().to_string())
            })();
            if result.is_err() {
                state.error(&ui, TextKey::ErrorCopy);
            } else {
                state.local_problem = None;
                state.notice = Some(TextKey::AddressCopied);
                state.render(&ui, false);
            }
        });
    }
    {
        let weak = ui.as_weak();
        let state = state.clone();
        ui.on_browser_result(move |opened| {
            let Some(ui) = weak.upgrade() else {
                return;
            };
            let mut state = state.borrow_mut();
            state.notice = None;
            if opened {
                state.local_problem = None;
                state.render(&ui, false);
            } else {
                state.error(&ui, TextKey::ErrorBrowser);
            }
        });
    }
    {
        let state = state.clone();
        ui.on_folder_edited(move || state.borrow_mut().invalidate_picker());
    }
    let picker_updates = Arc::new(Latest::<PickerReply>::default());
    {
        let weak = ui.as_weak();
        let state = state.clone();
        let replies = picker_updates.clone();
        ui.on_picker_ready(move || {
            let (Some(ui), Some(reply)) = (weak.upgrade(), replies.take()) else {
                return;
            };
            let mut state = state.borrow_mut();
            ui.set_picker_busy(false);
            if reply.ticket != state.picker_ticket
                || ui.get_running()
                || ui.get_confirming()
                || state
                    .snapshot
                    .as_ref()
                    .is_some_and(|snapshot| snapshot.exit_ready)
            {
                return;
            }
            if let Some(path) = reply.path {
                ui.set_folder(path.to_string_lossy().into_owned().into());
                state.native_folder = Some(path);
                state.local_problem = None;
                state.render(&ui, false);
            }
        });
    }
    {
        let weak = ui.as_weak();
        let state = state.clone();
        let handle = handle.clone();
        ui.on_choose_folder(move || {
            let Some(ui) = weak.upgrade() else {
                return;
            };
            let mut state = state.borrow_mut();
            state.picker_ticket = state.picker_ticket.wrapping_add(1);
            let ticket = state.picker_ticket;
            let title = text(state.locale, TextKey::ChooseFolder).to_owned();
            let replies = picker_updates.clone();
            let weak = ui.as_weak();
            ui.set_picker_busy(true);
            let result = handle.spawn_background(async move {
                let path = rfd::AsyncFileDialog::new()
                    .set_title(&title)
                    .pick_folder()
                    .await
                    .map(|folder| folder.path().to_path_buf());
                if replies.push(PickerReply { ticket, path })
                    && weak
                        .upgrade_in_event_loop(|ui| ui.invoke_picker_ready())
                        .is_err()
                {
                    replies.discard();
                }
            });
            if result.is_err() {
                ui.set_picker_busy(false);
                state.error(&ui, TextKey::ErrorPicker);
            }
        });
    }
    let result = ui.run();
    drop(bridge);
    result.map_err(Into::into)
}
