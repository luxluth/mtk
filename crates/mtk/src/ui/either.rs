//! Stack-allocated heterogeneous view branching for conditional and multi-branch UI composition.
//!
//! Provides [`Either`], the [`either`] combinator, [`ViewEitherExt`], and the [`switch!`] macro.
//! These allow different concrete [`View`] types to be unified conditionally without heap allocation
//! ([`BoxedView`](crate::ui::BoxedView)) or manual sum-type enums.

use crate::ui::event::EventResult;
use crate::ui::{Event, View, ViewSequence};
use crate::{Context, Node};

/// A sum type representing a view that is either of type `A` or type `B`.
///
/// Implements both [`View`] and [`ViewSequence`], enabling zero-allocation conditional rendering
/// in standalone positions, within tuple sequences, or directly as container children.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Either<A, B> {
    /// Left variant containing a view of type `A`.
    Left(A),
    /// Right variant containing a view of type `B`.
    Right(B),
}

impl<A, B> Either<A, B> {
    /// Returns `true` if this is [`Either::Left`].
    #[inline]
    pub const fn is_left(&self) -> bool {
        matches!(self, Either::Left(_))
    }

    /// Returns `true` if this is [`Either::Right`].
    #[inline]
    pub const fn is_right(&self) -> bool {
        matches!(self, Either::Right(_))
    }

    /// Returns an immutable reference to the left value, if present.
    #[inline]
    pub const fn left(&self) -> Option<&A> {
        match self {
            Either::Left(a) => Some(a),
            Either::Right(_) => None,
        }
    }

    /// Returns an immutable reference to the right value, if present.
    #[inline]
    pub const fn right(&self) -> Option<&B> {
        match self {
            Either::Left(_) => None,
            Either::Right(b) => Some(b),
        }
    }

    /// Returns a mutable reference to the left value, if present.
    #[inline]
    pub fn left_mut(&mut self) -> Option<&mut A> {
        match self {
            Either::Left(a) => Some(a),
            Either::Right(_) => None,
        }
    }

    /// Returns a mutable reference to the right value, if present.
    #[inline]
    pub fn right_mut(&mut self) -> Option<&mut B> {
        match self {
            Either::Left(_) => None,
            Either::Right(b) => Some(b),
        }
    }

    /// Converts `&Either<A, B>` into `Either<&A, &B>`.
    #[inline]
    pub const fn as_ref(&self) -> Either<&A, &B> {
        match self {
            Either::Left(a) => Either::Left(a),
            Either::Right(b) => Either::Right(b),
        }
    }

    /// Converts `&mut Either<A, B>` into `Either<&mut A, &mut B>`.
    #[inline]
    pub fn as_mut(&mut self) -> Either<&mut A, &mut B> {
        match self {
            Either::Left(a) => Either::Left(a),
            Either::Right(b) => Either::Right(b),
        }
    }

    /// Maps the left variant using the provided closure.
    #[inline]
    pub fn map_left<F, A2>(self, f: F) -> Either<A2, B>
    where
        F: FnOnce(A) -> A2,
    {
        match self {
            Either::Left(a) => Either::Left(f(a)),
            Either::Right(b) => Either::Right(b),
        }
    }

    /// Maps the right variant using the provided closure.
    #[inline]
    pub fn map_right<F, B2>(self, f: F) -> Either<A, B2>
    where
        F: FnOnce(B) -> B2,
    {
        match self {
            Either::Left(a) => Either::Left(a),
            Either::Right(b) => Either::Right(f(b)),
        }
    }

    /// Returns the left value, panicking if this is [`Either::Right`].
    #[inline]
    pub fn unwrap_left(self) -> A {
        match self {
            Either::Left(a) => a,
            Either::Right(_) => panic!("called `Either::unwrap_left` on a `Right` value"),
        }
    }

    /// Returns the right value, panicking if this is [`Either::Left`].
    #[inline]
    pub fn unwrap_right(self) -> B {
        match self {
            Either::Left(_) => panic!("called `Either::unwrap_right` on a `Left` value"),
            Either::Right(b) => b,
        }
    }
}

impl<A: Default, B> Default for Either<A, B> {
    fn default() -> Self {
        Either::Left(A::default())
    }
}

