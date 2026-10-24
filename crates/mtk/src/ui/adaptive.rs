//! Responsive container queries and layout-adaptive sub-views.
//!
//! Provides [`adaptive`] and [`Adaptive`], allowing view subtrees to dynamically reconfigure
//! their structure and styling based on their parent container's resolved layout bounds ([`Computed`]).

use std::marker::PhantomData;

use crate::debugger::SourceLocation;
use crate::layout::{Computed, Size};
use crate::style::Style;
use crate::ui::event::EventResult;
use crate::ui::{Event, View};
use crate::{Context, Node};

/// A container view whose child view tree is dynamically constructed from its resolved layout bounds.
pub struct Adaptive<F, State, Msg> {
    pub(crate) builder: F,
    pub(crate) style: Option<Style>,
    pub(crate) source_loc: Option<SourceLocation>,
    pub(crate) _marker: PhantomData<(State, Msg)>,
}

/// Creates a new responsive container view that adapts its child view based on resolved layout geometry.
///
/// The provided closure receives the container's [`Computed`] bounding box (which provides resolved
/// width `.w`, height `.h`, etc.) and returns the corresponding child view.
///
/// # Examples
/// ```rust,ignore
/// adaptive(|bounds| {
///     if bounds.w > 600.0 {
///         Either::Left(row((sidebar(), main_panel())))
///     } else {
///         Either::Right(column((mobile_nav(), main_panel())))
///     }
/// })
/// ```
#[track_caller]
pub fn adaptive<F, V, State>(builder: F) -> Adaptive<F, State, V::Message>
where
    F: Fn(Computed) -> V,
    V: View<State>,
{
    Adaptive {
        builder,
        style: None,
        source_loc: Some(SourceLocation::here("Adaptive")),
        _marker: PhantomData,
    }
}

impl<F, State, Msg> Adaptive<F, State, Msg> {
    /// Applies custom container styling to the adaptive wrapper node.
    pub fn style(mut self, style: Style) -> Self {
        self.style = Some(style);
        self
    }
}

/// Persistent DOM element state for an [`Adaptive`] view.
pub struct AdaptiveElement<V: View<State>, State> {
    pub(crate) container_node: Node,
    pub(crate) inner_view: Option<V>,
    pub(crate) inner_element: Option<V::Element>,
    pub(crate) last_computed: Option<Computed>,
    pub(crate) _marker: PhantomData<State>,
}

