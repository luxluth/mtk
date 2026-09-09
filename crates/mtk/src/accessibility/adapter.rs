use accesskit::{
    ActionHandler, ActionRequest, ActivationHandler, DeactivationHandler, Rect, TreeUpdate,
};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use winit::event_loop::EventLoopProxy;
use winit::raw_window_handle::RawWindowHandle;

/// Events received from platform accessibility bridges.
#[derive(Debug)]
pub enum A11yEvent {
    InitialTreeRequested,
    ActionRequested(ActionRequest),
    AccessibilityDeactivated,
}

struct ActivationGlue {
    tx: Sender<A11yEvent>,
    proxy: Arc<Mutex<Option<EventLoopProxy>>>,
}

impl ActivationHandler for ActivationGlue {
    fn request_initial_tree(&mut self) -> Option<TreeUpdate> {
        let _ = self.tx.send(A11yEvent::InitialTreeRequested);
        if let Ok(guard) = self.proxy.lock() {
            if let Some(proxy) = guard.as_ref() {
                proxy.wake_up();
            }
        }
        None
    }
}

struct ActionGlue {
    tx: Sender<A11yEvent>,
    proxy: Arc<Mutex<Option<EventLoopProxy>>>,
}

impl ActionHandler for ActionGlue {
    fn do_action(&mut self, request: ActionRequest) {
        let _ = self.tx.send(A11yEvent::ActionRequested(request));
        if let Ok(guard) = self.proxy.lock() {
            if let Some(proxy) = guard.as_ref() {
                proxy.wake_up();
            }
        }
    }
}

struct DeactivationGlue {
    tx: Sender<A11yEvent>,
    proxy: Arc<Mutex<Option<EventLoopProxy>>>,
}

impl DeactivationHandler for DeactivationGlue {
    fn deactivate_accessibility(&mut self) {
        let _ = self.tx.send(A11yEvent::AccessibilityDeactivated);
        if let Ok(guard) = self.proxy.lock() {
            if let Some(proxy) = guard.as_ref() {
                proxy.wake_up();
            }
        }
    }
}

#[cfg(any(
    target_os = "linux",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd"
))]
pub struct A11yAdapter {
    adapter: accesskit_unix::Adapter,
    is_initialized: bool,
}

#[cfg(any(
    target_os = "linux",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd"
))]
impl A11yAdapter {
    pub fn new(
        _window_handle: RawWindowHandle,
        tx: Sender<A11yEvent>,
        proxy: Arc<Mutex<Option<EventLoopProxy>>>,
    ) -> Self {
        let activation = ActivationGlue {
            tx: tx.clone(),
            proxy: proxy.clone(),
        };
        let action = ActionGlue {
            tx: tx.clone(),
            proxy: proxy.clone(),
        };
        let deactivation = DeactivationGlue { tx, proxy };
        let adapter = accesskit_unix::Adapter::new(activation, action, deactivation);
        Self {
            adapter,
            is_initialized: false,
        }
    }

    pub fn update_if_active(&mut self, updater: impl FnOnce(bool) -> TreeUpdate) {
        let is_initialized = &mut self.is_initialized;
        self.adapter.update_if_active(|| {
            let is_full = !*is_initialized;
            let update = updater(is_full);
            *is_initialized = true;
            update
        });
    }

    pub fn deactivate(&mut self) {
        self.is_initialized = false;
    }

    pub fn set_focus(&mut self, is_focused: bool) {
        self.adapter.update_window_focus_state(is_focused);
    }

    pub fn set_window_bounds(&mut self, outer: Rect, inner: Rect) {
        self.adapter.set_root_window_bounds(outer, inner);
    }
}

#[cfg(target_os = "windows")]
pub struct A11yAdapter {
    adapter: accesskit_windows::SubclassingAdapter,
    is_initialized: bool,
}

