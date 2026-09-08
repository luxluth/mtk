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
    command::{Command, IntoCommand},
    style::{Rect, ScrollbarVisibility},
    ui::{Event, ThumbScrollContext, View, event::EventResult},
    windowing::renderer::Renderer,
};

use std::sync::Mutex;
use std::sync::mpsc::{Receiver, Sender, channel};
use winit::event_loop::EventLoopProxy;

/// A thread-safe, cloneable handle to dispatch messages to the UI event loop from background threads.
pub struct WindowHandle<Msg: 'static + Send> {
    tx: Sender<Msg>,
    proxy: Arc<Mutex<Option<EventLoopProxy<()>>>>,
}

impl<Msg: 'static + Send> Clone for WindowHandle<Msg> {
    fn clone(&self) -> Self {
        Self {
            tx: self.tx.clone(),
            proxy: self.proxy.clone(),
        }
    }
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

#[derive(Clone, Debug)]
struct TouchScrollState {
    tracker: crate::ui::KineticTracker,
    accum_x: f32,
    accum_y: f32,
    last_move_time: Instant,
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
    update_fn: Option<Box<dyn FnMut(&mut S, V::Message) -> Command<V::Message>>>,

    view: Option<V>,
    element: Option<V::Element>,
    attr: WindowAttributes,
    cursor_pos: (f32, f32),
    last_frame_time: Instant,
    scroll_trackers: HashMap<Node, crate::ui::KineticTracker>,
    touch_scroll_states: HashMap<Node, TouchScrollState>,
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

impl From<WindowDimension> for winit::dpi::Size {
    fn from(value: WindowDimension) -> Self {
        winit::dpi::Size::Physical(PhysicalSize {
            width: value.width,
            height: value.height,
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
    pub fn with<U, F, C>(state: S, mut update_fn: U, mut view_fn: F) -> Self
    where
        U: FnMut(&mut S, V::Message) -> C + 'static,
        C: IntoCommand<V::Message>,
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
            let (style, spans) = if let Some(info) =
                userdata.and_then(|u| u.downcast_ref::<crate::TextRenderInfo>())
            {
                (&info.style, &info.spans[..])
            } else if let Some(style) = userdata.and_then(|u| u.downcast_ref::<TextStyle>()) {
                (style, &[][..])
            } else {
                (&default_style, &[][..])
            };

            let text_ctx = ctx.text_context.clone();
            crate::text::measure_text(text, style, avail_w, avail_h, &text_ctx, spans)
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
            update_fn: Some(Box::new(move |s, msg| update_fn(s, msg).into_command())),
            view: Some(view),
            attr: WindowAttributes::default(),
            element: Some(element),
            cursor_pos: (0.0, 0.0),
            last_frame_time: Instant::now(),
            scroll_trackers: HashMap::new(),
            touch_scroll_states: HashMap::new(),
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

    /// Executes a [`Command`] synchronously and/or asynchronously against this window.
    pub fn execute_command(&mut self, cmd: Command<V::Message>) {
        Self::execute_command_inner(&mut self.context, &self.msg_tx, &self.event_proxy, cmd);
    }

    fn execute_command_inner(
        context: &mut Context,
        msg_tx: &Sender<V::Message>,
        event_proxy: &Arc<Mutex<Option<EventLoopProxy<()>>>>,
        cmd: Command<V::Message>,
    ) {
        if cmd.is_empty() {
            return;
        }
        let (sync_actions, async_actions) = cmd.into_actions();
        for action in sync_actions {
            if let Some(msg) = action(context) {
                let _ = msg_tx.send(msg);
            }
        }
        if !async_actions.is_empty() {
            let handle = WindowHandle {
                tx: msg_tx.clone(),
                proxy: event_proxy.clone(),
            };
            for action in async_actions {
                action(handle.clone());
            }
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
            let mut state_changed = false;

            // Focus blur on mouse press:
            // "We always blur except you click on the same focused element"
            let mut focus_lost_event: Option<Event> = None;
            if let Event::MouseInput {
                pressed: true,
                ref hit_nodes,
                ..
            } = mtk_event
            {
                if let Some(prev_focused) = self.context.check_click_focus_blur(hit_nodes) {
                    focus_lost_event = Some(Event::FocusLost { node: prev_focused });
                }
            }

            // A) Tick kinetic velocity simulation for physical momentum scrolling
            if let Event::Tick { dt } = mtk_event {
                let mut is_animating = false;
                let nodes: Vec<Node> = self.scroll_trackers.keys().copied().collect();
                for node in nodes {
                    if self.drag_scroll_node.map(|(n, ..)| n) == Some(node)
                        || self.drag_scroll_x_node.map(|(n, ..)| n) == Some(node)
                    {
                        continue;
                    }
                    if let Some(tracker) = self.scroll_trackers.get_mut(&node) {
                        let constraints = node.get_constraints(&self.context).unwrap_or_default();
                        let (computed_w, computed_h, content_w, content_h) =
                            if let Some(computed) = node.get_computed(&self.context) {
                                (
                                    computed.w,
                                    computed.h,
                                    computed.content_w.max(computed.w),
                                    node.compute_content_height(&self.context),
                                )
                            } else {
                                (0.0, 0.0, 0.0, 0.0)
                            };
                        let max_scroll_y = (content_h - computed_h).max(0.0);
                        let max_scroll_x = (content_w - computed_w).max(0.0);

                        if let Some((dx, dy)) = tracker.update(dt) {
                            let next_scroll_y =
                                (constraints.scroll.y + dy).clamp(0.0, max_scroll_y);
                            let next_scroll_x =
                                (constraints.scroll.x + dx).clamp(0.0, max_scroll_x);

                            if next_scroll_y == 0.0 || next_scroll_y == max_scroll_y {
                                tracker.set_velocity(tracker.velocity().0, 0.0);
                            }
                            if next_scroll_x == 0.0 || next_scroll_x == max_scroll_x {
                                tracker.set_velocity(0.0, tracker.velocity().1);
                            }

                            if (next_scroll_y - constraints.scroll.y).abs() > 0.001
                                || (next_scroll_x - constraints.scroll.x).abs() > 0.001
                            {
                                node.update_constraints(&mut self.context, |c| {
                                    c.scroll.y = next_scroll_y;
                                    c.scroll.x = next_scroll_x;
                                });
                            }

                            if tracker.is_active() {
                                is_animating = true;
                            } else {
                                self.scroll_trackers.remove(&node);
                            }
                        } else {
                            self.scroll_trackers.remove(&node);
                        }
                    }
                }

                if is_animating {
                    if let Some(window) = &self.window {
                        window.request_redraw();
                    }
                }
            }

            let mut thumb_scroll_event: Option<Event> = None;

            // B) Scrollbar thumb drag start
            if let Event::MouseInput {
                pressed,
                x,
                y,
                ref hit_nodes,
                ..
            } = mtk_event
            {
                if pressed {
                    for node in hit_nodes.iter().rev() {
                        let constraints = node.get_constraints(&self.context).unwrap_or_default();
                        let sb_style = node.get_scrollbar_style(&self.context).unwrap_or_default();
                        if (constraints.overflow == crate::Overflow::Scroll
                            || constraints.overflow == crate::Overflow::Auto)
                            && constraints.scrollbar_visible
                            && sb_style.visibility != ScrollbarVisibility::Never
                        {
                            if let Some(computed) = node.get_computed(&self.context) {
                                let content_h = node.compute_content_height(&self.context);
                                let max_scroll_y = (content_h - computed.h).max(0.0);
                                let hit_zone_w = (sb_style.width + sb_style.margin * 2.0).max(12.0);
                                if max_scroll_y > 0.0 {
                                    if x >= computed.x + computed.w - hit_zone_w
                                        && x <= computed.x + computed.w
                                    {
                                        self.drag_scroll_node =
                                            Some((*node, y, constraints.scroll.y));
                                        let padding_top =
                                            constraints.padding.top + constraints.border.top;
                                        let padding_bottom =
                                            constraints.padding.bottom + constraints.border.bottom;
                                        let track_h =
                                            (computed.h - padding_top - padding_bottom).max(0.0);
                                        let ratio = (computed.h / content_h).clamp(0.0, 1.0);
                                        let thumb_h = (track_h * ratio)
                                            .clamp(sb_style.min_thumb_len.min(track_h), track_h);
                                        let scroll_pct =
                                            (constraints.scroll.y / max_scroll_y).clamp(0.0, 1.0);
                                        let thumb_x = computed.x + computed.w
                                            - constraints.border.right
                                            - sb_style.width
                                            - sb_style.margin;
                                        let thumb_y = computed.y
                                            + padding_top
                                            + scroll_pct * (track_h - thumb_h);
                                        let thumb = Rect {
                                            x: thumb_x,
                                            y: thumb_y,
                                            w: sb_style.width,
                                            h: thumb_h,
                                        };
                                        thumb_scroll_event = Some(Event::ThumbScroll {
                                            node: *node,
                                            context: ThumbScrollContext {
                                                thumb,
                                                scroll_pct,
                                                is_dragging: true,
                                            },
                                        });
                                        break;
                                    }
                                }

                                let content_w = computed.content_w.max(computed.w);
                                let max_scroll_x = (content_w - computed.w).max(0.0);
                                let hit_zone_h = (sb_style.width + sb_style.margin * 2.0).max(12.0);
                                if max_scroll_x > 0.0 {
                                    if y >= computed.y + computed.h - hit_zone_h
                                        && y <= computed.y + computed.h
                                    {
                                        self.drag_scroll_x_node =
                                            Some((*node, x, constraints.scroll.x));
                                        let padding_left =
                                            constraints.padding.left + constraints.border.left;
                                        let padding_right =
                                            constraints.padding.right + constraints.border.right;
                                        let track_w =
                                            (computed.w - padding_left - padding_right).max(0.0);
                                        let ratio = (computed.w / content_w).clamp(0.0, 1.0);
                                        let thumb_w = (track_w * ratio)
                                            .clamp(sb_style.min_thumb_len.min(track_w), track_w);
                                        let scroll_pct =
                                            (constraints.scroll.x / max_scroll_x).clamp(0.0, 1.0);
                                        let thumb_x = computed.x
                                            + padding_left
                                            + scroll_pct * (track_w - thumb_w);
                                        let thumb_y = computed.y + computed.h
                                            - constraints.border.bottom
                                            - sb_style.width
                                            - sb_style.margin;
                                        let thumb = Rect {
                                            x: thumb_x,
                                            y: thumb_y,
                                            w: thumb_w,
                                            h: sb_style.width,
                                        };
                                        thumb_scroll_event = Some(Event::ThumbScroll {
                                            node: *node,
                                            context: ThumbScrollContext {
                                                thumb,
                                                scroll_pct,
                                                is_dragging: true,
                                            },
                                        });
                                        break;
                                    }
                                }
                            }
                        }
                    }
                } else {
                    if let Some((node, _, _)) = self.drag_scroll_node.take() {
                        if let Some(computed) = node.get_computed(&self.context) {
                            let constraints =
                                node.get_constraints(&self.context).unwrap_or_default();
                            let content_h = node.compute_content_height(&self.context);
                            let max_scroll_y = (content_h - computed.h).max(0.0);
                            let sb_style =
                                node.get_scrollbar_style(&self.context).unwrap_or_default();
                            let padding_top = constraints.padding.top + constraints.border.top;
                            let padding_bottom =
                                constraints.padding.bottom + constraints.border.bottom;
                            let track_h = (computed.h - padding_top - padding_bottom).max(0.0);
                            let ratio = (computed.h / content_h).clamp(0.0, 1.0);
                            let thumb_h = (track_h * ratio)
                                .clamp(sb_style.min_thumb_len.min(track_h), track_h);
                            let scroll_pct = if max_scroll_y > 0.0 {
                                (constraints.scroll.y / max_scroll_y).clamp(0.0, 1.0)
                            } else {
                                0.0
                            };
                            let thumb_x = computed.x + computed.w
                                - constraints.border.right
                                - sb_style.width
                                - sb_style.margin;
                            let thumb_y =
                                computed.y + padding_top + scroll_pct * (track_h - thumb_h);
                            let thumb = Rect {
                                x: thumb_x,
                                y: thumb_y,
                                w: sb_style.width,
                                h: thumb_h,
                            };
                            thumb_scroll_event = Some(Event::ThumbScroll {
                                node,
                                context: ThumbScrollContext {
                                    thumb,
                                    scroll_pct,
                                    is_dragging: false,
                                },
                            });
                        }
                    }
                    if let Some((node, _, _)) = self.drag_scroll_x_node.take() {
                        if let Some(computed) = node.get_computed(&self.context) {
                            let constraints =
                                node.get_constraints(&self.context).unwrap_or_default();
                            let content_w = computed.content_w.max(computed.w);
                            let max_scroll_x = (content_w - computed.w).max(0.0);
                            let sb_style =
                                node.get_scrollbar_style(&self.context).unwrap_or_default();
                            let padding_left = constraints.padding.left + constraints.border.left;
                            let padding_right =
                                constraints.padding.right + constraints.border.right;
                            let track_w = (computed.w - padding_left - padding_right).max(0.0);
                            let ratio = (computed.w / content_w).clamp(0.0, 1.0);
                            let thumb_w = (track_w * ratio)
                                .clamp(sb_style.min_thumb_len.min(track_w), track_w);
                            let scroll_pct = if max_scroll_x > 0.0 {
                                (constraints.scroll.x / max_scroll_x).clamp(0.0, 1.0)
                            } else {
                                0.0
                            };
                            let thumb_x =
                                computed.x + padding_left + scroll_pct * (track_w - thumb_w);
                            let thumb_y = computed.y + computed.h
                                - constraints.border.bottom
                                - sb_style.width
                                - sb_style.margin;
                            let thumb = Rect {
                                x: thumb_x,
                                y: thumb_y,
                                w: thumb_w,
                                h: sb_style.width,
                            };
                            thumb_scroll_event = Some(Event::ThumbScroll {
                                node,
                                context: ThumbScrollContext {
                                    thumb,
                                    scroll_pct,
                                    is_dragging: false,
                                },
                            });
                        }
                    }
                }
            }

            // C) Scrollbar thumb drag motion
            if let Event::CursorMoved { x, y, .. } = mtk_event {
                if let Some((node, drag_start_y, drag_start_scroll_y)) = self.drag_scroll_node {
                    if let Some(computed) = node.get_computed(&self.context) {
                        let constraints = node.get_constraints(&self.context).unwrap_or_default();
                        let content_h = node.compute_content_height(&self.context);
                        let max_scroll_y = (content_h - computed.h).max(0.0);
                        if max_scroll_y > 0.0 {
                            let sb_style =
                                node.get_scrollbar_style(&self.context).unwrap_or_default();
                            let padding_top = constraints.padding.top + constraints.border.top;
                            let padding_bottom =
                                constraints.padding.bottom + constraints.border.bottom;
                            let track_h = (computed.h - padding_top - padding_bottom).max(0.0);
                            let ratio = (computed.h / content_h).clamp(0.0, 1.0);
                            let thumb_h = (track_h * ratio)
                                .clamp(sb_style.min_thumb_len.min(track_h), track_h);
                            let track_travel = (track_h - thumb_h).max(1.0);
                            let delta_y = y - drag_start_y;
                            let scroll_delta = (delta_y / track_travel) * max_scroll_y;
                            let new_scroll_y =
                                (drag_start_scroll_y + scroll_delta).clamp(0.0, max_scroll_y);
                            node.update_constraints(&mut self.context, |c| {
                                c.scroll.y = new_scroll_y;
                            });
                            self.scroll_trackers.remove(&node);
                            if let Some(window) = &self.window {
                                window.request_redraw();
                            }
                            let scroll_pct = (new_scroll_y / max_scroll_y).clamp(0.0, 1.0);
                            let thumb_x = computed.x + computed.w
                                - constraints.border.right
                                - sb_style.width
                                - sb_style.margin;
                            let thumb_y =
                                computed.y + padding_top + scroll_pct * (track_h - thumb_h);
                            let thumb = Rect {
                                x: thumb_x,
                                y: thumb_y,
                                w: sb_style.width,
                                h: thumb_h,
                            };
                            thumb_scroll_event = Some(Event::ThumbScroll {
                                node,
                                context: ThumbScrollContext {
                                    thumb,
                                    scroll_pct,
                                    is_dragging: true,
                                },
                            });
                        }
                    }
                }
                if let Some((node, drag_start_x, drag_start_scroll_x)) = self.drag_scroll_x_node {
                    if let Some(computed) = node.get_computed(&self.context) {
                        let constraints = node.get_constraints(&self.context).unwrap_or_default();
                        let content_w = computed.content_w.max(computed.w);
                        let max_scroll_x = (content_w - computed.w).max(0.0);
                        if max_scroll_x > 0.0 {
                            let sb_style =
                                node.get_scrollbar_style(&self.context).unwrap_or_default();
                            let padding_left = constraints.padding.left + constraints.border.left;
                            let padding_right =
                                constraints.padding.right + constraints.border.right;
                            let track_w = (computed.w - padding_left - padding_right).max(0.0);
                            let ratio = (computed.w / content_w).clamp(0.0, 1.0);
                            let thumb_w = (track_w * ratio)
                                .clamp(sb_style.min_thumb_len.min(track_w), track_w);
                            let track_travel = (track_w - thumb_w).max(1.0);
                            let delta_x = x - drag_start_x;
                            let scroll_delta = (delta_x / track_travel) * max_scroll_x;
                            let new_scroll_x =
                                (drag_start_scroll_x + scroll_delta).clamp(0.0, max_scroll_x);
                            node.update_constraints(&mut self.context, |c| {
                                c.scroll.x = new_scroll_x;
                            });
                            self.scroll_trackers.remove(&node);
                            if let Some(window) = &self.window {
                                window.request_redraw();
                            }
                            let scroll_pct = (new_scroll_x / max_scroll_x).clamp(0.0, 1.0);
                            let thumb_x =
                                computed.x + padding_left + scroll_pct * (track_w - thumb_w);
                            let thumb_y = computed.y + computed.h
                                - constraints.border.bottom
                                - sb_style.width
                                - sb_style.margin;
                            let thumb = Rect {
                                x: thumb_x,
                                y: thumb_y,
                                w: thumb_w,
                                h: sb_style.width,
                            };
                            thumb_scroll_event = Some(Event::ThumbScroll {
                                node,
                                context: ThumbScrollContext {
                                    thumb,
                                    scroll_pct,
                                    is_dragging: true,
                                },
                            });
                        }
                    }
                }
            }

            if let Some(ref focus_ev) = focus_lost_event {
                let (_focus_res, focus_msg) =
                    view.handle_event(element, &self.state, focus_ev.clone(), &mut self.context);
                if let Some(msg) = focus_msg {
                    let cmd = update_fn(&mut self.state, msg);
                    Self::execute_command_inner(
                        &mut self.context,
                        &self.msg_tx,
                        &self.event_proxy,
                        cmd,
                    );
                    state_changed = true;
                }
            }

            // Pass 1 - READONLY state down
            let (result, mut optional_msg) =
                view.handle_event(element, &self.state, mtk_event.clone(), &mut self.context);

            if let Some(thumb_ev) = thumb_scroll_event {
                let (_thumb_res, thumb_msg) =
                    view.handle_event(element, &self.state, thumb_ev, &mut self.context);
                if thumb_msg.is_some() {
                    optional_msg = thumb_msg;
                }
            }

            if result == EventResult::Ignored {
                if let Event::KeyboardInput {
                    event: ref k_event, ..
                } = mtk_event
                {
                    if k_event.state.is_pressed()
                        && k_event.logical_key
                            == winit::keyboard::Key::Named(winit::keyboard::NamedKey::Tab)
                    {
                        let prev_focus = self.context.focused_node();
                        if self.context.modifiers.shift_key() {
                            self.context.focus_prev();
                        } else {
                            self.context.focus_next();
                        }
                        if let Some(prev) = prev_focus {
                            if self.context.focused_node() != Some(prev) {
                                let (_tab_res, tab_msg) = view.handle_event(
                                    element,
                                    &self.state,
                                    Event::FocusLost { node: prev },
                                    &mut self.context,
                                );
                                if let Some(msg) = tab_msg {
                                    let cmd = update_fn(&mut self.state, msg);
                                    Self::execute_command_inner(
                                        &mut self.context,
                                        &self.msg_tx,
                                        &self.event_proxy,
                                        cmd,
                                    );
                                    state_changed = true;
                                }
                            }
                        }
                        if let Some(window) = &self.window {
                            window.request_redraw();
                        }
                    }
                }

                if let Event::MouseWheel {
                    delta_x,
                    delta_y,
                    is_touchpad,
                    phase,
                    ref hit_nodes,
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

                                if is_touchpad {
                                    use winit::event::TouchPhase;
                                    match phase {
                                        TouchPhase::Started => {
                                            self.scroll_trackers.remove(node);
                                            let mut tracker = crate::ui::KineticTracker::new(4.8);
                                            tracker.on_press(0.0, 0.0);
                                            self.touch_scroll_states.insert(
                                                *node,
                                                TouchScrollState {
                                                    tracker,
                                                    accum_x: 0.0,
                                                    accum_y: 0.0,
                                                    last_move_time: Instant::now(),
                                                },
                                            );
                                        }
                                        TouchPhase::Moved => {
                                            if delta_x.abs() < 0.001 && delta_y.abs() < 0.001 {
                                                // Handle zero-delta release (libinput / Wayland axis_stop)
                                                if let Some(mut state) =
                                                    self.touch_scroll_states.remove(node)
                                                {
                                                    let (vx, vy) = state.tracker.on_release();
                                                    if vx.abs() > 30.0 || vy.abs() > 30.0 {
                                                        let mut tracker =
                                                            crate::ui::KineticTracker::new(4.8);
                                                        tracker.set_velocity(vx, vy);
                                                        self.scroll_trackers.insert(*node, tracker);
                                                        scrolled = true;
                                                    }
                                                }
                                            } else {
                                                // Dynamic trackpad acceleration curve:
                                                // Slow precise adjustments remain ~1.0x - 1.1x.
                                                // Fast flick gestures accelerate smoothly up to 3.5x.
                                                let event_dist =
                                                    (delta_x * delta_x + delta_y * delta_y).sqrt();
                                                let accel =
                                                    (1.0 + (event_dist / 12.0)).clamp(1.0, 3.5);
                                                let mut eff_delta_x = delta_x * accel;
                                                let mut eff_delta_y = delta_y * accel;

                                                if is_scrollable_x
                                                    && !is_scrollable_y
                                                    && delta_x.abs() == 0.0
                                                {
                                                    eff_delta_x = eff_delta_y;
                                                    eff_delta_y = 0.0;
                                                }

                                                let mut new_scroll_y = constraints.scroll.y;
                                                let mut new_scroll_x = constraints.scroll.x;

                                                if is_scrollable_y
                                                    && (max_scroll_y > 0.0
                                                        || constraints.scroll.y > max_scroll_y)
                                                {
                                                    new_scroll_y = (constraints.scroll.y
                                                        - eff_delta_y)
                                                        .clamp(0.0, max_scroll_y);
                                                }
                                                if is_scrollable_x
                                                    && (max_scroll_x > 0.0
                                                        || constraints.scroll.x > max_scroll_x)
                                                {
                                                    new_scroll_x = (constraints.scroll.x
                                                        - eff_delta_x)
                                                        .clamp(0.0, max_scroll_x);
                                                }

                                                if new_scroll_y != constraints.scroll.y
                                                    || new_scroll_x != constraints.scroll.x
                                                {
                                                    node.update_constraints(
                                                        &mut self.context,
                                                        |c| {
                                                            c.scroll.y = new_scroll_y;
                                                            c.scroll.x = new_scroll_x;
                                                        },
                                                    );
                                                    scrolled = true;
                                                }

                                                let state = self
                                                    .touch_scroll_states
                                                    .entry(*node)
                                                    .or_insert_with(|| {
                                                        self.scroll_trackers.remove(node);
                                                        let mut tracker =
                                                            crate::ui::KineticTracker::new(4.8);
                                                        tracker.on_press(0.0, 0.0);
                                                        TouchScrollState {
                                                            tracker,
                                                            accum_x: 0.0,
                                                            accum_y: 0.0,
                                                            last_move_time: Instant::now(),
                                                        }
                                                    });

                                                state.accum_x -= eff_delta_x;
                                                state.accum_y -= eff_delta_y;
                                                state.tracker.on_move(state.accum_x, state.accum_y);
                                                state.last_move_time = Instant::now();
                                            }
                                        }
                                        TouchPhase::Ended => {
                                            if let Some(mut state) =
                                                self.touch_scroll_states.remove(node)
                                            {
                                                let (vx, vy) = state.tracker.on_release();
                                                if vx.abs() > 30.0 || vy.abs() > 30.0 {
                                                    let mut tracker =
                                                        crate::ui::KineticTracker::new(4.8);
                                                    tracker.set_velocity(vx, vy);
                                                    self.scroll_trackers.insert(*node, tracker);
                                                    scrolled = true;
                                                }
                                            }
                                        }
                                        TouchPhase::Cancelled => {
                                            self.touch_scroll_states.remove(node);
                                            self.scroll_trackers.remove(node);
                                        }
                                    }
                                } else {
                                    let (cur_vx, cur_vy) = self
                                        .scroll_trackers
                                        .get(node)
                                        .map(|t| t.velocity())
                                        .unwrap_or((0.0, 0.0));

                                    let mut new_vy = cur_vy;
                                    let mut new_vx = cur_vx;

                                    if is_scrollable_y
                                        && (max_scroll_y > 0.0
                                            || constraints.scroll.y > max_scroll_y)
                                        && delta_y.abs() > 0.0
                                    {
                                        let impulse_y = -delta_y * 45.0;
                                        new_vy = if (cur_vy > 0.0 && impulse_y > 0.0)
                                            || (cur_vy < 0.0 && impulse_y < 0.0)
                                        {
                                            (cur_vy * 0.7 + impulse_y).clamp(-15000.0, 15000.0)
                                        } else {
                                            impulse_y
                                        };
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
                                        && (max_scroll_x > 0.0
                                            || constraints.scroll.x > max_scroll_x)
                                        && scroll_delta_x.abs() > 0.0
                                    {
                                        let impulse_x = -scroll_delta_x * 45.0;
                                        new_vx = if (cur_vx > 0.0 && impulse_x > 0.0)
                                            || (cur_vx < 0.0 && impulse_x < 0.0)
                                        {
                                            (cur_vx * 0.7 + impulse_x).clamp(-15000.0, 15000.0)
                                        } else {
                                            impulse_x
                                        };
                                        scrolled = true;
                                    }

                                    if scrolled {
                                        let mut tracker = self
                                            .scroll_trackers
                                            .remove(node)
                                            .unwrap_or_else(|| crate::ui::KineticTracker::new(4.8));
                                        tracker.set_velocity(new_vx, new_vy);
                                        self.scroll_trackers.insert(*node, tracker);
                                    }
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

            // Pass 2 - we check if a logical message bubbled up to the root
            if let Some(msg) = optional_msg {
                let cmd = update_fn(&mut self.state, msg);
                Self::execute_command_inner(
                    &mut self.context,
                    &self.msg_tx,
                    &self.event_proxy,
                    cmd,
                );
                state_changed = true;
            }

            // Drain any pending or follow-up messages
            while let Ok(msg) = self.msg_rx.try_recv() {
                let cmd = update_fn(&mut self.state, msg);
                Self::execute_command_inner(
                    &mut self.context,
                    &self.msg_tx,
                    &self.event_proxy,
                    cmd,
                );
                state_changed = true;
            }

            // Pass 3 - we rebuild only when state has been updated
            if state_changed {
                let new_view = app_view_fn(&self.state);
                new_view.rebuild(view, &mut self.context, element);
                self.view = Some(new_view);
            }

            if let Some(window) = &self.window {
                if state_changed
                    || (!is_tick && result == EventResult::Handled)
                    || focus_lost_event.is_some()
                {
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
                let cmd = update_fn(&mut self.state, msg);
                Self::execute_command_inner(
                    &mut self.context,
                    &self.msg_tx,
                    &self.event_proxy,
                    cmd,
                );
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

        let window = Arc::new(event_loop.create_window(window_attributes).unwrap());
        window.set_ime_allowed(true);

        let scale_factor = window.scale_factor() as f32;
        self.context.scale_factor = scale_factor;

        let phys_size = window.inner_size();
        let logical_w = phys_size.width as f32 / scale_factor;
        let logical_h = phys_size.height as f32 / scale_factor;

        self.context.compute_layout(logical_w, logical_h);

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

    fn device_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _device_id: winit::event::DeviceId,
        event: winit::event::DeviceEvent,
    ) {
        if let winit::event::DeviceEvent::MouseMotion { delta } = event {
            if let Some(cap) = &self.context.captured_pointer {
                if cap.policy == crate::CursorGrabPolicy::Locked {
                    let dx = delta.0 as f32;
                    let dy = delta.1 as f32;
                    let hit_nodes = vec![cap.node];
                    let mtk_event = Event::CursorMoved {
                        x: self.cursor_pos.0,
                        y: self.cursor_pos.1,
                        delta_x: dx,
                        delta_y: dy,
                        hit_nodes,
                    };
                    self.dispatch_and_rebuild(mtk_event);
                }
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
                    event: event.into(),
                    is_synthetic,
                };
                self.dispatch_and_rebuild(mtk_event);
            }
            WindowEvent::Ime(ime) => {
                let mtk_event = Event::Ime(ime);
                self.dispatch_and_rebuild(mtk_event);
            }
            WindowEvent::Resized(size) => {
                let scale_factor = window.scale_factor() as f32;
                self.context.scale_factor = scale_factor;

                if let Some(renderer) = &mut self.renderer {
                    renderer.resize(size);
                }

                if let (Some(view), Some(element)) = (&self.view, &self.element) {
                    let root = view.get_node(element);
                    root.set_dirty(&mut self.context);
                }

                let logical_w = (size.width as f32 / scale_factor).round() as u32;
                let logical_h = (size.height as f32 / scale_factor).round() as u32;

                self.dispatch_and_rebuild(Event::WindowResized(WindowDimension {
                    width: logical_w,
                    height: logical_h,
                }));

                window.request_redraw();
            }
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                self.context.scale_factor = scale_factor as f32;
                if let (Some(view), Some(element)) = (&self.view, &self.element) {
                    let root = view.get_node(element);
                    root.set_dirty(&mut self.context);
                }
                window.request_redraw();
            }
            WindowEvent::CursorMoved { position, .. } => {
                let scale_factor = window.scale_factor();
                let x = (position.x / scale_factor) as f32;
                let y = (position.y / scale_factor) as f32;
                let delta_x = x - self.cursor_pos.0;
                let delta_y = y - self.cursor_pos.1;
                self.cursor_pos = (x, y);

                if self
                    .context
                    .captured_pointer
                    .as_ref()
                    .map_or(false, |c| c.policy == crate::CursorGrabPolicy::Locked)
                {
                    return;
                }

                let mut hit_nodes = self.context.pick(x, y).to_vec();
                if let Some(cap) = &self.context.captured_pointer {
                    if !hit_nodes.contains(&cap.node) {
                        hit_nodes.insert(0, cap.node);
                    }
                }

                let hit_top = hit_nodes.first().copied();
                if self.hovered_node != hit_top {
                    self.hovered_node = hit_top;
                    if let Some(tx) = &self.debug_tx {
                        let _ = tx.send(crate::debugger::DebugEvent::HoveredNode(
                            hit_top.map(|n| n.id()),
                        ));
                    }
                }

                let mtk_event = Event::CursorMoved {
                    x,
                    y,
                    delta_x,
                    delta_y,
                    hit_nodes,
                };
                self.dispatch_and_rebuild(mtk_event);
            }
            WindowEvent::MouseInput { state, button, .. } => {
                let pressed = state == winit::event::ElementState::Pressed;
                let mut hit_nodes = self
                    .context
                    .pick(self.cursor_pos.0, self.cursor_pos.1)
                    .to_vec();
                if let Some(cap) = &self.context.captured_pointer {
                    if !hit_nodes.contains(&cap.node) {
                        hit_nodes.insert(0, cap.node);
                    }
                }
                let mtk_event = Event::MouseInput {
                    button,
                    pressed,
                    x: self.cursor_pos.0,
                    y: self.cursor_pos.1,
                    hit_nodes,
                };
                self.dispatch_and_rebuild(mtk_event);
            }
            WindowEvent::MouseWheel { delta, phase, .. } => {
                let scale_factor = window.scale_factor();
                let (dx, dy, is_touchpad) = match delta {
                    MouseScrollDelta::LineDelta(x, y) => (x * 20.0, y * 20.0, false),
                    MouseScrollDelta::PixelDelta(pos) => (
                        (pos.x / scale_factor) as f32,
                        (pos.y / scale_factor) as f32,
                        true,
                    ),
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
                                    Node(crate::layout::NodeId {
                                        index: id as u32,
                                        generation: 0,
                                    })
                                });
                            }
                            crate::debugger::DebugCommand::RequestSnapshot => {
                                let scale_factor = window.scale_factor() as f32;
                                let size = window.inner_size();
                                let logical_w = size.width as f32 / scale_factor;
                                let logical_h = size.height as f32 / scale_factor;
                                self.context.compute_layout(logical_w, logical_h);
                                if let Some(tx) = &self.debug_tx {
                                    let snapshot = self.context.build_debug_snapshot(
                                        logical_w,
                                        logical_h,
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

                let scale_factor = window.scale_factor() as f32;
                self.context.scale_factor = scale_factor;

                let phys_size = window.inner_size();
                let logical_w = phys_size.width as f32 / scale_factor;
                let logical_h = phys_size.height as f32 / scale_factor;

                self.context.compute_layout(logical_w, logical_h);

                if let Some(tx) = &self.debug_tx {
                    let snapshot =
                        self.context
                            .build_debug_snapshot(logical_w, logical_h, self.hovered_node);
                    let _ = tx.send(crate::debugger::DebugEvent::LayoutUpdated(Box::new(
                        snapshot,
                    )));
                }

                let viewport = crate::style::Rect {
                    x: 0.0,
                    y: 0.0,
                    w: logical_w,
                    h: logical_h,
                };
                self.context.build_render_list(viewport);

                if let Some(renderer) = &mut self.renderer {
                    let focused_caret = renderer.render(&self.context);
                    if let Some(window) = &self.window {
                        if let Some(caret) = focused_caret {
                            let position = PhysicalPosition::new(
                                (caret[0] * scale_factor) as u32,
                                (caret[1] * scale_factor) as u32,
                            );
                            let size = PhysicalSize::new(
                                (caret[2] * scale_factor) as u32,
                                (caret[3] * scale_factor) as u32,
                            );
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

    #[test]
    fn test_window_command_sync_perform() {
        #[derive(Debug, PartialEq, Eq)]
        enum TestMsg {
            TriggerSync,
            FollowUp(i32),
        }

        let mut window = Window::with(
            0,
            |state: &mut i32, msg: TestMsg| match msg {
                TestMsg::TriggerSync => Command::perform(|ctx| {
                    ctx.clipboard_copy(crate::ClipboardData::Text("test_val".into()));
                    Some(TestMsg::FollowUp(100))
                }),
                TestMsg::FollowUp(val) => {
                    *state += val;
                    Command::none()
                }
            },
            |state: &i32| text(format!("{state}")),
        );

        let handle = window.handle();
        handle.send(TestMsg::TriggerSync).unwrap();

        window.process_pending_messages();
        assert_eq!(window.state, 100);
        assert_eq!(
            window.context.clipboard_get(),
            Some(crate::ClipboardData::Text("test_val".into()))
        );
    }

    #[test]
    fn test_window_command_async_perform() {
        let mut window = Window::with(
            0,
            |state: &mut i32, msg: i32| {
                if msg == 1 {
                    Some(Command::perform_async(
                        async {
                            std::thread::sleep(std::time::Duration::from_millis(10));
                            777
                        },
                        |val| val,
                    ))
                } else {
                    *state = msg;
                    None
                }
            },
            |state: &i32| text(format!("{state}")),
        );

        let handle = window.handle();
        handle.send(1).unwrap();
        window.process_pending_messages();
        assert_eq!(window.state, 0);

        // Wait for background worker thread to finish
        std::thread::sleep(std::time::Duration::from_millis(50));
        window.process_pending_messages();
        assert_eq!(window.state, 777);
    }
}
