use std::marker::PhantomData;

use crate::{
    Context, Node, PositionStrategy, Size,
    colors::Color,
    effects::Radius,
    style::{Overflow, Vector2},
    ui::{Event, View, event::EventResult},
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ScrollAxis {
    Horizontal,
    Vertical,
    Both,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ScrollOffset {
    Percent(f32),
    Pixel(f32),
}

#[derive(Clone, Copy, Debug)]
pub struct ScrollMetrics {
    pub current_y: f32,
    pub max_y: f32,
    pub viewport_h: f32,
    pub content_h: f32,
    pub current_x: f32,
    pub max_x: f32,
    pub viewport_w: f32,
    pub content_w: f32,
}

pub trait ScrollBar<State>: View<State> {
    fn update_metrics(
        &self,
        element: &mut Self::Element,
        ctx: &mut Context,
        metrics: ScrollMetrics,
    );
}

pub struct DefaultScrollBar<Msg> {
    _marker: PhantomData<Msg>,
}

impl<State, Msg> View<State> for DefaultScrollBar<Msg> {
    type Element = Node;
    type Message = Msg;

    fn build(&self, ctx: &mut Context) -> Self::Element {
        let node = ctx.create_node();
        node.update_constraints(ctx, |c| {
            c.positioning = PositionStrategy::Absolute {
                top: 0.0,
                left: f32::NAN,
                bottom: f32::NAN,
                right: 4.0,
            };
            c.width = Size::Fixed(6);
            c.height = Size::Fixed(0);
            c.z_index = 100;
        });
        node.update_effects(ctx, |e| {
            e.background_color = Color::new(100, 100, 100, 150);
            e.border.radius = Radius::all(3.0);
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

impl<State, Msg> ScrollBar<State> for DefaultScrollBar<Msg> {
    fn update_metrics(
        &self,
        element: &mut Self::Element,
        ctx: &mut Context,
        metrics: ScrollMetrics,
    ) {
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

            element.update_constraints(ctx, |c| {
                c.height = crate::style::Size::Fixed(final_h as u32);
                c.positioning = crate::style::PositionStrategy::Absolute {
                    top: top_pos,
                    left: f32::NAN,
                    bottom: f32::NAN,
                    right: 4.0,
                };
            });
        } else {
            element.update_constraints(ctx, |c| {
                c.height = crate::style::Size::Fixed(0);
            });
        }
    }
}

pub struct NoScrollBar<Msg> {
    _marker: PhantomData<Msg>,
}

impl<State, Msg> View<State> for NoScrollBar<Msg> {
    type Element = Node;
    type Message = Msg;

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

impl<State, Msg> ScrollBar<State> for NoScrollBar<Msg> {
    fn update_metrics(
        &self,
        _element: &mut Self::Element,
        _ctx: &mut Context,
        _metrics: ScrollMetrics,
    ) {
    }
}

pub struct ScrollView<V, S> {
    pub(crate) inner: V,
    pub(crate) axis: ScrollAxis,
    pub(crate) initial_x: Option<ScrollOffset>,
    pub(crate) initial_y: Option<ScrollOffset>,
    pub(crate) scrollbar: S,
}

pub fn scroll_view<V, Msg>(inner: V) -> ScrollView<V, DefaultScrollBar<Msg>> {
    ScrollView {
        inner,
        axis: ScrollAxis::Both,
        initial_x: None,
        initial_y: None,
        scrollbar: DefaultScrollBar {
            _marker: PhantomData,
        },
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

    pub fn no_scrollbar<Msg>(self) -> ScrollView<V, NoScrollBar<Msg>> {
        ScrollView {
            inner: self.inner,
            axis: self.axis,
            initial_x: self.initial_x,
            initial_y: self.initial_y,
            scrollbar: NoScrollBar {
                _marker: PhantomData,
            },
        }
    }
}

pub struct ScrollState {
    pub current_y: f32,
    pub target_y: f32,
    pub current_x: f32,
    pub target_x: f32,
    pub active_timer: f32,
    pub is_touching: bool,
    pub velocity_y: f32,
    pub velocity_x: f32,
    pub is_initialized: bool,
}

impl<State, Msg, V, S> View<State> for ScrollView<V, S>
where
    V: View<State, Message = Msg>,
    S: ScrollBar<State> + View<State, Message = Msg>,
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
                active_timer: 0.0,
                is_touching: false,
                velocity_y: 0.0,
                velocity_x: 0.0,
                is_initialized: false,
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
        let (mut result, mut emitted_msg) =
            self.scrollbar
                .handle_event(&mut element.2, state, event.clone(), ctx);

        if result == EventResult::Ignored {
            let (inner_res, inner_msg) =
                self.inner
                    .handle_event(&mut element.3, state, event.clone(), ctx);
            result = inner_res;
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
            Event::MouseWheel {
                delta_x,
                delta_y,
                is_touchpad,
                phase,
                hit_nodes,
            } => {
                if hit_nodes.contains(&element.1) {
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
                {
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
