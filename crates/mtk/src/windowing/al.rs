use std::{collections::HashMap, sync::Arc, time::Instant};

use winit::{
    application::ApplicationHandler,
    dpi::{PhysicalPosition, PhysicalSize},
    event::{MouseScrollDelta, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{Window as WWindow, WindowId},
};

use crate::{
    Context, Node, TextStyle,
    ui::{Event, View, event::EventResult},
    windowing::renderer::Renderer,
};

use std::sync::Mutex;
use std::sync::mpsc::{Receiver, Sender, channel};
use winit::event_loop::EventLoopProxy;

/// A thread-safe, cloneable handle to dispatch messages to the UI event loop from background threads.
#[derive(Clone)]
pub struct WindowHandle<Msg: 'static + Send> {
    tx: Sender<Msg>,
    proxy: Arc<Mutex<Option<EventLoopProxy<()>>>>,
}

impl<Msg: 'static + Send> WindowHandle<Msg> {
    /// Sends a message from any thread to the UI event loop, triggering `update` and view diffing.
    pub fn send(&self, msg: Msg) -> Result<(), Msg> {
        self.tx.send(msg).map_err(|e| e.0)?;
        if let Ok(guard) = self.proxy.lock() {
            if let Some(proxy) = guard.as_ref() {
                let _ = proxy.send_event(());
            }
        }
        Ok(())
    }
}

pub struct Window<'r, S, V>
where
    V: View<S>,
    V::Message: 'static + Send,
{
    msg_tx: Sender<V::Message>,
    msg_rx: Receiver<V::Message>,
    event_proxy: Arc<Mutex<Option<EventLoopProxy<()>>>>,
    renderer: Option<Renderer<'r>>,

    window: Option<Arc<WWindow>>,
    context: Context,
    state: S,

    app_view_fn: Option<Box<dyn FnMut(&S) -> V>>,
    update_fn: Option<Box<dyn FnMut(&mut S, V::Message)>>,

    view: Option<V>,
    element: Option<V::Element>,
    attr: WindowAttributes,
    cursor_pos: (f32, f32),
    last_frame_time: Instant,
    scroll_targets: HashMap<Node, f32>,
    scroll_targets_x: HashMap<Node, f32>,
    drag_scroll_node: Option<(Node, f32, f32)>,
    drag_scroll_x_node: Option<(Node, f32, f32)>,

    debug_tx: Option<std::sync::mpsc::Sender<crate::debugger::DebugEvent>>,
    debug_rx_cmd: Option<std::sync::mpsc::Receiver<crate::debugger::DebugCommand>>,
    hovered_node: Option<Node>,
}

#[derive(Debug, Clone, Copy)]
pub struct WindowDimension {
    pub width: u32,
    pub height: u32,
}

impl WindowDimension {
    pub fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }

    pub fn zero() -> Self {
        Self {
            width: 0,
            height: 0,
        }
    }
}

impl From<(u32, u32)> for WindowDimension {
    fn from((width, height): (u32, u32)) -> Self {
        WindowDimension::new(width, height)
    }
}

impl Into<winit::dpi::Size> for WindowDimension {
    fn into(self) -> winit::dpi::Size {
        winit::dpi::Size::Physical(PhysicalSize {
            width: self.width,
            height: self.height,
        })
    }
}

#[derive(Debug, Clone)]
pub struct WindowAttributes {
    pub resizable: bool,
    pub transparent: bool,
    pub blur: bool,
    pub decorations: bool,
    pub size: WindowDimension,
    pub min_size: Option<WindowDimension>,
    pub max_size: Option<WindowDimension>,
    pub title: String,
    #[cfg(target_os = "linux")]
    pub app_id: String,
}

macro_rules! attr_fn {
    ($name:ident, $field:ident, $t:ty) => {
        pub fn $name(mut self: Self, value: $t) -> Self {
            self.$field = value;
            self
        }
    };
}

macro_rules! attr_fn_string {
    ($name:ident, $field:ident) => {
        pub fn $name<S: ToString>(mut self: Self, value: S) -> Self {
            self.$field = value.to_string();
            self
        }
    };
}

impl WindowAttributes {
    pub fn new() -> Self {
        Self::default()
    }

