use std::sync::Mutex;

/// A passive sample cannot acknowledge an action sent after that sample.
/// Completion watermarks survive coalescing with later progress snapshots.
pub struct CommandCompletion<T> {
    pending: Option<(u64, T)>,
}

impl<T> Default for CommandCompletion<T> {
    fn default() -> Self {
        Self { pending: None }
    }
}

impl<T> CommandCompletion<T> {
    pub fn begin(&mut self, command: u64, action: T) {
        self.pending = Some((command, action));
    }

    pub fn is_pending(&self) -> bool {
        self.pending.is_some()
    }

    pub fn finish_through(&mut self, completed: u64) -> Option<T> {
        if self
            .pending
            .as_ref()
            .is_some_and(|(command, _)| *command <= completed)
        {
            self.pending.take().map(|(_, action)| action)
        } else {
            None
        }
    }
}

/// A stalled renderer retains one latest state and one scheduled delivery.
pub struct Latest<T>(Mutex<(Option<T>, bool)>);

impl<T> Default for Latest<T> {
    fn default() -> Self {
        Self(Mutex::new((None, false)))
    }
}

impl<T> Latest<T> {
    /// Returns true only when the caller needs to enqueue a delivery.
    pub fn push(&self, value: T) -> bool {
        let mut state = self.0.lock().unwrap_or_else(|error| error.into_inner());
        state.0 = Some(value);
        !std::mem::replace(&mut state.1, true)
    }

    pub fn take(&self) -> Option<T> {
        let mut state = self.0.lock().unwrap_or_else(|error| error.into_inner());
        state.1 = false;
        state.0.take()
    }

    pub fn discard(&self) {
        let mut state = self.0.lock().unwrap_or_else(|error| error.into_inner());
        *state = (None, false);
    }
}
