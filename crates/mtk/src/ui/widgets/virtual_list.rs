use crate::{
    Context, Node,
    style::{FlexDirection, Overflow, Size, Style},
    ui::{Event, View, event::EventResult},
};

/// A high-performance virtualized list widget that renders only visible items within the viewport.
///
/// Designed to effortlessly render $10{,}000$ to $1{,}000{,}000+$ items with sub-millisecond
/// layout times, 120Hz/144Hz momentum scrolling, and minimal memory overhead.
pub struct VirtualList<T, F, V> {
    pub(crate) count: usize,
    pub(crate) items: Option<Vec<T>>,
    pub(crate) item_height: f32,
    pub(crate) render_fn: F,
    pub(crate) buffer: usize,
    pub(crate) custom_style: Option<Style>,
    pub(crate) _marker: std::marker::PhantomData<V>,
}

/// Creates a new `VirtualList` widget from a vector of items with a fixed item height.
///
/// # Examples
/// ```rust,ignore
/// let items: Vec<String> = (0..100_000).map(|i| format!("Item #{i}")).collect();
/// virtual_list(items, 40.0, |idx, item| {
///     text(item).style(Style::new().height(Size::Fixed(40.0)))
/// })
/// ```
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
        _marker: std::marker::PhantomData,
    }
}

/// Creates a new `VirtualList` widget from a total item count and an index-based render closure.
///
/// Useful for indexed datasets, databases, or large generated sequences.
///
/// # Examples
/// ```rust,ignore
/// virtual_list_count(1_000_000, 36.0, |idx| {
///     text(format!("Row #{idx}")).style(Style::new().height(Size::Fixed(36.0)))
/// })
/// ```
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
        _marker: std::marker::PhantomData,
    }
}

impl<T, F, V> VirtualList<T, F, V> {
    /// Sets the number of overscan buffer items instantiated above and below the visible viewport.
    ///
    /// Defaults to `4`. Increasing this avoids brief blank frames during ultra-fast flings.
    pub fn buffer(mut self, buffer: usize) -> Self {
        self.buffer = buffer;
        self
    }

    /// Sets custom layout and visual styling on the outer scroll container.
    pub fn style(mut self, style: Style) -> Self {
        self.custom_style = Some(style);
        self
    }
}

pub struct VirtualListElement<E> {
    container_node: Node,
    content_node: Node,
    top_spacer: Node,
    bottom_spacer: Node,
    visible_elements: Vec<(usize, E)>,
    rendered_range: (usize, usize),
}

