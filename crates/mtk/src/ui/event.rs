//! Event handling wrappers, event consumption states, and view extension traits.
//!
//! This module provides declarative event listener wrappers ([`EventHandler`]), event results
//! ([`EventResult`]), interaction kinds ([`EventKind`]), and the [`ViewEventExt`] extension trait
//! for attaching click, hover, press, and release handlers to views.

use super::{Event, View};
use crate::{Context, Node, style::Rect};
use std::rc::Rc;

/// Categorizes high-level user interaction gesture triggers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum EventKind {
    /// Triggered on mouse button release while the cursor is over the view.
    Click,
    /// Triggered when the mouse cursor enters the view's layout bounds.
    HoverIn,
    /// Triggered when the mouse cursor exits the view's layout bounds.
    HoverOut,
    /// Triggered when a mouse button is pressed down over the view.
    Press,
    /// Triggered when a mouse button is released over the view.
    Release,
    /// Triggered when the user submits input (e.g., pressing Enter in a focused input field).
    Submit,
    /// Triggered when a scrollbar thumb is dragged or scrolled.
    ThumbScroll,
    /// Triggered when the view loses focus (e.g., on outside click or blur).
    FocusLost,
    /// Triggered when the view is scrolled (via mouse wheel, touchpad gesture, scrollbar thumb, or kinetic decay).
    Scroll,
}

/// Indicates whether a view successfully processed or ignored an incoming event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventResult {
    /// The event was processed by the view.
    Handled,
    /// The event was not consumed by the view and may propagate further.
    Ignored,
}

impl EventResult {
    /// Combines two [`EventResult`] values. Returns [`EventResult::Handled`] if either result is handled.
    pub fn or(self, other: EventResult) -> EventResult {
        match (self, other) {
            (EventResult::Handled, _) | (_, EventResult::Handled) => EventResult::Handled,
            _ => EventResult::Ignored,
        }
    }
}

/// A wrapper view that attaches an event listener closure to an inner view.
///
/// Created via [`ViewEventExt::on_event`].
pub struct EventHandler<State, V, F> {
    pub(crate) inner: V,
    pub(crate) kind: EventKind,
    pub(crate) handler: Rc<F>,
    pub(crate) _marker: std::marker::PhantomData<State>,
}

/// Persistent element state for an [`EventHandler`], tracking current hover state and inner element state.
pub struct EventElement<VEl> {
    pub(crate) inner_element: VEl,
    pub(crate) is_hovered: bool,
    pub(crate) is_pressed: bool,
}

impl<State, V: View<State>, F> View<State> for EventHandler<State, V, F>
where
    F: Fn(&State) -> Option<V::Message> + 'static,
{
    type Element = EventElement<V::Element>;
    type Message = V::Message;

    fn build(&self, ctx: &mut Context) -> Self::Element {
        EventElement {
            inner_element: self.inner.build(ctx),
            is_hovered: false,
            is_pressed: false,
        }
    }

    fn rebuild(&self, prev: &Self, ctx: &mut Context, element: &mut Self::Element) {
        self.inner
            .rebuild(&prev.inner, ctx, &mut element.inner_element);
    }

    fn rebuild_with_parent(
        &self,
        prev: &Self,
        ctx: &mut Context,
        element: &mut Self::Element,
        parent: Node,
        next_sibling: Option<Node>,
    ) {
        self.inner.rebuild_with_parent(
            &prev.inner,
            ctx,
            &mut element.inner_element,
            parent,
            next_sibling,
        );
    }

    fn teardown(&self, ctx: &mut Context, element: &mut Self::Element) {
        self.inner.teardown(ctx, &mut element.inner_element);
    }

    fn get_node(&self, element: &Self::Element) -> Node {
        self.inner.get_node(&element.inner_element)
    }

    fn handle_event(
        &self,
        element: &mut Self::Element,
        state: &State,
        event: Event,
        ctx: &mut Context,
    ) -> (EventResult, Option<Self::Message>) {
        let self_node = self.get_node(element);

        // Pre-track is_pressed on mouse-down for this node regardless of whether inner handles it
        if let Event::MouseInput {
            pressed, hit_nodes, ..
        } = &event
        {
            if *pressed && hit_nodes.contains(&self_node) {
                element.is_pressed = true;
            }
        }

        let (inner_res, inner_msg) =
            self.inner
                .handle_event(&mut element.inner_element, state, event.clone(), ctx);

        // If an inner child already produced a message, prioritize child and avoid duplicate parent actions
        if inner_msg.is_some() {
            if let Event::MouseInput { pressed, .. } = &event {
                if !*pressed {
                    element.is_pressed = false;
                }
            }
            return (inner_res, inner_msg);
        }

        // If inner handled the event, check if this handler should still inspect and process it:
        // 1. Submit on KeyboardInput when this node is focused
        // 2. Release / Click on MouseInput release when this node was previously pressed
        // 3. FocusLost when this node was previously focused
        if inner_res == EventResult::Handled {
            let allow_outer_processing = match &event {
                Event::KeyboardInput { .. } => self.kind == EventKind::Submit,
                Event::MouseInput { pressed: false, .. } => {
                    element.is_pressed
                        && (self.kind == EventKind::Release || self.kind == EventKind::Click)
                }
                Event::FocusLost { .. } => self.kind == EventKind::FocusLost,
                _ => false,
            };

            if !allow_outer_processing {
                if let Event::MouseInput { pressed, .. } = &event {
                    if !*pressed {
                        element.is_pressed = false;
                    }
                }
                return (inner_res, inner_msg);
            }
        }

        let mut handled = EventResult::Ignored;
        let mut emitted_msg = None;

        match &event {
            Event::FocusLost { node } => {
                if self.kind == EventKind::FocusLost && *node == self_node {
                    emitted_msg = (self.handler)(state);
                    handled = EventResult::Handled;
                }
            }
            Event::CursorMoved { hit_nodes, .. } => {
                let newly_hovered = hit_nodes.contains(&self_node);

                if newly_hovered != element.is_hovered {
                    element.is_hovered = newly_hovered;
                    if newly_hovered && self.kind == EventKind::HoverIn {
                        emitted_msg = (self.handler)(state);
                        handled = EventResult::Handled;
                    } else if !newly_hovered && self.kind == EventKind::HoverOut {
                        emitted_msg = (self.handler)(state);
                        handled = EventResult::Handled;
                    }
                }
            }
            Event::MouseInput {
                pressed, hit_nodes, ..
            } => {
                let is_hit = hit_nodes.contains(&self_node);
                if *pressed {
                    if is_hit {
                        element.is_pressed = true;
                        if self.kind == EventKind::Press {
                            emitted_msg = (self.handler)(state);
                            handled = EventResult::Handled;
                        }
                    }
                } else if element.is_pressed {
                    element.is_pressed = false;
                    if is_hit {
                        if self.kind == EventKind::Click || self.kind == EventKind::Release {
                            emitted_msg = (self.handler)(state);
                            handled = EventResult::Handled;
                        }
                    } else if self.kind == EventKind::Release {
                        emitted_msg = (self.handler)(state);
                        handled = EventResult::Handled;
                    }
                }
            }
            Event::KeyboardInput {
                event: key_event, ..
            } => {
                if self.kind == EventKind::Submit && key_event.state.is_pressed() {
                    if Some(self_node) == ctx.focused_node() {
                        let is_enter = match key_event.logical_key.as_ref() {
                            winit::keyboard::Key::Named(winit::keyboard::NamedKey::Enter) => true,
                            winit::keyboard::Key::Character(s) => s == "\r" || s == "\n",
                            _ => false,
                        };
                        if is_enter {
                            emitted_msg = (self.handler)(state);
                            handled = EventResult::Handled;
                        }
                    }
                }
            }
            _ => {}
        }

        (handled.or(inner_res), inner_msg.or(emitted_msg))
    }
}