#[cfg(target_os = "windows")]
impl A11yAdapter {
    pub fn new(
        window_handle: RawWindowHandle,
        tx: Sender<A11yEvent>,
        proxy: Arc<Mutex<Option<EventLoopProxy>>>,
    ) -> Self {
        let hwnd = match window_handle {
            RawWindowHandle::Win32(handle) => handle.hwnd.get() as *mut _,
            RawWindowHandle::WinRt(_) => unimplemented!(),
            _ => unreachable!(),
        };
        let activation = ActivationGlue {
            tx: tx.clone(),
            proxy: proxy.clone(),
        };
        let action = ActionGlue { tx, proxy };
        let adapter = accesskit_windows::SubclassingAdapter::new(
            accesskit_windows::HWND(hwnd),
            activation,
            action,
        );
        Self {
            adapter,
            is_initialized: false,
        }
    }

    pub fn update_if_active(&mut self, updater: impl FnOnce(bool) -> TreeUpdate) {
        let is_initialized = &mut self.is_initialized;
        if let Some(events) = self.adapter.update_if_active(|| {
            let is_full = !*is_initialized;
            let update = updater(is_full);
            *is_initialized = true;
            update
        }) {
            events.raise();
        }
    }

    pub fn deactivate(&mut self) {
        self.is_initialized = false;
    }

    pub fn set_focus(&mut self, _is_focused: bool) {}

    pub fn set_window_bounds(&mut self, _outer: Rect, _inner: Rect) {}
}

#[cfg(target_os = "macos")]
pub struct A11yAdapter {
    adapter: accesskit_macos::SubclassingAdapter,
    is_initialized: bool,
}

#[cfg(target_os = "macos")]
impl A11yAdapter {
    pub fn new(
        window_handle: RawWindowHandle,
        tx: Sender<A11yEvent>,
        proxy: Arc<Mutex<Option<EventLoopProxy>>>,
    ) -> Self {
        let view = match window_handle {
            RawWindowHandle::AppKit(handle) => handle.ns_view.as_ptr(),
            RawWindowHandle::UiKit(_) => unimplemented!(),
            _ => unreachable!(),
        };
        let activation = ActivationGlue {
            tx: tx.clone(),
            proxy: proxy.clone(),
        };
        let action = ActionGlue { tx, proxy };
        let adapter = unsafe { accesskit_macos::SubclassingAdapter::new(view, activation, action) };
        Self {
            adapter,
            is_initialized: false,
        }
    }

    pub fn update_if_active(&mut self, updater: impl FnOnce(bool) -> TreeUpdate) {
        let is_initialized = &mut self.is_initialized;
        if let Some(events) = self.adapter.update_if_active(|| {
            let is_full = !*is_initialized;
            let update = updater(is_full);
            *is_initialized = true;
            update
        }) {
            events.raise();
        }
    }

    pub fn deactivate(&mut self) {
        self.is_initialized = false;
    }

    pub fn set_focus(&mut self, is_focused: bool) {
        if let Some(events) = self.adapter.update_view_focus_state(is_focused) {
            events.raise();
        }
    }

    pub fn set_window_bounds(&mut self, _outer: Rect, _inner: Rect) {}
}

#[cfg(not(any(
    target_os = "linux",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd",
    target_os = "windows",
    target_os = "macos"
)))]
pub struct A11yAdapter;

#[cfg(not(any(
    target_os = "linux",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd",
    target_os = "windows",
    target_os = "macos"
)))]
impl A11yAdapter {
    pub fn new(
        _window_handle: RawWindowHandle,
        _tx: Sender<A11yEvent>,
        _proxy: Arc<Mutex<Option<EventLoopProxy>>>,
    ) -> Self {
        Self
    }
    pub fn update_if_active(&mut self, _updater: impl FnOnce(bool) -> TreeUpdate) {}
    pub fn deactivate(&mut self) {}
    pub fn set_focus(&mut self, _is_focused: bool) {}
    pub fn set_window_bounds(&mut self, _outer: Rect, _inner: Rect) {}
}