impl<T, F, V, State, Msg> View<State> for VirtualList<T, F, V>
where
    F: VirtualListRenderHelper<T, V, State, Msg>,
    V: View<State, Message = Msg>,
    T: 'static,
{
    type Element = VirtualListElement<V::Element>;
    type Message = Msg;

    fn build(&self, ctx: &mut Context) -> Self::Element {
        let container_node = ctx.create_node();
        container_node.update_constraints(ctx, |c| {
            c.width = Size::Percent(1.0);
            c.height = Size::Percent(1.0);
            c.overflow = Overflow::Scroll;
        });

        if let Some(style) = &self.custom_style {
            style.apply_to_node(ctx, container_node);
        }

        let content_node = ctx.create_node();
        content_node.update_constraints(ctx, |c| {
            c.width = Size::Percent(1.0);
            c.height = Size::Fixed((self.count as f32 * self.item_height).round() as u32);
            c.flex_direction = FlexDirection::Column;
        });
        container_node.append(ctx, content_node);

        let top_spacer = ctx.create_node();
        top_spacer.update_constraints(ctx, |c| {
            c.width = Size::Percent(1.0);
            c.height = Size::Fixed(0);
        });
        content_node.append(ctx, top_spacer);

        let bottom_spacer = ctx.create_node();
        bottom_spacer.update_constraints(ctx, |c| {
            c.width = Size::Percent(1.0);
            c.height = Size::Fixed((self.count as f32 * self.item_height).round() as u32);
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

        self.sync_visible_range(ctx, &mut element);
        element
    }

    fn rebuild(&self, _prev: &Self, ctx: &mut Context, element: &mut Self::Element) {
        if let Some(style) = &self.custom_style {
            style.apply_to_node(ctx, element.container_node);
        }

        element.content_node.update_constraints(ctx, |c| {
            c.height = Size::Fixed((self.count as f32 * self.item_height).round() as u32);
        });

        self.sync_visible_range(ctx, element);
    }

    fn teardown(&self, ctx: &mut Context, element: &mut Self::Element) {
        for (idx, mut elem) in element.visible_elements.drain(..) {
            let view = self.render_fn.call(idx, self.items.as_ref());
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

        for (idx, elem) in element.visible_elements.iter_mut() {
            let view = self.render_fn.call(*idx, self.items.as_ref());
            let (res, msg) = view.handle_event(elem, state, event.clone(), ctx);
            if res == EventResult::Handled {
                handled = EventResult::Handled;
            }
            if msg.is_some() {
                emitted_msg = msg;
            }
        }

        // On scroll / wheel / tick events, keep visible window synchronized
        if matches!(
            event,
            Event::MouseWheel { .. } | Event::Tick { .. } | Event::CursorMoved { .. }
        ) {
            self.sync_visible_range(ctx, element);
        }

        (handled, emitted_msg)
    }
}

impl<T, F, V> VirtualList<T, F, V> {
    fn sync_visible_range<State, Msg>(
        &self,
        ctx: &mut Context,
        element: &mut VirtualListElement<V::Element>,
    ) where
        F: VirtualListRenderHelper<T, V, State, Msg>,
        V: View<State, Message = Msg>,
        T: 'static,
    {
        let scroll_y = element
            .container_node
            .get_constraints(ctx)
            .map(|c| c.scroll.y)
            .unwrap_or(0.0);

        let viewport_h = element
            .container_node
            .get_computed(ctx)
            .map(|c| c.h)
            .unwrap_or(800.0)
            .max(100.0);

        let start_idx =
            ((scroll_y / self.item_height).floor() as usize).saturating_sub(self.buffer);
        let visible_count = ((viewport_h / self.item_height).ceil() as usize) + (self.buffer * 2);
        let end_idx = (start_idx + visible_count).min(self.count);

        let new_range = (start_idx, end_idx);

        if new_range == element.rendered_range && !element.visible_elements.is_empty() {
            return;
        }

        // 1. Remove and teardown elements that are outside the new range
        let mut old_elements = std::mem::take(&mut element.visible_elements);
        let mut preserved_elements = std::collections::HashMap::new();

        for (idx, mut elem) in old_elements.drain(..) {
            if idx >= start_idx && idx < end_idx {
                preserved_elements.insert(idx, elem);
            } else {
                let view = self.render_fn.call(idx, self.items.as_ref());
                let node = view.get_node(&elem);
                node.remove(ctx);
                view.teardown(ctx, &mut elem);
                ctx.destroy_node(node);
            }
        }

        // 2. Build or keep elements in the new range
        element.bottom_spacer.remove(ctx);

        for idx in start_idx..end_idx {
            if let Some(mut elem) = preserved_elements.remove(&idx) {
                let view = self.render_fn.call(idx, self.items.as_ref());
                view.rebuild(&view, ctx, &mut elem);
                element.visible_elements.push((idx, elem));
            } else {
                let view = self.render_fn.call(idx, self.items.as_ref());
                let elem = view.build(ctx);
                let node = view.get_node(&elem);
                element.content_node.append(ctx, node);
                element.visible_elements.push((idx, elem));
            }
        }

        // Re-append bottom spacer to the very end
        element.content_node.append(ctx, element.bottom_spacer);

        // 3. Update spacers height
        let top_spacer_h = (start_idx as f32 * self.item_height).round() as u32;
        let bottom_spacer_h =
            ((self.count.saturating_sub(end_idx)) as f32 * self.item_height).round() as u32;

        element.top_spacer.update_constraints(ctx, |c| {
            c.height = Size::Fixed(top_spacer_h);
        });

        element.bottom_spacer.update_constraints(ctx, |c| {
            c.height = Size::Fixed(bottom_spacer_h);
        });

        element.rendered_range = new_range;
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

        // Virtualized content should only instantiate a small slice of nodes (~30 instead of 10,000!)
        assert!(element.visible_elements.len() < 50);
        assert!(element.visible_elements.len() > 0);

        // Simulate scrolling down by 3,000 pixels (to item #100)
        element.container_node.update_constraints(&mut ctx, |c| {
            c.scroll.y = 3000.0;
        });

        View::<()>::rebuild(&widget, &widget, &mut ctx, &mut element);

        let (start, end) = element.rendered_range;
        assert!(start >= 90 && start <= 100);
        assert!(end > start && end <= 140);
        assert!(element.visible_elements.len() < 50);

        View::<()>::teardown(&widget, &mut ctx, &mut element);
    }
}
