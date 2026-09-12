use super::scroll_view::ScrollOffset;
use crate::debugger::SourceLocation;
use crate::{
    Context, Node,
    style::{FlexDirection, Overflow, ScrollbarStyle, ScrollbarVisibility, Size, Style},
    ui::{Event, View, event::EventResult},
};

/// A virtualized list widget that renders only visible items within the viewport.
pub struct VirtualList<T, F, V> {
    pub(crate) count: usize,
    pub(crate) items: Option<Vec<T>>,
    pub(crate) item_height: f32,
    pub(crate) render_fn: F,
    pub(crate) buffer: usize,
    pub(crate) custom_style: Option<Style>,
    pub(crate) source_loc: Option<SourceLocation>,
    pub(crate) scrollbar_style: Option<ScrollbarStyle>,
    pub(crate) scrollbar_visible: bool,
    pub(crate) offset: Option<ScrollOffset>,
    pub(crate) _marker: std::marker::PhantomData<V>,
}

/// Creates a new `VirtualList` widget from a vector of items with a fixed item height.
#[track_caller]
pub fn virtual_list<T, F, V>(items: Vec<T>, item_height: f32, render_fn: F) -> VirtualList<T, F, V>
where
    F: Fn(usize, &T) -> V,
{
    let count = items.len();
    VirtualList {
        count,
        items: Some(items),
        item_height: item_height.max(1.0),
        render_fn,
        buffer: 4,
        custom_style: None,
        source_loc: Some(SourceLocation::here("VirtualList")),
        scrollbar_style: None,
        scrollbar_visible: true,
        offset: None,
        _marker: std::marker::PhantomData,
    }
}

/// Creates a new `VirtualList` widget from a total item count and an index-based render closure.
#[track_caller]
pub fn virtual_list_count<F, V>(
    count: usize,
    item_height: f32,
    render_fn: F,
) -> VirtualList<(), CountRenderFn<F>, V>
where
    F: Fn(usize) -> V,
{
    VirtualList {
        count,
        items: None,
        item_height: item_height.max(1.0),
        render_fn: CountRenderFn(render_fn),
        buffer: 4,
        custom_style: None,
        source_loc: Some(SourceLocation::here("VirtualList")),
        scrollbar_style: None,
        scrollbar_visible: true,
        offset: None,
        _marker: std::marker::PhantomData,
    }
}

impl<T, F, V> VirtualList<T, F, V> {
    /// Sets the number of overscan buffer items instantiated above and below the visible viewport.
    pub fn buffer(mut self, buffer: usize) -> Self {
        self.buffer = buffer;
        self
    }

