use crate::{Context, Node, ui::event::EventResult, windowing::WindowDimension};

pub mod adapter;
pub mod event;
pub mod lens;
pub mod style;
pub mod widgets;

pub use event::{EventKind, ViewEventExt};
pub use lens::Lens;
pub use style::ViewStyleExt;

#[derive(Clone)]
pub enum Event {
    CursorMoved {
        x: f32,
        y: f32,
        hit_nodes: Vec<Node>,
    },
    MouseInput {
        pressed: bool,
        x: f32,
        y: f32,
        hit_nodes: Vec<Node>,
    },
    MouseWheel {
        delta_x: f32,
        delta_y: f32,
        is_touchpad: bool,
        phase: winit::event::TouchPhase,
        hit_nodes: Vec<Node>,
    },
    KeyboardInput {
        event: winit::event::KeyEvent,
        is_synthetic: bool,
    },
    Ime(winit::event::Ime),
    Tick {
        dt: f32,
    },
    WindowResized(WindowDimension),
}

pub trait View<State> {
    /// The persistent state we keep around between frames
    type Element;
    type Message;

    /// Called once when the widget is first created
    fn build(&self, ctx: &mut Context) -> Self::Element;

    /// Called when the state changes. We pass the old view and the new view.
    fn rebuild(&self, prev: &Self, ctx: &mut Context, element: &mut Self::Element);

    /// Called to destroy the element and its children
    fn teardown(&self, ctx: &mut Context, element: &mut Self::Element);

    /// Get the root node of this view (used by containers to attach it to the tree)
    fn get_node(&self, element: &Self::Element) -> Node;

    fn handle_event(
        &self,
        element: &mut Self::Element,
        state: &State,
        event: Event,
        ctx: &mut Context,
    ) -> (EventResult, Option<Self::Message>);
}

pub trait ViewSequence<State> {
    type Elements;
    type Message;

    fn build(&self, ctx: &mut Context, parent: Node) -> Self::Elements;
    fn rebuild(&self, prev: &Self, ctx: &mut Context, elements: &mut Self::Elements, parent: Node);
    fn teardown(&self, ctx: &mut Context, elements: &mut Self::Elements);

    fn handle_event(
        &self,
        elements: &mut Self::Elements,
        state: &State,
        event: Event,
        ctx: &mut Context,
    ) -> (EventResult, Option<Self::Message>);
}

macro_rules! impl_view_tuple {
    ( $($idx:tt => $t:ident),* ) => {
        impl<State, Msg, $($t),*> ViewSequence<State> for ($($t,)*)
        where
            $($t: View<State, Message = Msg>),*
        {
            type Elements = ($($t::Element,)*);
            type Message = Msg;

            fn build(&self, ctx: &mut Context, parent: Node) -> Self::Elements {
                (
                    $({
                        let child_element = self.$idx.build(ctx);
                        parent.append(ctx, self.$idx.get_node(&child_element));
                        child_element
                    },)*
                )
            }

            fn rebuild(&self, prev: &Self, ctx: &mut Context, elements: &mut Self::Elements, _parent: Node) {
                $(
                    self.$idx.rebuild(&prev.$idx, ctx, &mut elements.$idx);
                )*
            }

            fn teardown(&self, ctx: &mut Context, elements: &mut Self::Elements) {
                $(
                    self.$idx.teardown(ctx, &mut elements.$idx);
                )*
            }

            fn handle_event(
                &self,
                elements: &mut Self::Elements,
                state: &State,
                event: Event,
                ctx: &mut Context,
            ) -> (EventResult, Option<Self::Message>) {
                let mut handled = EventResult::Ignored;
                let mut emitted_msg = None;

                $(
                    if handled == EventResult::Ignored && emitted_msg.is_none() {
                        let (res, msg) = self.$idx.handle_event(
                            &mut elements.$idx,
                            state,
                            event.clone(),
                            ctx
                        );
                        handled = handled.or(res);
                        emitted_msg = msg;
                    }
                )*

                (handled, emitted_msg)
            }
        }
    };
}

// Generate implementations for tuples up to 10 elements
impl_view_tuple!(0 => A);
impl_view_tuple!(0 => A, 1 => B);
impl_view_tuple!(0 => A, 1 => B, 2 => C);
impl_view_tuple!(0 => A, 1 => B, 2 => C, 3 => D);
impl_view_tuple!(0 => A, 1 => B, 2 => C, 3 => D, 4 => E);
impl_view_tuple!(0 => A, 1 => B, 2 => C, 3 => D, 4 => E, 5 => F);
impl_view_tuple!(0 => A, 1 => B, 2 => C, 3 => D, 4 => E, 5 => F, 6 => G);
impl_view_tuple!(0 => A, 1 => B, 2 => C, 3 => D, 4 => E, 5 => F, 6 => G, 7 => H);
impl_view_tuple!(0 => A, 1 => B, 2 => C, 3 => D, 4 => E, 5 => F, 6 => G, 7 => H, 8 => I);
impl_view_tuple!(0 => A, 1 => B, 2 => C, 3 => D, 4 => E, 5 => F, 6 => G, 7 => H, 8 => I, 9 => J);

// Implement ViewSequence for Vec<V> to support dynamic lists
impl<State, Msg, V> ViewSequence<State> for Vec<V>
where
    V: View<State, Message = Msg>,
{
    type Elements = Vec<V::Element>;
    type Message = Msg;

    fn build(&self, ctx: &mut Context, parent: Node) -> Self::Elements {
        let mut elements = Vec::with_capacity(self.len());
        for view in self {
            let el = view.build(ctx);
            parent.append(ctx, view.get_node(&el));
            elements.push(el);
        }
        elements
    }

    fn rebuild(&self, prev: &Self, ctx: &mut Context, elements: &mut Self::Elements, parent: Node) {
        let min_len = self.len().min(prev.len());

        for i in 0..min_len {
            self[i].rebuild(&prev[i], ctx, &mut elements[i]);
        }

        for i in min_len..self.len() {
            let el = self[i].build(ctx);
            parent.append(ctx, self[i].get_node(&el));
            elements.push(el);
        }

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

    fn handle_event(
        &self,
        elements: &mut Self::Elements,
        state: &State,
        event: Event,
        ctx: &mut Context,
    ) -> (EventResult, Option<Self::Message>) {
        let mut handled = EventResult::Ignored;
        let mut emitted_msg = None;

        for (i, v) in self.iter().enumerate() {
            if handled == EventResult::Ignored && emitted_msg.is_none() {
                let (res, msg) = v.handle_event(&mut elements[i], state, event.clone(), ctx);
                handled = handled.or(res);
                emitted_msg = msg;
            }
        }

        (handled, emitted_msg)
    }
}