impl<F, V, State, Msg> View<State> for Adaptive<F, State, Msg>
where
    F: Fn(Computed) -> V,
    V: View<State, Message = Msg>,
{
    type Element = AdaptiveElement<V, State>;
    type Message = Msg;

    fn build(&self, ctx: &mut Context) -> Self::Element {
        let container_node = ctx.create_node();
        if let Some(loc) = self.source_loc {
            ctx.set_node_source(container_node, loc);
        }

        container_node.update_constraints(ctx, |c| {
            c.width = Size::Fill;
            c.height = Size::Fit;
        });

        if let Some(style) = &self.style {
            style.apply_to_node(ctx, container_node);
        }

        let initial_comp = container_node.get_computed(ctx).unwrap_or_default();
        let inner_view = (self.builder)(initial_comp);
        let inner_element = inner_view.build(ctx);
        let inner_node = inner_view.get_node(&inner_element);
        container_node.append(ctx, inner_node);

        AdaptiveElement {
            container_node,
            inner_view: Some(inner_view),
            inner_element: Some(inner_element),
            last_computed: None,
            _marker: PhantomData,
        }
    }

    fn rebuild(&self, _prev: &Self, ctx: &mut Context, element: &mut Self::Element) {
        if let Some(style) = &self.style {
            style.apply_to_node(ctx, element.container_node);
        }

        let computed = element.last_computed.unwrap_or_default();
        let new_inner_view = (self.builder)(computed);
        if let (Some(old_inner_view), Some(inner_element)) =
            (element.inner_view.as_ref(), element.inner_element.as_mut())
        {
            new_inner_view.rebuild_with_parent(
                old_inner_view,
                ctx,
                inner_element,
                element.container_node,
                None,
            );
        }
        element.inner_view = Some(new_inner_view);
    }

    fn rebuild_with_parent(
        &self,
        prev: &Self,
        ctx: &mut Context,
        element: &mut Self::Element,
        parent: Node,
        next_sibling: Option<Node>,
    ) {
        self.rebuild(prev, ctx, element);
        if element.container_node.parent(ctx) != Some(parent) {
            if let Some(sibling) = next_sibling {
                element.container_node.put_before(ctx, sibling);
            } else {
                parent.append(ctx, element.container_node);
            }
        }
    }

    fn teardown(&self, ctx: &mut Context, element: &mut Self::Element) {
        if let (Some(inner_view), Some(mut inner_element)) =
            (element.inner_view.take(), element.inner_element.take())
        {
            inner_view.teardown(ctx, &mut inner_element);
        }
        element.container_node.remove(ctx);
        ctx.destroy_node(element.container_node);
    }

    fn get_node(&self, element: &Self::Element) -> Node {
        element.container_node
    }

    fn on_layout(&self, element: &mut Self::Element, computed: Computed, ctx: &mut Context) {
        let changed = match element.last_computed {
            Some(prev) => (prev.w - computed.w).abs() > 0.5 || (prev.h - computed.h).abs() > 0.5,
            None => true,
        };

        if changed {
            element.last_computed = Some(computed);
            let new_inner_view = (self.builder)(computed);
            if let (Some(old_inner_view), Some(inner_element)) =
                (element.inner_view.as_ref(), element.inner_element.as_mut())
            {
                new_inner_view.rebuild_with_parent(
                    old_inner_view,
                    ctx,
                    inner_element,
                    element.container_node,
                    None,
                );
                element.inner_view = Some(new_inner_view);
            } else {
                let inner_element = new_inner_view.build(ctx);
                let inner_node = new_inner_view.get_node(&inner_element);
                element.container_node.append(ctx, inner_node);
                element.inner_view = Some(new_inner_view);
                element.inner_element = Some(inner_element);
            }
            element.container_node.set_dirty(ctx);
        }

        if let (Some(inner_view), Some(inner_element)) =
            (element.inner_view.as_ref(), element.inner_element.as_mut())
        {
            let inner_node = inner_view.get_node(inner_element);
            let inner_comp = inner_node.get_computed(ctx).unwrap_or_default();
            inner_view.on_layout(inner_element, inner_comp, ctx);
        }
    }

    fn handle_event(
        &self,
        element: &mut Self::Element,
        state: &State,
        event: Event,
        ctx: &mut Context,
    ) -> (EventResult, Option<Self::Message>) {
        if let (Some(inner_view), Some(inner_element)) =
            (element.inner_view.as_ref(), element.inner_element.as_mut())
        {
            inner_view.handle_event(inner_element, state, event, ctx)
        } else {
            (EventResult::Ignored, None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::widgets::text;

    #[test]
    fn test_adaptive_build_and_initial_layout() {
        let mut ctx = Context::new();
        let root = ctx.create_node();
        root.update_constraints(&mut ctx, |c| {
            c.width = Size::Fill;
            c.height = Size::Fill;
        });
        ctx.root_attach(root);

        let adapt_view = adaptive::<_, _, ()>(|bounds| {
            if bounds.w > 500.0 {
                text::<&str, ()>("Wide")
            } else {
                text::<&str, ()>("Narrow")
            }
        });
        let mut element = adapt_view.build(&mut ctx);
        root.append(&mut ctx, adapt_view.get_node(&element));

        ctx.compute_layout(400.0, 300.0);
        let comp = adapt_view.get_node(&element).get_computed(&ctx).unwrap();
        // Container fills width
        assert_eq!(comp.w, 400.0);

        // on_layout triggers a rebuild with correct bounds
        adapt_view.on_layout(&mut element, comp, &mut ctx);
        assert!(element.last_computed.is_some());
        assert_eq!(element.last_computed.unwrap().w, 400.0);

        // Resize viewport to 800.0
        ctx.compute_layout(800.0, 600.0);
        let comp_wide = adapt_view.get_node(&element).get_computed(&ctx).unwrap();
        assert_eq!(comp_wide.w, 800.0);
        adapt_view.on_layout(&mut element, comp_wide, &mut ctx);
        assert_eq!(element.last_computed.unwrap().w, 800.0);
    }

    #[test]
    fn test_adaptive_either_switching() {
        use crate::ui::Either;
        use crate::ui::widgets::{column, row};

        let mut ctx = Context::new();
        let root = ctx.create_node();
        root.update_constraints(&mut ctx, |c| {
            c.width = Size::Fill;
            c.height = Size::Fill;
        });
        ctx.root_attach(root);

        let adapt_view = adaptive::<_, _, ()>(|bounds| {
            if bounds.w > 500.0 {
                Either::Left(row((text::<&str, ()>("Wide1"), text::<&str, ()>("Wide2"))))
            } else {
                Either::Right(column((
                    text::<&str, ()>("Narrow1"),
                    text::<&str, ()>("Narrow2"),
                )))
            }
        });

        let mut element = adapt_view.build(&mut ctx);
        root.append(&mut ctx, adapt_view.get_node(&element));

        // Initial layout at 400px (narrow)
        ctx.compute_layout(400.0, 300.0);
        let comp = adapt_view.get_node(&element).get_computed(&ctx).unwrap();
        assert_eq!(comp.w, 400.0);

        adapt_view.on_layout(&mut element, comp, &mut ctx);
        if ctx.has_dirties() {
            ctx.compute_layout(400.0, 300.0);
        }

        let container_node = adapt_view.get_node(&element);
        let first_child = container_node.first_child(&ctx);
        assert!(
            first_child.is_some(),
            "Container node should have child after initial on_layout"
        );

        // Resize viewport to 800px (wide)
        ctx.compute_layout(800.0, 600.0);
        let comp_wide = adapt_view.get_node(&element).get_computed(&ctx).unwrap();
        assert_eq!(comp_wide.w, 800.0);

        // on_layout should rebuild inner from Either::Right to Either::Left
        adapt_view.on_layout(&mut element, comp_wide, &mut ctx);
        assert!(ctx.has_dirties());
        ctx.compute_layout(800.0, 600.0);

        let new_child = container_node.first_child(&ctx);
        assert!(
            new_child.is_some(),
            "Container node must have child attached after switching to Either::Left"
        );
        assert_eq!(new_child.unwrap().parent(&ctx), Some(container_node));
    }
}