    attr_fn_string!(with_title, title);
    attr_fn!(with_resizable, resizable, bool);
    attr_fn!(with_transparency, transparent, bool);
    attr_fn!(with_blur, blur, bool);
    attr_fn!(with_decorations, decorations, bool);
    attr_fn!(with_size, size, WindowDimension);
    attr_fn!(with_min_size, min_size, Option<WindowDimension>);
    attr_fn!(with_max_size, max_size, Option<WindowDimension>);

    #[cfg(any(
        target_os = "linux",
        target_os = "freebsd",
        target_os = "netbsd",
        target_os = "openbsd",
        target_os = "dragonfly"
    ))]
    attr_fn_string!(with_app_id, app_id);
}

impl Default for WindowAttributes {
    fn default() -> Self {
        Self {
            resizable: true,
            title: "MTK".to_string(),
            size: WindowDimension::new(800, 600),
            min_size: None,
            max_size: None,

            transparent: true,
            blur: false,
            decorations: false,

            #[cfg(any(
                target_os = "linux",
                target_os = "freebsd",
                target_os = "netbsd",
                target_os = "openbsd",
                target_os = "dragonfly"
            ))]
            app_id: "".to_string(),
        }
    }
}

impl<'r, S, V> Window<'r, S, V>
where
    V: View<S>,
    V::Message: 'static + Send,
{
    pub fn with<U, F>(state: S, update_fn: U, mut view_fn: F) -> Self
    where
        U: FnMut(&mut S, V::Message) + 'static,
        F: FnMut(&S) -> V + 'static,
    {
        let (msg_tx, msg_rx) = channel();
        let event_proxy = Arc::new(Mutex::new(None));

        let mut ctx = Context::new();

        let view = view_fn(&state);
        let element = view.build(&mut ctx);

        let root_node = view.get_node(&element);
        ctx.root_attach(root_node);

        ctx.set_text_sizing_func(move |ctx, _node, text, userdata, avail_w, avail_h| {
            let default_style = TextStyle::default();
            let style = if let Some(info) =
                userdata.and_then(|u| u.downcast_ref::<crate::TextRenderInfo>())
            {
                &info.style
            } else if let Some(style) = userdata.and_then(|u| u.downcast_ref::<TextStyle>()) {
                style
            } else {
                &default_style
            };

            let text_ctx = ctx.text_context.clone();
            crate::text::measure_text(text, style, avail_w, avail_h, &text_ctx)
        });

        Self {
            msg_tx,
            msg_rx,
            event_proxy,
            renderer: None,
            window: None,
            context: ctx,
            state,
            app_view_fn: Some(Box::new(view_fn)),
            update_fn: Some(Box::new(update_fn)),
            view: Some(view),
            attr: WindowAttributes::default(),
            element: Some(element),
            cursor_pos: (0.0, 0.0),
            last_frame_time: Instant::now(),
            scroll_targets: HashMap::new(),
            scroll_targets_x: HashMap::new(),
            drag_scroll_node: None,
            drag_scroll_x_node: None,
            debug_tx: None,
            debug_rx_cmd: None,
            hovered_node: None,
        }
    }

    /// Returns a thread-safe [`WindowHandle`] that can be cloned and moved to background threads to dispatch messages.
    pub fn handle(&self) -> WindowHandle<V::Message> {
        WindowHandle {
            tx: self.msg_tx.clone(),
            proxy: self.event_proxy.clone(),
        }
    }

    /// Spawns the interactive Ratatui TUI Layout Debugger in the terminal.
    pub fn enable_terminal_debugger(&mut self) -> &mut Self {
        let (tx_event, rx_cmd) = crate::debugger::TerminalDebugger::spawn();
        self.debug_tx = Some(tx_event);
        self.debug_rx_cmd = Some(rx_cmd);
        self
    }

    /// Registers raw font bytes (e.g. from `include_bytes!(...)`) into the window's text context.
    pub fn with_font_bytes(mut self, data: &[u8]) -> Self {
        self.context.register_fonts(data.to_vec());
        self
    }

    /// Loads and registers a font file from disk into the window's text context.
    pub fn with_font_file(mut self, path: impl AsRef<std::path::Path>) -> std::io::Result<Self> {
        self.context.register_font_file(path)?;
        Ok(self)
    }

    pub fn present(&mut self) {
        let event_loop = EventLoop::new().unwrap();
        if let Ok(mut guard) = self.event_proxy.lock() {
            *guard = Some(event_loop.create_proxy());
        }
        if self.debug_tx.is_some() {
            event_loop.set_control_flow(ControlFlow::wait_duration(
                std::time::Duration::from_millis(16),
            ));
        } else {
            event_loop.set_control_flow(ControlFlow::Wait);
        }
        event_loop.run_app(self).unwrap();
    }

    pub fn present_with(&mut self, attr: WindowAttributes) {
        self.attr = attr;
        self.present();
    }

    fn dispatch_and_rebuild(&mut self, mtk_event: Event) {
        if let (Some(view), Some(element), Some(app_view_fn), Some(update_fn)) = (
            &self.view,
            &mut self.element,
            &mut self.app_view_fn,
            &mut self.update_fn,
        ) {
            let is_tick = matches!(mtk_event, Event::Tick { .. });

            // Sync scroll targets when widgets update constraints.scroll directly
            if !matches!(mtk_event, Event::Tick { .. } | Event::MouseWheel { .. }) {
                let targets_y: Vec<Node> = self.scroll_targets.keys().copied().collect();
                for node in targets_y {
                    if let Some(c) = node.get_constraints(&self.context) {
                        self.scroll_targets.insert(node, c.scroll.y);
                    }
                }
                let targets_x: Vec<Node> = self.scroll_targets_x.keys().copied().collect();
                for node in targets_x {
                    if let Some(c) = node.get_constraints(&self.context) {
                        self.scroll_targets_x.insert(node, c.scroll.x);
                    }
                }
            }

            // A) Tick kinetic lerping for ultra-smooth 120Hz/60Hz scrolling
            if let Event::Tick { dt } = mtk_event {
                let mut lerp_animating = false;
                let keys: Vec<Node> = self.scroll_targets.keys().copied().collect();
                for node in keys {
                    if self.drag_scroll_node.map(|(n, ..)| n) == Some(node) {
                        continue;
                    }
                    if let Some(target_scroll_y) = self.scroll_targets.get(&node).copied() {
                        let constraints = node.get_constraints(&self.context).unwrap_or_default();
                        let (computed_h, content_h) =
                            if let Some(computed) = node.get_computed(&self.context) {
                                (computed.h, node.compute_content_height(&self.context))
                            } else {
                                (0.0, 0.0)
                            };
                        let max_scroll_y = (content_h - computed_h).max(0.0);
                        let clamped_target = target_scroll_y.clamp(0.0, max_scroll_y);

                        let diff = clamped_target - constraints.scroll.y;
                        if diff.abs() > 0.1 {
                            lerp_animating = true;
                            let factor = 1.0 - (-22.0 * dt).exp();
                            let new_scroll_y = constraints.scroll.y + diff * factor;
                            node.update_constraints(&mut self.context, |c| {
                                c.scroll.y = new_scroll_y;
                            });
                        } else {
                            if (constraints.scroll.y - clamped_target).abs() > 0.001 {
                                node.update_constraints(&mut self.context, |c| {
                                    c.scroll.y = clamped_target;
                                });
                            }
                            self.scroll_targets.insert(node, constraints.scroll.y);
                        }
                    }
                }
                let keys_x: Vec<Node> = self.scroll_targets_x.keys().copied().collect();
                for node in keys_x {
                    if self.drag_scroll_x_node.map(|(n, ..)| n) == Some(node) {
                        continue;
                    }
                    if let Some(target_scroll_x) = self.scroll_targets_x.get(&node).copied() {
                        let constraints = node.get_constraints(&self.context).unwrap_or_default();
                        let (computed_w, content_w) =
                            if let Some(computed) = node.get_computed(&self.context) {
                                (computed.w, computed.content_w.max(computed.w))
                            } else {
                                (0.0, 0.0)
                            };
                        let max_scroll_x = (content_w - computed_w).max(0.0);
                        let clamped_target = target_scroll_x.clamp(0.0, max_scroll_x);

                        let diff = clamped_target - constraints.scroll.x;
                        if diff.abs() > 0.1 {
                            lerp_animating = true;
                            let factor = 1.0 - (-22.0 * dt).exp();
                            let new_scroll_x = constraints.scroll.x + diff * factor;
                            node.update_constraints(&mut self.context, |c| {
                                c.scroll.x = new_scroll_x;
                            });
                        } else {
                            if (constraints.scroll.x - clamped_target).abs() > 0.001 {
                                node.update_constraints(&mut self.context, |c| {
                                    c.scroll.x = clamped_target;
                                });
                            }
                            self.scroll_targets_x.insert(node, constraints.scroll.x);
                        }
                    }
                }
                if lerp_animating {
                    if let Some(window) = &self.window {
                        window.request_redraw();
                    }
                }
            }

            // B) Scrollbar thumb drag start
            if let Event::MouseInput {
                pressed,
                x,
                y,
                ref hit_nodes,
            } = mtk_event
            {
                if pressed {
                    for node in hit_nodes.iter().rev() {
                        let constraints = node.get_constraints(&self.context).unwrap_or_default();
                        if constraints.overflow == crate::Overflow::Scroll
                            || constraints.overflow == crate::Overflow::Auto
                        {
                            if let Some(computed) = node.get_computed(&self.context) {
                                let content_h = node.compute_content_height(&self.context);
                                let max_scroll_y = (content_h - computed.h).max(0.0);
                                if max_scroll_y > 0.0 {
                                    if x >= computed.x + computed.w - 14.0
                                        && x <= computed.x + computed.w
                                    {
                                        self.drag_scroll_node =
                                            Some((*node, y, constraints.scroll.y));
                                        break;
                                    }
                                }

                                let content_w = computed.content_w.max(computed.w);
                                let max_scroll_x = (content_w - computed.w).max(0.0);
                                if max_scroll_x > 0.0 {
                                    if y >= computed.y + computed.h - 14.0
                                        && y <= computed.y + computed.h
                                    {
                                        self.drag_scroll_x_node =
                                            Some((*node, x, constraints.scroll.x));
                                        break;
                                    }
                                }
                            }
                        }
                    }
                } else {
                    self.drag_scroll_node = None;
                    self.drag_scroll_x_node = None;
                }
            }

            // C) Scrollbar thumb drag motion
            if let Event::CursorMoved { x, y, .. } = mtk_event {
                if let Some((node, drag_start_y, drag_start_scroll_y)) = self.drag_scroll_node {
                    if let Some(computed) = node.get_computed(&self.context) {
                        let content_h = node.compute_content_height(&self.context);
                        let max_scroll_y = (content_h - computed.h).max(0.0);
                        if max_scroll_y > 0.0 {
                            let track_h = (computed.h - 8.0).max(0.0);
                            let thumb_h = ((track_h / content_h) * track_h).clamp(24.0, track_h);
                            let track_travel = (track_h - thumb_h).max(1.0);
                            let delta_y = y - drag_start_y;
                            let scroll_delta = (delta_y / track_travel) * max_scroll_y;
                            let new_scroll_y =
                                (drag_start_scroll_y + scroll_delta).clamp(0.0, max_scroll_y);
                            node.update_constraints(&mut self.context, |c| {
                                c.scroll.y = new_scroll_y;
                            });
                            self.scroll_targets.insert(node, new_scroll_y);
                            if let Some(window) = &self.window {
                                window.request_redraw();
                            }
                        }
                    }
                }
                if let Some((node, drag_start_x, drag_start_scroll_x)) = self.drag_scroll_x_node {
                    if let Some(computed) = node.get_computed(&self.context) {
                        let content_w = computed.content_w.max(computed.w);
                        let max_scroll_x = (content_w - computed.w).max(0.0);
                        if max_scroll_x > 0.0 {
                            let track_w = (computed.w - 8.0).max(0.0);
                            let thumb_w = ((track_w / content_w) * track_w).clamp(24.0, track_w);
                            let track_travel = (track_w - thumb_w).max(1.0);
                            let delta_x = x - drag_start_x;
                            let scroll_delta = (delta_x / track_travel) * max_scroll_x;
                            let new_scroll_x =
                                (drag_start_scroll_x + scroll_delta).clamp(0.0, max_scroll_x);
                            node.update_constraints(&mut self.context, |c| {
                                c.scroll.x = new_scroll_x;
                            });
                            self.scroll_targets_x.insert(node, new_scroll_x);
                            if let Some(window) = &self.window {
                                window.request_redraw();
                            }
                        }
                    }
                }
            }

            // Pass 1 - READONLY state down
            let (result, optional_msg) =
                view.handle_event(element, &self.state, mtk_event.clone(), &mut self.context);

            if result == EventResult::Ignored {
                if let Event::KeyboardInput {
                    event: ref k_event, ..
                } = mtk_event
                {
                    if k_event.state.is_pressed()
                        && k_event.logical_key
                            == winit::keyboard::Key::Named(winit::keyboard::NamedKey::Tab)
                    {
                        if self.context.modifiers.shift_key() {
                            self.context.focus_prev();
                        } else {
                            self.context.focus_next();
                        }
                        if let Some(window) = &self.window {
                            window.request_redraw();
                        }
                    }
                }

                if let Event::MouseWheel {
                    delta_x,
                    delta_y,
                    ref hit_nodes,
                    ..
                } = mtk_event
                {
                    for node in hit_nodes.iter().rev() {
                        let constraints = node.get_constraints(&self.context).unwrap_or_default();
                        let is_scrollable_y = constraints.overflow == crate::Overflow::Scroll
                            || constraints.overflow == crate::Overflow::Auto;
                        let is_scrollable_x = constraints.overflow == crate::Overflow::Scroll
                            || constraints.overflow == crate::Overflow::Auto
                            || constraints.overflow == crate::Overflow::Hidden;

                        if is_scrollable_y || is_scrollable_x {
                            if let Some(computed) = node.get_computed(&self.context) {
                                let content_h = node.compute_content_height(&self.context);
                                let max_scroll_y = (content_h - computed.h).max(0.0);

                                let content_w = computed.content_w.max(computed.w);
                                let max_scroll_x = (content_w - computed.w).max(0.0);

                                let mut scrolled = false;

                                if is_scrollable_y && max_scroll_y > 0.0 && delta_y.abs() > 0.0 {
                                    let current_target = self
                                        .scroll_targets
                                        .get(node)
                                        .copied()
                                        .unwrap_or(constraints.scroll.y);
                                    let new_target =
                                        (current_target - delta_y * 1.6).clamp(0.0, max_scroll_y);
                                    self.scroll_targets.insert(*node, new_target);
                                    scrolled = true;
                                }

                                let scroll_delta_x = if delta_x.abs() > 0.0 {
                                    delta_x
                                } else if max_scroll_y == 0.0 || !is_scrollable_y {
                                    delta_y
                                } else {
                                    0.0
                                };

                                if is_scrollable_x
                                    && max_scroll_x > 0.0
                                    && scroll_delta_x.abs() > 0.0
                                {
                                    let current_target = self
                                        .scroll_targets_x
                                        .get(node)
                                        .copied()
                                        .unwrap_or(constraints.scroll.x);
                                    let new_target = (current_target - scroll_delta_x * 1.6)
                                        .clamp(0.0, max_scroll_x);
                                    self.scroll_targets_x.insert(*node, new_target);
                                    scrolled = true;
                                }

                                if scrolled {
                                    if let Some(window) = &self.window {
                                        window.request_redraw();
                                    }
                                    break;
                                }
                            }
                        }
                    }
                }
            }

            let mut state_changed = false;

            // Pass 2 - we check if a logical message bubbled up to the root
            if let Some(msg) = optional_msg {
                update_fn(&mut self.state, msg);
                state_changed = true;
            }

            // Pass 3 - we rebuild only when state has been updated
            if state_changed {
                let new_view = app_view_fn(&self.state);
                new_view.rebuild(view, &mut self.context, element);
                self.view = Some(new_view);
            }

            if let Some(window) = &self.window {
                if state_changed || (!is_tick && result == EventResult::Handled) {
                    window.request_redraw();
                }
            }
        }
    }

    fn process_pending_messages(&mut self) {
        if let (Some(view), Some(element), Some(app_view_fn), Some(update_fn)) = (
            &self.view,
            &mut self.element,
            &mut self.app_view_fn,
            &mut self.update_fn,
        ) {
            let mut state_changed = false;
            while let Ok(msg) = self.msg_rx.try_recv() {
                update_fn(&mut self.state, msg);
                state_changed = true;
            }

            if state_changed {
                let new_view = app_view_fn(&self.state);
                new_view.rebuild(view, &mut self.context, element);
                self.view = Some(new_view);

                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
        }
    }
}

impl<'r, S, V> ApplicationHandler for Window<'r, S, V>
where
    V: View<S>,
    V::Message: 'static + Send,
{
    fn user_event(&mut self, _event_loop: &ActiveEventLoop, _event: ()) {
        self.process_pending_messages();
    }

    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let attr = self.attr.clone();
        let mut window_attributes = WWindow::default_attributes()
            .with_title(attr.title)
            .with_decorations(attr.decorations)
            .with_transparent(attr.transparent)
            .with_blur(attr.blur)
            .with_resizable(attr.resizable)
            .with_inner_size(attr.size);

        #[cfg(any(
            target_os = "linux",
            target_os = "freebsd",
            target_os = "netbsd",
            target_os = "openbsd",
            target_os = "dragonfly"
        ))]
        {
            use winit::platform::wayland::WindowAttributesExtWayland;
            window_attributes = window_attributes.with_name(attr.app_id.clone(), "");
        }

        if let Some(min_size) = attr.min_size {
            window_attributes = window_attributes.with_min_inner_size(min_size);
        }

        if let Some(max_size) = attr.max_size {
            window_attributes = window_attributes.with_max_inner_size(max_size);
        }

        self.context
            .compute_layout(attr.size.width as f32, attr.size.height as f32);

        let window = Arc::new(event_loop.create_window(window_attributes).unwrap());
        window.set_ime_allowed(true);

        self.window = Some(window.clone());
        self.context.window = Some(window.clone());

        let renderer = pollster::block_on(Renderer::new(
            event_loop.owned_display_handle(),
            window.clone(),
        ));
        self.renderer = Some(renderer);
        window.request_redraw();
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        self.process_pending_messages();
        if self.debug_tx.is_some() {
            if let Some(window) = &self.window {
                window.request_redraw();
            }
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        let window = self.window.as_ref().unwrap().clone();
        if id != window.id() {
            return;
        }

        match event {
            WindowEvent::CloseRequested => {
                // TODO: add a before_close hook that may
                // decide if we close the window or not
                if let Some(tx) = &self.debug_tx {
                    let _ = tx.send(crate::debugger::DebugEvent::Closed);
                }
                event_loop.exit();
            }
            WindowEvent::ModifiersChanged(modifiers) => {
                self.context.modifiers = modifiers.state();
            }
            WindowEvent::KeyboardInput {
                device_id: _,
                event,
                is_synthetic,
            } => {
                let mtk_event = Event::KeyboardInput {
                    event,
                    is_synthetic,
                };
                self.dispatch_and_rebuild(mtk_event);
            }
            WindowEvent::Ime(ime) => {
                let mtk_event = Event::Ime(ime);
                self.dispatch_and_rebuild(mtk_event);
            }
            WindowEvent::Resized(size) => {
                if let Some(renderer) = &mut self.renderer {
                    renderer.resize(size);
                }

                if let (Some(view), Some(element)) = (&self.view, &self.element) {
                    let root = view.get_node(element);
                    root.set_dirty(&mut self.context);
                }

                self.dispatch_and_rebuild(Event::WindowResized(WindowDimension {
                    width: size.width,
                    height: size.height,
                }));

                window.request_redraw();
            }
            WindowEvent::CursorMoved { position, .. } => {
                let x = position.x as f32;
                let y = position.y as f32;
                self.cursor_pos = (x, y);
                let hit_nodes = self.context.pick(x, y);

                let hit_top = hit_nodes.first().copied();
                if self.hovered_node != hit_top {
                    self.hovered_node = hit_top;
                    if let Some(tx) = &self.debug_tx {
                        let _ = tx.send(crate::debugger::DebugEvent::HoveredNode(
                            hit_top.map(|n| n.id()),
                        ));
                    }
                }

                let mtk_event = Event::CursorMoved { x, y, hit_nodes };
                self.dispatch_and_rebuild(mtk_event);
            }
            WindowEvent::MouseInput { state, .. } => {
                let pressed = state == winit::event::ElementState::Pressed;
                let hit_nodes = self.context.pick(self.cursor_pos.0, self.cursor_pos.1);
                let mtk_event = Event::MouseInput {
                    pressed,
                    x: self.cursor_pos.0,
                    y: self.cursor_pos.1,
                    hit_nodes,
                };
                self.dispatch_and_rebuild(mtk_event);
            }
            WindowEvent::MouseWheel { delta, phase, .. } => {
                let (dx, dy, is_touchpad) = match delta {
                    MouseScrollDelta::LineDelta(x, y) => (x * 20.0, y * 20.0, false),
                    MouseScrollDelta::PixelDelta(pos) => (pos.x as f32, pos.y as f32, true),
                };
                let hit_nodes = self.context.pick(self.cursor_pos.0, self.cursor_pos.1);
                let mtk_event = Event::MouseWheel {
                    delta_x: dx,
                    delta_y: dy,
                    is_touchpad,
                    phase,
                    hit_nodes,
                };
                self.dispatch_and_rebuild(mtk_event);
            }
            WindowEvent::RedrawRequested => {
                // Process incoming commands from TUI debugger
                if let Some(rx_cmd) = &self.debug_rx_cmd {
                    while let Ok(cmd) = rx_cmd.try_recv() {
                        match cmd {
                            crate::debugger::DebugCommand::HighlightNode(maybe_id) => {
                                self.context.highlight_node = maybe_id.map(|id| {
                                    Node(crate::sys::muNode {
                                        numeral: id as usize,
                                        generation: 0,
                                    })
                                });
                            }
                            crate::debugger::DebugCommand::RequestSnapshot => {
                                let size = window.inner_size();
                                self.context
                                    .compute_layout(size.width as f32, size.height as f32);
                                if let Some(tx) = &self.debug_tx {
                                    let snapshot = self.context.build_debug_snapshot(
                                        size.width as f32,
                                        size.height as f32,
                                        self.hovered_node,
                                    );
                                    let _ = tx.send(crate::debugger::DebugEvent::LayoutUpdated(
                                        Box::new(snapshot),
                                    ));
                                }
                            }
                        }
                    }
                }

                let now = Instant::now();
                let dt = now.duration_since(self.last_frame_time).as_secs_f32();
                self.last_frame_time = now;
                self.context.dt = dt;
                self.dispatch_and_rebuild(Event::Tick { dt });

                let size = window.inner_size();
                self.context
                    .compute_layout(size.width as f32, size.height as f32);

                if let Some(tx) = &self.debug_tx {
                    let snapshot = self.context.build_debug_snapshot(
                        size.width as f32,
                        size.height as f32,
                        self.hovered_node,
                    );
                    let _ = tx.send(crate::debugger::DebugEvent::LayoutUpdated(Box::new(
                        snapshot,
                    )));
                }

                let viewport = crate::style::Rect {
                    x: 0.0,
                    y: 0.0,
                    w: size.width as f32,
                    h: size.height as f32,
                };
                self.context.build_render_list(viewport);

                if let Some(renderer) = &mut self.renderer {
                    let focused_caret = renderer.render(&self.context);
                    if let Some(window) = &self.window {
                        if let Some(caret) = focused_caret {
                            let position = PhysicalPosition::new(caret[0] as u32, caret[1] as u32);
                            let size = PhysicalSize::new(caret[2] as u32, caret[3] as u32);
                            window.set_ime_cursor_area(position, size);
                        }
                    }
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::widgets::text;

    #[test]
    fn test_window_handle_send_across_threads() {
        let mut window = Window::with(
            0,
            |state: &mut i32, msg: i32| *state += msg,
            |state: &i32| text(format!("{state}")),
        );

        let handle = window.handle();
        let handle_clone = handle.clone();

        let thread = std::thread::spawn(move || {
            let res = handle_clone.send(42);
            assert!(res.is_ok());
        });

        thread.join().unwrap();

        assert_eq!(window.state, 0);
        window.process_pending_messages();
        assert_eq!(window.state, 42);
    }
}
