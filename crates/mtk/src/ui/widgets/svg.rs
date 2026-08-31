//! SVG widget for displaying vector graphics with resolution-aware GPU caching.

use std::marker::PhantomData;

use crate::colors::Color;
use crate::debugger::SourceLocation;
use crate::image::{ObjectFit, SvgData, SvgStyle};
use crate::style::Style;
use crate::ui::event::EventResult;
use crate::ui::{Event, View};
use crate::{Context, Node};

/// A widget that displays a resolution-independent vector SVG.
pub struct Svg<Msg> {
    pub(crate) data: SvgData,
    pub(crate) fit: ObjectFit,
    pub(crate) style_opts: SvgStyle,
    pub(crate) style: Option<Style>,
    pub(crate) source_loc: Option<SourceLocation>,
    _marker: PhantomData<Msg>,
}

/// Creates a new `Svg` widget displaying the provided `SvgData`.
#[track_caller]
pub fn svg<Msg>(data: SvgData) -> Svg<Msg> {
    Svg {
        data,
        fit: ObjectFit::default(),
        style_opts: SvgStyle::default(),
        style: None,
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
        self.style_opts.color = Some(color);
        self
    }

    /// Sets the `fill` color for SVG elements (use `Color::new(0, 0, 0, 0)` for `fill: none`).
    pub fn fill(mut self, fill: Color) -> Self {
        self.style_opts.fill = Some(fill);
        self
    }

    /// Sets the `stroke` color for SVG elements (use `Color::new(0, 0, 0, 0)` for `stroke: none`).
    pub fn stroke(mut self, stroke: Color) -> Self {
        self.style_opts.stroke = Some(stroke);
        self
    }

    /// Sets the `stroke-width` in pixels for SVG elements.
    pub fn stroke_width(mut self, width: f32) -> Self {
        self.style_opts.stroke_width = Some(width);
        self
    }

    /// Appends a raw custom CSS stylesheet string for `usvg`.
    pub fn custom_css(mut self, css: impl Into<String>) -> Self {
        self.style_opts.custom_css = Some(css.into());
        self
    }

    /// Sets comprehensive dynamic SVG CSS styling options.
    pub fn svg_style(mut self, style: SvgStyle) -> Self {
        self.style_opts = style;
        self
    }

    /// Sets custom layout styles (width, height, corner radius, borders, shadows) for the SVG container.
    pub fn style(mut self, style: Style) -> Self {
        self.style = Some(style);
        self
    }
}

pub struct SvgElement {
    pub(crate) node: Node,
}

impl<State, Msg> View<State> for Svg<Msg> {
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

        if self.data.height > 0.0 && self.data.width > 0.0 {
            let intrinsic_ar = self.data.width / self.data.height;
            node.update_constraints(ctx, |c| {
                if c.aspect_ratio == 0.0 {
                    c.aspect_ratio = intrinsic_ar;
                }
            });
        }

        let data = if self.style_opts != SvgStyle::default() {
            self.data
                .with_style(&self.style_opts)
                .unwrap_or_else(|_| self.data.clone())
        } else {
            self.data.clone()
        };

        ctx.svgs.borrow_mut().insert(node, (data, self.fit));

        SvgElement { node }
    }

    fn rebuild(&self, prev: &Self, ctx: &mut Context, element: &mut Self::Element) {
        let data = if self.style_opts != SvgStyle::default() {
            self.data
                .with_style(&self.style_opts)
                .unwrap_or_else(|_| self.data.clone())
        } else {
            self.data.clone()
        };

        if self.data.id != prev.data.id
            || self.fit != prev.fit
            || self.style_opts != prev.style_opts
        {
            ctx.svgs.borrow_mut().insert(element.node, (data, self.fit));
            element.node.set_dirty(ctx);
        }

        if let Some(ref style) = self.style {
            style.apply_to_node(ctx, element.node);
        }

        if self.data.height > 0.0 && self.data.width > 0.0 {
            let intrinsic_ar = self.data.width / self.data.height;
            element.node.update_constraints(ctx, |c| {
                if c.aspect_ratio == 0.0 {
                    c.aspect_ratio = intrinsic_ar;
                }
            });
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
        _element: &mut Self::Element,
        _state: &State,
        _event: Event,
        _ctx: &mut Context,
    ) -> (EventResult, Option<Self::Message>) {
        (EventResult::Ignored, None)
    }
}
