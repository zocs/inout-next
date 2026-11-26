#![cfg(feature = "gui")]

use std::{cell::RefCell, rc::Rc, time::Duration};

use i_slint_backend_testing::{TestingBackend, TestingBackendOptions};
use i_slint_core::{
    input::{InternalKeyEvent, KeyEvent, KeyEventType},
    item_tree::{ItemRc, ParentItemTraversalMode},
    items::{AccessibleRole, TextInput},
    window::WindowInner,
};
use share_native_slint::input_clipboard::InputClipboardPlatform;
use slint::{
    platform::{
        Clipboard, EventLoopProxy, Key, Platform, PlatformError, WindowAdapter, WindowEvent,
    },
    ComponentHandle, LogicalPosition,
};

slint::include_modules!();

const PUBLIC_DUMMY: &str = "PUBLIC-DUMMY-KEY";
const FOLDER: &str = "PUBLIC-FOLDER-DRAFT";
const ADDRESS: &str = "http://127.0.0.1:8123";

/// Only the operating system clipboard is substituted. The window, compiled
/// application UI and input dispatcher are the actual locked toolkit.
struct MemoryClipboardPlatform {
    backend: TestingBackend,
    channels: Rc<RefCell<[String; 2]>>,
}

impl Default for MemoryClipboardPlatform {
    fn default() -> Self {
        Self {
            backend: TestingBackend::new(TestingBackendOptions {
                mock_time: true,
                ..Default::default()
            }),
            channels: Rc::new(RefCell::new([String::new(), String::new()])),
        }
    }
}

impl Platform for MemoryClipboardPlatform {
    fn bind_context(
        &self,
        context: i_slint_core::SlintContextWeak,
        token: i_slint_core::InternalToken,
    ) {
        self.backend.bind_context(context, token);
    }

    fn create_window_adapter(&self) -> Result<Rc<dyn WindowAdapter>, PlatformError> {
        self.backend.create_window_adapter()
    }

    fn new_event_loop_proxy(&self) -> Option<Box<dyn EventLoopProxy>> {
        self.backend.new_event_loop_proxy()
    }

    fn duration_since_start(&self) -> Duration {
        self.backend.duration_since_start()
    }

    fn set_clipboard_text(&self, text: &str, clipboard: Clipboard) {
        if let Some(index) = channel_index(clipboard) {
            self.channels.borrow_mut()[index] = text.to_owned();
        }
    }

    fn clipboard_text(&self, clipboard: Clipboard) -> Option<String> {
        channel_index(clipboard).map(|index| self.channels.borrow()[index].clone())
    }
}

fn channel_index(clipboard: Clipboard) -> Option<usize> {
    match clipboard {
        Clipboard::DefaultClipboard => Some(0),
        Clipboard::SelectionClipboard => Some(1),
        _ => None,
    }
}

fn settle() {
    i_slint_backend_testing::mock_elapsed_time(Duration::ZERO);
}

fn app_fixture() -> (AppWindow, Rc<RefCell<[String; 2]>>) {
    let platform = MemoryClipboardPlatform::default();
    let channels = platform.channels.clone();
    slint::platform::set_platform(Box::new(InputClipboardPlatform::new(Box::new(platform))))
        .unwrap();
    let app = AppWindow::new().unwrap();
    app.set_product_name("Public clipboard regression fixture".into());
    app.set_ready(true);
    app.set_opening_visible(false);
    app.set_folder(FOLDER.into());
    app.set_key(PUBLIC_DUMMY.into());
    app.window()
        .dispatch_event(WindowEvent::WindowActiveChanged(true));
    app.show().unwrap();
    settle();
    (app, channels)
}

fn type_text(window: &slint::Window, text: &str) {
    window.dispatch_event(WindowEvent::KeyPressed { text: text.into() });
    window.dispatch_event(WindowEvent::KeyReleased { text: text.into() });
    settle();
}

fn control(window: &slint::Window, text: &str) {
    // Slint's logical Control modifier represents Command on macOS.
    window.dispatch_event(WindowEvent::KeyPressed {
        text: Key::Control.into(),
    });
    type_text(window, text);
    window.dispatch_event(WindowEvent::KeyReleased {
        text: Key::Control.into(),
    });
    settle();
}

fn tab(window: &slint::Window, backwards: bool) {
    if backwards {
        window.dispatch_event(WindowEvent::KeyPressed {
            text: Key::Shift.into(),
        });
    }
    let tab_text: slint::SharedString = Key::Tab.into();
    type_text(window, tab_text.as_str());
    if backwards {
        window.dispatch_event(WindowEvent::KeyReleased {
            text: Key::Shift.into(),
        });
    }
    settle();
}