/// Extension trait for [`View`] providing event handling combinators.
pub trait ViewEventExt<State>: View<State> + Sized {
    /// Attaches an event listener closure that runs when `event` occurs on this view.
    ///
    /// # Parameters
    /// - `event`: The [`EventKind`] trigger (e.g. `EventKind::Click`, `EventKind::HoverIn`).
    /// - `handler`: A closure evaluating current application state and optionally returning a message.
    fn on_event<F>(self, event: EventKind, handler: F) -> EventHandler<State, Self, F>
    where
        F: Fn(&State) -> Option<Self::Message> + 'static;

    /// Attaches an automatic pointer-captured drag gesture to this view.
    fn on_drag<F>(self, handler: F) -> DragHandler<State, Self, F>
    where
        F: Fn(&State, DragContext) -> Option<Self::Message> + 'static;

    /// Attaches a relative-motion drag gesture (cursor locked & hidden) for continuous scrubbing.
    fn on_drag_relative<F>(self, handler: F) -> DragHandler<State, Self, F>
    where
        F: Fn(&State, DragContext) -> Option<Self::Message> + 'static;

    /// Attaches a key-down listener fired when this view has keyboard focus.
    fn on_key_down<F>(self, handler: F) -> KeyHandler<State, Self, F>
    where
        F: Fn(&State, KeyEventContext) -> Option<Self::Message> + 'static;

    /// Attaches a key-up listener fired when this view has keyboard focus.
    fn on_key_up<F>(self, handler: F) -> KeyHandler<State, Self, F>
    where
        F: Fn(&State, KeyEventContext) -> Option<Self::Message> + 'static;

    /// Attaches a continuous key-press listener (initial press and auto-repeats) when focused.
    fn on_key_press<F>(self, handler: F) -> KeyHandler<State, Self, F>
    where
        F: Fn(&State, KeyEventContext) -> Option<Self::Message> + 'static;

    /// Attaches a global window-level shortcut listener regardless of focus.
    fn on_global_key_down<F>(self, handler: F) -> KeyHandler<State, Self, F>
    where
        F: Fn(&State, KeyEventContext) -> Option<Self::Message> + 'static;

    /// Attaches a frame tick listener that runs on every render frame tick with elapsed delta time `dt` (seconds).
    fn on_tick<F>(self, handler: F) -> TickHandler<State, Self, F>
    where
        F: Fn(&State, f32) -> Option<Self::Message> + 'static;

    /// Attaches a scrollbar thumb scroll listener fired when the scroll thumb moves or is scrubbed.
    fn on_thumb_scroll<F>(self, handler: F) -> ThumbScrollHandler<State, Self, F>
    where
        F: Fn(&State, ThumbScrollContext) -> Option<Self::Message> + 'static;

    /// Attaches a scroll listener fired when this view is scrolled via mouse wheel, touchpad, thumb drag, or kinetic momentum.
    fn on_scroll<F>(self, handler: F) -> ScrollHandler<State, Self, F>
    where
        F: Fn(&State, ScrollContext) -> Option<Self::Message> + 'static;

    /// Attaches a focus blur listener fired when this view loses keyboard focus.
    fn on_focus_lost<F>(self, handler: F) -> EventHandler<State, Self, F>
    where
        F: Fn(&State) -> Option<Self::Message> + 'static;

    /// Attaches a focus blur listener fired when this view loses keyboard focus (alias for [`on_focus_lost`](Self::on_focus_lost)).
    fn on_blur<F>(self, handler: F) -> EventHandler<State, Self, F>
    where
        F: Fn(&State) -> Option<Self::Message> + 'static;
}

impl<State, V: View<State>> ViewEventExt<State> for V {
    fn on_event<F>(self, event: EventKind, handler: F) -> EventHandler<State, Self, F>
    where
        F: Fn(&State) -> Option<Self::Message> + 'static,
    {
        EventHandler {
            inner: self,
            kind: event,
            handler: Rc::new(handler),
            _marker: std::marker::PhantomData,
        }
    }

    fn on_drag<F>(self, handler: F) -> DragHandler<State, Self, F>
    where
        F: Fn(&State, DragContext) -> Option<Self::Message> + 'static,
    {
        DragHandler {
            inner: self,
            policy: crate::CursorGrabPolicy::Normal,
            handler: Rc::new(handler),
            _marker: std::marker::PhantomData,
        }
    }

    fn on_drag_relative<F>(self, handler: F) -> DragHandler<State, Self, F>
    where
        F: Fn(&State, DragContext) -> Option<Self::Message> + 'static,
    {
        DragHandler {
            inner: self,
            policy: crate::CursorGrabPolicy::Locked,
            handler: Rc::new(handler),
            _marker: std::marker::PhantomData,
        }
    }

    fn on_key_down<F>(self, handler: F) -> KeyHandler<State, Self, F>
    where
        F: Fn(&State, KeyEventContext) -> Option<Self::Message> + 'static,
    {
        KeyHandler {
            inner: self,
            action: KeyActionKind::Down,
            scope: KeyScope::Focused,
            handler: Rc::new(handler),
            _marker: std::marker::PhantomData,
        }
    }

    fn on_key_up<F>(self, handler: F) -> KeyHandler<State, Self, F>
    where
        F: Fn(&State, KeyEventContext) -> Option<Self::Message> + 'static,
    {
        KeyHandler {
            inner: self,
            action: KeyActionKind::Up,
            scope: KeyScope::Focused,
            handler: Rc::new(handler),
            _marker: std::marker::PhantomData,
        }
    }

    fn on_key_press<F>(self, handler: F) -> KeyHandler<State, Self, F>
    where
        F: Fn(&State, KeyEventContext) -> Option<Self::Message> + 'static,
    {
        KeyHandler {
            inner: self,
            action: KeyActionKind::Press,
            scope: KeyScope::Focused,
            handler: Rc::new(handler),
            _marker: std::marker::PhantomData,
        }
    }

    fn on_global_key_down<F>(self, handler: F) -> KeyHandler<State, Self, F>
    where
        F: Fn(&State, KeyEventContext) -> Option<Self::Message> + 'static,
    {
        KeyHandler {
            inner: self,
            action: KeyActionKind::Down,
            scope: KeyScope::Global,
            handler: Rc::new(handler),
            _marker: std::marker::PhantomData,
        }
    }

