use crate::{
    Context, Node, PositionStrategy, Size,
    colors::Color,
    effects::Radius,
    style::{Overflow, Vector2},
    ui::{Event, View, event::EventResult},
};

/// Defines which axes a `ScrollView` is allowed to scroll on.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ScrollAxis {
    /// Only allow horizontal scrolling.
    Horizontal,
    /// Only allow vertical scrolling.
    Vertical,
    /// Allow scrolling on both the horizontal and vertical axes.
    Both,
}

/// Represents an explicit initial scroll position or a programmatic jump offset.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ScrollOffset {
    /// A percentage-based offset, where `0.0` is the start and `1.0` is the very end of the scrollable content.
    Percent(f32),
    /// An absolute pixel offset.
    Pixel(f32),
}

/// Data provided to a scrollbar to calculate its dimensions and position.
/// This includes the current scroll offset, maximum scroll offset, and the dimensions
/// of both the visible viewport and the entire scrollable content.
#[derive(Clone, Copy, Debug)]
pub struct ScrollMetrics {
    /// The current vertical scroll offset in pixels (0.0 is the top).
    pub current_y: f32,
    /// The maximum possible vertical scroll offset (content height - viewport height).
    pub max_y: f32,
    /// The height of the visible area.
    pub viewport_h: f32,
    /// The total height of the scrollable content.
    pub content_h: f32,

    /// The current horizontal scroll offset in pixels (0.0 is the left edge).
    pub current_x: f32,
    /// The maximum possible horizontal scroll offset.
    pub max_x: f32,
    /// The width of the visible area.
    pub viewport_w: f32,
    /// The total width of the scrollable content.
    pub content_w: f32,
}

/// Messages emitted by a `ScrollBar` to instruct the parent `ScrollView` to change its scroll position.
/// For example, when a user clicks and drags the scrollbar thumb, the scrollbar emits `SetY`
/// to immediately sync the `ScrollView` to the new calculated offset.
pub enum ScrollBarMessage {
    /// Request the `ScrollView` to instantly snap to this absolute vertical pixel offset.
    SetY(f32),
    /// Request the `ScrollView` to instantly snap to this absolute horizontal pixel offset.
    SetX(f32),
}

/// A specialized `View` trait for implementing custom scrollbars.
///
/// Custom scrollbars must implement this trait in addition to the standard `View` trait.
/// The `View` implementation should emit `ScrollBarMessage` when the user interacts
/// with the scrollbar (e.g. dragging the thumb), which the parent `ScrollView` will intercept.
pub trait ScrollBar<State>: View<State, Message = ScrollBarMessage> {
    /// Called by the parent `ScrollView` whenever the layout changes, the user scrolls,
    /// or an overscroll physics animation occurs.
    ///
    /// Use this method to update the size, position, and constraints of your scrollbar's `Node`
    /// based on the provided `metrics`.
    fn update_metrics(
        &self,
        element: &mut Self::Element,
        ctx: &mut Context,
        metrics: ScrollMetrics,
    );

    /// Called by the parent `ScrollView` when user activity is detected nearby
    /// (e.g. when the user's cursor moves inside the `ScrollView`).
    ///
    /// This is typically used to wake up the scrollbar from an idle state and trigger
    /// fade-in animations or reset inactivity timers. Be sure to call `ctx.request_frame()`
    /// if you need to start an animation.
    fn wake_up(&self, element: &mut Self::Element, ctx: &mut Context);
}

pub struct DefaultScrollBar;

pub struct DefaultScrollBarElement {
    track_node: Node,
    thumb_node: Node,
    thumb_opacity: f32,
    thumb_width: f32,
    idle_time: f32,
    is_hovering: bool,
    is_dragging: bool,
    drag_start_y: f32,
    drag_start_scroll_y: f32,
    metrics: Option<ScrollMetrics>,
}

impl<State> View<State> for DefaultScrollBar {
    type Element = DefaultScrollBarElement;
    type Message = ScrollBarMessage;

