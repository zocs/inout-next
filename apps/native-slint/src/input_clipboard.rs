use std::{
    cell::RefCell,
    rc::{Rc, Weak},
    time::Duration,
};

use i_slint_core::{
    accessibility::AccessibleStringProperty,
    item_tree::ParentItemTraversalMode,
    items::{AccessibleRole, TextInput},
    window::WindowInner,
};
use slint::platform::{Clipboard, EventLoopProxy, Platform, PlatformError, WindowAdapter};

pub struct InputClipboardPlatform {
    backend: Box<dyn Platform>,
    windows: RefCell<Vec<Weak<dyn WindowAdapter>>>,
}

impl InputClipboardPlatform {
    pub fn new(backend: Box<dyn Platform>) -> Self {
        Self {
            backend,
            windows: RefCell::default(),
        }
    }

    fn can_export(&self, clipboard: &Clipboard) -> bool {
        let windows: Vec<_> = self
            .windows
            .borrow()
            .iter()
            .filter_map(Weak::upgrade)
            .collect();
        let focused: Vec<_> = windows
            .iter()
            .filter_map(|adapter| {
                let inner = WindowInner::from_pub(adapter.window());
                let item = inner.focus_item.borrow().upgrade()?;
                let input = item.downcast::<TextInput>()?;
                let input = input.as_pin_ref();
                let mut protected = input.is_password();
                if !protected && input.has_selection() {
                    // Missing metadata does not identify an ordinary selection.
                    protected = true;
                    // Reveal changes InputType but preserves the TextField's empty
                    // accessibility value. Read this existing export contract from
                    // the nearest accessible input, without keeping host focus state.
                    let mut ancestor = Some(item);
                    while let Some(current) = ancestor {
                        if current.accessible_role() == AccessibleRole::TextInput {
                            protected = current
                                .accessible_string_property(AccessibleStringProperty::Value)
                                .is_none_or(|value| value.is_empty());
                            break;
                        }
                        ancestor = current.parent_item(ParentItemTraversalMode::StopAtPopups);
                    }
                }
                Some((inner.active(), protected))
            })
            .collect();
        if focused.iter().any(|(_, protected)| *protected) {
            return false;
        }
        match clipboard {
            Clipboard::DefaultClipboard => true,
            // An unfocused release after popup/deactivation cannot identify an
            // ordinary active input. Do not publish that selection implicitly.
            Clipboard::SelectionClipboard => focused.iter().any(|(active, _)| *active),
            _ => false,
        }
    }
}

impl Platform for InputClipboardPlatform {
    fn bind_context(
        &self,
        context: i_slint_core::SlintContextWeak,
        token: i_slint_core::InternalToken,
    ) {
        self.backend.bind_context(context, token);
    }

    #[allow(deprecated)]
    fn set_event_loop_quit_on_last_window_closed(&self, quit_on_last_window_closed: bool) {
        self.backend
            .set_event_loop_quit_on_last_window_closed(quit_on_last_window_closed);
    }

    fn create_window_adapter(&self) -> Result<Rc<dyn WindowAdapter>, PlatformError> {
        let adapter = self.backend.create_window_adapter()?;
        let mut windows = self.windows.borrow_mut();
        windows.retain(|window| window.strong_count() != 0);
        windows.push(Rc::downgrade(&adapter));
        Ok(adapter)
    }

    fn run_event_loop(&self) -> Result<(), PlatformError> {
        self.backend.run_event_loop()
    }

    fn process_events(
        &self,
        timeout: Option<Duration>,
        token: i_slint_core::InternalToken,
    ) -> Result<core::ops::ControlFlow<()>, PlatformError> {
        self.backend.process_events(timeout, token)
    }

    fn new_event_loop_proxy(&self) -> Option<Box<dyn EventLoopProxy>> {
        self.backend.new_event_loop_proxy()
    }

    fn duration_since_start(&self) -> Duration {
        self.backend.duration_since_start()
    }

    fn click_interval(&self) -> Duration {
        self.backend.click_interval()
    }

    fn cursor_flash_cycle(&self) -> Duration {
        self.backend.cursor_flash_cycle()
    }

    fn set_clipboard_text(&self, text: &str, clipboard: Clipboard) {
        if self.can_export(&clipboard) {
            self.backend.set_clipboard_text(text, clipboard);
        }
    }

    fn clipboard_text(&self, clipboard: Clipboard) -> Option<String> {
        self.backend.clipboard_text(clipboard)
    }

    fn debug_log(&self, arguments: core::fmt::Arguments) {
        self.backend.debug_log(arguments);
    }

    fn open_url(&self, url: &str) -> Result<(), PlatformError> {
        self.backend.open_url(url)
    }

    #[cfg(target_os = "android")]
    fn long_press_interval(&self, token: i_slint_core::InternalToken) -> Duration {
        self.backend.long_press_interval(token)
    }
}