/// Creates an [`Either`] view by evaluating a boolean condition.
///
/// If `condition` is `true`, returns [`Either::Left(true_view)`]. Otherwise returns [`Either::Right(false_view)`].
///
/// # Examples
/// ```rust,ignore
/// either(
///     is_loading,
///     spinner(),
///     user_profile(),
/// )
/// ```
#[inline]
pub fn either<A, B>(condition: bool, true_view: A, false_view: B) -> Either<A, B> {
    if condition {
        Either::Left(true_view)
    } else {
        Either::Right(false_view)
    }
}

/// Extension trait providing `.either_left()` and `.either_right()` constructors on any view or element.
pub trait ViewEitherExt: Sized {
    /// Wraps this value in [`Either::Left`].
    #[inline]
    fn either_left<B>(self) -> Either<Self, B> {
        Either::Left(self)
    }

    /// Wraps this value in [`Either::Right`].
    #[inline]
    fn either_right<A>(self) -> Either<A, Self> {
        Either::Right(self)
    }
}

impl<T> ViewEitherExt for T {}

impl<State, Msg, A, B> View<State> for Either<A, B>
where
    A: View<State, Message = Msg>,
    B: View<State, Message = Msg>,
{
    type Element = Either<A::Element, B::Element>;
    type Message = Msg;

    fn build(&self, ctx: &mut Context) -> Self::Element {
        match self {
            Either::Left(a) => Either::Left(a.build(ctx)),
            Either::Right(b) => Either::Right(b.build(ctx)),
        }
    }

    fn rebuild_with_parent(
        &self,
        prev: &Self,
        ctx: &mut Context,
        element: &mut Self::Element,
        parent: Node,
        next_sibling: Option<Node>,
    ) {
        match (self, prev) {
            (Either::Left(new_a), Either::Left(old_a)) => {
                if let Either::Left(el_a) = element {
                    new_a.rebuild_with_parent(old_a, ctx, el_a, parent, next_sibling);
                }
            }
            (Either::Right(new_b), Either::Right(old_b)) => {
                if let Either::Right(el_b) = element {
                    new_b.rebuild_with_parent(old_b, ctx, el_b, parent, next_sibling);
                }
            }
            (Either::Right(new_b), Either::Left(old_a)) => {
                if let Either::Left(mut old_el) =
                    std::mem::replace(element, Either::Right(new_b.build(ctx)))
                {
                    let old_node = old_a.get_node(&old_el);
                    old_node.remove(ctx);
                    old_a.teardown(ctx, &mut old_el);
                }
                if let Either::Right(new_el) = element {
                    let new_node = new_b.get_node(new_el);
                    if let Some(sibling) = next_sibling {
                        new_node.put_before(ctx, sibling);
                    } else {
                        parent.append(ctx, new_node);
                    }
                }
            }
            (Either::Left(new_a), Either::Right(old_b)) => {
                if let Either::Right(mut old_el) =
                    std::mem::replace(element, Either::Left(new_a.build(ctx)))
                {
                    let old_node = old_b.get_node(&old_el);
                    old_node.remove(ctx);
                    old_b.teardown(ctx, &mut old_el);
                }
                if let Either::Left(new_el) = element {
                    let new_node = new_a.get_node(new_el);
                    if let Some(sibling) = next_sibling {
                        new_node.put_before(ctx, sibling);
                    } else {
                        parent.append(ctx, new_node);
                    }
                }
            }
        }
    }

    fn rebuild(&self, prev: &Self, ctx: &mut Context, element: &mut Self::Element) {
        match (self, prev) {
            (Either::Left(new_a), Either::Left(old_a)) => {
                if let Either::Left(el_a) = element {
                    new_a.rebuild(old_a, ctx, el_a);
                }
            }
            (Either::Right(new_b), Either::Right(old_b)) => {
                if let Either::Right(el_b) = element {
                    new_b.rebuild(old_b, ctx, el_b);
                }
            }
            (Either::Right(new_b), Either::Left(old_a)) => {
                if let Either::Left(mut old_el) =
                    std::mem::replace(element, Either::Right(new_b.build(ctx)))
                {
                    let old_node = old_a.get_node(&old_el);
                    let is_root = ctx.layout.root == Some(old_node.0);
                    old_node.remove(ctx);
                    old_a.teardown(ctx, &mut old_el);
                    if let Either::Right(new_el) = element {
                        if is_root {
                            ctx.root_attach(new_b.get_node(new_el));
                        }
                    }
                }
            }
            (Either::Left(new_a), Either::Right(old_b)) => {
                if let Either::Right(mut old_el) =
                    std::mem::replace(element, Either::Left(new_a.build(ctx)))
                {
                    let old_node = old_b.get_node(&old_el);
                    let is_root = ctx.layout.root == Some(old_node.0);
                    old_node.remove(ctx);
                    old_b.teardown(ctx, &mut old_el);
                    if let Either::Left(new_el) = element {
                        if is_root {
                            ctx.root_attach(new_a.get_node(new_el));
                        }
                    }
                }
            }
        }
    }

    fn teardown(&self, ctx: &mut Context, element: &mut Self::Element) {
        match (self, element) {
            (Either::Left(a), Either::Left(el)) => a.teardown(ctx, el),
            (Either::Right(b), Either::Right(el)) => b.teardown(ctx, el),
            _ => {}
        }
    }

    fn get_node(&self, element: &Self::Element) -> Node {
        match (self, element) {
            (Either::Left(a), Either::Left(el)) => a.get_node(el),
            (Either::Right(b), Either::Right(el)) => b.get_node(el),
            _ => Node::get_invalid(),
        }
    }

    fn handle_event(
        &self,
        element: &mut Self::Element,
        state: &State,
        event: Event,
        ctx: &mut Context,
    ) -> (EventResult, Option<Self::Message>) {
        match (self, element) {
            (Either::Left(a), Either::Left(el)) => a.handle_event(el, state, event, ctx),
            (Either::Right(b), Either::Right(el)) => b.handle_event(el, state, event, ctx),
            _ => (EventResult::Ignored, None),
        }
    }
}

