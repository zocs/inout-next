#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod view;

use share_domain::Product;
use share_host::{HostController, SessionCommand, StartOptions};
use share_native_slint::{
    branding::wordmark,
    bridge::{Bridge, BridgeHandle, BridgeUpdate},
    folders,
    input_clipboard::InputClipboardPlatform,
    preferences::{self, Preferences},
    presentation::{parse_port, text, Access, Locale, TextKey},
    updates::Latest,
    web,
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

/// Save presentation choices and setup drafts. The access key is never saved,
/// and a failed write leaves this session's choices in effect.
fn remember(ui: &AppWindow, state: &ViewState, path: Option<&Path>) {
    let Some(path) = path else {
        return;
    };
    let theme = ui.global::<Theme>();
    let _ = Preferences {
        locale: state.locale_chosen.then_some(state.locale),
        accent: theme.get_accent_index(),
        appearance: theme.get_appearance(),
        folder: ui.get_folder().to_string(),
        port: ui.get_port().to_string(),
        network: ui.get_network(),
        access: Access::from_index(ui.get_access_index()),
    }
    .save(path);
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let started = Instant::now();
    let trace = std::env::var_os("INOUT_NATIVE_TRACE").is_some_and(|value| value == "1");
    let product = Product::default();
    let preferences_path = preferences::location(&product.application_id);
    let saved = preferences_path
        .as_deref()
        .map(Preferences::load)
        .unwrap_or_default();
    let locale = saved.locale.unwrap_or_else(|| {
        sys_locale::get_locales()
            .find_map(|tag| Locale::from_tag(&tag))
            .unwrap_or(Locale::English)
    });
    // Pages compiled into the executable need no folder beside it, so any
    // folder can be shared, including one that contains this application.
    let (host, resources_ready) = match web::bundle() {
        Some(bundle) => (HostController::new_embedded(bundle), true),
        None => {
            let resources = std::env::current_exe()?
                .parent()
                .ok_or_else(|| std::io::Error::other("Executable directory unavailable"))?
                .to_path_buf();
            let fallback = Path::new(env!("CARGO_MANIFEST_DIR")).join("../web/out");
            let resolved = share_host::resolve_web_dir(&resources, Some(&fallback));
            let ready = resolved.is_ok();
            (
                HostController::new(resolved.unwrap_or_else(|_| resources.join("web"))),
                ready,
            )
        }
    };
    let backend = i_slint_backend_selector::create_backend()?;
    slint::platform::set_platform(Box::new(InputClipboardPlatform::new(backend)))?;
    let ui = AppWindow::new()?;
    // Read before the first frame. Detecting the OS motion policy is a
    // separate native adapter.
    let reduce_motion =
        std::env::var_os("INOUT_NATIVE_REDUCE_MOTION").is_some_and(|value| value == "1");
    {
        let theme = ui.global::<Theme>();
        theme.set_reduce_motion(reduce_motion);
        theme.set_accent_index(saved.accent);
        theme.set_appearance(saved.appearance);
    }
    ui.set_folder(saved.folder.clone().into());
    ui.set_port(saved.port.clone().into());
    ui.set_network(saved.network);
    ui.set_access_index(saved.access.index());
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
    let state = Rc::new(RefCell::new(ViewState::new(
        locale,
        saved.locale.is_some(),
        resources_ready,
        folders::detect(),
        &ui,
    )));
    let preferences_path: Rc<Option<PathBuf>> = Rc::new(preferences_path);
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
    let bridge = Bridge::spawn(host, move |snapshot| {
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
                if reduce_motion {
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
        let preferences = preferences_path.clone();
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
                .unwrap_or_else(|| folders::expand(display.as_str(), folders::home().as_deref()));
            state.picker_ticket = state.picker_ticket.wrapping_add(1);
            let key = ui.get_key();
            let access = Access::from_index(ui.get_access_index());
            let options = StartOptions {
                network: ui.get_network(),
                port,
                uploads: access.uploads(),
                manage: access.manage(),
                key: (!key.is_empty()).then(|| key.to_string()),
            };
            remember(&ui, &state, preferences.as_deref());
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
        let preferences = preferences_path.clone();
        ui.on_locale_selected(move |index| {
            if let Some(ui) = weak.upgrade() {
                let mut state = state.borrow_mut();
                state.locale = Locale::from_index(index);
                state.locale_chosen = true;
                state.render(&ui, true);
                remember(&ui, &state, preferences.as_deref());
            }
        });
    }
    for accent in [true, false] {
        let weak = ui.as_weak();
        let state = state.clone();
        let preferences = preferences_path.clone();
        let saved = move |_: i32| {
            if let Some(ui) = weak.upgrade() {
                remember(&ui, &state.borrow(), preferences.as_deref());
            }
        };
        if accent {
            ui.on_accent_selected(saved);
        } else {
            ui.on_appearance_selected(saved);
        }
    }
    {
        let weak = ui.as_weak();
        let state = state.clone();
        ui.on_endpoint_selected(move |index| {
            if let Some(ui) = weak.upgrade() {
                state.borrow().select_endpoint(&ui, index);
            }
        });
    }
    {
        let weak = ui.as_weak();
        let state = state.clone();
        ui.on_quick_folder(move |index| {
            let Some(ui) = weak.upgrade() else {
                return;
            };
            let mut state = state.borrow_mut();
            let Some((_, path)) = usize::try_from(index)
                .ok()
                .and_then(|index| state.common.get(index).cloned())
            else {
                return;
            };
            state.picker_ticket = state.picker_ticket.wrapping_add(1);
            ui.set_folder(path.to_string_lossy().into_owned().into());
            state.native_folder = Some(path);
            state.local_problem = None;
            state.render(&ui, false);
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
    // Ordinary close also saves drafts that have not been used to start a run.
    remember(&ui, &state.borrow(), preferences_path.as_deref());
    drop(bridge);
    result.map_err(Into::into)
}
