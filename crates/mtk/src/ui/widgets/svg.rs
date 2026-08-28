//! SVG widget for displaying vector graphics with resolution-aware GPU caching.

use std::marker::PhantomData;

use crate::colors::Color;
use crate::debugger::SourceLocation;
use crate::image::{ObjectFit, SvgData};
use crate::style::Style;
use crate::ui::event::EventResult;
use crate::ui::{Event, View};
use crate::{Context, Node};

/// A widget that displays a resolution-independent vector SVG.
pub struct Svg<Msg> {
    pub(crate) data: SvgData,
    pub(crate) fit: ObjectFit,
    pub(crate) color: Option<Color>,
    pub(crate) style: Option<Style>,
    pub(crate) on_click: Option<Msg>,
    pub(crate) source_loc: Option<SourceLocation>,
    _marker: PhantomData<Msg>,
}

/// Creates a new `Svg` widget displaying the provided `SvgData`.
#[track_caller]
pub fn svg<Msg>(data: SvgData) -> Svg<Msg> {
    Svg {
        data,
        fit: ObjectFit::default(),
        color: None,
        style: None,
        on_click: None,
        source_loc: Some(SourceLocation::here("Svg")),
        _marker: PhantomData,
    }
}

impl<Msg> Svg<Msg> {
    /// Sets the object-fit mode (how the SVG scales to fit layout constraints).
    pub fn fit(mut self, fit: ObjectFit) -> Self {
        self.fit = fit;
        self
    }

    /// Sets the CSS `currentColor` value for the SVG.
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    /// Sets custom styles (width, height, corner radius, borders, shadows) for the SVG container.
    pub fn style(mut self, style: Style) -> Self {
        self.style = Some(style);
        self
    }

    /// Sets the message to emit when the SVG is clicked.
    pub fn on_click(mut self, msg: Msg) -> Self {
        self.on_click = Some(msg);
        self
    }
}

pub struct SvgElement {
    pub(crate) node: Node,
    is_pressed: bool,
    is_hovered: bool,
}

impl<State, Msg: Clone> View<State> for Svg<Msg> {
    type Element = SvgElement;
    type Message = Msg;

    fn build(&self, ctx: &mut Context) -> Self::Element {
        let node = ctx.create_node();
        if let Some(loc) = self.source_loc {
            ctx.set_node_source(node, loc);
        }

        if let Some(ref style) = self.style {
            style.apply_to_node(ctx, node);
        }

        let data = if let Some(color) = self.color {
            self.data
                .with_color(color)
                .unwrap_or_else(|_| self.data.clone())
        } else {
            self.data.clone()
        };

        ctx.svgs.borrow_mut().insert(node, (data, self.fit));

        SvgElement {
            node,
            is_pressed: false,
            is_hovered: false,
        }
    }

    fn rebuild(&self, prev: &Self, ctx: &mut Context, element: &mut Self::Element) {
        let data = if let Some(color) = self.color {
            self.data
                .with_color(color)
                .unwrap_or_else(|_| self.data.clone())
        } else {
            self.data.clone()
        };

        if self.data.id != prev.data.id || self.fit != prev.fit || self.color != prev.color {
            ctx.svgs.borrow_mut().insert(element.node, (data, self.fit));
            element.node.set_dirty(ctx);
        }

        if let Some(ref style) = self.style {
            style.apply_to_node(ctx, element.node);
        }
    }

    fn teardown(&self, ctx: &mut Context, element: &mut Self::Element) {
        ctx.svgs.borrow_mut().remove(&element.node);
        element.node.remove(ctx);
        ctx.destroy_node(element.node);
    }

    fn get_node(&self, element: &Self::Element) -> Node {
        element.node
    }

    fn handle_event(
        &self,
        element: &mut Self::Element,
        _state: &State,
        event: Event,
        ctx: &mut Context,
    ) -> (EventResult, Option<Self::Message>) {
        if self.on_click.is_none() {
            return (EventResult::Ignored, None);
        }

        match event {
            Event::CursorMoved { hit_nodes, .. } => {
                let is_hit = hit_nodes.contains(&element.node);
                if is_hit != element.is_hovered {
                    element.is_hovered = is_hit;
                    element.node.set_dirty(ctx);
                }
                (EventResult::Ignored, None)
            }
            Event::MouseInput {
                pressed, hit_nodes, ..
            } => {
                let is_hit = hit_nodes.contains(&element.node);
                if is_hit {
                    if pressed {
                        element.is_pressed = true;
                        element.node.set_dirty(ctx);
                        (EventResult::Handled, None)
                    } else if element.is_pressed {
                        element.is_pressed = false;
                        element.node.set_dirty(ctx);
                        (EventResult::Handled, self.on_click.clone())
                    } else {
                        (EventResult::Ignored, None)
                    }
                } else {
                    if element.is_pressed {
                        element.is_pressed = false;
                        element.node.set_dirty(ctx);
                    }
                    (EventResult::Ignored, None)
                }
            }
            _ => (EventResult::Ignored, None),
        }
    }
}
