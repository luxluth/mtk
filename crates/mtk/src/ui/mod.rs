use crate::{Context, Node};

pub mod event;
pub mod lens;
pub mod style;
pub mod widgets;

pub use event::{EventKind, ViewEventExt};
pub use lens::{Lens, LensWrap};
pub use style::{AnimationTarget, Style, ViewStyleExt};

#[derive(Clone)]
pub enum Event {
    CursorMoved {
        x: f32,
        y: f32,
        hit_nodes: Vec<Node>,
    },
    MouseInput {
        pressed: bool,
    },
    Tick,
    // TODO: Add more winit mapped events here
}

pub trait View<State> {
    /// The persistent state we keep around between frames
    type Element;

    /// Called once when the widget is first created
    fn build(&self, ctx: &mut Context) -> Self::Element;

    /// Called when the state changes. We pass the old view and the new view.
    fn rebuild(&self, prev: &Self, ctx: &mut Context, element: &mut Self::Element);

    /// Called to destroy the element and its children
    fn teardown(&self, ctx: &mut Context, element: &mut Self::Element);

    /// Get the root node of this view (used by containers to attach it to the tree)
    fn get_node(&self, element: &Self::Element) -> Node;

    /// Called to dispatch UI events
    fn message(&self, element: &mut Self::Element, state: &mut State, event: Event);
}

pub trait ViewSequence<State> {
    type Elements;

    fn build(&self, ctx: &mut Context, parent: Node) -> Self::Elements;
    fn rebuild(&self, prev: &Self, ctx: &mut Context, elements: &mut Self::Elements);
    fn teardown(&self, ctx: &mut Context, elements: &mut Self::Elements);
    fn message(&self, elements: &mut Self::Elements, state: &mut State, event: Event);
}

macro_rules! impl_view_tuple {
    ( $($idx:tt => $t:ident),* ) => {
        impl<State, $($t: View<State>),*> ViewSequence<State> for ($($t,)*) {
            type Elements = ($($t::Element,)*);

            fn build(&self, ctx: &mut Context, parent: Node) -> Self::Elements {
                (
                    $({
                        let child_element = self.$idx.build(ctx);
                        parent.append(ctx, self.$idx.get_node(&child_element));
                        child_element
                    },)*
                )
            }

            fn rebuild(&self, prev: &Self, ctx: &mut Context, elements: &mut Self::Elements) {
                $(
                    self.$idx.rebuild(&prev.$idx, ctx, &mut elements.$idx);
                )*
            }

            fn teardown(&self, ctx: &mut Context, elements: &mut Self::Elements) {
                $(
                    self.$idx.teardown(ctx, &mut elements.$idx);
                )*
            }

            fn message(&self, elements: &mut Self::Elements, state: &mut State, event: Event) {
                $(
                    self.$idx.message(&mut elements.$idx, state, event.clone());
                )*
            }
        }
    };
}

// Generate implementations for tuples up to 9 elements
impl_view_tuple!(0 => A);
impl_view_tuple!(0 => A, 1 => B);
impl_view_tuple!(0 => A, 1 => B, 2 => C);
impl_view_tuple!(0 => A, 1 => B, 2 => C, 3 => D);
impl_view_tuple!(0 => A, 1 => B, 2 => C, 3 => D, 4 => E);
impl_view_tuple!(0 => A, 1 => B, 2 => C, 3 => D, 4 => E, 5 => F);
impl_view_tuple!(0 => A, 1 => B, 2 => C, 3 => D, 4 => E, 5 => F, 6 => G);
impl_view_tuple!(0 => A, 1 => B, 2 => C, 3 => D, 4 => E, 5 => F, 6 => G, 7 => H);
impl_view_tuple!(0 => A, 1 => B, 2 => C, 3 => D, 4 => E, 5 => F, 6 => G, 7 => H, 8 => I);

// Implement ViewSequence for Vec<V> to support dynamic lists
impl<State, V: View<State>> ViewSequence<State> for Vec<V> {
    type Elements = Vec<V::Element>;

    fn build(&self, ctx: &mut Context, parent: Node) -> Self::Elements {
        let mut elements = Vec::with_capacity(self.len());
        for view in self {
            let el = view.build(ctx);
            parent.append(ctx, view.get_node(&el));
            elements.push(el);
        }
        elements
    }

    fn rebuild(&self, prev: &Self, ctx: &mut Context, elements: &mut Self::Elements) {
        let min_len = self.len().min(prev.len());

        // Rebuild existing items
        for i in 0..min_len {
            self[i].rebuild(&prev[i], ctx, &mut elements[i]);
        }

        // Add new items
        // NOTE: For Vec rebuilds, we actually need the parent node to append.
        // Since we don't pass `parent` to rebuild, we'll need to fetch the parent
        // from one of the existing sibling elements. But for now, we'll assume
        // static-sized lists, and we can add the parent to `rebuild` args if needed!
        if self.len() > prev.len() {
            panic!("Dynamic growth in Vec requires parent node in rebuild. This is a WIP!");
        }

        // Remove old items
        if self.len() < prev.len() {
            for i in min_len..prev.len() {
                prev[i].teardown(ctx, &mut elements[i]);
            }
            elements.truncate(self.len());
        }
    }

    fn teardown(&self, ctx: &mut Context, elements: &mut Self::Elements) {
        for (i, view) in self.iter().enumerate() {
            view.teardown(ctx, &mut elements[i]);
        }
    }

    fn message(&self, elements: &mut Self::Elements, state: &mut State, event: Event) {
        for (i, view) in self.iter().enumerate() {
            view.message(&mut elements[i], state, event.clone());
        }
    }
}