    fn on_tick<F>(self, handler: F) -> TickHandler<State, Self, F>
    where
        F: Fn(&State, f32) -> Option<Self::Message> + 'static,
    {
        TickHandler {
            inner: self,
            handler: Rc::new(handler),
            _marker: std::marker::PhantomData,
        }
    }

    fn on_thumb_scroll<F>(self, handler: F) -> ThumbScrollHandler<State, Self, F>
    where
        F: Fn(&State, ThumbScrollContext) -> Option<Self::Message> + 'static,
    {
        ThumbScrollHandler {
            inner: self,
            handler: Rc::new(handler),
            _marker: std::marker::PhantomData,
        }
    }

    fn on_scroll<F>(self, handler: F) -> ScrollHandler<State, Self, F>
    where
        F: Fn(&State, ScrollContext) -> Option<Self::Message> + 'static,
    {
        ScrollHandler {
            inner: self,
            handler: Rc::new(handler),
            _marker: std::marker::PhantomData,
        }
    }

    fn on_focus_lost<F>(self, handler: F) -> EventHandler<State, Self, F>
    where
        F: Fn(&State) -> Option<Self::Message> + 'static,
    {
        self.on_event(EventKind::FocusLost, handler)
    }

    fn on_blur<F>(self, handler: F) -> EventHandler<State, Self, F>
    where
        F: Fn(&State) -> Option<Self::Message> + 'static,
    {
        self.on_focus_lost(handler)
    }
}

/// Contextual payload passed to scrollbar thumb scroll event listeners.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ThumbScrollContext {
    /// Bounding rectangle (x, y, w, h) of the scrollbar thumb in logical pixels.
    pub thumb: Rect,
    /// Normalized scroll progress from 0.0 (top/left) to 1.0 (bottom/right).
    pub scroll_pct: f32,
    /// Whether the thumb is currently actively being dragged.
    pub is_dragging: bool,
}

/// A wrapper view that attaches a scrollbar thumb scroll event listener to an inner view.
pub struct ThumbScrollHandler<State, V, F> {
    pub(crate) inner: V,
    pub(crate) handler: Rc<F>,
    pub(crate) _marker: std::marker::PhantomData<State>,
}

impl<State, V: View<State>, F> View<State> for ThumbScrollHandler<State, V, F>
where
    F: Fn(&State, ThumbScrollContext) -> Option<V::Message> + 'static,
{
    type Element = V::Element;
    type Message = V::Message;

    fn build(&self, ctx: &mut Context) -> Self::Element {
        self.inner.build(ctx)
    }

    fn rebuild(&self, prev: &Self, ctx: &mut Context, element: &mut Self::Element) {
        self.inner.rebuild(&prev.inner, ctx, element);
    }

    fn rebuild_with_parent(
        &self,
        prev: &Self,
        ctx: &mut Context,
        element: &mut Self::Element,
        parent: Node,
        next_sibling: Option<Node>,
    ) {
        self.inner
            .rebuild_with_parent(&prev.inner, ctx, element, parent, next_sibling);
    }

    fn teardown(&self, ctx: &mut Context, element: &mut Self::Element) {
        self.inner.teardown(ctx, element);
    }

    fn get_node(&self, element: &Self::Element) -> Node {
        self.inner.get_node(element)
    }

    fn handle_event(
        &self,
        element: &mut Self::Element,
        state: &State,
        event: Event,
        ctx: &mut Context,
    ) -> (EventResult, Option<Self::Message>) {
        let (inner_res, inner_msg) = self.inner.handle_event(element, state, event.clone(), ctx);
        if inner_msg.is_some() {
            return (inner_res, inner_msg);
        }

        let self_node = self.get_node(element);
        if let Event::ThumbScroll { node, context } = event {
            if node == self_node {
                let msg = (self.handler)(state, context);
                return (EventResult::Handled, msg);
            }
        }

        (inner_res, None)
    }
}

/// Describes the input origin that initiated or sustained a scroll action.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ScrollSource {
    /// Generated by mouse wheel interaction.
    Wheel,
    /// Generated by direct touchpad gesture pan/swipe.
    Touchpad,
    /// Generated by dragging or scrubbing a scrollbar thumb.
    Thumb,
    /// Generated by decaying kinetic momentum / fling physics after gesture release.
    Kinetic,
}

/// Contextual layout and motion payload delivered to scroll event listeners.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScrollContext {
    /// Horizontal scroll offset in logical pixels.
    pub scroll_x: f32,
    /// Vertical scroll offset in logical pixels.
    pub scroll_y: f32,
    /// Maximum reachable horizontal scroll offset (`content_w - viewport_w`).
    pub max_scroll_x: f32,
    /// Maximum reachable vertical scroll offset (`content_h - viewport_h`).
    pub max_scroll_y: f32,
    /// Width of the viewport container in logical pixels.
    pub viewport_w: f32,
    /// Height of the viewport container in logical pixels.
    pub viewport_h: f32,
    /// Total width of the scrollable content in logical pixels.
    pub content_w: f32,
    /// Total height of the scrollable content in logical pixels.
    pub content_h: f32,
    /// Normalized horizontal scroll progress from 0.0 (left) to 1.0 (right).
    pub scroll_pct_x: f32,
    /// Normalized vertical scroll progress from 0.0 (top) to 1.0 (bottom).
    pub scroll_pct_y: f32,
    /// Horizontal pixel delta scrolled since the previous scroll event.
    pub delta_x: f32,
    /// Vertical pixel delta scrolled since the previous scroll event.
    pub delta_y: f32,
    /// Origin input mechanism that produced this scroll motion.
    pub source: ScrollSource,
}

impl ScrollContext {
    /// Constructs a `ScrollContext` from the layout and constraint metrics of a node.
    pub fn from_node(
        node: Node,
        context: &Context,
        prev_scroll_x: f32,
        prev_scroll_y: f32,
        source: ScrollSource,
    ) -> Option<Self> {
        let computed = node.get_computed(context)?;
        let constraints = node.get_constraints(context).unwrap_or_default();
        let content_w = computed.content_w.max(computed.w);
        let content_h = node.compute_content_height(context);
        let max_scroll_x = (content_w - computed.w).max(0.0);
        let max_scroll_y = (content_h - computed.h).max(0.0);

        let scroll_x = constraints.scroll.x;
        let scroll_y = constraints.scroll.y;

        let scroll_pct_x = if max_scroll_x > 0.0 {
            (scroll_x / max_scroll_x).clamp(0.0, 1.0)
        } else {
            0.0
        };

        let scroll_pct_y = if max_scroll_y > 0.0 {
            (scroll_y / max_scroll_y).clamp(0.0, 1.0)
        } else {
            0.0
        };

        let delta_x = scroll_x - prev_scroll_x;
        let delta_y = scroll_y - prev_scroll_y;

        Some(Self {
            scroll_x,
            scroll_y,
            max_scroll_x,
            max_scroll_y,
            viewport_w: computed.w,
            viewport_h: computed.h,
            content_w,
            content_h,
            scroll_pct_x,
            scroll_pct_y,
            delta_x,
            delta_y,
            source,
        })
    }

    /// Normalized vertical scroll progress from 0.0 (top) to 1.0 (bottom) (alias for [`scroll_pct_y`](Self::scroll_pct_y)).
    pub fn scroll_pct(&self) -> f32 {
        self.scroll_pct_y
    }