fn focused_input(window: &slint::Window) -> Option<ItemRc> {
    let item = WindowInner::from_pub(window).focus_item.borrow().upgrade();
    item.filter(|item| item.downcast::<TextInput>().is_some())
}

fn focus_input(app: &AppWindow, expected: &str) {
    if app.get_running() {
        app.invoke_focus_stop();
    } else {
        app.invoke_focus_start();
    }
    for _ in 0..24 {
        if focused_input(app.window()).is_some_and(|item| {
            item.downcast::<TextInput>()
                .unwrap()
                .as_pin_ref()
                .text()
                .as_str()
                == expected
        }) {
            return;
        }
        tab(app.window(), !app.get_running());
    }
    panic!("the real application input did not receive keyboard focus");
}

fn pointer(window: &slint::Window, position: LogicalPosition, pressed: bool) {
    let button = slint::platform::PointerEventButton::Left;
    window.dispatch_event(if pressed {
        WindowEvent::PointerPressed { position, button }
    } else {
        WindowEvent::PointerReleased { position, button }
    });
}

fn drag_selection(window: &slint::Window, deactivate: bool) {
    let item = focused_input(window).unwrap();
    let origin = item.map_to_native_window(Default::default());
    // The FixedTestFont hit-test uses the item's top, without the physical
    // renderer's vertical alignment. Keep the pointer on its first text line.
    let start = LogicalPosition::new(origin.x + 1., origin.y + 7.);
    let end = LogicalPosition::new(origin.x + item.geometry().size.width + 20., origin.y + 7.);
    window.dispatch_event(WindowEvent::PointerMoved { position: start });
    pointer(window, start, true);
    window.dispatch_event(WindowEvent::PointerMoved { position: end });
    if deactivate {
        window.dispatch_event(WindowEvent::WindowActiveChanged(false));
    }
    pointer(window, end, false);
    settle();
}

fn reveal_key(window: &slint::Window) {
    let mut field = focused_input(window).unwrap();
    while field.accessible_role() != AccessibleRole::TextInput {
        field = field
            .parent_item(ParentItemTraversalMode::StopAtPopups)
            .unwrap();
    }
    let button = field
        .visit_descendants(|item| {
            if item.accessible_role() == AccessibleRole::Button {
                core::ops::ControlFlow::Break(item.clone())
            } else {
                core::ops::ControlFlow::Continue(())
            }
        })
        .expect("the actual secret TextField has a Show button");
    // Mapping takes a point in the item's parent coordinates, so include the
    // button's own x/y rather than treating the mapped parent origin as its top.
    let center = button.map_to_native_window(button.geometry().center());
    let position = LogicalPosition::new(center.x, center.y);
    window.dispatch_event(WindowEvent::PointerMoved { position });
    pointer(window, position, true);
    pointer(window, position, false);
    settle();
    assert!(
        focused_input(window).is_some_and(|item| {
            !item
                .downcast::<TextInput>()
                .unwrap()
                .as_pin_ref()
                .is_password()
        }),
        "the actual Show action must reveal the key for this control"
    );
}