    fn build(&self, ctx: &mut Context) -> Self::Element {
        let track_node = ctx.create_node();
        let thumb_node = ctx.create_node();
        track_node.append(ctx, thumb_node);

        track_node.update_constraints(ctx, |c| {
            c.positioning = PositionStrategy::Absolute {
                top: 0.0,
                bottom: 0.0,
                right: 0.0,
                left: f32::NAN,
            };
            c.width = Size::Fixed(14);
            c.height = Size::Fill;
            c.z_index = 100;
        });

        track_node.update_effects(ctx, |e| {
            e.background_color = Color::new(100, 100, 100, 0); // Transparent initially
        });

        thumb_node.update_constraints(ctx, |c| {
            c.positioning = PositionStrategy::Absolute {
                top: 0.0,
                left: f32::NAN,
                bottom: f32::NAN,
                right: 4.0, // padded a bit from right edge
            };
            c.width = Size::Fixed(4);
            c.height = Size::Fixed(0);
        });

        thumb_node.update_effects(ctx, |e| {
            // Pill shape
            e.background_color = Color::new(100, 100, 100, 180);
            e.border.radius = Radius::all(6.0);
            e.opacity = 0.0;
        });

        DefaultScrollBarElement {
            track_node,
            thumb_node,
            thumb_opacity: 0.0,
            thumb_width: 4.0,
            idle_time: 0.0,
            is_hovering: false,
            is_dragging: false,
            drag_start_y: 0.0,
            drag_start_scroll_y: 0.0,
            metrics: None,
        }
    }

    fn rebuild(&self, _prev: &Self, _ctx: &mut Context, _element: &mut Self::Element) {}

    fn teardown(&self, ctx: &mut Context, element: &mut Self::Element) {
        element.thumb_node.remove(ctx);
        ctx.destroy_node(element.thumb_node);
        element.track_node.remove(ctx);
        ctx.destroy_node(element.track_node);
    }

    fn handle_event(
        &self,
        element: &mut Self::Element,
        _state: &State,
        event: Event,
        ctx: &mut Context,
    ) -> (EventResult, Option<Self::Message>) {
        let mut msg = None;

        match event {
            Event::MouseInput {
                pressed,
                x: _,
                y,
                hit_nodes,
            } => {
                if pressed {
                    if hit_nodes.contains(&element.thumb_node) {
                        element.is_dragging = true;
                        element.drag_start_y = y;
                        if let Some(metrics) = &element.metrics {
                            element.drag_start_scroll_y = metrics.current_y;
                        }
                        element.idle_time = 0.0;
                    }
                } else {
                    element.is_dragging = false;
                }
            }
            Event::CursorMoved { x: _, y, hit_nodes } => {
                let is_hovering = hit_nodes.contains(&element.track_node)
                    || hit_nodes.contains(&element.thumb_node);
                if is_hovering != element.is_hovering {
                    element.is_hovering = is_hovering;
                    ctx.request_frame();
                }

                if is_hovering || element.is_dragging {
                    element.idle_time = 0.0;
                }

                if element.is_dragging {
                    if let Some(metrics) = &element.metrics {
                        if metrics.max_y > 0.0 {
                            let scrollbar_height = ((metrics.viewport_h / metrics.content_h)
                                * metrics.viewport_h)
                                .max(20.0)
                                .min(metrics.viewport_h);
                            let max_scrollbar_y = metrics.viewport_h - scrollbar_height;

                            if max_scrollbar_y > 0.0 {
                                let dy = y - element.drag_start_y;
                                let scroll_ratio = dy / max_scrollbar_y;
                                let new_scroll_y = (element.drag_start_scroll_y
                                    + scroll_ratio * metrics.max_y)
                                    .clamp(0.0, metrics.max_y);
                                msg = Some(ScrollBarMessage::SetY(new_scroll_y));
                            }
                        }
                    }
                }
            }
            Event::Tick { dt } => {
                element.idle_time += dt;

                let target_thumb_opacity = if element.idle_time < 1.0 || element.is_dragging {
                    1.0
                } else {
                    0.0
                };
                let target_width = if element.is_hovering || element.is_dragging {
                    8.0
                } else {
                    4.0
                };

                let mut changed = false;

                let new_thumb_opacity = if target_thumb_opacity > 0.0 {
                    (element.thumb_opacity + dt * 8.0).min(1.0)
                } else {
                    (element.thumb_opacity - dt * 4.0).max(0.0)
                };
                if new_thumb_opacity != element.thumb_opacity {
                    element.thumb_opacity = new_thumb_opacity;
                    changed = true;
                }

                let smoothing = 1.0 - f32::exp(-15.0 * dt);

                let w_diff = target_width - element.thumb_width;
                if w_diff.abs() > 0.1 {
                    element.thumb_width += w_diff * smoothing;
                    changed = true;
                } else if w_diff != 0.0 {
                    element.thumb_width = target_width;
                    changed = true;
                }

                if changed {
                    element.thumb_node.update_effects(ctx, |e| {
                        e.opacity = element.thumb_opacity;
                    });
                    element.thumb_node.update_constraints(ctx, |c| {
                        c.width = Size::Fixed(element.thumb_width as u32);
                    });

                    if let Some(w) = &ctx.window {
                        w.request_redraw();
                    }
                }

                if element.idle_time < 1.0 || element.thumb_opacity > 0.0 || w_diff.abs() > 0.1 {
                    ctx.request_frame();
                }
            }
            _ => {}
        }

        (EventResult::Ignored, msg)
    }