    /// Whether the view is scrolled to the top edge (within 0.001 logical pixel tolerance).
    pub fn is_at_top(&self) -> bool {
        self.scroll_y <= 0.001
    }

    /// Whether the view is scrolled to the bottom edge (within 0.001 logical pixel tolerance).
    pub fn is_at_bottom(&self) -> bool {
        self.scroll_y >= self.max_scroll_y - 0.001
    }

    /// Whether the view is scrolled to the left/start edge (within 0.001 logical pixel tolerance).
    pub fn is_at_start(&self) -> bool {
        self.scroll_x <= 0.001
    }

    /// Whether the view is scrolled to the right/end edge (within 0.001 logical pixel tolerance).
    pub fn is_at_end(&self) -> bool {
        self.scroll_x >= self.max_scroll_x - 0.001
    }
}

/// A wrapper view that attaches a general scroll event listener to an inner view.
pub struct ScrollHandler<State, V, F> {
    pub(crate) inner: V,
    pub(crate) handler: Rc<F>,
    pub(crate) _marker: std::marker::PhantomData<State>,
}

impl<State, V: View<State>, F> View<State> for ScrollHandler<State, V, F>
where
    F: Fn(&State, ScrollContext) -> Option<V::Message> + 'static,
{
    type Element = V::Element;
    type Message = V::Message;

    fn build(&self, ctx: &mut Context) -> Self::Element {
        self.inner.build(ctx)
    }

    fn rebuild(&self, prev: &Self, ctx: &mut Context, element: &mut Self::Element) {
        self.inner.rebuild(&prev.inner, ctx, element);
    }

    fn rebuild_with_parent(
        &self,
        prev: &Self,
        ctx: &mut Context,
        element: &mut Self::Element,
        parent: Node,
        next_sibling: Option<Node>,
    ) {
        self.inner
            .rebuild_with_parent(&prev.inner, ctx, element, parent, next_sibling);
    }

    fn teardown(&self, ctx: &mut Context, element: &mut Self::Element) {
        self.inner.teardown(ctx, element);
    }

    fn get_node(&self, element: &Self::Element) -> Node {
        self.inner.get_node(element)
    }

    fn handle_event(
        &self,
        element: &mut Self::Element,
        state: &State,
        event: Event,
        ctx: &mut Context,
    ) -> (EventResult, Option<Self::Message>) {
        let (inner_res, inner_msg) = self.inner.handle_event(element, state, event.clone(), ctx);
        if inner_msg.is_some() {
            return (inner_res, inner_msg);
        }

        let self_node = self.get_node(element);
        if let Event::Scroll { node, context } = event {
            if node == self_node {
                let msg = (self.handler)(state, context);
                return (EventResult::Handled, msg);
            }
        }

        (inner_res, None)
    }
}

/// A wrapper view that attaches a frame tick listener to an inner view.
pub struct TickHandler<State, V, F> {
    pub(crate) inner: V,
    pub(crate) handler: Rc<F>,
    pub(crate) _marker: std::marker::PhantomData<State>,
}

impl<State, V: View<State>, F> View<State> for TickHandler<State, V, F>
where
    F: Fn(&State, f32) -> Option<V::Message> + 'static,
{
    type Element = V::Element;
    type Message = V::Message;

    fn build(&self, ctx: &mut Context) -> Self::Element {
        self.inner.build(ctx)
    }

    fn rebuild(&self, prev: &Self, ctx: &mut Context, element: &mut Self::Element) {
        self.inner.rebuild(&prev.inner, ctx, element);
    }

    fn rebuild_with_parent(
        &self,
        prev: &Self,
        ctx: &mut Context,
        element: &mut Self::Element,
        parent: Node,
        next_sibling: Option<Node>,
    ) {
        self.inner
            .rebuild_with_parent(&prev.inner, ctx, element, parent, next_sibling);
    }

    fn teardown(&self, ctx: &mut Context, element: &mut Self::Element) {
        self.inner.teardown(ctx, element);
    }

    fn get_node(&self, element: &Self::Element) -> Node {
        self.inner.get_node(element)
    }

    fn handle_event(
        &self,
        element: &mut Self::Element,
        state: &State,
        event: Event,
        ctx: &mut Context,
    ) -> (EventResult, Option<Self::Message>) {
        let (inner_res, inner_msg) = self.inner.handle_event(element, state, event.clone(), ctx);
        if inner_msg.is_some() {
            return (inner_res, inner_msg);
        }

        if let Event::Tick { dt } = event {
            if let Some(msg) = (self.handler)(state, dt) {
                return (EventResult::Handled, Some(msg));
            }
        }

        (inner_res, None)
    }
}

/// Lifecycle phase of an active drag gesture.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DragPhase {
    /// Initial pointer press down starting the drag gesture.
    Start,
    /// Continuous pointer motion during drag.
    Move,
    /// Pointer button released terminating the drag gesture.
    End,
}

/// Contextual payload passed to drag event listeners.
#[derive(Clone, Debug)]
pub struct DragContext {
    /// The current lifecycle phase of the drag gesture.
    pub phase: DragPhase,
    /// The initial pointer position where the drag started.
    pub start_pos: (f32, f32),
    /// The current pointer position.
    pub current_pos: (f32, f32),
    /// Incremental delta (dx, dy) moved since the previous frame.
    pub delta: (f32, f32),
    /// Cumulative delta (dx, dy) moved since the drag started.
    pub total_delta: (f32, f32),
    /// Keyboard modifier keys active during this drag interaction.
    pub modifiers: winit::keyboard::ModifiersState,
}

/// A wrapper view that attaches a pointer-captured drag gesture to an inner view.
pub struct DragHandler<State, V, F> {
    pub(crate) inner: V,
    pub(crate) policy: crate::CursorGrabPolicy,
    pub(crate) handler: Rc<F>,
    pub(crate) _marker: std::marker::PhantomData<State>,
}

/// Persistent element state for [`DragHandler`].
pub struct DragElement<VEl> {
    pub(crate) inner_element: VEl,
    pub(crate) is_dragging: bool,
    pub(crate) start_pos: (f32, f32),
    pub(crate) last_pos: (f32, f32),
    pub(crate) total_delta: (f32, f32),
}