impl<State, Msg, A, B> ViewSequence<State> for Either<A, B>
where
    A: View<State, Message = Msg>,
    B: View<State, Message = Msg>,
{
    type Elements = Either<A::Element, B::Element>;
    type Message = Msg;

    fn build(&self, ctx: &mut Context, parent: Node) -> Self::Elements {
        match self {
            Either::Left(a) => {
                let el = a.build(ctx);
                parent.append(ctx, a.get_node(&el));
                Either::Left(el)
            }
            Either::Right(b) => {
                let el = b.build(ctx);
                parent.append(ctx, b.get_node(&el));
                Either::Right(el)
            }
        }
    }

    fn rebuild(&self, prev: &Self, ctx: &mut Context, elements: &mut Self::Elements, parent: Node) {
        match (self, prev) {
            (Either::Left(new_a), Either::Left(old_a)) => {
                if let Either::Left(el_a) = elements {
                    new_a.rebuild_with_parent(old_a, ctx, el_a, parent, None);
                }
            }
            (Either::Right(new_b), Either::Right(old_b)) => {
                if let Either::Right(el_b) = elements {
                    new_b.rebuild_with_parent(old_b, ctx, el_b, parent, None);
                }
            }
            (Either::Right(new_b), Either::Left(old_a)) => {
                if let Either::Left(mut old_el) =
                    std::mem::replace(elements, Either::Right(new_b.build(ctx)))
                {
                    let old_node = old_a.get_node(&old_el);
                    let next_sibling = old_node.next_sibling(ctx);
                    old_node.remove(ctx);
                    old_a.teardown(ctx, &mut old_el);
                    if let Either::Right(new_el) = elements {
                        let new_node = new_b.get_node(new_el);
                        if let Some(sibling) = next_sibling {
                            new_node.put_before(ctx, sibling);
                        } else {
                            parent.append(ctx, new_node);
                        }
                    }
                }
            }
            (Either::Left(new_a), Either::Right(old_b)) => {
                if let Either::Right(mut old_el) =
                    std::mem::replace(elements, Either::Left(new_a.build(ctx)))
                {
                    let old_node = old_b.get_node(&old_el);
                    let next_sibling = old_node.next_sibling(ctx);
                    old_node.remove(ctx);
                    old_b.teardown(ctx, &mut old_el);
                    if let Either::Left(new_el) = elements {
                        let new_node = new_a.get_node(new_el);
                        if let Some(sibling) = next_sibling {
                            new_node.put_before(ctx, sibling);
                        } else {
                            parent.append(ctx, new_node);
                        }
                    }
                }
            }
        }
    }

    fn teardown(&self, ctx: &mut Context, elements: &mut Self::Elements) {
        match (self, elements) {
            (Either::Left(a), Either::Left(el)) => a.teardown(ctx, el),
            (Either::Right(b), Either::Right(el)) => b.teardown(ctx, el),
            _ => {}
        }
    }

    fn handle_event(
        &self,
        elements: &mut Self::Elements,
        state: &State,
        event: Event,
        ctx: &mut Context,
    ) -> (EventResult, Option<Self::Message>) {
        match (self, elements) {
            (Either::Left(a), Either::Left(el)) => a.handle_event(el, state, event, ctx),
            (Either::Right(b), Either::Right(el)) => b.handle_event(el, state, event, ctx),
            _ => (EventResult::Ignored, None),
        }
    }
}

