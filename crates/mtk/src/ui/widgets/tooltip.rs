use std::time::Instant;

use crate::debugger::SourceLocation;
use crate::style::{PositionStrategy, Style, TextStyle};
use crate::text_property::FontWeight;
use crate::ui::event::EventResult;
use crate::ui::{Event, View};
use crate::{Context, Node, clr, rgb, rgba};

/// A hover tooltip wrapper component that displays a floating hint when hovering over its child view.
pub struct Tooltip<V> {
    pub(crate) inner: V,
    pub(crate) text: String,
    pub(crate) source_loc: Option<SourceLocation>,
}

/// Wraps a view with a hover tooltip.
///
/// # Examples
/// ```rust,ignore
/// tooltip(button("Save"), "Saves current changes to disk")
/// ```
#[track_caller]
pub fn tooltip<V>(inner: V, text: impl Into<String>) -> Tooltip<V> {
    Tooltip {
        inner,
        text: text.into(),
        source_loc: Some(SourceLocation::here("Tooltip")),
    }
}

pub struct TooltipElement<E> {
    inner_el: E,
    tooltip_node: Node,
    is_attached: bool,
    hover_start: Option<Instant>,
}

impl<State, V: View<State>> View<State> for Tooltip<V> {
    type Element = TooltipElement<V::Element>;
    type Message = V::Message;

    fn build(&self, ctx: &mut Context) -> Self::Element {
        let inner_el = self.inner.build(ctx);
        let tooltip_node = ctx.create_node();
        if let Some(loc) = self.source_loc {
            ctx.set_node_source(tooltip_node, loc);
        }

        tooltip_node.set_text(ctx, &self.text);

        Style::new()
            .position(PositionStrategy::Absolute {
                top: -28.0,
                left: 0.0,
                bottom: f32::NAN,
                right: f32::NAN,
            })
            .padding_xy(8.0, 4.0)
            .corner_radius(4.0)
            .bg_color(rgb!(15, 23, 42))
            .border(1.0, rgb!(51, 65, 85))
            .shadow(rgba!(0, 0, 0, 80), 8.0, 0.5)
            .set_text_style(TextStyle {
                font_size: 11.0,
                font_weight: FontWeight::MEDIUM,
                color: clr!(white),
                wrap: false,
                ..Default::default()
            })
            .z_index(3000)
            .apply_to_node(ctx, tooltip_node);

        TooltipElement {
            inner_el,
            tooltip_node,
            is_attached: false,
            hover_start: None,
        }
    }

    fn rebuild(&self, prev: &Self, ctx: &mut Context, element: &mut Self::Element) {
        self.inner.rebuild(&prev.inner, ctx, &mut element.inner_el);
        if ctx.active_layer != crate::layer::ActiveLayerId::Base && element.is_attached {
            element.tooltip_node.remove(ctx);
            element.is_attached = false;
            element.hover_start = None;
        }
        if self.text != prev.text {
            element.tooltip_node.set_text_with_userdata(
                ctx,
                &self.text,
                TextStyle {
                    font_size: 11.0,
                    font_weight: FontWeight::MEDIUM,
                    color: clr!(white),
                    ..Default::default()
                },
            );
        }
    }

    fn teardown(&self, ctx: &mut Context, element: &mut Self::Element) {
        if element.is_attached {
            element.tooltip_node.remove(ctx);
        }
        ctx.destroy_node(element.tooltip_node);
        self.inner.teardown(ctx, &mut element.inner_el);
    }

    fn get_node(&self, element: &Self::Element) -> Node {
        self.inner.get_node(&element.inner_el)
    }

    fn handle_event(
        &self,
        element: &mut Self::Element,
        state: &State,
        event: Event,
        ctx: &mut Context,
    ) -> (EventResult, Option<Self::Message>) {
        let node = self.inner.get_node(&element.inner_el);

        match &event {
            Event::CursorMoved { hit_nodes, .. } => {
                let is_hit = hit_nodes.contains(&node)
                    && ctx.active_layer == crate::layer::ActiveLayerId::Base;
                if is_hit {
                    if element.hover_start.is_none() {
                        element.hover_start = Some(Instant::now());
                    }
                    if !element.is_attached {
                        node.append(ctx, element.tooltip_node);
                        element.is_attached = true;
                    }
                } else if element.is_attached {
                    element.tooltip_node.remove(ctx);
                    element.is_attached = false;
                    element.hover_start = None;
                }
            }
            Event::MouseInput { pressed: true, .. } => {
                if element.is_attached {
                    element.tooltip_node.remove(ctx);
                    element.is_attached = false;
                    element.hover_start = None;
                }
            }
            _ => {}
        }

        self.inner
            .handle_event(&mut element.inner_el, state, event, ctx)
    }
}