impl<State, V: View<State>, F> View<State> for DragHandler<State, V, F>
where
    F: Fn(&State, DragContext) -> Option<V::Message> + 'static,
{
    type Element = DragElement<V::Element>;
    type Message = V::Message;

    fn build(&self, ctx: &mut Context) -> Self::Element {
        DragElement {
            inner_element: self.inner.build(ctx),
            is_dragging: false,
            start_pos: (0.0, 0.0),
            last_pos: (0.0, 0.0),
            total_delta: (0.0, 0.0),
        }
    }

    fn rebuild(&self, prev: &Self, ctx: &mut Context, element: &mut Self::Element) {
        self.inner
            .rebuild(&prev.inner, ctx, &mut element.inner_element);
    }

    fn rebuild_with_parent(
        &self,
        prev: &Self,
        ctx: &mut Context,
        element: &mut Self::Element,
        parent: Node,
        next_sibling: Option<Node>,
    ) {
        self.inner.rebuild_with_parent(
            &prev.inner,
            ctx,
            &mut element.inner_element,
            parent,
            next_sibling,
        );
    }

    fn teardown(&self, ctx: &mut Context, element: &mut Self::Element) {
        if element.is_dragging {
            element.is_dragging = false;
            ctx.release_pointer();
        }
        self.inner.teardown(ctx, &mut element.inner_element);
    }

    fn get_node(&self, element: &Self::Element) -> Node {
        self.inner.get_node(&element.inner_element)
    }

    fn handle_event(
        &self,
        element: &mut Self::Element,
        state: &State,
        event: Event,
        ctx: &mut Context,
    ) -> (EventResult, Option<Self::Message>) {
        let self_node = self.get_node(element);

        let (inner_res, inner_msg) =
            self.inner
                .handle_event(&mut element.inner_element, state, event.clone(), ctx);
        if inner_msg.is_some() {
            return (inner_res, inner_msg);
        }

        let mut handled = EventResult::Ignored;
        let mut emitted_msg = None;

        match &event {
            Event::MouseInput {
                button,
                pressed,
                x,
                y,
                hit_nodes,
            } => {
                if *button == winit::event::MouseButton::Left {
                    if *pressed {
                        if hit_nodes.contains(&self_node) && !element.is_dragging {
                            element.is_dragging = true;
                            element.start_pos = (*x, *y);
                            element.last_pos = (*x, *y);
                            element.total_delta = (0.0, 0.0);
                            ctx.capture_pointer(self_node, self.policy);

                            let drag_ctx = DragContext {
                                phase: DragPhase::Start,
                                start_pos: element.start_pos,
                                current_pos: (*x, *y),
                                delta: (0.0, 0.0),
                                total_delta: (0.0, 0.0),
                                modifiers: ctx.modifiers,
                            };
                            emitted_msg = (self.handler)(state, drag_ctx);
                            handled = EventResult::Handled;
                        }
                    } else if element.is_dragging {
                        element.is_dragging = false;
                        ctx.release_pointer();

                        let delta = (*x - element.last_pos.0, *y - element.last_pos.1);
                        element.total_delta.0 += delta.0;
                        element.total_delta.1 += delta.1;

                        let drag_ctx = DragContext {
                            phase: DragPhase::End,
                            start_pos: element.start_pos,
                            current_pos: (*x, *y),
                            delta,
                            total_delta: element.total_delta,
                            modifiers: ctx.modifiers,
                        };
                        emitted_msg = (self.handler)(state, drag_ctx);
                        handled = EventResult::Handled;
                    }
                }
            }
            Event::CursorMoved {
                x,
                y,
                delta_x,
                delta_y,
                ..
            } => {
                if element.is_dragging {
                    let delta = if self.policy == crate::CursorGrabPolicy::Locked {
                        (*delta_x, *delta_y)
                    } else {
                        (*x - element.last_pos.0, *y - element.last_pos.1)
                    };
                    element.last_pos = (*x, *y);
                    element.total_delta.0 += delta.0;
                    element.total_delta.1 += delta.1;

                    let drag_ctx = DragContext {
                        phase: DragPhase::Move,
                        start_pos: element.start_pos,
                        current_pos: (*x, *y),
                        delta,
                        total_delta: element.total_delta,
                        modifiers: ctx.modifiers,
                    };
                    emitted_msg = (self.handler)(state, drag_ctx);
                    handled = EventResult::Handled;
                }
            }
            _ => {}
        }

        (handled.or(inner_res), inner_msg.or(emitted_msg))
    }
}

/// Contextual payload passed to keyboard event listeners.
#[derive(Clone, Debug)]
pub struct KeyEventContext {
    /// The resolved logical key representation (character or named key).
    pub logical_key: winit::keyboard::Key,
    /// Physical scancode on the hardware keyboard, independent of OS keyboard layout.
    pub physical_key: winit::keyboard::PhysicalKey,
    /// UTF-8 text representation generated by this key event, if any.
    pub text: Option<String>,
    /// `true` if generated by the OS key-repeat timer while holding down the key.
    pub repeat: bool,
    /// Active keyboard modifier state (Shift, Ctrl, Alt, Meta).
    pub modifiers: winit::keyboard::ModifiersState,
}

impl KeyEventContext {
    /// Returns `true` if the logical key matches `key_str` (case-insensitive for characters).
    pub fn key_matches(&self, key_str: &str) -> bool {
        match &self.logical_key {
            winit::keyboard::Key::Character(c) => c.eq_ignore_ascii_case(key_str),
            winit::keyboard::Key::Named(named) => {
                let name = format!("{:?}", named);
                name.eq_ignore_ascii_case(key_str)
            }
            _ => false,
        }
    }

    /// Returns `true` if the Escape key was pressed.
    pub fn is_escape(&self) -> bool {
        self.logical_key == winit::keyboard::Key::Named(winit::keyboard::NamedKey::Escape)
    }

    /// Returns `true` if the Enter or Return key was pressed.
    pub fn is_enter(&self) -> bool {
        matches!(
            self.logical_key,
            winit::keyboard::Key::Named(winit::keyboard::NamedKey::Enter)
        ) || matches!(self.text.as_deref(), Some("\r") | Some("\n"))
    }

    /// Returns `true` if the Space key was pressed.
    pub fn is_space(&self) -> bool {
        self.logical_key == " "
            || matches!(self.logical_key, winit::keyboard::Key::Character(ref s) if s == " ")
    }

    /// Returns `true` if the Tab key was pressed.
    pub fn is_tab(&self) -> bool {
        self.logical_key == winit::keyboard::Key::Named(winit::keyboard::NamedKey::Tab)
    }

    /// Returns `true` if the Control key is currently held.
    pub fn with_ctrl(&self) -> bool {
        self.modifiers.control_key()
    }

    /// Returns `true` if the Shift key is currently held.
    pub fn with_shift(&self) -> bool {
        self.modifiers.shift_key()
    }

    /// Returns `true` if the Alt / Option key is currently held.
    pub fn with_alt(&self) -> bool {
        self.modifiers.alt_key()
    }

    /// Returns `true` if the Super / Meta / Windows / Command key is currently held.
    pub fn with_super(&self) -> bool {
        self.modifiers.meta_key()
    }
}

/// Action trigger kind for keyboard handlers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyActionKind {
    /// Fired when key is initially pressed down (excluding repeat).
    Down,
    /// Fired when key is released.
    Up,
    /// Fired on initial press and auto-repeat keystrokes.
    Press,
}

/// Scope of keyboard listening.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyScope {
    /// Fired only when the attached view's node has keyboard focus.
    Focused,
    /// Fired globally across the window regardless of focus.
    Global,
}

/// A wrapper view that attaches a keyboard listener to an inner view.
pub struct KeyHandler<State, V, F> {
    pub(crate) inner: V,
    pub(crate) action: KeyActionKind,
    pub(crate) scope: KeyScope,
    pub(crate) handler: Rc<F>,
    pub(crate) _marker: std::marker::PhantomData<State>,
}

