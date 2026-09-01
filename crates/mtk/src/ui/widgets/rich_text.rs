use std::marker::PhantomData;

use crate::debugger::SourceLocation;
use crate::style::{Rect, Style, TextStyle};
use crate::text::{TextRenderInfo, TextSpan};
use crate::ui::event::EventResult;
use crate::ui::{Event, View};
use crate::{Context, Node};

/// Geometric bounding box information for an interactive text span.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct SpanGeometry {
    /// Bounding rectangle in window/screen coordinates.
    pub rect: Rect,
    /// Bounding rectangle in local coordinates relative to the rich text widget.
    pub local_rect: Rect,
}

impl std::ops::Deref for SpanGeometry {
    type Target = Rect;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.rect
    }
}

/// A widget that displays styled rich text with syntax-highlighted spans and interactive hover/click callbacks.
pub struct RichText<Msg, Id = ()> {
    pub(crate) text: String,
    pub(crate) spans: Vec<TextSpan<Id>>,
    pub(crate) style: Option<Style>,
    pub(crate) text_style: Option<TextStyle>,
    pub(crate) on_span_click: Option<Box<dyn Fn(Id, SpanGeometry) -> Option<Msg>>>,
    pub(crate) on_span_hover: Option<Box<dyn Fn(Id, bool, SpanGeometry) -> Option<Msg>>>,
    pub(crate) source_loc: Option<SourceLocation>,
    _marker: PhantomData<Msg>,
}

/// Creates a new `RichText` widget displaying styled text with support for sub-range styling and span interactivity.
#[track_caller]
pub fn rich_text<S: Into<String>, Msg>(text: S) -> RichText<Msg, ()> {
    RichText {
        text: text.into(),
        spans: Vec::new(),
        style: None,
        text_style: None,
        on_span_click: None,
        on_span_hover: None,
        source_loc: Some(SourceLocation::here("RichText")),
        _marker: PhantomData,
    }
}

impl<Msg> RichText<Msg, ()> {
    /// Sets the list of styled spans over the text buffer, transitioning to an identified span model if IDs are provided.
    pub fn spans<Id: Clone + PartialEq + 'static>(
        self,
        spans: Vec<TextSpan<Id>>,
    ) -> RichText<Msg, Id> {
        RichText {
            text: self.text,
            spans,
            style: self.style,
            text_style: self.text_style,
            on_span_click: None,
            on_span_hover: None,
            source_loc: self.source_loc,
            _marker: PhantomData,
        }
    }
}

impl<Msg, Id: Clone + PartialEq + 'static> RichText<Msg, Id> {
    /// Replaces the list of styled spans with a new set of the same identifier type.
    pub fn set_spans(mut self, spans: Vec<TextSpan<Id>>) -> Self {
        self.spans = spans;
        self
    }

    /// Adds a single styled span over the text buffer.
    pub fn span(mut self, span: TextSpan<Id>) -> Self {
        self.spans.push(span);
        self
    }

    /// Sets the box layout and container style.
    pub fn style(mut self, style: Style) -> Self {
        self.style = Some(style);
        self
    }

    /// Sets the base text style (font size, font family, line height, wrap, etc.).
    pub fn text_style(mut self, style: TextStyle) -> Self {
        self.text_style = Some(style);
        self
    }

    /// Attaches a callback triggered when an identified span is clicked, passing the span ID and geometric `SpanGeometry`.
    pub fn on_span_click<F>(mut self, handler: F) -> Self
    where
        F: Fn(Id, SpanGeometry) -> Option<Msg> + 'static,
    {
        self.on_span_click = Some(Box::new(handler));
        self
    }

    /// Attaches a callback triggered when the mouse enters or leaves an identified span, passing the span ID, hover status, and geometric `SpanGeometry`.
    pub fn on_span_hover<F>(mut self, handler: F) -> Self
    where
        F: Fn(Id, bool, SpanGeometry) -> Option<Msg> + 'static,
    {
        self.on_span_hover = Some(Box::new(handler));
        self
    }
}

pub struct RichTextElement<Id> {
    pub(crate) node: Node,
    pub(crate) current_text: String,
    pub(crate) current_spans: Vec<TextSpan<Id>>,
    pub(crate) hovered_span: Option<Id>,
    pub(crate) hovered_span_geom: Option<SpanGeometry>,
}

