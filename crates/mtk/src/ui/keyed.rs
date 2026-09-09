use std::collections::HashMap;
use std::hash::Hash;

use crate::{
    Context, Node,
    ui::{Event, View, ViewSequence, event::EventResult},
};

/// An individual view item paired with a stable, unique key for dynamic sequence reconciliation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Keyed<K, V> {
    /// Stable key identifying this item across frames.
    pub key: K,
    /// The inner view instance.
    pub view: V,
}

/// Constructs a new [`Keyed`] item pairing a stable key with a view.
///
/// # Examples
/// ```rust,ignore
/// column(keyed_sequence(items.iter().map(|item| {
///     keyed(item.id, text(&item.title))
/// })))
/// ```
pub fn keyed<K, V>(key: K, view: V) -> Keyed<K, V> {
    Keyed { key, view }
}

impl<K, V> From<(K, V)> for Keyed<K, V> {
    fn from((key, view): (K, V)) -> Self {
        Keyed { key, view }
    }
}

impl<State, K, V: View<State>> View<State> for Keyed<K, V> {
    type Element = V::Element;
    type Message = V::Message;

    fn build(&self, ctx: &mut Context) -> Self::Element {
        self.view.build(ctx)
    }

    fn rebuild_with_parent(
        &self,
        prev: &Self,
        ctx: &mut Context,
        element: &mut Self::Element,
        parent: Node,
        next_sibling: Option<Node>,
    ) {
        self.view
            .rebuild_with_parent(&prev.view, ctx, element, parent, next_sibling);
    }

    fn rebuild(&self, prev: &Self, ctx: &mut Context, element: &mut Self::Element) {
        self.view.rebuild(&prev.view, ctx, element);
    }

    fn teardown(&self, ctx: &mut Context, element: &mut Self::Element) {
        self.view.teardown(ctx, element);
    }

    fn get_node(&self, element: &Self::Element) -> Node {
        self.view.get_node(element)
    }

    fn handle_event(
        &self,
        element: &mut Self::Element,
        state: &State,
        event: Event,
        ctx: &mut Context,
    ) -> (EventResult, Option<Self::Message>) {
        self.view.handle_event(element, state, event, ctx)
    }
}

/// A dynamic sequence of keyed views reconciled by stable key identity across frames.
///
/// Unlike standard positional `Vec<V>` sequences which diff children strictly by array index,
/// `KeyedViewSequence` tracks items by key. When items are inserted, deleted, or reordered:
/// - Existing widget states (cursor positions, text selections, scroll offsets, focus) are preserved.
/// - Unmodified layout nodes are retained rather than repeatedly destroyed and reallocated.
/// - Layout tree sibling order is reconciled with minimal tree mutations via [`Node::put_before`] and [`Node::append`].
pub struct KeyedViewSequence<K, V> {
    /// Ordered list of keyed child items.
    pub items: Vec<Keyed<K, V>>,
}

impl<K, V> KeyedViewSequence<K, V> {
    /// Creates a new `KeyedViewSequence` from a vector of [`Keyed`] items.
    pub fn new(items: Vec<Keyed<K, V>>) -> Self {
        Self { items }
    }