/// Internal macro helper for constructing nested `Either::Right(...)` chains at compile time.
#[macro_export]
#[doc(hidden)]
macro_rules! __either_wrap {
    ([] $expr:expr) => {
        $expr
    };
    ([Right $($rest:ident)*] $expr:expr) => {
        $crate::ui::Either::Right($crate::__either_wrap!([$($rest)*] $expr))
    };
}

/// Declarative multi-branch conditional view macro.
///
/// Constructs a statically-typed tree of [`Either`] views with zero heap allocations,
/// enabling `match` expressions or `if / else if / else` chains where each branch returns a different concrete view type.
///
/// # Examples
///
/// ### Match Syntax
/// ```rust,ignore
/// switch! {
///     match state.tab {
///         Tab::Home => home_view(),
///         Tab::Profile => profile_view(),
///         Tab::Settings => settings_view(),
///     }
/// }
/// ```
///
/// ### Shorthand Match Syntax
/// ```rust,ignore
/// switch! {
///     state.status,
///     Status::Loading => spinner(),
///     Status::Error(e) => error_banner(e),
///     Status::Success(data) => content_view(data),
/// }
/// ```
///
/// ### If / Else If / Else Syntax
/// ```rust,ignore
/// switch! {
///     if is_loading => spinner(),
///     else if has_error => error_view(),
///     else => content_view(),
/// }
/// ```
#[macro_export]
macro_rules! switch {
    // 1. If / else if / else syntax
    (if $cond:expr => $then:expr, else if $($rest:tt)*) => {
        if $cond {
            $crate::ui::Either::Left($then)
        } else {
            $crate::ui::Either::Right($crate::switch!(if $($rest)*))
        }
    };
    (if $cond:expr => $then:expr, else => $else:expr $(,)?) => {
        if $cond {
            $crate::ui::Either::Left($then)
        } else {
            $crate::ui::Either::Right($else)
        }
    };

    // 2. Match syntax: match (expr) { ... }, match ident { ... }, match ident.field { ... }, or match expr, ...
    (match ($($val:tt)+) { $($arms:tt)* }) => {
        $crate::switch!(@munch ($($val)+), [], [], $($arms)*)
    };
    (match $val:ident { $($arms:tt)* }) => {
        $crate::switch!(@munch $val, [], [], $($arms)*)
    };
    (match $val:ident . $field:ident { $($arms:tt)* }) => {
        $crate::switch!(@munch ($val.$field), [], [], $($arms)*)
    };
    (match $val:ident . $field:ident . $sub:ident { $($arms:tt)* }) => {
        $crate::switch!(@munch ($val.$field.$sub), [], [], $($arms)*)
    };
    (match $val:expr, $($arms:tt)+) => {
        $crate::switch!(@munch $val, [], [], $($arms)+)
    };

    // 3. Shorthand match syntax: expr, pat => view...
    ($val:expr, $($arms:tt)+) => {
        $crate::switch!(@munch $val, [], [], $($arms)+)
    };

    // TT-muncher: intermediate arm (there is another arm following)
    (@munch $val:expr, [ $($rights:ident)* ], [ $($collected:tt)* ], $pat:pat $(if $guard:expr)? => $body:expr, $next_pat:pat $(if $next_guard:expr)? => $($rest:tt)*) => {
        $crate::switch!(
            @munch $val,
            [ $($rights)* Right ],
            [
                $($collected)*
                $pat $(if $guard)? => $crate::__either_wrap!([ $($rights)* ] $crate::ui::Either::Left($body)),
            ],
            $next_pat $(if $next_guard)? => $($rest)*
        )
    };

    // TT-muncher: last arm
    (@munch $val:expr, [ $($rights:ident)* ], [ $($collected:tt)* ], $pat:pat $(if $guard:expr)? => $body:expr $(,)?) => {
        match $val {
            $($collected)*
            $pat $(if $guard)? => $crate::__either_wrap!([ $($rights)* ] $body),
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::widgets::{button, column, text};

    #[test]
    fn test_either_left_and_right_render() {
        let mut ctx = Context::new();

        let view_left: Either<_, _> =
            either(true, text::<_, ()>("Hello Left"), button("Click Right"));
        let mut el_left = View::<()>::build(&view_left, &mut ctx);
        let node_left = View::<()>::get_node(&view_left, &el_left);
        assert!(node_left.is_valid());

        let view_right: Either<_, _> =
            either(false, text::<_, ()>("Hello Left"), button("Click Right"));
        let mut el_right = View::<()>::build(&view_right, &mut ctx);
        let node_right = View::<()>::get_node(&view_right, &el_right);
        assert!(node_right.is_valid());
        assert_ne!(node_left, node_right);

        View::<()>::teardown(&view_left, &mut ctx, &mut el_left);
        View::<()>::teardown(&view_right, &mut ctx, &mut el_right);
    }

    #[test]
    fn test_either_in_place_rebuild() {
        let mut ctx = Context::new();

        let v1: Either<_, crate::ui::widgets::Button<()>> = Either::Left(text::<_, ()>("Initial"));
        let mut el = View::<()>::build(&v1, &mut ctx);
        let initial_node = View::<()>::get_node(&v1, &el);

        let v2: Either<_, crate::ui::widgets::Button<()>> = Either::Left(text::<_, ()>("Updated"));
        View::<()>::rebuild(&v2, &v1, &mut ctx, &mut el);
        let updated_node = View::<()>::get_node(&v2, &el);

        // Same variant should preserve layout node in-place
        assert_eq!(initial_node, updated_node);
        assert_eq!(updated_node.get_text(&ctx), Some("Updated"));

        View::<()>::teardown(&v2, &mut ctx, &mut el);
    }

    #[test]
    fn test_either_variant_transition() {
        let mut ctx = Context::new();

        // 1. Initial Left (Text)
        let v1: Either<_, _> = either(true, text::<_, ()>("Text View"), button("Button View"));
        let mut el = View::<()>::build(&v1, &mut ctx);
        let node1 = View::<()>::get_node(&v1, &el);
        assert!(node1.is_valid());

        // 2. Transition to Right (Button)
        let v2: Either<_, _> = either(false, text::<_, ()>("Text View"), button("Button View"));
        View::<()>::rebuild(&v2, &v1, &mut ctx, &mut el);
        let node2 = View::<()>::get_node(&v2, &el);
        assert!(node2.is_valid());
        assert_ne!(node1, node2);

        // 3. Transition back to Left (Text)
        let v3: Either<_, _> = either(true, text::<_, ()>("New Text"), button("Button View"));
        View::<()>::rebuild(&v3, &v2, &mut ctx, &mut el);
        let node3 = View::<()>::get_node(&v3, &el);
        assert!(node3.is_valid());
        assert_ne!(node2, node3);
        assert_eq!(node3.get_text(&ctx), Some("New Text"));

        View::<()>::teardown(&v3, &mut ctx, &mut el);
    }

    #[test]
    fn test_either_inside_container_sibling_ordering() {
        let mut ctx = Context::new();

        // Initial: [BANNER, Left(text), FOOTER]
        let view_1 = column((
            text::<_, ()>("BANNER"),
            either(
                true,
                text::<_, ()>("Active Mode"),
                button("Inactive Button"),
            ),
            text::<_, ()>("FOOTER"),
        ));

        let mut el = View::<()>::build(&view_1, &mut ctx);
        let parent = View::<()>::get_node(&view_1, &el);

        let children_1 = parent.children(&ctx);
        assert_eq!(children_1.len(), 3);
        let banner_node = children_1[0];
        let middle_node_1 = children_1[1];
        let footer_node = children_1[2];
        assert_eq!(middle_node_1.get_text(&ctx), Some("Active Mode"));

        // Rebuild: [BANNER, Right(button), FOOTER]
        let view_2 = column((
            text::<_, ()>("BANNER"),
            either(
                false,
                text::<_, ()>("Active Mode"),
                button("Inactive Button"),
            ),
            text::<_, ()>("FOOTER"),
        ));

        View::<()>::rebuild(&view_2, &view_1, &mut ctx, &mut el);

        let children_2 = parent.children(&ctx);
        assert_eq!(children_2.len(), 3);
        assert_eq!(children_2[0], banner_node);
        assert_ne!(children_2[1], middle_node_1);
        assert_eq!(children_2[2], footer_node);

        // Rebuild back: [BANNER, Left(text), FOOTER]
        let view_3 = column((
            text::<_, ()>("BANNER"),
            either(
                true,
                text::<_, ()>("Active Mode 2"),
                button("Inactive Button"),
            ),
            text::<_, ()>("FOOTER"),
        ));

        View::<()>::rebuild(&view_3, &view_2, &mut ctx, &mut el);

        let children_3 = parent.children(&ctx);
        assert_eq!(children_3.len(), 3);
        assert_eq!(children_3[0], banner_node);
        assert_eq!(children_3[1].get_text(&ctx), Some("Active Mode 2"));
        assert_eq!(children_3[2], footer_node);

        View::<()>::teardown(&view_3, &mut ctx, &mut el);
    }

    #[test]
    fn test_either_view_sequence_direct() {
        let mut ctx = Context::new();

        // Pass either directly as children to column
        let v1 = column(either(
            true,
            text::<_, ()>("Only Left"),
            button("Only Right"),
        ));
        let mut el = View::<()>::build(&v1, &mut ctx);
        let parent = View::<()>::get_node(&v1, &el);

        let children_1 = parent.children(&ctx);
        assert_eq!(children_1.len(), 1);
        assert_eq!(children_1[0].get_text(&ctx), Some("Only Left"));

        let v2 = column(either(
            false,
            text::<_, ()>("Only Left"),
            button("Only Right"),
        ));
        View::<()>::rebuild(&v2, &v1, &mut ctx, &mut el);

        let children_2 = parent.children(&ctx);
        assert_eq!(children_2.len(), 1);
        assert_ne!(children_2[0], children_1[0]);

        View::<()>::teardown(&v2, &mut ctx, &mut el);
    }

    #[test]
    fn test_either_extension_trait() {
        let text_view = text::<_, ()>("Ext Left");
        let either_left: Either<_, crate::ui::widgets::Button<()>> = text_view.either_left();
        assert!(either_left.is_left());

        let btn_view = button::<&str, ()>("Ext Right");
        let either_right: Either<crate::ui::widgets::Text<()>, _> = btn_view.either_right();
        assert!(either_right.is_right());
    }

    #[derive(Clone, Copy, Debug, PartialEq)]
    enum TestTab {
        First,
        Second,
        Third,
        Fourth,
    }

    #[test]
    fn test_switch_match_syntax() {
        let mut ctx = Context::new();

        let tab = TestTab::Second;
        let view = switch! {
            match tab {
                TestTab::First => text::<_, ()>("Tab 1"),
                TestTab::Second => button("Tab 2 Button"),
                TestTab::Third => text::<_, ()>("Tab 3"),
                TestTab::Fourth => button("Tab 4 Button"),
            }
        };

        let mut el = View::<()>::build(&view, &mut ctx);
        let node = View::<()>::get_node(&view, &el);
        assert!(node.is_valid());

        // Second branch is an Either::Right(Either::Left(...))
        assert!(view.is_right());
        let inner = view.right().unwrap();
        assert!(inner.is_left());

        View::<()>::teardown(&view, &mut ctx, &mut el);
    }

    #[test]
    fn test_switch_shorthand_syntax() {
        let mut ctx = Context::new();

        let tab = TestTab::Third;
        let view = switch! {
            tab,
            TestTab::First => text::<_, ()>("Tab 1"),
            TestTab::Second => button("Tab 2 Button"),
            TestTab::Third => text::<_, ()>("Tab 3"),
            TestTab::Fourth => button("Tab 4 Button"),
        };

        let mut el = View::<()>::build(&view, &mut ctx);
        let node = View::<()>::get_node(&view, &el);
        assert!(node.is_valid());
        assert_eq!(node.get_text(&ctx), Some("Tab 3"));

        View::<()>::teardown(&view, &mut ctx, &mut el);
    }

    #[test]
    fn test_switch_if_else_syntax() {
        let mut ctx = Context::new();

        let count = 2;
        let view = switch! {
            if count == 0 => text::<_, ()>("Zero"),
            else if count == 1 => button("One"),
            else => text::<_, ()>("Multiple"),
        };

        let mut el = View::<()>::build(&view, &mut ctx);
        let node = View::<()>::get_node(&view, &el);
        assert!(node.is_valid());
        assert_eq!(node.get_text(&ctx), Some("Multiple"));

        View::<()>::teardown(&view, &mut ctx, &mut el);
    }

    #[test]
    fn test_switch_guards() {
        let count = 5;
        let view = switch! {
            count,
            c if c < 0 => text::<_, ()>("Negative"),
            0 => text::<_, ()>("Zero"),
            c if c > 0 => button::<_, ()>("Positive Button"),
            _ => text::<_, ()>("Unknown"),
        };

        assert!(view.is_right());
        assert!(view.right().unwrap().is_right());
        assert!(view.right().unwrap().right().unwrap().is_left());
    }

    #[test]
    fn test_switch_deep_transitions() {
        let mut ctx = Context::new();

        let make_view = |tab: TestTab| {
            column((
                text::<_, ()>("Header"),
                switch! {
                    match tab {
                        TestTab::First => text::<_, ()>("1: First"),
                        TestTab::Second => button("2: Second"),
                        TestTab::Third => text::<_, ()>("3: Third"),
                        TestTab::Fourth => button("4: Fourth"),
                    }
                },
                text::<_, ()>("Footer"),
            ))
        };

        // 1. Initial tab = First
        let v1 = make_view(TestTab::First);
        let mut el = View::<()>::build(&v1, &mut ctx);
        let parent = View::<()>::get_node(&v1, &el);
        let c1 = parent.children(&ctx);
        assert_eq!(c1.len(), 3);
        assert_eq!(c1[1].get_text(&ctx), Some("1: First"));

        // 2. Jump to Third
        let v2 = make_view(TestTab::Third);
        View::<()>::rebuild(&v2, &v1, &mut ctx, &mut el);
        let c2 = parent.children(&ctx);
        assert_eq!(c2.len(), 3);
        assert_eq!(c2[0], c1[0]);
        assert_eq!(c2[1].get_text(&ctx), Some("3: Third"));
        assert_eq!(c2[2], c1[2]);

        // 3. Jump to Second (Button)
        let v3 = make_view(TestTab::Second);
        View::<()>::rebuild(&v3, &v2, &mut ctx, &mut el);
        let c3 = parent.children(&ctx);
        assert_eq!(c3.len(), 3);
        assert_eq!(c3[0], c1[0]);
        assert_ne!(c3[1], c2[1]);
        assert_eq!(c3[2], c1[2]);

        // 4. Jump to Fourth
        let v4 = make_view(TestTab::Fourth);
        View::<()>::rebuild(&v4, &v3, &mut ctx, &mut el);
        let c4 = parent.children(&ctx);
        assert_eq!(c4.len(), 3);
        assert_eq!(c4[0], c1[0]);
        assert_ne!(c4[1], c3[1]);
        assert_eq!(c4[2], c1[2]);

        // 5. Jump back to First
        let v5 = make_view(TestTab::First);
        View::<()>::rebuild(&v5, &v4, &mut ctx, &mut el);
        let c5 = parent.children(&ctx);
        assert_eq!(c5.len(), 3);
        assert_eq!(c5[0], c1[0]);
        assert_eq!(c5[1].get_text(&ctx), Some("1: First"));
        assert_eq!(c5[2], c1[2]);

        View::<()>::teardown(&v5, &mut ctx, &mut el);
    }
}
