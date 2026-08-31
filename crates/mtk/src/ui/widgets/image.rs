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
    pub(crate) on_click: Option<Msg>,
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
        on_click: None,
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

    /// Sets the message to emit when the image is clicked.
    pub fn on_click(mut self, msg: Msg) -> Self {
        self.on_click = Some(msg);
        self
    }
}

pub struct ImageElement {
    pub(crate) node: Node,
    is_pressed: bool,
    is_hovered: bool,
}

impl<State, Msg: Clone> View<State> for Image<Msg> {
    type Element = ImageElement;
    type Message = Msg;

    fn build(&self, ctx: &mut Context) -> Self::Element {
        let node = ctx.create_node();
        if let Some(loc) = self.source_loc {
            ctx.set_node_source(node, loc);
        }

        // Set default intrinsic sizing if no style overrides width/height
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

        ImageElement {
            node,
            is_pressed: false,
            is_hovered: false,
        }
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