impl<State, V: View<State>, F> View<State> for KeyHandler<State, V, F>
where
    F: Fn(&State, KeyEventContext) -> Option<V::Message> + 'static,
{
    type Element = V::Element;
    type Message = V::Message;

    fn build(&self, ctx: &mut Context) -> Self::Element {
        self.inner.build(ctx)
    }

    fn rebuild(&self, prev: &Self, ctx: &mut Context, element: &mut Self::Element) {
        self.inner.rebuild(&prev.inner, ctx, element);
    }

    fn rebuild_with_parent(
        &self,
        prev: &Self,
        ctx: &mut Context,
        element: &mut Self::Element,
        parent: Node,
        next_sibling: Option<Node>,
    ) {
        self.inner
            .rebuild_with_parent(&prev.inner, ctx, element, parent, next_sibling);
    }

    fn teardown(&self, ctx: &mut Context, element: &mut Self::Element) {
        self.inner.teardown(ctx, element);
    }

    fn get_node(&self, element: &Self::Element) -> Node {
        self.inner.get_node(element)
    }

    fn handle_event(
        &self,
        element: &mut Self::Element,
        state: &State,
        event: Event,
        ctx: &mut Context,
    ) -> (EventResult, Option<Self::Message>) {
        let (inner_res, inner_msg) = self.inner.handle_event(element, state, event.clone(), ctx);
        if inner_msg.is_some() {
            return (inner_res, inner_msg);
        }

        let mut handled = EventResult::Ignored;
        let mut emitted_msg = None;

        if let Event::KeyboardInput { event: k_event, .. } = &event {
            let is_focused = Some(self.get_node(element)) == ctx.focused_node();
            let scope_ok = match self.scope {
                KeyScope::Focused => is_focused,
                KeyScope::Global => true,
            };

            if scope_ok {
                let matches_action = match self.action {
                    KeyActionKind::Down => k_event.state.is_pressed() && !k_event.repeat,
                    KeyActionKind::Up => !k_event.state.is_pressed(),
                    KeyActionKind::Press => k_event.state.is_pressed(),
                };

                if matches_action {
                    let key_ctx = KeyEventContext {
                        logical_key: k_event.logical_key.clone(),
                        physical_key: k_event.physical_key,
                        text: k_event.text.as_ref().map(|s| s.to_string()),
                        repeat: k_event.repeat,
                        modifiers: ctx.modifiers,
                    };
                    emitted_msg = (self.handler)(state, key_ctx);
                    if emitted_msg.is_some() {
                        handled = EventResult::Handled;
                    }
                }
            }
        }

        (handled.or(inner_res), inner_msg.or(emitted_msg))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::KeyEvent;
    use crate::ui::widgets::{row, text};

    #[derive(Clone, Debug, PartialEq)]
    enum TestMsg {
        ParentClick,
        ChildClick,
    }

    #[test]
    fn test_nested_event_handler_child_priority() {
        let mut ctx = Context::new();

        let child_view = row((text::<_, TestMsg>("Child"),))
            .on_event(EventKind::Click, |_| Some(TestMsg::ChildClick));

        let parent_view = row((child_view, text::<_, TestMsg>("Parent Text")))
            .on_event(EventKind::Click, |_| Some(TestMsg::ParentClick));

        let mut element = View::<()>::build(&parent_view, &mut ctx);
        let parent_node = View::<()>::get_node(&parent_view, &element);
        let child_node =
            View::<()>::get_node(&parent_view.inner.children.0, &element.inner_element.1.0);

        // 1. Click child node: both child and parent are in hit_nodes
        // Press on child
        let (res_down, msg_down) = View::<()>::handle_event(
            &parent_view,
            &mut element,
            &(),
            Event::MouseInput {
                button: winit::event::MouseButton::Left,
                pressed: true,
                hit_nodes: vec![child_node, parent_node],
                x: 0.0,
                y: 0.0,
            },
            &mut ctx,
        );
        assert_eq!(res_down, EventResult::Ignored);
        assert_eq!(msg_down, None);

        // Release on child: child handler must fire, parent handler must NOT fire
        let (res_up, msg_up) = View::<()>::handle_event(
            &parent_view,
            &mut element,
            &(),
            Event::MouseInput {
                button: winit::event::MouseButton::Left,
                pressed: false,
                hit_nodes: vec![child_node, parent_node],
                x: 0.0,
                y: 0.0,
            },
            &mut ctx,
        );
        assert_eq!(res_up, EventResult::Handled);
        assert_eq!(msg_up, Some(TestMsg::ChildClick));

        // 2. Click parent node directly (child not hit)
        let (p_down_res, p_down_msg) = View::<()>::handle_event(
            &parent_view,
            &mut element,
            &(),
            Event::MouseInput {
                button: winit::event::MouseButton::Left,
                pressed: true,
                hit_nodes: vec![parent_node],
                x: 0.0,
                y: 0.0,
            },
            &mut ctx,
        );
        assert_eq!(p_down_res, EventResult::Ignored);
        assert_eq!(p_down_msg, None);

        let (p_up_res, p_up_msg) = View::<()>::handle_event(
            &parent_view,
            &mut element,
            &(),
            Event::MouseInput {
                button: winit::event::MouseButton::Left,
                pressed: false,
                hit_nodes: vec![parent_node],
                x: 0.0,
                y: 0.0,
            },
            &mut ctx,
        );
        assert_eq!(p_up_res, EventResult::Handled);
        assert_eq!(p_up_msg, Some(TestMsg::ParentClick));
    }

    #[test]
    fn test_chained_press_and_release() {
        #[derive(Clone, Debug, PartialEq)]
        enum BtnMsg {
            Press,
            Release,
        }

        let mut ctx = Context::new();
        let btn_view = text::<_, BtnMsg>("7")
            .on_event(EventKind::Press, |_| Some(BtnMsg::Press))
            .on_event(EventKind::Release, |_| Some(BtnMsg::Release));

        let mut element = View::<()>::build(&btn_view, &mut ctx);
        let btn_node = View::<()>::get_node(&btn_view, &element);

        // 1. Mouse down on button
        let (down_res, down_msg) = View::<()>::handle_event(
            &btn_view,
            &mut element,
            &(),
            Event::MouseInput {
                button: winit::event::MouseButton::Left,
                pressed: true,
                hit_nodes: vec![btn_node],
                x: 0.0,
                y: 0.0,
            },
            &mut ctx,
        );
        assert_eq!(down_res, EventResult::Handled);
        assert_eq!(down_msg, Some(BtnMsg::Press));

        // 2. Mouse up on button -> Release MUST fire
        let (up_res, up_msg) = View::<()>::handle_event(
            &btn_view,
            &mut element,
            &(),
            Event::MouseInput {
                button: winit::event::MouseButton::Left,
                pressed: false,
                hit_nodes: vec![btn_node],
                x: 0.0,
                y: 0.0,
            },
            &mut ctx,
        );
        assert_eq!(up_res, EventResult::Handled);
        assert_eq!(up_msg, Some(BtnMsg::Release));
    }

    #[test]
    fn test_drag_gesture_lifecycle() {
        #[derive(Clone, Debug, PartialEq)]
        enum DragMsg {
            Drag(DragPhase, (f32, f32), (f32, f32)),
        }

        let mut ctx = Context::new();
        let target = text::<_, DragMsg>("Draggable")
            .on_drag(|_state, d| Some(DragMsg::Drag(d.phase, d.delta, d.total_delta)));

        let mut element = View::<()>::build(&target, &mut ctx);
        let node = View::<()>::get_node(&target, &element);

        assert!(!ctx.has_pointer_capture());
        assert!(!ctx.is_pointer_captured(node));

        // 1. Mouse down on draggable target -> Starts drag and captures pointer
        let (start_res, start_msg) = View::<()>::handle_event(
            &target,
            &mut element,
            &(),
            Event::MouseInput {
                button: winit::event::MouseButton::Left,
                pressed: true,
                hit_nodes: vec![node],
                x: 50.0,
                y: 50.0,
            },
            &mut ctx,
        );
        assert_eq!(start_res, EventResult::Handled);
        assert_eq!(
            start_msg,
            Some(DragMsg::Drag(DragPhase::Start, (0.0, 0.0), (0.0, 0.0)))
        );
        assert!(ctx.has_pointer_capture());
        assert!(ctx.is_pointer_captured(node));
        assert_eq!(ctx.captured_node(), Some(node));

        // 2. Mouse move (even if hit_nodes doesn't include node, pointer is captured)
        let (move_res, move_msg) = View::<()>::handle_event(
            &target,
            &mut element,
            &(),
            Event::CursorMoved {
                x: 75.0,
                y: 60.0,
                delta_x: 25.0,
                delta_y: 10.0,
                hit_nodes: vec![],
            },
            &mut ctx,
        );
        assert_eq!(move_res, EventResult::Handled);
        assert_eq!(
            move_msg,
            Some(DragMsg::Drag(DragPhase::Move, (25.0, 10.0), (25.0, 10.0)))
        );

        // 3. Second mouse move
        let (move2_res, move2_msg) = View::<()>::handle_event(
            &target,
            &mut element,
            &(),
            Event::CursorMoved {
                x: 80.0,
                y: 70.0,
                delta_x: 5.0,
                delta_y: 10.0,
                hit_nodes: vec![],
            },
            &mut ctx,
        );
        assert_eq!(move2_res, EventResult::Handled);
        assert_eq!(
            move2_msg,
            Some(DragMsg::Drag(DragPhase::Move, (5.0, 10.0), (30.0, 20.0)))
        );

        // 4. Mouse up -> Ends drag and releases pointer
        let (end_res, end_msg) = View::<()>::handle_event(
            &target,
            &mut element,
            &(),
            Event::MouseInput {
                button: winit::event::MouseButton::Left,
                pressed: false,
                hit_nodes: vec![],
                x: 80.0,
                y: 70.0,
            },
            &mut ctx,
        );
        assert_eq!(end_res, EventResult::Handled);
        assert_eq!(
            end_msg,
            Some(DragMsg::Drag(DragPhase::End, (0.0, 0.0), (30.0, 20.0)))
        );
        assert!(!ctx.has_pointer_capture());
        assert_eq!(ctx.captured_node(), None);
    }

    #[test]
    fn test_drag_relative_gesture() {
        #[derive(Clone, Debug, PartialEq)]
        enum RelMsg {
            Delta(f32, f32),
        }

        let mut ctx = Context::new();
        let dial = text::<_, RelMsg>("Dial")
            .on_drag_relative(|_state, d| Some(RelMsg::Delta(d.delta.0, d.delta.1)));

        let mut element = View::<()>::build(&dial, &mut ctx);
        let node = View::<()>::get_node(&dial, &element);

        // Mouse down
        let _ = View::<()>::handle_event(
            &dial,
            &mut element,
            &(),
            Event::MouseInput {
                button: winit::event::MouseButton::Left,
                pressed: true,
                hit_nodes: vec![node],
                x: 100.0,
                y: 100.0,
            },
            &mut ctx,
        );
        assert!(ctx.has_pointer_capture());
        assert_eq!(
            ctx.captured_pointer.as_ref().map(|c| c.policy),
            Some(crate::CursorGrabPolicy::Locked)
        );

        // Mouse motion with raw hardware delta
        let (res, msg) = View::<()>::handle_event(
            &dial,
            &mut element,
            &(),
            Event::CursorMoved {
                x: 100.0,
                y: 100.0,
                delta_x: 12.5,
                delta_y: -4.0,
                hit_nodes: vec![],
            },
            &mut ctx,
        );
        assert_eq!(res, EventResult::Handled);
        assert_eq!(msg, Some(RelMsg::Delta(12.5, -4.0)));

        // Release
        let _ = View::<()>::handle_event(
            &dial,
            &mut element,
            &(),
            Event::MouseInput {
                button: winit::event::MouseButton::Left,
                pressed: false,
                hit_nodes: vec![],
                x: 100.0,
                y: 100.0,
            },
            &mut ctx,
        );
        assert!(!ctx.has_pointer_capture());
    }

    #[test]
    fn test_key_events_focused_and_global() {
        #[derive(Clone, Debug, PartialEq)]
        enum KeyMsg {
            EscPressed,
            EnterUp,
            GlobalA,
        }

        let mut ctx = Context::new();
        let editor_input = text::<_, KeyMsg>("Input")
            .on_key_down(|_state, key| {
                if key.is_escape() {
                    Some(KeyMsg::EscPressed)
                } else {
                    None
                }
            })
            .on_key_up(|_state, key| {
                if key.is_enter() {
                    Some(KeyMsg::EnterUp)
                } else {
                    None
                }
            })
            .on_global_key_down(|_state, key| {
                if key.key_matches("a") {
                    Some(KeyMsg::GlobalA)
                } else {
                    None
                }
            });

        let mut element = View::<()>::build(&editor_input, &mut ctx);
        let node = View::<()>::get_node(&editor_input, &element);

        // 1. Without focus, on_key_down(Escape) should NOT fire
        let esc_event = KeyEvent::new(
            winit::keyboard::Key::Named(winit::keyboard::NamedKey::Escape),
            winit::event::ElementState::Pressed,
        );
        let (esc_res, esc_msg) = View::<()>::handle_event(
            &editor_input,
            &mut element,
            &(),
            Event::KeyboardInput {
                event: esc_event.clone(),
                is_synthetic: false,
            },
            &mut ctx,
        );
        assert_eq!(esc_res, EventResult::Ignored);
        assert_eq!(esc_msg, None);

        // 2. Global key down SHOULD fire even without focus
        let mut a_event = KeyEvent::new(
            winit::keyboard::Key::Character("a".into()),
            winit::event::ElementState::Pressed,
        );
        a_event.text = Some("a".into());
        let (a_res, a_msg) = View::<()>::handle_event(
            &editor_input,
            &mut element,
            &(),
            Event::KeyboardInput {
                event: a_event,
                is_synthetic: false,
            },
            &mut ctx,
        );
        assert_eq!(a_res, EventResult::Handled);
        assert_eq!(a_msg, Some(KeyMsg::GlobalA));

        // 3. Now request focus on element
        ctx.request_focus(node);

        // Now on_key_down(Escape) MUST fire
        let (f_res, f_msg) = View::<()>::handle_event(
            &editor_input,
            &mut element,
            &(),
            Event::KeyboardInput {
                event: esc_event,
                is_synthetic: false,
            },
            &mut ctx,
        );
        assert_eq!(f_res, EventResult::Handled);
        assert_eq!(f_msg, Some(KeyMsg::EscPressed));

        // on_key_up(Enter) when released
        let enter_up_event = KeyEvent::new(
            winit::keyboard::Key::Named(winit::keyboard::NamedKey::Enter),
            winit::event::ElementState::Released,
        );
        let (enter_res, enter_msg) = View::<()>::handle_event(
            &editor_input,
            &mut element,
            &(),
            Event::KeyboardInput {
                event: enter_up_event,
                is_synthetic: false,
            },
            &mut ctx,
        );
        assert_eq!(enter_res, EventResult::Handled);
        assert_eq!(enter_msg, Some(KeyMsg::EnterUp));
    }

    #[test]
    fn test_thumb_scroll_handler() {
        use crate::style::Rect;
        use crate::ui::widgets::button;
        let mut ctx = Context::new();

        #[derive(Debug, PartialEq, Clone)]
        struct ScrolledData {
            pct: f32,
            dragging: bool,
            y: f32,
        }

        let inner = button("test");
        let handled = inner.on_thumb_scroll(|_state, ctx| {
            Some(ScrolledData {
                pct: ctx.scroll_pct,
                dragging: ctx.is_dragging,
                y: ctx.thumb.y,
            })
        });

        let mut element = View::<()>::build(&handled, &mut ctx);
        let node = View::<()>::get_node(&handled, &element);

        // Matching node receives event
        let (res, msg) = View::<()>::handle_event(
            &handled,
            &mut element,
            &(),
            Event::ThumbScroll {
                node,
                context: ThumbScrollContext {
                    thumb: Rect {
                        x: 95.0,
                        y: 45.0,
                        w: 5.0,
                        h: 20.0,
                    },
                    scroll_pct: 0.5,
                    is_dragging: true,
                },
            },
            &mut ctx,
        );
        assert_eq!(res, EventResult::Handled);
        assert_eq!(
            msg,
            Some(ScrolledData {
                pct: 0.5,
                dragging: true,
                y: 45.0,
            })
        );

        // Unrelated node does not trigger handler
        let other_node = ctx.create_node();
        let (other_res, other_msg) = View::<()>::handle_event(
            &handled,
            &mut element,
            &(),
            Event::ThumbScroll {
                node: other_node,
                context: ThumbScrollContext {
                    thumb: Rect::default(),
                    scroll_pct: 0.0,
                    is_dragging: false,
                },
            },
            &mut ctx,
        );
        assert_eq!(other_res, EventResult::Ignored);
        assert_eq!(other_msg, None);
    }

    #[test]
    fn test_scroll_handler() {
        use crate::ui::widgets::button;
        let mut ctx = Context::new();

        let inner = button("test");
        let handled = inner.on_scroll(|_state, ctx| Some(ctx.scroll_y));

        let mut element = View::<()>::build(&handled, &mut ctx);
        let node = View::<()>::get_node(&handled, &element);

        let scroll_ctx = ScrollContext {
            scroll_x: 0.0,
            scroll_y: 120.0,
            max_scroll_x: 0.0,
            max_scroll_y: 500.0,
            viewport_w: 200.0,
            viewport_h: 300.0,
            content_w: 200.0,
            content_h: 800.0,
            scroll_pct_x: 0.0,
            scroll_pct_y: 0.24,
            delta_x: 0.0,
            delta_y: 20.0,
            source: ScrollSource::Wheel,
        };

        // Matching node receives event
        let (res, msg) = View::<()>::handle_event(
            &handled,
            &mut element,
            &(),
            Event::Scroll {
                node,
                context: scroll_ctx,
            },
            &mut ctx,
        );
        assert_eq!(res, EventResult::Handled);
        assert_eq!(msg, Some(120.0));

        // Unrelated node does not trigger handler
        let other_node = ctx.create_node();
        let (other_res, other_msg) = View::<()>::handle_event(
            &handled,
            &mut element,
            &(),
            Event::Scroll {
                node: other_node,
                context: scroll_ctx,
            },
            &mut ctx,
        );
        assert_eq!(other_res, EventResult::Ignored);
        assert_eq!(other_msg, None);
    }

    #[test]
    fn test_scroll_context_helpers() {
        let top_start = ScrollContext {
            scroll_x: 0.0,
            scroll_y: 0.0,
            max_scroll_x: 300.0,
            max_scroll_y: 600.0,
            viewport_w: 100.0,
            viewport_h: 100.0,
            content_w: 400.0,
            content_h: 700.0,
            scroll_pct_x: 0.0,
            scroll_pct_y: 0.0,
            delta_x: 0.0,
            delta_y: 0.0,
            source: ScrollSource::Touchpad,
        };
        assert!(top_start.is_at_top());
        assert!(top_start.is_at_start());
        assert!(!top_start.is_at_bottom());
        assert!(!top_start.is_at_end());
        assert_eq!(top_start.scroll_pct(), 0.0);

        let bottom_end = ScrollContext {
            scroll_x: 300.0,
            scroll_y: 600.0,
            max_scroll_x: 300.0,
            max_scroll_y: 600.0,
            viewport_w: 100.0,
            viewport_h: 100.0,
            content_w: 400.0,
            content_h: 700.0,
            scroll_pct_x: 1.0,
            scroll_pct_y: 1.0,
            delta_x: 10.0,
            delta_y: 15.0,
            source: ScrollSource::Thumb,
        };
        assert!(!bottom_end.is_at_top());
        assert!(!bottom_end.is_at_start());
        assert!(bottom_end.is_at_bottom());
        assert!(bottom_end.is_at_end());
        assert_eq!(bottom_end.scroll_pct(), 1.0);
    }

    #[test]
    fn test_scroll_context_from_node() {
        let mut ctx = Context::new();
        let node = ctx.create_node();

        node.update_constraints(&mut ctx, |c| {
            c.width = crate::style::Size::Fixed(200);
            c.height = crate::style::Size::Fixed(100);
            c.scroll.y = 50.0;
            c.scroll.x = 25.0;
        });

        // Compute layout
        ctx.root_attach(node);
        ctx.compute_layout(800.0, 600.0);

        let scroll_ctx = ScrollContext::from_node(node, &ctx, 10.0, 30.0, ScrollSource::Kinetic)
            .expect("ScrollContext should be generated for laid out node");

        assert_eq!(scroll_ctx.scroll_x, 25.0);
        assert_eq!(scroll_ctx.scroll_y, 50.0);
        assert_eq!(scroll_ctx.delta_x, 15.0);
        assert_eq!(scroll_ctx.delta_y, 20.0);
        assert_eq!(scroll_ctx.source, ScrollSource::Kinetic);
        assert_eq!(scroll_ctx.viewport_w, 200.0);
        assert_eq!(scroll_ctx.viewport_h, 100.0);
    }
}