    fn get_node(&self, element: &Self::Element) -> Node {
        element.track_node
    }
}

impl<State> ScrollBar<State> for DefaultScrollBar {
    fn update_metrics(
        &self,
        element: &mut Self::Element,
        ctx: &mut Context,
        metrics: ScrollMetrics,
    ) {
        element.metrics = Some(metrics);

        if metrics.max_y > 0.0 {
            let scrollbar_height = ((metrics.viewport_h / metrics.content_h) * metrics.viewport_h)
                .max(20.0)
                .min(metrics.viewport_h);
            let max_scrollbar_y = metrics.viewport_h - scrollbar_height;
            let mut top_pos = (metrics.current_y / metrics.max_y).clamp(0.0, 1.0) * max_scrollbar_y;

            let mut final_h = scrollbar_height;
            if metrics.current_y < 0.0 {
                let overscroll = -metrics.current_y;
                top_pos = 0.0;
                final_h = (scrollbar_height - overscroll * 0.2).max(10.0);
            } else if metrics.current_y > metrics.max_y {
                let overscroll = metrics.current_y - metrics.max_y;
                final_h = (scrollbar_height - overscroll * 0.2).max(10.0);
                top_pos = metrics.viewport_h - final_h;
            }

            element.thumb_node.update_constraints(ctx, |c| {
                c.height = crate::style::Size::Fixed(final_h as u32);
                c.positioning = crate::style::PositionStrategy::Absolute {
                    top: top_pos,
                    left: f32::NAN,
                    bottom: f32::NAN,
                    right: 4.0,
                };
            });
        } else {
            element.thumb_node.update_constraints(ctx, |c| {
                c.height = crate::style::Size::Fixed(0);
            });
        }
    }

    fn wake_up(&self, element: &mut Self::Element, ctx: &mut Context) {
        element.idle_time = 0.0;
        ctx.request_frame();
    }
}

pub struct NoScrollBar;

impl<State> View<State> for NoScrollBar {
    type Element = Node;
    type Message = ScrollBarMessage;

    fn build(&self, ctx: &mut Context) -> Self::Element {
        let node = ctx.create_node();
        node.update_constraints(ctx, |c| {
            c.positioning = PositionStrategy::Absolute {
                top: 0.0,
                left: 0.0,
                bottom: 0.0,
                right: 0.0,
            };
            c.width = Size::Fixed(0);
            c.height = Size::Fixed(0);
        });
        node
    }

