use crate::debugger::SourceLocation;
use crate::{
    Context, Node,
    style::{Overflow, ScrollbarStyle, ScrollbarVisibility},
    ui::{Event, View, event::EventResult},
};

/// Defines which axes a [ScrollView] is allowed to scroll on.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ScrollAxis {
    /// Only allow horizontal scrolling.
    Horizontal,
    /// Only allow vertical scrolling.
    Vertical,
    /// Allow scrolling on both the horizontal and vertical axes.
    Both,
}

/// Represents an explicit initial scroll position or a programmatic jump offset.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ScrollOffset {
    /// A percentage-based offset (0.0 to 1.0).
    Percent(f32),
    /// An absolute pixel offset.
    Pixel(f32),
}

pub struct DefaultScrollBar;
pub struct NoScrollBar;

pub struct ScrollView<V> {
    pub(crate) inner: V,
    pub(crate) axis: ScrollAxis,
    pub(crate) initial_x: Option<ScrollOffset>,
    pub(crate) initial_y: Option<ScrollOffset>,
    pub(crate) source_loc: Option<SourceLocation>,
    pub(crate) scrollbar_style: Option<ScrollbarStyle>,
    pub(crate) scrollbar_visible: bool,
}

pub struct ScrollViewElement<E> {
    container_node: Node,
    inner_element: E,
}

/// Creates a new `ScrollView` widget wrapping the provided inner content.
///
/// A `ScrollView` automatically uses MTK's intrinsic seamless scrolling engine,
/// rendering floating overlay scrollbars and supporting kinetic 120Hz scrolling,
/// mouse drag interactivity, touch panning, and keyboard navigation.
#[track_caller]
pub fn scroll_view<V>(inner: V) -> ScrollView<V> {
    ScrollView {
        inner,
        axis: ScrollAxis::Both,
        initial_x: None,
        initial_y: None,
        source_loc: Some(SourceLocation::here("ScrollView")),
        scrollbar_style: None,
        scrollbar_visible: true,
    }
}

impl<V> ScrollView<V> {
    pub fn axis(mut self, axis: ScrollAxis) -> Self {
        self.axis = axis;
        self
    }

    pub fn start_offset_x(mut self, offset: ScrollOffset) -> Self {
        self.initial_x = Some(offset);
        self
    }

    pub fn start_offset_y(mut self, offset: ScrollOffset) -> Self {
        self.initial_y = Some(offset);
        self
    }

    pub fn scrollbar(mut self, scrollbar: ScrollbarStyle) -> Self {
        self.scrollbar_visible = scrollbar.visibility != ScrollbarVisibility::Never;
        self.scrollbar_style = Some(scrollbar);
        self
    }

    pub fn no_scrollbar(mut self) -> Self {
        self.scrollbar_visible = false;
        if let Some(sb) = &mut self.scrollbar_style {
            sb.visibility = ScrollbarVisibility::Never;
        }
        self
    }
}

impl<State, Msg, V> View<State> for ScrollView<V>
where
    V: View<State, Message = Msg>,
{
    type Element = ScrollViewElement<V::Element>;
    type Message = Msg;

    fn build(&self, ctx: &mut Context) -> Self::Element {
        let container_node = ctx.create_node();
        if let Some(loc) = self.source_loc {
            ctx.set_node_source(container_node, loc);
        }
        container_node.update_constraints(ctx, |c| {
            c.width = crate::style::Size::Percent(1.0);
            c.height = match self.axis {
                ScrollAxis::Horizontal => crate::style::Size::Fit,
                _ => crate::style::Size::Percent(1.0),
            };
            c.overflow = Overflow::Scroll;
            c.scrollbar_visible = self.scrollbar_visible;
        });
        if let Some(sb) = &self.scrollbar_style {
            container_node.set_scrollbar_style(ctx, sb.clone());
        }

        let inner_element = self.inner.build(ctx);
        let inner_node = self.inner.get_node(&inner_element);
        container_node.append(ctx, inner_node);

        match self.initial_x {
            Some(ScrollOffset::Pixel(px)) => {
                container_node.update_constraints(ctx, |c| c.scroll.x = px);
            }
            Some(ScrollOffset::Percent(pct)) => {
                container_node.update_constraints(ctx, |c| c.scroll.x = -pct.abs() - 0.0001);
            }
            None => {}
        }
        match self.initial_y {
            Some(ScrollOffset::Pixel(py)) => {
                container_node.update_constraints(ctx, |c| c.scroll.y = py);
            }
            Some(ScrollOffset::Percent(pct)) => {
                container_node.update_constraints(ctx, |c| c.scroll.y = -pct.abs() - 0.0001);
            }
            None => {}
        }

        ScrollViewElement {
            container_node,
            inner_element,
        }
    }

    fn rebuild(&self, prev: &Self, ctx: &mut Context, element: &mut Self::Element) {
        element.container_node.update_constraints(ctx, |c| {
            c.overflow = Overflow::Scroll;
            c.scrollbar_visible = self.scrollbar_visible;
        });
        if let Some(sb) = &self.scrollbar_style {
            element.container_node.set_scrollbar_style(ctx, sb.clone());
        }
        self.inner
            .rebuild(&prev.inner, ctx, &mut element.inner_element);
    }

    fn teardown(&self, ctx: &mut Context, element: &mut Self::Element) {
        self.inner.teardown(ctx, &mut element.inner_element);
        element.container_node.remove(ctx);
        ctx.destroy_node(element.container_node);
    }

    fn get_node(&self, element: &Self::Element) -> Node {
        element.container_node
    }

    fn handle_event(
        &self,
        element: &mut Self::Element,
        state: &State,
        event: Event,
        ctx: &mut Context,
    ) -> (EventResult, Option<Self::Message>) {
        self.inner
            .handle_event(&mut element.inner_element, state, event, ctx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::widgets::text;

    #[test]
    fn test_scroll_view_no_scrollbar() {
        let mut ctx = Context::new();
        let sv = scroll_view(text::<_, ()>("Hello")).no_scrollbar();
        let el = View::<()>::build(&sv, &mut ctx);
        let node = View::<()>::get_node(&sv, &el);

        let cons = node.get_constraints(&ctx).unwrap();
        assert!(!cons.scrollbar_visible);
    }

    #[test]
    fn test_scroll_view_custom_scrollbar() {
        let mut ctx = Context::new();
        let style = ScrollbarStyle {
            width: 12.0,
            gap: 4.0,
            ..Default::default()
        };
        let sv = scroll_view(text::<_, ()>("Hello")).scrollbar(style.clone());
        let el = View::<()>::build(&sv, &mut ctx);
        let node = View::<()>::get_node(&sv, &el);

        let cons = node.get_constraints(&ctx).unwrap();
        assert!(cons.scrollbar_visible);
        let sb = node.get_scrollbar_style(&ctx).expect("scrollbar style set");
        assert_eq!(sb.width, 12.0);
        assert_eq!(sb.gap, 4.0);
    }
}