fn compute_span_geometry(
    text: &str,
    text_style: &TextStyle,
    inner_w: f32,
    inner_h: f32,
    computed: &crate::Computed,
    padding: &crate::style::Edges,
    range: std::ops::Range<usize>,
    text_cx: &crate::text::SharedTextContext,
    spans: &[TextSpan<()>],
) -> SpanGeometry {
    let local_rects =
        crate::text::get_range_geometry(text, text_style, inner_w, inner_h, range, text_cx, spans);

    let (min_x, min_y, max_x, max_y) = local_rects.iter().fold(
        (
            f32::INFINITY,
            f32::INFINITY,
            f32::NEG_INFINITY,
            f32::NEG_INFINITY,
        ),
        |(mx, my, mx2, my2), r| {
            (
                mx.min(r[0]),
                my.min(r[1]),
                mx2.max(r[0] + r[2]),
                my2.max(r[1] + r[3]),
            )
        },
    );

    let local_rect = if min_x <= max_x {
        Rect {
            x: min_x + padding.left,
            y: min_y + padding.top,
            w: max_x - min_x,
            h: max_y - min_y,
        }
    } else {
        Rect::default()
    };

    let screen_rect = Rect {
        x: computed.x + local_rect.x,
        y: computed.y + local_rect.y,
        w: local_rect.w,
        h: local_rect.h,
    };

    SpanGeometry {
        rect: screen_rect,
        local_rect,
    }
}

impl<State, Msg: 'static, Id: Clone + PartialEq + 'static> View<State> for RichText<Msg, Id> {
    type Message = Msg;
    type Element = RichTextElement<Id>;

    fn build(&self, ctx: &mut Context) -> Self::Element {
        let node = ctx.create_node();
        let untyped_spans: Vec<TextSpan<()>> = self.spans.iter().map(|s| s.to_untyped()).collect();
        let base_style = self.text_style.clone().unwrap_or_default();
        let render_info = TextRenderInfo {
            style: base_style,
            cursor: None,
            selection: None,
            preedit_range: None,
            spans: untyped_spans,
        };
        node.set_text_with_userdata(ctx, &self.text, render_info);

        if let Some(ref style) = self.style {
            style.apply_to_node(ctx, node);
        }

        RichTextElement {
            node,
            current_text: self.text.clone(),
            current_spans: self.spans.clone(),
            hovered_span: None,
            hovered_span_geom: None,
        }
    }

    fn rebuild(&self, _prev: &Self, ctx: &mut Context, element: &mut Self::Element) {
        let text_changed = element.current_text != self.text;
        let spans_changed = element.current_spans != self.spans;

        if text_changed {
            element.current_text = self.text.clone();
        }

        if spans_changed || self.text_style.is_some() || text_changed {
            let untyped_spans: Vec<TextSpan<()>> =
                self.spans.iter().map(|s| s.to_untyped()).collect();
            let base_style = self.text_style.clone().unwrap_or_else(|| {
                element
                    .node
                    .get_text_userdata::<TextRenderInfo>(ctx)
                    .map(|i| i.style.clone())
                    .unwrap_or_default()
            });

            let render_info = TextRenderInfo {
                style: base_style,
                cursor: None,
                selection: None,
                preedit_range: None,
                spans: untyped_spans,
            };
            element
                .node
                .set_text_with_userdata(ctx, &element.current_text, render_info);
            element.current_spans = self.spans.clone();
        }

        if text_changed || spans_changed {
            element.node.set_dirty(ctx);
        }

        if let Some(ref style) = self.style {
            style.apply_to_node(ctx, element.node);
        }
    }