    fn rebuild(&self, _prev: &Self, _ctx: &mut Context, _element: &mut Self::Element) {}

    fn teardown(&self, ctx: &mut Context, element: &mut Self::Element) {
        element.remove(ctx);
        ctx.destroy_node(*element);
    }

    fn handle_event(
        &self,
        _element: &mut Self::Element,
        _state: &State,
        _event: Event,
        _ctx: &mut Context,
    ) -> (EventResult, Option<Self::Message>) {
        (EventResult::Ignored, None)
    }

    fn get_node(&self, element: &Self::Element) -> Node {
        *element
    }
}

impl<State> ScrollBar<State> for NoScrollBar {
    fn update_metrics(
        &self,
        _element: &mut Self::Element,
        _ctx: &mut Context,
        _metrics: ScrollMetrics,
    ) {
    }

    fn wake_up(&self, _element: &mut Self::Element, _ctx: &mut Context) {}
}

pub struct ScrollView<V, S> {
    pub(crate) inner: V,
    pub(crate) axis: ScrollAxis,
    pub(crate) initial_x: Option<ScrollOffset>,
    pub(crate) initial_y: Option<ScrollOffset>,
    pub(crate) scrollbar: S,
}

pub fn scroll_view<V>(inner: V) -> ScrollView<V, DefaultScrollBar> {
    ScrollView {
        inner,
        axis: ScrollAxis::Both,
        initial_x: None,
        initial_y: None,
        scrollbar: DefaultScrollBar,
    }
}

impl<V, S> ScrollView<V, S> {
    pub fn axis(mut self, axis: ScrollAxis) -> Self {
        self.axis = axis;
        self
    }

    pub fn start_offset_x(mut self, offset: ScrollOffset) -> Self {
        self.initial_x = Some(offset);
        self
    }

    pub fn start_offset_y(mut self, offset: ScrollOffset) -> Self {
        self.initial_y = Some(offset);
        self
    }

    pub fn scrollbar<S2>(self, scrollbar: S2) -> ScrollView<V, S2> {
        ScrollView {
            inner: self.inner,
            axis: self.axis,
            initial_x: self.initial_x,
            initial_y: self.initial_y,
            scrollbar,
        }
    }

    pub fn no_scrollbar(self) -> ScrollView<V, NoScrollBar> {
        ScrollView {
            inner: self.inner,
            axis: self.axis,
            initial_x: self.initial_x,
            initial_y: self.initial_y,
            scrollbar: NoScrollBar,
        }
    }
}

pub struct ScrollState {
    pub target_y: f32,
    pub target_x: f32,
    pub current_y: f32,
    pub current_x: f32,
    pub is_touching: bool,
    pub velocity_y: f32,
    pub velocity_x: f32,
    pub is_initialized: bool,
    pub needs_sync: bool,
}