#[test]
fn secret_clipboard_exports_preserve_existing_data_and_editable_selection() {
    let (app, channels) = app_fixture();
    for shortcut in ["c", "x"] {
        app.set_key(PUBLIC_DUMMY.into());
        settle();
        focus_input(&app, PUBLIC_DUMMY);
        control(app.window(), "a");
        channels.borrow_mut()[0] = "before-default".into();
        control(app.window(), shortcut);
        assert_eq!(
            channels.borrow()[0],
            "before-default",
            "secret Copy/Cut must preserve the clipboard"
        );
        assert_eq!(
            app.get_key().as_str(),
            PUBLIC_DUMMY,
            "Cut must preserve the key"
        );
        type_text(app.window(), "X");
        assert_eq!(
            app.get_key().as_str(),
            "X",
            "the protected selection must remain editable"
        );
    }

    app.set_key(PUBLIC_DUMMY.into());
    settle();
    focus_input(&app, PUBLIC_DUMMY);
    channels.borrow_mut()[1] = "before-primary".into();
    drag_selection(app.window(), false);
    assert_eq!(
        channels.borrow()[1],
        "before-primary",
        "masked pointer selection must preserve PRIMARY"
    );
    type_text(app.window(), "X");
    assert_eq!(app.get_key().as_str(), "X");

    app.set_key(PUBLIC_DUMMY.into());
    settle();
    focus_input(&app, PUBLIC_DUMMY);
    reveal_key(app.window());
    channels.borrow_mut()[1] = "before-primary".into();
    drag_selection(app.window(), false);
    assert_eq!(
        channels.borrow()[1],
        "before-primary",
        "Show must not enable secret PRIMARY export"
    );
    channels.borrow_mut()[0] = "before-default".into();
    control(app.window(), "x");
    assert_eq!(channels.borrow()[0], "before-default");
    assert_eq!(app.get_key().as_str(), PUBLIC_DUMMY);

    channels.borrow_mut()[1] = "before-primary".into();
    drag_selection(app.window(), true);
    assert_eq!(
        channels.borrow()[1],
        "before-primary",
        "deactivation must not publish the secret selection"
    );
    app.window()
        .dispatch_event(WindowEvent::WindowActiveChanged(true));
    type_text(app.window(), "X");
    assert_eq!(
        app.get_key().as_str(),
        "X",
        "interrupted selection must stay editable"
    );

    control(app.window(), "a");
    channels.borrow_mut()[0] = "PUBLIC-PASTE".into();
    control(app.window(), "v");
    assert_eq!(
        app.get_key().as_str(),
        "PUBLIC-PASTE",
        "secret paste must remain supported"
    );
    control(app.window(), "a");
    app.window()
        .dispatch_event(WindowEvent::internal(InternalKeyEvent {
            event_type: KeyEventType::UpdateComposition,
            preedit_text: "gongxiang".into(),
            preedit_selection: Some(0..9),
            ..Default::default()
        }));
    assert_eq!(app.get_key().as_str(), "PUBLIC-PASTE");
    let mut key_event = KeyEvent::default();
    key_event.text = "共享".into();
    app.window()
        .dispatch_event(WindowEvent::internal(InternalKeyEvent {
            event_type: KeyEventType::CommitComposition,
            key_event,
            ..Default::default()
        }));
    assert_eq!(
        app.get_key().as_str(),
        "共享",
        "composition must commit through the real input dispatcher"
    );
    app.hide().unwrap();
}

#[test]
fn ordinary_editing_and_address_copy_survive_secret_field_removal_and_recreation() {
    let (app, channels) = app_fixture();
    focus_input(&app, FOLDER);
    control(app.window(), "a");
    channels.borrow_mut()[0] = "before-default".into();
    control(app.window(), "x");
    assert_eq!(
        channels.borrow()[0],
        FOLDER,
        "ordinary Cut must publish its selected text"
    );
    assert_eq!(
        app.get_folder().as_str(),
        "",
        "ordinary Cut must still edit its field"
    );
    channels.borrow_mut()[0] = "共享😀".into();
    control(app.window(), "v");
    assert_eq!(
        app.get_folder().as_str(),
        "共享😀",
        "ordinary Unicode paste must remain supported"
    );

    app.set_selected_address(ADDRESS.into());
    app.set_running(true);
    settle();
    focus_input(&app, ADDRESS);
    control(app.window(), "a");
    channels.borrow_mut()[0] = "before-default".into();
    control(app.window(), "c");
    assert_eq!(
        channels.borrow()[0],
        ADDRESS,
        "the running address must remain copyable"
    );
    channels.borrow_mut()[1] = "before-primary".into();
    drag_selection(app.window(), false);
    assert_eq!(
        channels.borrow()[1],
        ADDRESS,
        "ordinary address PRIMARY must remain supported"
    );
    control(app.window(), "x");
    type_text(app.window(), "X");
    assert_eq!(
        app.get_selected_address().as_str(),
        ADDRESS,
        "the address stays read-only"
    );

    app.set_key("".into());
    app.set_running(false);
    settle();
    focus_input(&app, "");
    type_text(app.window(), PUBLIC_DUMMY);
    assert_eq!(
        app.get_key().as_str(),
        PUBLIC_DUMMY,
        "the recreated key field must receive the new input"
    );
    control(app.window(), "a");
    channels.borrow_mut()[0] = "before-default".into();
    control(app.window(), "x");
    assert_eq!(
        channels.borrow()[0],
        "before-default",
        "the recreated key stays protected"
    );
    assert_eq!(app.get_key().as_str(), PUBLIC_DUMMY);
    app.hide().unwrap();
}
