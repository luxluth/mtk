use std::marker::PhantomData;
use std::path::PathBuf;

use crate::debugger::SourceLocation;
use crate::image::{ImageCache, ObjectFit};
use crate::style::Style;
use crate::ui::event::EventResult;
use crate::ui::{Event, View};
use crate::{Context, Node};

/// A widget that asynchronously streams and displays an image from a file path without blocking the UI thread.
pub struct AsyncImage<Msg> {
    pub(crate) path: PathBuf,
    pub(crate) fit: ObjectFit,
    pub(crate) style: Option<Style>,
    pub(crate) on_click: Option<Msg>,
    pub(crate) source_loc: Option<SourceLocation>,
    _marker: PhantomData<Msg>,
}

/// Creates a new `AsyncImage` widget that streams the image from disk in the background.
#[track_caller]
pub fn async_image<Msg>(path: impl Into<PathBuf>) -> AsyncImage<Msg> {
    AsyncImage {
        path: path.into(),
        fit: ObjectFit::default(),
        style: None,
        on_click: None,
        source_loc: Some(SourceLocation::here("AsyncImage")),
        _marker: PhantomData,
    }
}

impl<Msg> AsyncImage<Msg> {
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

pub struct AsyncImageElement {
    pub(crate) node: Node,
    pub(crate) current_path: PathBuf,
    pub(crate) attached_image_id: Option<u64>,
    is_pressed: bool,
    is_hovered: bool,
}

impl<State, Msg: Clone> View<State> for AsyncImage<Msg> {
    type Element = AsyncImageElement;
    type Message = Msg;

    fn build(&self, ctx: &mut Context) -> Self::Element {
        let node = ctx.create_node();
        if let Some(loc) = self.source_loc {
            ctx.set_node_source(node, loc);
        }

        if let Some(ref style) = self.style {
            style.apply_to_node(ctx, node);
        }

        let mut attached_image_id = None;

        if let Some(data) = ImageCache::global().get(&self.path) {
            if data.height > 0 && data.width > 0 {
                let intrinsic_ar = data.width as f32 / data.height as f32;
                node.update_constraints(ctx, |c| {
                    if c.aspect_ratio == 0.0 {
                        c.aspect_ratio = intrinsic_ar;
                    }
                });
            }
            ctx.images
                .borrow_mut()
                .insert(node, (data.clone(), self.fit));
            attached_image_id = Some(data.id);
        } else {
            let window = ctx.window();
            let path_clone = self.path.clone();
            ImageCache::global().load_async(
                path_clone,
                Some(move |_result| {
                    if let Some(win) = window {
                        win.request_redraw();
                    }
                }),
            );
        }

        AsyncImageElement {
            node,
            current_path: self.path.clone(),
            attached_image_id,
            is_pressed: false,
            is_hovered: false,
        }
    }

    fn rebuild(&self, _prev: &Self, ctx: &mut Context, element: &mut Self::Element) {
        if let Some(ref style) = self.style {
            style.apply_to_node(ctx, element.node);
        }

        let path_changed = element.current_path != self.path;
        if path_changed {
            element.current_path = self.path.clone();
            element.attached_image_id = None;
        }

        if let Some(data) = ImageCache::global().get(&self.path) {
            let needs_update = element.attached_image_id != Some(data.id);
            if needs_update {
                if data.height > 0 && data.width > 0 {
                    let intrinsic_ar = data.width as f32 / data.height as f32;
                    element.node.update_constraints(ctx, |c| {
                        if c.aspect_ratio == 0.0 {
                            c.aspect_ratio = intrinsic_ar;
                        }
                    });
                }
                ctx.images
                    .borrow_mut()
                    .insert(element.node, (data.clone(), self.fit));
                element.attached_image_id = Some(data.id);
                element.node.set_dirty(ctx);
            }
        } else if !ImageCache::global().is_loading(&self.path) {
            let window = ctx.window();
            let path_clone = self.path.clone();
            ImageCache::global().load_async(
                path_clone,
                Some(move |_result| {
                    if let Some(win) = window {
                        win.request_redraw();
                    }
                }),
            );
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
                    if !pressed && element.is_pressed {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::image::ImageData;

    #[test]
    fn test_async_image_lifecycle() {
        let mut ctx = Context::new();
        let path = std::path::PathBuf::from("/tmp/test_mtk_sample_image.png");

        // Seed cache
        let dummy_data = ImageData::from_rgba8(10, 10, vec![255; 400]).unwrap();
        ImageCache::global().insert(path.clone(), dummy_data.clone());

        let widget: AsyncImage<()> = async_image(path.clone());
        let mut element = View::<()>::build(&widget, &mut ctx);

        assert_eq!(element.attached_image_id, Some(dummy_data.id));

        View::<()>::rebuild(&widget, &widget, &mut ctx, &mut element);
        View::<()>::teardown(&widget, &mut ctx, &mut element);
    }
}
