//! Declarative, reactive view hierarchy and event dispatch system for MTK.
//!
//! This module defines the core [`View`] trait, [`ViewSequence`] container composition,
//! state lenses, adapters, and input events ([`Event`]) used to compose user interfaces.

use crate::{Context, Node, ui::event::EventResult, windowing::WindowDimension};

pub mod adapter;
pub mod event;
pub mod lens;
pub mod memoize;
pub mod style;
pub mod widgets;

pub use adapter::{ViewAdaptExt, adapt};
pub use event::{EventKind, ViewEventExt};
pub use lens::Lens;
pub use style::ViewStyleExt;

/// Represents user interaction, layout lifecycle, and system input events dispatched down the view tree.
#[derive(Clone, Debug)]
pub enum Event {
    /// Dispatched when the mouse cursor moves across the viewport.
    CursorMoved {
        /// Absolute horizontal pixel position.
        x: f32,
        /// Absolute vertical pixel position.
        y: f32,
        /// Ordered list of layout nodes hit-tested under the cursor.
        hit_nodes: Vec<Node>,
    },
    /// Dispatched when a mouse button press or release action occurs.
    MouseInput {
        /// `true` if button was pressed down; `false` if released.
        pressed: bool,
        /// Absolute horizontal pixel position.
        x: f32,
        /// Absolute vertical pixel position.
        y: f32,
        /// Ordered list of layout nodes hit-tested under the cursor.
        hit_nodes: Vec<Node>,
    },
    /// Dispatched when mouse scroll wheel or touchpad scroll gestures are detected.
    MouseWheel {
        /// Horizontal scroll displacement.
        delta_x: f32,
        /// Vertical scroll displacement.
        delta_y: f32,
        /// `true` if scrolling originated from a continuous touchpad surface.
        is_touchpad: bool,
        /// Touch phase state associated with gesture scrolling.
        phase: winit::event::TouchPhase,
        /// Ordered list of layout nodes hit-tested under the cursor.
        hit_nodes: Vec<Node>,
    },
    /// Dispatched when a physical or virtual keyboard key is pressed or released.
    KeyboardInput {
        /// Low-level key event payload provided by `winit`.
        event: winit::event::KeyEvent,
        /// `true` if generated synthetically by MTK event repeat logic.
        is_synthetic: bool,
    },
    /// Dispatched when an OS Input Method Editor (IME) updates preedit or commits text.
    Ime(winit::event::Ime),
    /// Dispatched once per frame tick to drive animations and physics interpolation.
    Tick {
        /// Elapsed time delta in seconds since the previous frame tick.
        dt: f32,
    },
    /// Dispatched when the parent application window size changes.
    WindowResized(WindowDimension),
}

/// The foundational trait for all declarative, reactive UI components in MTK.
///
/// A `View` represents a lightweight blueprint for constructing and updating underlying
/// layout nodes ([`Node`]) bound to an application state (`State`).
pub trait View<State> {
    /// The persistent DOM-like element state maintained between render frames.
    type Element;
    /// The message type emitted by this view upon user interaction.
    type Message;

    /// Instantiates initial layout nodes and state primitives for this view.
    fn build(&self, ctx: &mut Context) -> Self::Element;

    /// Diffs and updates persistent element nodes when application state or properties change.
    fn rebuild(&self, prev: &Self, ctx: &mut Context, element: &mut Self::Element);

    /// Destroys persistent layout nodes and frees resources associated with `element`.
    fn teardown(&self, ctx: &mut Context, element: &mut Self::Element);

    /// Returns the root layout node handle representing this view.
    fn get_node(&self, element: &Self::Element) -> Node;

    /// Handles incoming user interaction events, returning event consumption status and optional domain messages.
    fn handle_event(
        &self,
        element: &mut Self::Element,
        state: &State,
        event: Event,
        ctx: &mut Context,
    ) -> (EventResult, Option<Self::Message>);
}

/// Defines container composition for sequential views, such as tuples `(ViewA, ViewB)` or `Vec<V>`.
pub trait ViewSequence<State> {
    /// The persistent element tuple or collection corresponding to child views.
    type Elements;
    /// The common message type emitted by child views in this sequence.
    type Message;

    /// Instantiates initial layout nodes for all items in the sequence and appends them to `parent`.
    fn build(&self, ctx: &mut Context, parent: Node) -> Self::Elements;

    /// Diffs and updates layout nodes for all items in the sequence.
    fn rebuild(&self, prev: &Self, ctx: &mut Context, elements: &mut Self::Elements, parent: Node);

    /// Destroys layout nodes and cleans up resources for all items in the sequence.
    fn teardown(&self, ctx: &mut Context, elements: &mut Self::Elements);

    /// Routes events through items in the sequence sequentially until consumed.
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
impl_view_tuple!(0 => A, 1 => B, 2 => C, 3 => D, 4 => E, 5 => F, 6 => G, 7 => H, 8 => I, 9 => J, 10 => K);
impl_view_tuple!(0 => A, 1 => B, 2 => C, 3 => D, 4 => E, 5 => F, 6 => G, 7 => H, 8 => I, 9 => J, 10 => K, 11 => L);
impl_view_tuple!(0 => A, 1 => B, 2 => C, 3 => D, 4 => E, 5 => F, 6 => G, 7 => H, 8 => I, 9 => J, 10 => K, 11 => L, 12 => M);

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
