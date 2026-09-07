//! Elm-inspired batchable command primitives for side-effects and asynchronous tasks.
//!
//! Provides [`Command`] and [`IntoCommand`] for performing actions against [`Context`]
//! and dispatching background tasks back into the event loop.

use crate::{Context, Node, windowing::WindowHandle};
use std::future::Future;

type SyncAction<Msg> = Box<dyn FnOnce(&mut Context) -> Option<Msg> + Send>;
type AsyncAction<Msg> = Box<dyn FnOnce(WindowHandle<Msg>) + Send>;

/// A batchable container of side-effecting operations and background tasks.
pub struct Command<Msg: 'static + Send> {
    pub(crate) sync_actions: Vec<SyncAction<Msg>>,
    pub(crate) async_actions: Vec<AsyncAction<Msg>>,
}

impl<Msg: 'static + Send> Command<Msg> {
    /// Creates an empty command that performs no actions.
    pub fn none() -> Self {
        Self {
            sync_actions: Vec::new(),
            async_actions: Vec::new(),
        }
    }

    /// Performs a synchronous side-effect with full, direct access to [`Context`].
    ///
    /// The closure can optionally return a follow-up message to be dispatched into the event loop.
    pub fn perform<F>(action: F) -> Self
    where
        F: FnOnce(&mut Context) -> Option<Msg> + Send + 'static,
    {
        Self {
            sync_actions: vec![Box::new(action)],
            async_actions: Vec::new(),
        }
    }

    /// Spawns an asynchronous task on a background worker thread, executing `future` to completion
    /// and mapping its output into a message sent back to the application.
    pub fn perform_async<Fut, T, M>(future: Fut, mapper: M) -> Self
    where
        Fut: Future<Output = T> + Send + 'static,
        T: Send + 'static,
        M: FnOnce(T) -> Msg + Send + 'static,
    {
        Self {
            sync_actions: Vec::new(),
            async_actions: vec![Box::new(move |handle: WindowHandle<Msg>| {
                std::thread::spawn(move || {
                    let result = pollster::block_on(future);
                    let msg = mapper(result);
                    let _ = handle.send(msg);
                });
            })],
        }
    }

    /// Batches multiple commands together into a single composite command.
    pub fn batch(commands: impl IntoIterator<Item = Command<Msg>>) -> Self {
        let mut combined = Self::none();
        for cmd in commands {
            combined.sync_actions.extend(cmd.sync_actions);
            combined.async_actions.extend(cmd.async_actions);
        }
        combined
    }

    /// Convenience command to copy text data to the system clipboard.
    pub fn clipboard_set(text: impl Into<String>) -> Self {
        let text = text.into();
        Self::perform(move |ctx| {
            ctx.clipboard_copy(crate::ClipboardData::Text(text));
            None
        })
    }

    /// Convenience command to request focus for a specific [`Node`].
    pub fn focus(node: Node) -> Self {
        Self::perform(move |ctx| {
            ctx.request_focus(node);
            None
        })
    }

    /// Convenience command to clear keyboard focus.
    pub fn clear_focus() -> Self {
        Self::perform(|ctx| {
            ctx.clear_focus();
            None
        })
    }

    /// Extracts the internal action vectors from this command.
    pub fn into_actions(self) -> (Vec<SyncAction<Msg>>, Vec<AsyncAction<Msg>>) {
        (self.sync_actions, self.async_actions)
    }

    /// Returns `true` if this command contains no actions.
    pub fn is_empty(&self) -> bool {
        self.sync_actions.is_empty() && self.async_actions.is_empty()
    }
}

impl<Msg: 'static + Send> Default for Command<Msg> {
    fn default() -> Self {
        Self::none()
    }
}

/// Conversion trait allowing update functions to return `()`, `Command`, `Option<Command>`, or `Vec<Command>`.
pub trait IntoCommand<Msg: 'static + Send> {
    /// Converts `self` into a [`Command`].
    fn into_command(self) -> Command<Msg>;
}

impl<Msg: 'static + Send> IntoCommand<Msg> for () {
    fn into_command(self) -> Command<Msg> {
        Command::none()
    }
}

impl<Msg: 'static + Send> IntoCommand<Msg> for Command<Msg> {
    fn into_command(self) -> Command<Msg> {
        self
    }
}

impl<Msg: 'static + Send> IntoCommand<Msg> for Option<Command<Msg>> {
    fn into_command(self) -> Command<Msg> {
        self.unwrap_or_else(Command::none)
    }
}

impl<Msg: 'static + Send> IntoCommand<Msg> for Vec<Command<Msg>> {
    fn into_command(self) -> Command<Msg> {
        Command::batch(self)
    }
}

impl<Msg: 'static + Send, const N: usize> IntoCommand<Msg> for [Command<Msg>; N] {
    fn into_command(self) -> Command<Msg> {
        Command::batch(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_batch_and_into() {
        let cmd1: Command<i32> = Command::none();
        let cmd2: Command<i32> = Command::perform(|_| Some(42));
        let batched = Command::batch([cmd1, cmd2]);
        assert_eq!(batched.sync_actions.len(), 1);
        assert_eq!(batched.async_actions.len(), 0);

        let from_unit: Command<i32> = ().into_command();
        assert!(from_unit.is_empty());

        let from_option: Command<i32> = Some(Command::none()).into_command();
        assert!(from_option.is_empty());

        let from_vec: Command<i32> = vec![Command::none(), Command::none()].into_command();
        assert!(from_vec.is_empty());
    }
}