impl<State, Msg, V, S> View<State> for ScrollView<V, S>
where
    V: View<State, Message = Msg>,
    S: ScrollBar<State>,
{
    // Element = (Wrapper Node, ScrollContainer Node, S::Element, Inner Element, ScrollState)
    type Element = (Node, Node, S::Element, V::Element, ScrollState);
    type Message = Msg;

    fn build(&self, ctx: &mut Context) -> Self::Element {
        let wrapper = ctx.create_node();

        let scroll_container = ctx.create_node();
        scroll_container.update_constraints(ctx, |c| {
            c.width = crate::style::Size::Fill;
            c.height = crate::style::Size::Fill;
            c.overflow = Overflow::Scroll;
        });

        let mut target_x = 0.0;
        let mut target_y = 0.0;
        if let Some(ScrollOffset::Pixel(p)) = self.initial_x {
            target_x = p;
        }
        if let Some(ScrollOffset::Pixel(p)) = self.initial_y {
            target_y = p;
        }

        scroll_container.update_constraints(ctx, |c| {
            c.scroll = Vector2 {
                x: target_x,
                y: target_y,
            };
        });

        let scrollbar_y = self.scrollbar.build(ctx);

        let child_element = self.inner.build(ctx);
        scroll_container.append(ctx, self.inner.get_node(&child_element));

        wrapper.append(ctx, scroll_container);
        wrapper.append(ctx, self.scrollbar.get_node(&scrollbar_y));

        ctx.request_frame();

        (
            wrapper,
            scroll_container,
            scrollbar_y,
            child_element,
            ScrollState {
                current_x: target_x,
                current_y: target_y,
                target_x,
                target_y,
                is_touching: false,
                velocity_y: 0.0,
                velocity_x: 0.0,
                is_initialized: false,
                needs_sync: false,
            },
        )
    }

    fn rebuild(&self, prev: &Self, ctx: &mut Context, element: &mut Self::Element) {
        self.scrollbar.rebuild(&prev.scrollbar, ctx, &mut element.2);
        self.inner.rebuild(&prev.inner, ctx, &mut element.3);

        let scroll_x = element.4.current_x;
        let scroll_y = element.4.current_y;

        element.1.update_constraints(ctx, |c| {
            c.overflow = Overflow::Scroll;
            c.scroll = Vector2 {
                x: scroll_x,
                y: scroll_y,
            };
        });
    }

    fn teardown(&self, ctx: &mut Context, element: &mut Self::Element) {
        self.scrollbar.teardown(ctx, &mut element.2);
        self.inner.teardown(ctx, &mut element.3);
        element.0.remove(ctx);
        ctx.destroy_node(element.0);
    }

    fn get_node(&self, element: &Self::Element) -> Node {
        element.0
    }

    fn handle_event(
        &self,
        element: &mut Self::Element,
        state: &State,
        event: Event,
        ctx: &mut Context,
    ) -> (EventResult, Option<Self::Message>) {
        let (mut result, mut emitted_msg) = (EventResult::Ignored, None);

        let (sb_result, sb_msg) =
            self.scrollbar
                .handle_event(&mut element.2, state, event.clone(), ctx);

        result = result.or(sb_result);

        if let Some(msg) = sb_msg {
            match msg {
                ScrollBarMessage::SetY(y) => {
                    element.4.target_y = y;
                    element.4.current_y = y;
                    element.4.needs_sync = true;
                    ctx.request_frame();
                }
                ScrollBarMessage::SetX(x) => {
                    element.4.target_x = x;
                    element.4.current_x = x;
                    element.4.needs_sync = true;
                    ctx.request_frame();
                }
            }
        }

        if result == EventResult::Ignored {
            let (inner_res, inner_msg) =
                self.inner
                    .handle_event(&mut element.3, state, event.clone(), ctx);
            result = result.or(inner_res);
            emitted_msg = emitted_msg.or(inner_msg);
        }

        if result == EventResult::Handled {
            return (result, emitted_msg);
        }

        let parent_comp = element
            .1
            .get_computed(ctx)
            .unwrap_or(crate::style::Computed {
                x: 0.,
                y: 0.,
                w: 0.,
                h: 0.,
            });
        let inner_node = self.inner.get_node(&element.3);
        let inner_comp = inner_node
            .get_computed(ctx)
            .unwrap_or(crate::style::Computed {
                x: 0.,
                y: 0.,
                w: 0.,
                h: 0.,
            });

        let max_y = (inner_comp.h - parent_comp.h).max(0.0);
        let max_x = (inner_comp.w - parent_comp.w).max(0.0);

        match event {
            Event::CursorMoved { ref hit_nodes, .. } => {
                let sb_node = self.scrollbar.get_node(&element.2);
                if hit_nodes.contains(&element.1) || hit_nodes.contains(&sb_node) {
                    self.scrollbar.wake_up(&mut element.2, ctx);
                }
            }
            Event::MouseWheel {
                delta_x,
                delta_y,
                is_touchpad,
                phase,
                ref hit_nodes,
            } => {
                let sb_node = self.scrollbar.get_node(&element.2);
                if hit_nodes.contains(&element.1) || hit_nodes.contains(&sb_node) {
                    self.scrollbar.wake_up(&mut element.2, ctx);

                    if is_touchpad {
                        use winit::event::TouchPhase;
                        element.4.is_touching =
                            phase == TouchPhase::Started || phase == TouchPhase::Moved;

                        let mut dy = if matches!(self.axis, ScrollAxis::Vertical | ScrollAxis::Both)
                        {
                            -delta_y * 2.5
                        } else {
                            0.0
                        };
                        let mut dx =
                            if matches!(self.axis, ScrollAxis::Horizontal | ScrollAxis::Both) {
                                -delta_x * 2.5
                            } else {
                                0.0
                            };

                        // Track velocity for inertia kick
                        if element.4.is_touching {
                            // Exponential moving average for velocity smoothing
                            element.4.velocity_y = element.4.velocity_y * 0.4 + dy * 0.6;
                            element.4.velocity_x = element.4.velocity_x * 0.4 + dx * 0.6;
                        }

                        // Rubber-band resistance
                        if element.4.target_y < 0.0 {
                            let overscroll = -element.4.target_y;
                            let friction = 1.0 / (1.0 + overscroll * 0.03);
                            dy *= friction;
                        } else if element.4.target_y > max_y {
                            let overscroll = element.4.target_y - max_y;
                            let friction = 1.0 / (1.0 + overscroll * 0.03);
                            dy *= friction;
                        }

                        if element.4.target_x < 0.0 {
                            let overscroll = -element.4.target_x;
                            let friction = 1.0 / (1.0 + overscroll * 0.03);
                            dx *= friction;
                        } else if element.4.target_x > max_x {
                            let overscroll = element.4.target_x - max_x;
                            let friction = 1.0 / (1.0 + overscroll * 0.03);
                            dx *= friction;
                        }

                        element.4.target_y += dy;
                        element.4.target_x += dx;
                    } else {
                        // Mechanical mouse: smooth scroll, hard clamp target (no rubber-band)
                        let dy = if matches!(self.axis, ScrollAxis::Vertical | ScrollAxis::Both) {
                            -delta_y * 4.0
                        } else {
                            0.0
                        };
                        let dx = if matches!(self.axis, ScrollAxis::Horizontal | ScrollAxis::Both) {
                            -delta_x * 4.0
                        } else {
                            0.0
                        };

                        element.4.target_y = (element.4.target_y + dy).clamp(0.0, max_y);
                        element.4.target_x = (element.4.target_x + dx).clamp(0.0, max_x);
                        // NOTE: we don't snap current_y to target_y; we let Event::Tick smooth it out (browser style)
                    }

                    if let Some(w) = &ctx.window {
                        w.request_redraw();
                    }
                    // Return Ignored to prevent the entire UI tree from rebuilding.
                    // We only updated internal scroll state and requested a redraw.
                    result = EventResult::Ignored;
                }
            }
            Event::Tick { dt } => {
                if !element.4.is_initialized {
                    if let (Some(computed), Some(inner_computed)) = (
                        element.1.get_computed(ctx),
                        self.inner.get_node(&element.3).get_computed(ctx),
                    ) {
                        if computed.w > 0.0 || computed.h > 0.0 {
                            let max_y = (inner_computed.h - computed.h).max(0.0);
                            let max_x = (inner_computed.w - computed.w).max(0.0);

                            if let Some(ScrollOffset::Percent(p)) = &self.initial_y {
                                element.4.target_y = (p * max_y).clamp(0.0, max_y);
                                element.4.current_y = element.4.target_y;
                            }

                            if let Some(ScrollOffset::Percent(p)) = &self.initial_x {
                                element.4.target_x = (p * max_x).clamp(0.0, max_x);
                                element.4.current_x = element.4.target_x;
                            }

                            element.4.is_initialized = true;

                            let current_y = element.4.current_y;
                            let current_x = element.4.current_x;
                            element.1.update_constraints(ctx, |c| {
                                c.overflow = Overflow::Scroll;
                                c.scroll = Vector2 {
                                    x: current_x,
                                    y: current_y,
                                };
                            });

                            self.scrollbar.update_metrics(
                                &mut element.2,
                                ctx,
                                ScrollMetrics {
                                    current_y,
                                    max_y,
                                    viewport_h: computed.h,
                                    content_h: inner_computed.h,
                                    current_x,
                                    max_x,
                                    viewport_w: computed.w,
                                    content_w: inner_computed.w,
                                },
                            );
                        }
                    }
                }

                if !element.4.is_touching {
                    // we apply kinetic inertia (the "kick")
                    if element.4.velocity_y.abs() > 0.1 || element.4.velocity_x.abs() > 0.1 {
                        // and we only apply inertia if we are within bounds (or applying friction if out of bounds)
                        if element.4.target_y >= 0.0 && element.4.target_y <= max_y {
                            element.4.target_y += element.4.velocity_y;
                        }
                        if element.4.target_x >= 0.0 && element.4.target_x <= max_x {
                            element.4.target_x += element.4.velocity_x;
                        }

                        // Friction decay
                        let friction = f32::exp(-4.0 * dt);
                        element.4.velocity_y *= friction;
                        element.4.velocity_x *= friction;
                    }

                    // Spring back if out of bounds (only happens for touchpads since mice are clamped instantly)
                    if element.4.target_y < 0.0 {
                        element.4.target_y +=
                            (0.0 - element.4.target_y) * (1.0 - f32::exp(-10.0 * dt));
                    } else if element.4.target_y > max_y {
                        element.4.target_y +=
                            (max_y - element.4.target_y) * (1.0 - f32::exp(-10.0 * dt));
                    }

                    if element.4.target_x < 0.0 {
                        element.4.target_x +=
                            (0.0 - element.4.target_x) * (1.0 - f32::exp(-10.0 * dt));
                    } else if element.4.target_x > max_x {
                        element.4.target_x +=
                            (max_x - element.4.target_x) * (1.0 - f32::exp(-10.0 * dt));
                    }
                }

                let diff_y = element.4.target_y - element.4.current_y;
                let diff_x = element.4.target_x - element.4.current_x;

                if diff_y.abs() > 0.1
                    || diff_x.abs() > 0.1
                    || element.4.velocity_y.abs() > 0.1
                    || element.4.velocity_x.abs() > 0.1
                    || element.4.needs_sync
                {
                    element.4.needs_sync = false;
                    // Frame-rate independent exponential smoothing (logarithmic decay of distance)
                    let smoothing = 1.0 - f32::exp(-12.0 * dt);
                    element.4.current_y += diff_y * smoothing;
                    element.4.current_x += diff_x * smoothing;

                    let current_y = element.4.current_y;
                    let current_x = element.4.current_x;

                    element.1.update_constraints(ctx, |c| {
                        c.overflow = Overflow::Scroll;
                        c.scroll = Vector2 {
                            x: current_x,
                            y: current_y,
                        };
                    });

                    self.scrollbar.update_metrics(
                        &mut element.2,
                        ctx,
                        ScrollMetrics {
                            current_y,
                            max_y,
                            viewport_h: parent_comp.h,
                            content_h: inner_comp.h,
                            current_x,
                            max_x,
                            viewport_w: parent_comp.w,
                            content_w: inner_comp.w,
                        },
                    );

                    if let Some(w) = &ctx.window {
                        w.request_redraw();
                    }
                }
            }
            _ => {}
        }

        (result, emitted_msg)
    }
}
