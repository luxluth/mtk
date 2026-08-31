//! Image widget for displaying decoded bitmap graphics (PNG, JPEG, WebP, GIF, etc.).

use std::marker::PhantomData;

use crate::debugger::SourceLocation;
use crate::image::{ImageData, ObjectFit};
use crate::style::Style;
use crate::ui::event::EventResult;
use crate::ui::{Event, View};
use crate::{Context, Node};

/// A widget that displays a decoded raster image.
pub struct Image<Msg> {
    pub(crate) data: ImageData,
    pub(crate) fit: ObjectFit,
    pub(crate) style: Option<Style>,
    pub(crate) source_loc: Option<SourceLocation>,
    _marker: PhantomData<Msg>,
}

/// Creates a new `Image` widget displaying the provided `ImageData`.
#[track_caller]
pub fn image<Msg>(data: ImageData) -> Image<Msg> {
    Image {
        data,
        fit: ObjectFit::default(),
        style: None,
        source_loc: Some(SourceLocation::here("Image")),
        _marker: PhantomData,
    }
}

impl<Msg> Image<Msg> {
    /// Sets the object-fit mode (how the image scales to fit layout constraints).
    pub fn fit(mut self, fit: ObjectFit) -> Self {
        self.fit = fit;
        self
    }

    /// Sets custom styles (width, height, corner radius, borders, shadows) for the image container.
    pub fn style(mut self, style: Style) -> Self {
        self.style = Some(style);
        self
    }
}

pub struct ImageElement {
    pub(crate) node: Node,
}

impl<State, Msg> View<State> for Image<Msg> {
    type Element = ImageElement;
    type Message = Msg;

    fn build(&self, ctx: &mut Context) -> Self::Element {
        let node = ctx.create_node();
        if let Some(loc) = self.source_loc {
            ctx.set_node_source(node, loc);
        }

        if let Some(ref style) = self.style {
            style.apply_to_node(ctx, node);
        }

        if self.data.height > 0 && self.data.width > 0 {
            let intrinsic_ar = self.data.width as f32 / self.data.height as f32;
            node.update_constraints(ctx, |c| {
                if c.aspect_ratio == 0.0 {
                    c.aspect_ratio = intrinsic_ar;
                }
            });
        }

        ctx.images
            .borrow_mut()
            .insert(node, (self.data.clone(), self.fit));

        ImageElement { node }
    }

    fn rebuild(&self, prev: &Self, ctx: &mut Context, element: &mut Self::Element) {
        if self.data.id != prev.data.id || self.fit != prev.fit {
            ctx.images
                .borrow_mut()
                .insert(element.node, (self.data.clone(), self.fit));
            element.node.set_dirty(ctx);
        }

        if let Some(ref style) = self.style {
            style.apply_to_node(ctx, element.node);
        }

        if self.data.height > 0 && self.data.width > 0 {
            let intrinsic_ar = self.data.width as f32 / self.data.height as f32;
            element.node.update_constraints(ctx, |c| {
                if c.aspect_ratio == 0.0 {
                    c.aspect_ratio = intrinsic_ar;
                }
            });
        }
    }

    fn teardown(&self, ctx: &mut Context, element: &mut Self::Element) {
        ctx.images.borrow_mut().remove(&element.node);
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