    fn teardown(&self, _ctx: &mut Context, _element: &mut Self::Element) {}

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
        match event {
            Event::CursorMoved { x, y, hit_nodes } => {
                let is_hit = hit_nodes.contains(&element.node);
                if is_hit {
                    if let Some(computed) = element.node.get_computed(ctx) {
                        let constraints = element.node.get_constraints(ctx).unwrap_or_default();
                        let rel_x =
                            x - computed.x - constraints.padding.left + constraints.scroll.x;
                        let rel_y = y - computed.y - constraints.padding.top + constraints.scroll.y;
                        let inner_w =
                            (computed.w - constraints.padding.left - constraints.padding.right)
                                .max(0.0);
                        let inner_h =
                            (computed.h - constraints.padding.top - constraints.padding.bottom)
                                .max(0.0);

                        let info = element.node.get_text_userdata::<TextRenderInfo>(ctx);
                        let text_style = info.map(|i| &i.style).cloned().unwrap_or_default();
                        let spans = info.map(|i| &i.spans[..]).unwrap_or(&[]);

                        let offset = crate::text::hit_test_text(
                            &element.current_text,
                            &text_style,
                            inner_w,
                            inner_h,
                            rel_x,
                            rel_y,
                            &ctx.text_context,
                            spans,
                        );

                        let matched_span = element
                            .current_spans
                            .iter()
                            .find(|s| s.range.contains(&offset));
                        let new_hover_id = matched_span.and_then(|s| s.id.clone());

                        if new_hover_id != element.hovered_span {
                            let old_id = element.hovered_span.take();
                            let old_geom = element.hovered_span_geom.take().unwrap_or_default();

                            let new_geom = matched_span.map(|s| {
                                compute_span_geometry(
                                    &element.current_text,
                                    &text_style,
                                    inner_w,
                                    inner_h,
                                    &computed,
                                    &constraints.padding,
                                    s.range.clone(),
                                    &ctx.text_context,
                                    spans,
                                )
                            });

                            element.hovered_span = new_hover_id.clone();
                            element.hovered_span_geom = new_geom;

                            if let Some(ref on_hover) = self.on_span_hover {
                                if let Some(old_id) = old_id {
                                    if let Some(msg) = on_hover(old_id, false, old_geom) {
                                        return (EventResult::Handled, Some(msg));
                                    }
                                }
                                if let (Some(new_id), Some(geom)) = (new_hover_id, new_geom) {
                                    if let Some(msg) = on_hover(new_id, true, geom) {
                                        return (EventResult::Handled, Some(msg));
                                    }
                                }
                            }
                        }
                    }
                } else if element.hovered_span.is_some() {
                    let old_id = element.hovered_span.take().unwrap();
                    let old_geom = element.hovered_span_geom.take().unwrap_or_default();
                    if let Some(ref on_hover) = self.on_span_hover {
                        if let Some(msg) = on_hover(old_id, false, old_geom) {
                            return (EventResult::Handled, Some(msg));
                        }
                    }
                }
                (EventResult::Ignored, None)
            }
            Event::MouseInput {
                pressed: false,
                hit_nodes,
                ..
            } => {
                if hit_nodes.contains(&element.node) {
                    if let (Some(id), Some(geom)) =
                        (&element.hovered_span, element.hovered_span_geom)
                    {
                        if let Some(ref on_click) = self.on_span_click {
                            if let Some(msg) = on_click(id.clone(), geom) {
                                return (EventResult::Handled, Some(msg));
                            }
                        }
                    }
                }
                (EventResult::Ignored, None)
            }
            _ => (EventResult::Ignored, None),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clr;

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum Token {
        Keyword,
        FunctionName,
    }

    #[derive(Clone, Debug, PartialEq)]
    enum TestMsg {
        Hovered(Token, bool, SpanGeometry),
        Clicked(Token, SpanGeometry),
    }

    #[test]
    fn test_rich_text_lifecycle_and_spans() {
        let mut ctx = Context::new();
        let widget = rich_text("fn calculate()")
            .spans(vec![
                TextSpan::new(0..2).color(clr!(blue)).id(Token::Keyword),
                TextSpan::new(3..12).bold().id(Token::FunctionName),
            ])
            .on_span_hover(|id, h, geom| Some(TestMsg::Hovered(id, h, geom)))
            .on_span_click(|id, geom| Some(TestMsg::Clicked(id, geom)));

        let mut element = View::<()>::build(&widget, &mut ctx);

        let info = element
            .node
            .get_text_userdata::<TextRenderInfo>(&ctx)
            .unwrap();
        assert_eq!(info.spans.len(), 2);
        assert_eq!(info.spans[0].range, 0..2);
        assert_eq!(info.spans[1].range, 3..12);

        // Rebuild with updated spans
        let updated_widget = rich_text("fn calculate()").spans(vec![
            TextSpan::new(0..2).color(clr!(red)).id(Token::Keyword),
        ]);
        View::<()>::rebuild(&updated_widget, &widget, &mut ctx, &mut element);

        let info_updated = element
            .node
            .get_text_userdata::<TextRenderInfo>(&ctx)
            .unwrap();
        assert_eq!(info_updated.spans.len(), 1);
        assert_eq!(info_updated.spans[0].style.color, Some(clr!(red)));
    }

    #[test]
    fn test_rich_text_range_geometry() {
        let mut ctx = Context::new();
        let widget = rich_text::<_, ()>("pub fn calculate(x: i32) -> bool").spans(vec![
            TextSpan::new(0..3).color(clr!(blue)).id(Token::Keyword),
            TextSpan::new(7..16).bold().id(Token::FunctionName),
        ]);

        let element = View::<()>::build(&widget, &mut ctx);

        // Query geometry of the FunctionName span ("calculate")
        let rects = element.node.get_text_range_geometry(&ctx, 7..16);
        assert!(
            !rects.is_empty(),
            "Must produce bounding rects for token span"
        );
        assert!(rects[0][2] > 0.0, "Width of token must be positive");
        assert!(rects[0][3] > 0.0, "Height of token must be positive");
    }
}
