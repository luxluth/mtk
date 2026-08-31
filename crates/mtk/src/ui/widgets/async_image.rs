use std::marker::PhantomData;
use std::path::PathBuf;

use crate::debugger::SourceLocation;
use crate::image::{CacheKey, ImageCache, ObjectFit};
use crate::style::Style;
use crate::ui::event::EventResult;
use crate::ui::{Event, View};
use crate::{Context, Node};

/// A widget that asynchronously streams and displays an image from a file path without blocking the UI thread.
///
/// Automatically downscales large images in the background to match the node's computed layout resolution and display DPI.
pub struct AsyncImage<Msg> {
    pub(crate) path: PathBuf,
    pub(crate) fit: ObjectFit,
    pub(crate) style: Option<Style>,
    pub(crate) max_dim: Option<(u32, u32)>,
    pub(crate) source_loc: Option<SourceLocation>,
    _marker: PhantomData<Msg>,
}

/// Creates a new `AsyncImage` widget that streams and automatically scales the image from disk in the background.
#[track_caller]
pub fn async_image<Msg>(path: impl Into<PathBuf>) -> AsyncImage<Msg> {
    AsyncImage {
        path: path.into(),
        fit: ObjectFit::default(),
        style: None,
        max_dim: None,
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

    /// Explicitly overrides the maximum thumbnail bounds in physical pixels.
    pub fn max_dimension(mut self, width: u32, height: u32) -> Self {
        self.max_dim = Some((width.max(1), height.max(1)));
        self
    }

    fn resolve_target_dim(&self, ctx: &Context, node: Node) -> Option<(u32, u32)> {
        if let Some(dim) = self.max_dim {
            return Some(dim);
        }

        let scale = ctx.scale_factor.max(1.0);

        // 1. Check computed layout bounds from layout pass
        if let Some(comp) = node.get_computed(ctx) {
            if comp.w > 0.0 && comp.h > 0.0 {
                let tw = (comp.w * scale).ceil().max(1.0) as u32;
                let th = (comp.h * scale).ceil().max(1.0) as u32;
                return Some((tw, th));
            }
        }

        // 2. Check explicit style constraints on node
        if let Some(cons) = node.get_constraints(ctx) {
            let w = match cons.width {
                crate::style::Size::Fixed(v) if v > 0 => {
                    Some(((v as f32) * scale).ceil().max(1.0) as u32)
                }
                _ => None,
            };
            let h = match cons.height {
                crate::style::Size::Fixed(v) if v > 0 => {
                    Some(((v as f32) * scale).ceil().max(1.0) as u32)
                }
                _ => None,
            };
            match (w, h) {
                (Some(tw), Some(th)) => return Some((tw, th)),
                (Some(tw), None) => return Some((tw, tw)),
                (None, Some(th)) => return Some((th, th)),
                (None, None) => {}
            }
        }

        // Default ceiling for unconstrained layouts
        Some((1024, 1024))
    }
}

pub struct AsyncImageElement {
    pub(crate) node: Node,
    pub(crate) current_path: PathBuf,
    pub(crate) target_dim: Option<(u32, u32)>,
    pub(crate) attached_image_id: Option<u64>,
}

impl<State, Msg> View<State> for AsyncImage<Msg> {
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

        let target_dim = self.resolve_target_dim(ctx, node);
        let key = CacheKey {
            path: self.path.clone(),
            max_dim: target_dim,
        };

        let mut attached_image_id = None;

        if let Some(data) = ImageCache::global().get_keyed(&key) {
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
            ImageCache::global().load_async_keyed(
                key,
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
            target_dim,
            attached_image_id,
        }
    }

    fn rebuild(&self, _prev: &Self, ctx: &mut Context, element: &mut Self::Element) {
        if let Some(ref style) = self.style {
            style.apply_to_node(ctx, element.node);
        }

        let target_dim = self.resolve_target_dim(ctx, element.node);
        let path_changed = element.current_path != self.path || element.target_dim != target_dim;
        if path_changed {
            element.current_path = self.path.clone();
            element.target_dim = target_dim;
            element.attached_image_id = None;
        }

        let key = CacheKey {
            path: self.path.clone(),
            max_dim: target_dim,
        };

        if let Some(data) = ImageCache::global().get_keyed(&key) {
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
        } else if !ImageCache::global().is_loading_keyed(&key) {
            let window = ctx.window();
            ImageCache::global().load_async_keyed(
                key,
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
        if matches!(event, Event::Tick { .. }) {
            if element.attached_image_id.is_none() {
                let target_dim = self.resolve_target_dim(ctx, element.node);
                let key = CacheKey {
                    path: element.current_path.clone(),
                    max_dim: target_dim,
                };
                if let Some(data) = ImageCache::global().get_keyed(&key) {
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
                    ctx.request_frame();
                }
            }
        }

        (EventResult::Ignored, None)
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

    #[test]
    fn test_async_image_tick_resolution() {
        let mut ctx = Context::new();
        let path = std::path::PathBuf::from("/virtual/delayed_image.png");

        // Build before image is in cache
        let widget: AsyncImage<()> = async_image(path.clone());
        let mut element = View::<()>::build(&widget, &mut ctx);
        assert_eq!(element.attached_image_id, None);

        // Later, image finishes decoding into cache
        let dummy_data = ImageData::from_rgba8(4, 4, vec![100; 64]).unwrap();
        ImageCache::global().insert(path.clone(), dummy_data.clone());

        // Event::Tick arrives
        widget.handle_event(&mut element, &(), Event::Tick { dt: 0.016 }, &mut ctx);

        // Should now be attached immediately on first tick
        assert_eq!(element.attached_image_id, Some(dummy_data.id));
        assert!(ctx.images.borrow().contains_key(&element.node));

        View::<()>::teardown(&widget, &mut ctx, &mut element);
    }
}