    /// Returns the number of keyed items in the sequence.
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Returns `true` if the sequence contains no items.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

/// Creates a new [`KeyedViewSequence`] from an iterator of [`Keyed`] items.
pub fn keyed_sequence<K, V, I>(items: I) -> KeyedViewSequence<K, V>
where
    I: IntoIterator<Item = Keyed<K, V>>,
{
    KeyedViewSequence {
        items: items.into_iter().collect(),
    }
}

impl<K, V> FromIterator<Keyed<K, V>> for KeyedViewSequence<K, V> {
    fn from_iter<T: IntoIterator<Item = Keyed<K, V>>>(iter: T) -> Self {
        Self {
            items: iter.into_iter().collect(),
        }
    }
}

impl<K, V> FromIterator<(K, V)> for KeyedViewSequence<K, V> {
    fn from_iter<T: IntoIterator<Item = (K, V)>>(iter: T) -> Self {
        Self {
            items: iter.into_iter().map(Keyed::from).collect(),
        }
    }
}

impl<K, V> From<Vec<Keyed<K, V>>> for KeyedViewSequence<K, V> {
    fn from(items: Vec<Keyed<K, V>>) -> Self {
        Self { items }
    }
}

impl<State, Msg, K, V> ViewSequence<State> for KeyedViewSequence<K, V>
where
    K: Eq + Hash + Clone,
    V: View<State, Message = Msg>,
{
    type Elements = Vec<(K, V::Element)>;
    type Message = Msg;

    fn build(&self, ctx: &mut Context, parent: Node) -> Self::Elements {
        let mut elements = Vec::with_capacity(self.items.len());
        for item in &self.items {
            let el = item.view.build(ctx);
            let node = item.view.get_node(&el);
            parent.append(ctx, node);
            elements.push((item.key.clone(), el));
        }
        elements
    }

    fn rebuild(&self, prev: &Self, ctx: &mut Context, elements: &mut Self::Elements, parent: Node) {
        let n = self.items.len();
        let m = prev.items.len();

        // Fast path 1: Unchanged sequence of keys
        if n == m
            && self
                .items
                .iter()
                .zip(prev.items.iter())
                .all(|(a, b)| a.key == b.key)
        {
            for i in 0..n {
                self.items[i]
                    .view
                    .rebuild(&prev.items[i].view, ctx, &mut elements[i].1);
            }
            return;
        }

        // Fast path 2: Pure append to unchanged prefix
        if n > m
            && self.items[0..m]
                .iter()
                .zip(prev.items.iter())
                .all(|(a, b)| a.key == b.key)
        {
            for i in 0..m {
                self.items[i]
                    .view
                    .rebuild(&prev.items[i].view, ctx, &mut elements[i].1);
            }
            for i in m..n {
                let el = self.items[i].view.build(ctx);
                let node = self.items[i].view.get_node(&el);
                parent.append(ctx, node);
                elements.push((self.items[i].key.clone(), el));
            }
            return;
        }

        // Fast path 3: Pure truncation of unchanged prefix
        if n < m
            && self
                .items
                .iter()
                .zip(prev.items[0..n].iter())
                .all(|(a, b)| a.key == b.key)
        {
            for i in 0..n {
                self.items[i]
                    .view
                    .rebuild(&prev.items[i].view, ctx, &mut elements[i].1);
            }
            for i in n..m {
                let node = prev.items[i].view.get_node(&elements[i].1);
                node.remove(ctx);
                prev.items[i].view.teardown(ctx, &mut elements[i].1);
            }
            elements.truncate(n);
            return;
        }

        // General path: Arbitrary reordering, insertions, and deletions
        let old_elements = std::mem::take(elements);
        let mut old_map = HashMap::with_capacity(old_elements.len());
        for ((k, el), old_item) in old_elements.into_iter().zip(prev.items.iter()) {
            old_map.insert(k, (el, &old_item.view));
        }

        elements.reserve(n);
        for item in &self.items {
            let el = if let Some((mut old_el, old_view)) = old_map.remove(&item.key) {
                item.view
                    .rebuild_with_parent(old_view, ctx, &mut old_el, parent, None);
                old_el
            } else {
                item.view.build(ctx)
            };
            elements.push((item.key.clone(), el));
        }

        // Teardown and unmount removed items
        for (_key, (mut old_el, old_view)) in old_map {
            let node = old_view.get_node(&old_el);
            node.remove(ctx);
            old_view.teardown(ctx, &mut old_el);
        }

        // Resolve sibling layout order via reverse sweep
        for i in (0..n).rev() {
            let curr_node = self.items[i].view.get_node(&elements[i].1);
            if !curr_node.is_valid() {
                continue;
            }

            let next_valid_node = elements[(i + 1)..n]
                .iter()
                .enumerate()
                .map(|(offset, (_, el))| self.items[i + 1 + offset].view.get_node(el))
                .find(|n| n.is_valid());

            if let Some(next_node) = next_valid_node {
                if curr_node.next_sibling(ctx) != Some(next_node) {
                    curr_node.put_before(ctx, next_node);
                }
            } else if parent.last_child(ctx) != Some(curr_node) {
                parent.append(ctx, curr_node);
            }
        }
    }

    fn teardown(&self, ctx: &mut Context, elements: &mut Self::Elements) {
        for (item, (_key, el)) in self.items.iter().zip(elements.iter_mut()) {
            item.view.teardown(ctx, el);
        }
    }

    fn handle_event(
        &self,
        elements: &mut Self::Elements,
        state: &State,
        event: Event,
        ctx: &mut Context,
    ) -> (EventResult, Option<Self::Message>) {
        let is_tick = matches!(event, Event::Tick { .. });
        let mut handled = EventResult::Ignored;
        let mut emitted_msg = None;

        for (item, (_key, el)) in self.items.iter().zip(elements.iter_mut()) {
            if (is_tick || handled == EventResult::Ignored) && emitted_msg.is_none() {
                let (res, msg) = item.view.handle_event(el, state, event.clone(), ctx);
                handled = handled.or(res);
                if msg.is_some() {
                    emitted_msg = msg;
                }
            }
        }

        (handled, emitted_msg)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::widgets::{button, column, input_text};

    #[test]
    fn test_keyed_sequence_reorder_preserves_node_identities() {
        let mut ctx = Context::new();

        let seq_init = column(keyed_sequence([
            keyed(1, button::<_, ()>("One")),
            keyed(2, button::<_, ()>("Two")),
            keyed(3, button::<_, ()>("Three")),
        ]));

        let mut el = View::<()>::build(&seq_init, &mut ctx);
        let parent = View::<()>::get_node(&seq_init, &el);

        let initial_children = parent.children(&ctx);
        assert_eq!(initial_children.len(), 3);
        let node_1 = initial_children[0];
        let node_2 = initial_children[1];
        let node_3 = initial_children[2];

        // Reorder to [3, 1, 2]
        let seq_reordered = column(keyed_sequence([
            keyed(3, button::<_, ()>("Three (Updated)")),
            keyed(1, button::<_, ()>("One (Updated)")),
            keyed(2, button::<_, ()>("Two (Updated)")),
        ]));

        View::<()>::rebuild(&seq_reordered, &seq_init, &mut ctx, &mut el);

        let updated_children = parent.children(&ctx);
        assert_eq!(updated_children.len(), 3);
        // Verify nodes were preserved and rearranged exactly to [3, 1, 2]
        assert_eq!(updated_children[0], node_3);
        assert_eq!(updated_children[1], node_1);
        assert_eq!(updated_children[2], node_2);

        View::<()>::teardown(&seq_reordered, &mut ctx, &mut el);
    }

    #[test]
    fn test_keyed_sequence_insert_at_head() {
        let mut ctx = Context::new();

        let seq_init = column(keyed_sequence([
            keyed(2, button::<_, ()>("Two")),
            keyed(3, button::<_, ()>("Three")),
        ]));

        let mut el = View::<()>::build(&seq_init, &mut ctx);
        let parent = View::<()>::get_node(&seq_init, &el);

        let initial_children = parent.children(&ctx);
        let node_2 = initial_children[0];
        let node_3 = initial_children[1];

        // Prepend item 1: [1, 2, 3]
        let seq_prepended = column(keyed_sequence([
            keyed(1, button::<_, ()>("One")),
            keyed(2, button::<_, ()>("Two")),
            keyed(3, button::<_, ()>("Three")),
        ]));

        View::<()>::rebuild(&seq_prepended, &seq_init, &mut ctx, &mut el);

        let updated_children = parent.children(&ctx);
        assert_eq!(updated_children.len(), 3);
        let node_1 = updated_children[0];
        assert_ne!(node_1, node_2);
        assert_ne!(node_1, node_3);
        assert_eq!(updated_children[1], node_2);
        assert_eq!(updated_children[2], node_3);

        View::<()>::teardown(&seq_prepended, &mut ctx, &mut el);
    }

    #[test]
    fn test_keyed_sequence_remove_middle() {
        let mut ctx = Context::new();

        let seq_init = column(keyed_sequence([
            keyed(1, button::<_, ()>("One")),
            keyed(2, button::<_, ()>("Two")),
            keyed(3, button::<_, ()>("Three")),
        ]));

        let mut el = View::<()>::build(&seq_init, &mut ctx);
        let parent = View::<()>::get_node(&seq_init, &el);

        let initial_children = parent.children(&ctx);
        let node_1 = initial_children[0];
        let node_2 = initial_children[1];
        let node_3 = initial_children[2];

        // Remove item 2: [1, 3]
        let seq_removed = column(keyed_sequence([
            keyed(1, button::<_, ()>("One")),
            keyed(3, button::<_, ()>("Three")),
        ]));

        View::<()>::rebuild(&seq_removed, &seq_init, &mut ctx, &mut el);

        let updated_children = parent.children(&ctx);
        assert_eq!(updated_children.len(), 2);
        assert_eq!(updated_children[0], node_1);
        assert_eq!(updated_children[1], node_3);
        // Node 2 must no longer be in the layout hierarchy
        assert!(!node_2.is_valid() || node_2.parent(&ctx).is_none());

        View::<()>::teardown(&seq_removed, &mut ctx, &mut el);
    }

    #[test]
    fn test_keyed_sequence_preserves_input_text_state() {
        let mut ctx = Context::new();

        let seq_init = column(keyed_sequence([
            keyed(1, input_text()),
            keyed(2, input_text()),
        ]));

        let mut el = View::<String>::build(&seq_init, &mut ctx);

        // Modify internal editor state of item 1 and item 2
        el.1[0].1.editor_mut().set_text("Alpha");
        el.1[1].1.editor_mut().set_text("Beta");
        assert_eq!(el.1[0].1.editor().text(), "Alpha");
        assert_eq!(el.1[1].1.editor().text(), "Beta");

        // Reorder to [2, 1]
        let seq_reordered = column(keyed_sequence([
            keyed(2, input_text()),
            keyed(1, input_text()),
        ]));

        View::<String>::rebuild(&seq_reordered, &seq_init, &mut ctx, &mut el);

        // el.1 is the Elements: Vec<(i32, InputTextElement)>
        assert_eq!(el.1.len(), 2);
        assert_eq!(el.1[0].0, 2);
        assert_eq!(el.1[0].1.editor().text(), "Beta");
        assert_eq!(el.1[1].0, 1);
        assert_eq!(el.1[1].1.editor().text(), "Alpha");

        View::<String>::teardown(&seq_reordered, &mut ctx, &mut el);
    }

    #[test]
    fn test_keyed_sequence_event_routing() {
        let mut ctx = Context::new();

        let seq_init = column(keyed_sequence([
            keyed("a", button::<_, i32>("Btn A").on_click(10)),
            keyed("b", button::<_, i32>("Btn B").on_click(20)),
        ]));

        let mut el = View::<()>::build(&seq_init, &mut ctx);

        // Reorder to ["b", "a"]
        let seq_reordered = column(keyed_sequence([
            keyed("b", button::<_, i32>("Btn B").on_click(20)),
            keyed("a", button::<_, i32>("Btn A").on_click(10)),
        ]));

        View::<()>::rebuild(&seq_reordered, &seq_init, &mut ctx, &mut el);
        let parent = View::<()>::get_node(&seq_reordered, &el);
        let target_node = parent.children(&ctx)[0];
        let click_event = Event::MouseInput {
            button: winit::event::MouseButton::Left,
            pressed: false,
            x: 0.0,
            y: 0.0,
            hit_nodes: vec![target_node],
        };

        // First simulate mouse down
        let down_event = Event::MouseInput {
            button: winit::event::MouseButton::Left,
            pressed: true,
            x: 0.0,
            y: 0.0,
            hit_nodes: vec![target_node],
        };
        View::<()>::handle_event(&seq_reordered, &mut el, &(), down_event, &mut ctx);

        let (res, msg) =
            View::<()>::handle_event(&seq_reordered, &mut el, &(), click_event, &mut ctx);
        assert_eq!(res, EventResult::Handled);
        assert_eq!(msg, Some(20));

        View::<()>::teardown(&seq_reordered, &mut ctx, &mut el);
    }

    #[test]
    fn test_keyed_sequence_complex_shuffle() {
        let mut ctx = Context::new();

        let keys_init = [1, 2, 3, 4, 5, 6, 7, 8];
        let seq_init = column(keyed_sequence(
            keys_init
                .iter()
                .map(|&k| keyed(k, button::<_, ()>(format!("{k}")))),
        ));

        let mut el = View::<()>::build(&seq_init, &mut ctx);
        let parent = View::<()>::get_node(&seq_init, &el);

        let old_children = parent.children(&ctx);
        assert_eq!(old_children.len(), 8);
        let old_node_map: std::collections::HashMap<i32, Node> = keys_init
            .iter()
            .copied()
            .zip(old_children.iter().copied())
            .collect();

        // Target: [8, 3, 10, 5, 1, 9, 7]
        // Preserved: 8, 3, 5, 1, 7
        // Removed: 2, 4, 6
        // Inserted: 10, 9
        let keys_target = [8, 3, 10, 5, 1, 9, 7];
        let seq_target = column(keyed_sequence(
            keys_target
                .iter()
                .map(|&k| keyed(k, button::<_, ()>(format!("{k}")))),
        ));

        View::<()>::rebuild(&seq_target, &seq_init, &mut ctx, &mut el);

        let new_children = parent.children(&ctx);
        assert_eq!(new_children.len(), 7);

        // Verify preserved nodes match exactly
        assert_eq!(new_children[0], old_node_map[&8]);
        assert_eq!(new_children[1], old_node_map[&3]);
        assert_eq!(new_children[3], old_node_map[&5]);
        assert_eq!(new_children[4], old_node_map[&1]);
        assert_eq!(new_children[6], old_node_map[&7]);

        // Verify inserted nodes are brand new
        assert!(!old_children.contains(&new_children[2]));
        assert!(!old_children.contains(&new_children[5]));

        // Verify removed nodes are detached
        for removed_key in [2, 4, 6] {
            let removed_node = old_node_map[&removed_key];
            assert!(!removed_node.is_valid() || removed_node.parent(&ctx).is_none());
        }

        View::<()>::teardown(&seq_target, &mut ctx, &mut el);
    }
}