    /// Sets custom layout and visual styling on the outer scroll container.
    pub fn style(mut self, style: Style) -> Self {
        self.custom_style = Some(style);
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

    /// Sets the initial or programmatic vertical scroll offset.
    pub fn start_offset_y(mut self, offset: ScrollOffset) -> Self {
        self.offset = Some(offset);
        self
    }

    /// Sets the initial or programmatic vertical scroll offset (alias for [`start_offset_y`](Self::start_offset_y)).
    pub fn start_offset(self, offset: ScrollOffset) -> Self {
        self.start_offset_y(offset)
    }

    /// Sets the vertical scroll offset (alias for [`start_offset_y`](Self::start_offset_y)).
    pub fn scroll_offset_y(self, offset: ScrollOffset) -> Self {
        self.start_offset_y(offset)
    }

    /// Sets the vertical scroll offset (alias for [`start_offset_y`](Self::start_offset_y)).
    pub fn scroll_offset(self, offset: ScrollOffset) -> Self {
        self.start_offset_y(offset)
    }

    /// Sets the initial or programmatic scroll offset to a specific item index.
    pub fn start_offset_index(self, index: usize) -> Self {
        let px = index as f32 * self.item_height;
        self.start_offset_y(ScrollOffset::Pixel(px))
    }

    /// Sets the scroll offset to a specific item index (alias for [`start_offset_index`](Self::start_offset_index)).
    pub fn scroll_to_index(self, index: usize) -> Self {
        self.start_offset_index(index)
    }
}

pub struct VirtualListElement<V, E> {
    container_node: Node,
    content_node: Node,
    top_spacer: Node,
    bottom_spacer: Node,
    visible_elements: Vec<(usize, V, E)>,
    rendered_range: (usize, usize),
}

impl<T, F, V, State, Msg> View<State> for VirtualList<T, F, V>
where
    F: VirtualListRenderHelper<T, V, State, Msg>,
    V: View<State, Message = Msg>,
    T: 'static,
{
    type Element = VirtualListElement<V, V::Element>;
    type Message = Msg;

    fn build(&self, ctx: &mut Context) -> Self::Element {
        let container_node = ctx.create_node();
        if let Some(loc) = self.source_loc {
            ctx.set_node_source(container_node, loc);
        }
        container_node.update_constraints(ctx, |c| {
            c.width = Size::Percent(1.0);
            c.height = Size::Percent(1.0);
            c.overflow = Overflow::Scroll;
            c.scrollbar_visible = self.scrollbar_visible;
        });

        if let Some(style) = &self.custom_style {
            style.apply_to_node(ctx, container_node);
        }

        container_node.update_constraints(ctx, |c| {
            c.overflow = Overflow::Scroll;
            c.scrollbar_visible = self.scrollbar_visible;
        });
        if let Some(sb) = &self.scrollbar_style {
            container_node.set_scrollbar_style(ctx, sb.clone());
        }

        match self.offset {
            Some(ScrollOffset::Pixel(py)) => {
                container_node.update_constraints(ctx, |c| c.scroll.y = py);
            }
            Some(ScrollOffset::Percent(pct)) => {
                container_node.update_constraints(ctx, |c| c.scroll.y = -pct.abs() - 0.0001);
            }
            None => {}
        }

        let total_h = (self.count as f32 * self.item_height).round() as u32;
        let content_node = ctx.create_node();
        content_node.update_constraints(ctx, |c| {
            c.width = Size::Percent(1.0);
            c.height = Size::Fixed(total_h);
            c.min_height = total_h as f32;
            c.max_height = total_h as f32;
            c.flex_shrink = 0.0;
            c.flex_grow = 0.0;
            c.flex_direction = FlexDirection::Column;
        });
        container_node.append(ctx, content_node);

        let top_spacer = ctx.create_node();
        top_spacer.update_constraints(ctx, |c| {
            c.width = Size::Percent(1.0);
            c.height = Size::Fixed(0);
            c.min_height = 0.0;
            c.max_height = 0.0;
            c.flex_shrink = 0.0;
            c.flex_grow = 0.0;
        });
        content_node.append(ctx, top_spacer);

        let bottom_spacer = ctx.create_node();
        bottom_spacer.update_constraints(ctx, |c| {
            c.width = Size::Percent(1.0);
            c.height = Size::Fixed(total_h);
            c.min_height = total_h as f32;
            c.max_height = total_h as f32;
            c.flex_shrink = 0.0;
            c.flex_grow = 0.0;
        });
        content_node.append(ctx, bottom_spacer);

        let mut element = VirtualListElement {
            container_node,
            content_node,
            top_spacer,
            bottom_spacer,
            visible_elements: Vec::new(),
            rendered_range: (0, 0),
        };

        self.sync_visible_range(ctx, &mut element, true);
        element
    }

    fn rebuild(&self, prev: &Self, ctx: &mut Context, element: &mut Self::Element) {
        if let Some(style) = &self.custom_style {
            style.apply_to_node(ctx, element.container_node);
        }
        element.container_node.update_constraints(ctx, |c| {
            c.overflow = Overflow::Scroll;
            c.scrollbar_visible = self.scrollbar_visible;
        });
        if let Some(sb) = &self.scrollbar_style {
            element.container_node.set_scrollbar_style(ctx, sb.clone());
        }

        if self.offset != prev.offset {
            match self.offset {
                Some(ScrollOffset::Pixel(py)) => {
                    element
                        .container_node
                        .update_constraints(ctx, |c| c.scroll.y = py);
                }
                Some(ScrollOffset::Percent(pct)) => {
                    element
                        .container_node
                        .update_constraints(ctx, |c| c.scroll.y = -pct.abs() - 0.0001);
                }
                None => {}
            }
        }

        let total_h = (self.count as f32 * self.item_height).round() as u32;
        element.content_node.update_constraints(ctx, |c| {
            c.height = Size::Fixed(total_h);
            c.min_height = total_h as f32;
            c.max_height = total_h as f32;
            c.flex_shrink = 0.0;
            c.flex_grow = 0.0;
        });

        self.sync_visible_range(ctx, element, true);
    }

    fn teardown(&self, ctx: &mut Context, element: &mut Self::Element) {
        for (_idx, view, mut elem) in element.visible_elements.drain(..) {
            view.teardown(ctx, &mut elem);
        }
        element.top_spacer.remove(ctx);
        ctx.destroy_node(element.top_spacer);
        element.bottom_spacer.remove(ctx);
        ctx.destroy_node(element.bottom_spacer);
        element.content_node.remove(ctx);
        ctx.destroy_node(element.content_node);
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
        let mut handled = EventResult::Ignored;
        let mut emitted_msg = None;

        for (_idx, view, elem) in element.visible_elements.iter_mut() {
            let (res, msg) = view.handle_event(elem, state, event.clone(), ctx);
            if res == EventResult::Handled {
                handled = EventResult::Handled;
            }
            if msg.is_some() {
                emitted_msg = msg;
            }
        }

        if matches!(
            event,
            Event::MouseWheel { .. }
                | Event::Tick { .. }
                | Event::ThumbScroll { .. }
                | Event::Scroll { .. }
                | Event::CursorMoved { .. }
                | Event::WindowResized(..)
        ) {
            self.sync_visible_range(ctx, element, false);
        }

        (handled, emitted_msg)
    }
}

impl<T, F, V> VirtualList<T, F, V> {
    fn sync_visible_range<State, Msg>(
        &self,
        ctx: &mut Context,
        element: &mut VirtualListElement<V, V::Element>,
        force_rebuild: bool,
    ) where
        F: VirtualListRenderHelper<T, V, State, Msg>,
        V: View<State, Message = Msg>,
        T: 'static,
    {
        let raw_scroll_y = element
            .container_node
            .get_constraints(ctx)
            .map(|c| c.scroll.y)
            .unwrap_or(0.0);

        let viewport_h = element
            .container_node
            .get_computed(ctx)
            .map(|c| c.h)
            .unwrap_or(800.0)
            .max(50.0);

        let item_h = self.item_height.max(1.0);
        let total_h = self.count as f32 * item_h;

        let scroll_y = if raw_scroll_y < 0.0 {
            let pct = (-raw_scroll_y - 0.0001).clamp(0.0, 1.0);
            let max_scroll = (total_h - viewport_h).max(0.0);
            pct * max_scroll
        } else {
            raw_scroll_y
        };

        let start_idx = ((scroll_y / item_h).floor() as usize)
            .saturating_sub(self.buffer)
            .min(self.count);
        let visible_count = ((viewport_h / item_h).ceil() as usize) + (self.buffer * 2);
        let end_idx = (start_idx + visible_count).min(self.count);

        let new_range = (start_idx, end_idx);

        if !force_rebuild
            && new_range == element.rendered_range
            && !element.visible_elements.is_empty()
        {
            return;
        }

        // 1. Separate preserved elements from those outside the range
        let mut old_elements = std::mem::take(&mut element.visible_elements);
        let mut preserved = std::collections::HashMap::new();

        for (idx, prev_view, mut elem) in old_elements.drain(..) {
            if idx >= start_idx && idx < end_idx {
                let node = prev_view.get_node(&elem);
                node.remove(ctx);
                preserved.insert(idx, (prev_view, elem));
            } else {
                let node = prev_view.get_node(&elem);
                node.remove(ctx);
                prev_view.teardown(ctx, &mut elem);
            }
        }

        // 2. Detach spacers before appending children in strict sequential order
        element.top_spacer.remove(ctx);
        element.bottom_spacer.remove(ctx);

        // 3. Append in exact sequential order: top_spacer -> child[start..end] -> bottom_spacer
        element.content_node.append(ctx, element.top_spacer);

        for idx in start_idx..end_idx {
            if let Some((prev_view, mut elem)) = preserved.remove(&idx) {
                let new_view = self.render_fn.call(idx, self.items.as_ref());
                new_view.rebuild(&prev_view, ctx, &mut elem);
                let node = new_view.get_node(&elem);
                element.content_node.append(ctx, node);
                element.visible_elements.push((idx, new_view, elem));
            } else {
                let new_view = self.render_fn.call(idx, self.items.as_ref());
                let elem = new_view.build(ctx);
                let node = new_view.get_node(&elem);
                element.content_node.append(ctx, node);
                element.visible_elements.push((idx, new_view, elem));
            }
        }

        element.content_node.append(ctx, element.bottom_spacer);

        // 4. Update spacer constraints
        let top_spacer_h = (start_idx as f32 * item_h).round() as u32;
        let bottom_spacer_h = ((self.count.saturating_sub(end_idx)) as f32 * item_h).round() as u32;

        element.top_spacer.update_constraints(ctx, |c| {
            c.width = Size::Percent(1.0);
            c.height = Size::Fixed(top_spacer_h);
            c.min_height = top_spacer_h as f32;
            c.max_height = top_spacer_h as f32;
            c.flex_shrink = 0.0;
            c.flex_grow = 0.0;
        });

        element.bottom_spacer.update_constraints(ctx, |c| {
            c.width = Size::Percent(1.0);
            c.height = Size::Fixed(bottom_spacer_h);
            c.min_height = bottom_spacer_h as f32;
            c.max_height = bottom_spacer_h as f32;
            c.flex_shrink = 0.0;
            c.flex_grow = 0.0;
        });

        let total_h = (self.count as f32 * item_h).round() as u32;
        element.content_node.update_constraints(ctx, |c| {
            c.width = Size::Percent(1.0);
            c.height = Size::Fixed(total_h);
            c.min_height = total_h as f32;
            c.max_height = total_h as f32;
            c.flex_shrink = 0.0;
            c.flex_grow = 0.0;
            c.flex_direction = FlexDirection::Column;
        });

        element.rendered_range = new_range;
        element.content_node.set_dirty(ctx);
        element.container_node.set_dirty(ctx);
    }
}

pub trait VirtualListRenderHelper<T, V, State, Msg> {
    fn call(&self, index: usize, items: Option<&Vec<T>>) -> V;
}

impl<T, F, V, State, Msg> VirtualListRenderHelper<T, V, State, Msg> for F
where
    F: Fn(usize, &T) -> V,
    V: View<State, Message = Msg>,
{
    fn call(&self, index: usize, items: Option<&Vec<T>>) -> V {
        if let Some(items) = items {
            if let Some(item) = items.get(index) {
                return (self)(index, item);
            }
        }
        panic!("VirtualList: index out of bounds: {index}");
    }
}

pub struct CountRenderFn<F>(pub F);

impl<F, V, State, Msg> VirtualListRenderHelper<(), V, State, Msg> for CountRenderFn<F>
where
    F: Fn(usize) -> V,
    V: View<State, Message = Msg>,
{
    fn call(&self, index: usize, _items: Option<&Vec<()>>) -> V {
        (self.0)(index)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::style::ViewStyleExt;
    use crate::ui::widgets::text;

    #[test]
    fn test_virtual_list_lifecycle_and_virtualization() {
        let mut ctx = Context::new();
        let items: Vec<String> = (0..10_000).map(|i| format!("Row {i}")).collect();

        let widget = virtual_list(items, 30.0, |_idx, item| {
            text::<_, ()>(item.clone()).style(Style::new().height(Size::Fixed(30)))
        });

        let mut element = View::<()>::build(&widget, &mut ctx);

        assert!(element.visible_elements.len() < 50);
        assert!(!element.visible_elements.is_empty());

        // Simulate scrolling down by 3,000 pixels (to item #100)
        element.container_node.update_constraints(&mut ctx, |c| {
            c.scroll.y = 3000.0;
        });

        View::<()>::rebuild(&widget, &widget, &mut ctx, &mut element);

        let (start, end) = element.rendered_range;
        assert!(start >= 90 && start <= 100);
        assert!(end > start && end <= 140);
        assert!(element.visible_elements.len() < 50);

        // Verify sequential scrolling and reverse scrolling
        for scroll in [500.0, 12000.0, 200.0, 8000.0, 0.0, 50000.0] {
            element.container_node.update_constraints(&mut ctx, |c| {
                c.scroll.y = scroll;
            });
            View::<()>::rebuild(&widget, &widget, &mut ctx, &mut element);
        }

        View::<()>::teardown(&widget, &mut ctx, &mut element);
    }

    #[test]
    fn test_virtual_list_1m_layout_and_render_list() {
        let mut ctx = Context::new();
        let header = crate::ui::widgets::text::<&str, ()>("Header")
            .style(Style::new().height(Size::Fixed(50)));
        let vlist = virtual_list_count(1_000_000, 36.0, |idx| {
            crate::ui::widgets::text::<String, ()>(format!("Row {idx}"))
                .style(Style::new().height(Size::Fixed(36)))
        })
        .style(
            Style::new()
                .width(Size::Percent(1.0))
                .flex_grow(1.0)
                .flex_shrink(1.0)
                .min_height(0.0),
        );

        let root_view = crate::ui::widgets::column((header, vlist)).style(
            Style::new()
                .width(Size::Percent(1.0))
                .height(Size::Percent(1.0)),
        );

        let mut element = View::<()>::build(&root_view, &mut ctx);
        let root_node = View::<()>::get_node(&root_view, &element);
        ctx.root_attach(root_node);

        let t0 = std::time::Instant::now();
        ctx.compute_layout(800.0, 600.0);
        println!("compute_layout took: {:?}", t0.elapsed());

        let t1 = std::time::Instant::now();
        ctx.build_render_list(crate::style::Rect {
            x: 0.0,
            y: 0.0,
            w: 800.0,
            h: 600.0,
        });
        println!("build_render_list took: {:?}", t1.elapsed());

        // Check computed dimensions
        let comp = root_node.get_computed(&ctx).unwrap();
        assert_eq!(comp.w, 800.0);
        assert_eq!(comp.h, 600.0);

        let container_comp = element.0.1.1.container_node.get_computed(&ctx).unwrap();
        assert_eq!(container_comp.h, 550.0);
        assert_eq!(container_comp.content_h, 36_000_000.0);

        for _ in 0..5 {
            root_view.handle_event(&mut element, &(), Event::Tick { dt: 0.016 }, &mut ctx);
        }

        let (start, end) = element.0.1.1.rendered_range;
        assert_eq!(start, 0);
        assert!(end <= 30);
        assert!(element.0.1.1.visible_elements.len() <= 30);

        View::<()>::teardown(&root_view, &mut ctx, &mut element);
    }

    #[test]
    fn test_virtual_list_start_offset_pixel_and_index() {
        let mut ctx = Context::new();
        let items: Vec<String> = (0..1000).map(|i| format!("Row {i}")).collect();

        // 1. start_offset_index(50) with item_height = 30.0 -> pixel offset 1500.0
        let widget = virtual_list(items.clone(), 30.0, |_idx, item| {
            text::<_, ()>(item.clone()).style(Style::new().height(Size::Fixed(30)))
        })
        .start_offset_index(50);

        let mut element = View::<()>::build(&widget, &mut ctx);

        let constraints = element.container_node.get_constraints(&ctx).unwrap();
        assert_eq!(constraints.scroll.y, 1500.0);

        let (start, end) = element.rendered_range;
        assert!(start >= 46 && start <= 50);
        assert!(end > 50);

        View::<()>::teardown(&widget, &mut ctx, &mut element);
    }

    #[test]
    fn test_virtual_list_start_offset_percent() {
        let mut ctx = Context::new();
        let items: Vec<String> = (0..1000).map(|i| format!("Row {i}")).collect();

        // 50% scroll offset on 1000 items (total height 30,000px)
        let widget = virtual_list(items, 30.0, |_idx, item| {
            text::<_, ()>(item.clone()).style(Style::new().height(Size::Fixed(30)))
        })
        .scroll_offset(ScrollOffset::percent(0.5));

        let mut element = View::<()>::build(&widget, &mut ctx);

        // Pre-layout sync encodes negative percentage offset and translates to around 50%
        let (start, end) = element.rendered_range;
        assert!(start >= 450 && start <= 500, "start={start}");
        assert!(end > start);

        View::<()>::teardown(&widget, &mut ctx, &mut element);
    }

    #[test]
    fn test_virtual_list_rebuild_offset_change() {
        let mut ctx = Context::new();
        fn render_row(idx: usize) -> crate::ui::widgets::Text<()> {
            text::<String, ()>(format!("Row {idx}"))
        }

        let widget_v1 = virtual_list_count(1000, 30.0, render_row).scroll_to_index(10);

        let mut element = View::<()>::build(&widget_v1, &mut ctx);
        let (start1, _) = element.rendered_range;
        assert!(start1 <= 10);

        // Simulate manual user scrolling to item 30
        element.container_node.update_constraints(&mut ctx, |c| {
            c.scroll.y = 900.0;
        });

        // Rebuilding with SAME offset preserves user's manual scroll
        let widget_v1_same = virtual_list_count(1000, 30.0, render_row).scroll_to_index(10);

        View::<()>::rebuild(&widget_v1_same, &widget_v1, &mut ctx, &mut element);
        let cons = element.container_node.get_constraints(&ctx).unwrap();
        assert_eq!(cons.scroll.y, 900.0);

        // Rebuilding with DIFFERENT offset triggers programmatic jump
        let widget_v2 = virtual_list_count(1000, 30.0, render_row).scroll_to_index(100);

        View::<()>::rebuild(&widget_v2, &widget_v1_same, &mut ctx, &mut element);
        let cons2 = element.container_node.get_constraints(&ctx).unwrap();
        assert_eq!(cons2.scroll.y, 3000.0);

        let (start2, end2) = element.rendered_range;
        assert!(start2 >= 96 && start2 <= 100);
        assert!(end2 > 100);

        View::<()>::teardown(&widget_v2, &mut ctx, &mut element);
    }
}
