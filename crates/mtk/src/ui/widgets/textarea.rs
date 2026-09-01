use std::time::Instant;

use crate::debugger::SourceLocation;
use crate::style::Overflow;
use crate::ui::event::EventResult;
use crate::ui::widgets::editor::Editor;
use crate::ui::{Event, View};
use crate::{Context, Node, TextRenderInfo, TextStyle};
use winit::event::Ime;
use winit::keyboard::{Key, NamedKey};

/// A multi-line text area widget with full multiline cursor navigation, selection, line wrapping,
/// mouse-wheel & drag scrolling, automatic shadow overlay scrollbars, and IME composition support.
///
/// [TextArea] maps to a multi-line [String] state and emits updated [String] messages whenever the user types
/// or modifies the text.
pub struct TextArea {
    pub(crate) captures_tab: bool,
    pub(crate) custom_style: Option<crate::style::Style>,
    pub(crate) source_loc: Option<SourceLocation>,
}

/// Creates a new `TextArea` widget.
///
/// # Examples
/// ```rust,ignore
/// adapt(
///     text_area().style(Style::new().width(Size::Fixed(400)).height(Size::Fixed(200))),
///     AppState::bio,
///     AppMsg::UpdateBio,
/// )
/// ```
#[track_caller]
pub fn text_area() -> TextArea {
    TextArea {
        captures_tab: true,
        custom_style: None,
        source_loc: Some(SourceLocation::here("TextArea")),
    }
}

impl TextArea {
    /// Configures whether this text area captures the `Tab` key to insert 4 spaces.
    ///
    /// Defaults to `true` for [TextArea].
    pub fn captures_tab(mut self, captures: bool) -> Self {
        self.captures_tab = captures;
        self
    }

    /// Applies custom layout, visual effects, and typography styling to this text area.
    pub fn style(mut self, style: crate::style::Style) -> Self {
        self.custom_style = Some(style);
        self
    }

    fn apply_custom_style(&self, ctx: &mut Context, node: Node) {
        if let Some(style) = &self.custom_style {
            let is_focused = Some(node) == ctx.focused_node();
            let mut target = style.clone();
            if is_focused {
                if let Some(focus) = &style.focus {
                    target = target.merge((**focus).clone());
                }
            }
            target.apply_to_node(ctx, node);
        }
    }

    fn sync_render_nodes(&self, ctx: &mut Context, element: &mut TextAreaInner) {
        let text_style = if let Some(ref style) = self.custom_style {
            let is_focused = Some(element.node.clone()) == ctx.focused_node();
            if is_focused && let Some(focus) = &style.focus {
                focus.base_text_style.clone()
            } else {
                style.base_text_style.clone()
            }
        } else if let Some(info) = element.node.get_text_userdata::<TextRenderInfo>(ctx) {
            info.style.clone()
        } else if let Some(style) = element.node.get_text_userdata::<TextStyle>(ctx) {
            style.clone()
        } else {
            TextStyle::default()
        };

        let is_focused = Some(element.node.clone()) == ctx.focused_node();

        let render_info = TextRenderInfo {
            style: text_style,
            cursor: if is_focused {
                Some(element.editor.display_cursor())
            } else {
                None
            },
            selection: if is_focused {
                element.editor.selection()
            } else {
                None
            },
            preedit_range: element.editor.preedit_range(),
            spans: Vec::new(),
        };
        element
            .node
            .set_text_with_userdata(ctx, &element.editor.display_text(), render_info);
    }
}

pub struct TextAreaInner {
    node: Node,
    editor: Editor,
    caret: Node,
    is_dragging_text: bool,
    last_click: Option<Instant>,
    click_count: u8,
}

impl TextAreaInner {
    pub fn new(node: Node, editor: Editor, caret: Node) -> Self {
        Self {
            node,
            editor,
            caret,
            is_dragging_text: false,
            last_click: None,
            click_count: 0,
        }
    }
}

impl View<String> for TextArea {
    type Element = TextAreaInner;
    type Message = String;

    fn build(&self, ctx: &mut Context) -> Self::Element {
        let node = ctx.create_node();
        if let Some(loc) = self.source_loc {
            ctx.set_node_source(node.clone(), loc);
        }
        let mut editor = Editor::new();
        editor.set_text("");

        ctx.register_focusable(node.clone());

        let caret = ctx.create_node();
        node.append(ctx, caret.clone());

        node.update_constraints(ctx, |c| {
            c.overflow = Overflow::Scroll;
        });

        self.apply_custom_style(ctx, node.clone());

        let mut inner = TextAreaInner::new(node, editor, caret);
        self.sync_render_nodes(ctx, &mut inner);
        inner
    }

    fn rebuild(&self, _prev: &Self, ctx: &mut Context, element: &mut Self::Element) {
        self.apply_custom_style(ctx, element.node.clone());
        self.sync_render_nodes(ctx, element);
    }

    fn teardown(&self, ctx: &mut Context, element: &mut Self::Element) {
        ctx.unregister_focusable(element.node);
        element.caret.remove(ctx);
        ctx.destroy_node(element.caret);
        element.node.remove(ctx);
        ctx.destroy_node(element.node);
    }

    fn get_node(&self, element: &Self::Element) -> Node {
        element.node.clone()
    }

    fn handle_event(
        &self,
        element: &mut Self::Element,
        state: &String,
        event: Event,
        ctx: &mut Context,
    ) -> (EventResult, Option<Self::Message>) {
        let cursor_before = element.editor.cursor();

        let mut handled = EventResult::Ignored;
        let mut emitted_msg: Option<String> = None;

        // Immediately sync editor state from incoming state if external state changed
        if element.editor.text() != *state {
            element.editor.set_text(state);
            element.node.update_constraints(ctx, |c| {
                c.scroll.x = 0.0;
                c.scroll.y = 0.0;
            });
            self.sync_render_nodes(ctx, element);
        }

        match event {
            Event::MouseInput {
                pressed,
                x,
                y,
                hit_nodes,
            } => {
                let is_hit = hit_nodes.iter().any(|n| *n == element.node);
                let is_focused = Some(element.node.clone()) == ctx.focused_node();

                if pressed {
                    if is_hit {
                        ctx.request_focus(element.node.clone());
                        handled = EventResult::Handled;

                        if let Some(computed) = element.node.get_computed(ctx) {
                            let constraints = element.node.get_constraints(ctx).unwrap_or_default();
                            let rel_x =
                                x - computed.x - constraints.padding.left + constraints.scroll.x;
                            let rel_y =
                                y - computed.y - constraints.padding.top + constraints.scroll.y;
                            let shift = ctx.modifiers().shift_key();

                            let inner_w =
                                (computed.w - constraints.padding.left - constraints.padding.right)
                                    .max(0.0);
                            let inner_h =
                                (computed.h - constraints.padding.top - constraints.padding.bottom)
                                    .max(0.0);

                            let mut text_style = TextStyle::default();
                            if let Some(info) =
                                element.node.get_text_userdata::<TextRenderInfo>(ctx)
                            {
                                text_style = info.style.clone();
                            } else if let Some(style) =
                                element.node.get_text_userdata::<TextStyle>(ctx)
                            {
                                text_style = style.clone();
                            }

                            let index = crate::text::hit_test_text(
                                &element.editor.display_text(),
                                &text_style,
                                inner_w,
                                inner_h,
                                rel_x,
                                rel_y,
                                &ctx.text_context,
                                &[],
                            );

                            let now = Instant::now();
                            let mut click_count = 1;
                            if let Some(last_click) = element.last_click {
                                if now.duration_since(last_click).as_millis() < 500 {
                                    click_count = element.click_count + 1;
                                }
                            }
                            element.last_click = Some(now);
                            element.click_count = click_count;

                            if click_count == 2 {
                                element.editor.set_cursor(index);
                                element.editor.set_selection_anchor(Some(index));
                                element.editor.move_word_left(false);
                                let start = element.editor.cursor();
                                element.editor.move_word_right(true);
                                let end = element.editor.cursor();
                                element.editor.set_selection_anchor(Some(start));
                                element.editor.set_cursor(end);
                            } else if click_count >= 3 {
                                element.editor.select_all();
                            } else {
                                if shift {
                                    if element.editor.selection_anchor().is_none() {
                                        element
                                            .editor
                                            .set_selection_anchor(Some(element.editor.cursor()));
                                    }
                                    element.editor.set_cursor(index);
                                } else {
                                    element.editor.set_selection_anchor(None);
                                    element.editor.set_cursor(index);
                                }
                            }
                            ctx.request_frame();
                        }
                        element.is_dragging_text = true;
                    } else if is_focused {
                        ctx.clear_focus();
                    }
                } else {
                    element.is_dragging_text = false;
                }
            }
            Event::CursorMoved { x, y, .. } => {
                if element.is_dragging_text {
                    if let Some(computed) = element.node.get_computed(ctx) {
                        let constraints = element.node.get_constraints(ctx).unwrap_or_default();
                        let rel_x =
                            x - computed.x - constraints.padding.left + constraints.scroll.x;
                        let rel_y = y - computed.y - constraints.padding.top + constraints.scroll.y;

                        let inner_w =
                            (computed.w - constraints.padding.left - constraints.padding.right)
                                .max(0.0);
                        let inner_h =
                            (computed.h - constraints.padding.top - constraints.padding.bottom)
                                .max(0.0);

                        let mut text_style = TextStyle::default();
                        if let Some(info) = element.node.get_text_userdata::<TextRenderInfo>(ctx) {
                            text_style = info.style.clone();
                        } else if let Some(style) = element.node.get_text_userdata::<TextStyle>(ctx)
                        {
                            text_style = style.clone();
                        }

                        let index = crate::text::hit_test_text(
                            &element.editor.display_text(),
                            &text_style,
                            inner_w,
                            inner_h,
                            rel_x,
                            rel_y,
                            &ctx.text_context,
                            &[],
                        );

                        if element.editor.selection_anchor().is_none() {
                            element
                                .editor
                                .set_selection_anchor(Some(element.editor.cursor()));
                        }
                        element.editor.set_cursor(index);
                        ctx.request_frame();
                    }
                }
            }
            Event::Ime(ref ime) => {
                let is_focused = Some(element.node.clone()) == ctx.focused_node();
                if is_focused {
                    match ime {
                        Ime::Enabled => {}
                        Ime::Preedit(text, cursor) => {
                            element.editor.set_ime_preedit(text.clone(), *cursor);
                            ctx.request_frame();
                            handled = EventResult::Handled;
                        }
                        Ime::Commit(text) => {
                            element.editor.commit_ime(text);
                            emitted_msg = Some(element.editor.text().to_string());
                            ctx.request_frame();
                            handled = EventResult::Handled;
                        }
                        Ime::Disabled => {
                            element.editor.set_ime_preedit(String::new(), None);
                            ctx.request_frame();
                        }
                    }
                }
            }
            Event::KeyboardInput { ref event, .. } => {
                let is_focused = Some(element.node.clone()) == ctx.focused_node();
                if is_focused && event.state.is_pressed() {
                    let shift = ctx.modifiers().shift_key();
                    let ctrl_alt = ctx.modifiers().control_key()
                        || ctx.modifiers().alt_key()
                        || ctx.modifiers().super_key();

                    match event.logical_key.as_ref() {
                        Key::Named(NamedKey::ArrowLeft) => {
                            if ctrl_alt {
                                element.editor.move_word_left(shift);
                            } else {
                                element.editor.move_left(shift);
                            }
                            handled = EventResult::Handled;
                            ctx.request_frame();
                        }
                        Key::Named(NamedKey::ArrowRight) => {
                            if ctrl_alt {
                                element.editor.move_word_right(shift);
                            } else {
                                element.editor.move_right(shift);
                            }
                            handled = EventResult::Handled;
                            ctx.request_frame();
                        }
                        Key::Named(NamedKey::ArrowUp) => {
                            element.editor.move_up(shift);
                            handled = EventResult::Handled;
                            ctx.request_frame();
                        }
                        Key::Named(NamedKey::ArrowDown) => {
                            element.editor.move_down(shift);
                            handled = EventResult::Handled;
                            ctx.request_frame();
                        }
                        Key::Named(NamedKey::Home) => {
                            if ctrl_alt {
                                element.editor.move_to_start(shift);
                            } else {
                                element.editor.move_line_start(shift);
                            }
                            handled = EventResult::Handled;
                            ctx.request_frame();
                        }
                        Key::Named(NamedKey::End) => {
                            if ctrl_alt {
                                element.editor.move_to_end(shift);
                            } else {
                                element.editor.move_line_end(shift);
                            }
                            handled = EventResult::Handled;
                            ctx.request_frame();
                        }
                        Key::Named(NamedKey::PageUp) => {
                            element.editor.move_up(shift);
                            handled = EventResult::Handled;
                            ctx.request_frame();
                        }
                        Key::Named(NamedKey::PageDown) => {
                            element.editor.move_down(shift);
                            handled = EventResult::Handled;
                            ctx.request_frame();
                        }
                        Key::Named(NamedKey::Backspace) => {
                            if ctrl_alt {
                                element.editor.delete_word_backward();
                            } else {
                                element.editor.delete_backward();
                            }
                            emitted_msg = Some(element.editor.display_text().to_string());
                            handled = EventResult::Handled;
                            ctx.request_frame();
                        }
                        Key::Named(NamedKey::Delete) => {
                            if ctrl_alt {
                                element.editor.delete_word_forward();
                            } else {
                                element.editor.delete_forward();
                            }
                            emitted_msg = Some(element.editor.display_text().to_string());
                            handled = EventResult::Handled;
                            ctx.request_frame();
                        }
                        Key::Named(NamedKey::Enter) => {
                            element.editor.insert("\n");
                            emitted_msg = Some(element.editor.display_text().to_string());
                            handled = EventResult::Handled;
                            ctx.request_frame();
                        }
                        Key::Named(NamedKey::Tab) => {
                            if ctrl_alt {
                                handled = EventResult::Ignored;
                            } else if self.captures_tab {
                                element.editor.insert("    ");
                                emitted_msg = Some(element.editor.display_text().to_string());
                                handled = EventResult::Handled;
                                ctx.request_frame();
                            } else {
                                if shift {
                                    ctx.focus_prev();
                                } else {
                                    ctx.focus_next();
                                }
                                handled = EventResult::Handled;
                            }
                        }
                        Key::Character(s) if ctrl_alt && (s == "a" || s == "A") => {
                            element.editor.select_all();
                            handled = EventResult::Handled;
                            ctx.request_frame();
                        }
                        Key::Character(s) if ctrl_alt && (s == "c" || s == "C") => {
                            if let Some((start, end)) = element.editor.selection() {
                                let display_str = element.editor.display_text();
                                if start < display_str.len() && end <= display_str.len() {
                                    ctx.clipboard_copy(crate::ClipboardData::Text(
                                        display_str[start..end].to_string(),
                                    ));
                                }
                            }
                            handled = EventResult::Handled;
                        }
                        Key::Character(s) if ctrl_alt && (s == "x" || s == "X") => {
                            if let Some((start, end)) = element.editor.selection() {
                                let display_str = element.editor.display_text();
                                if start < display_str.len() && end <= display_str.len() {
                                    ctx.clipboard_copy(crate::ClipboardData::Text(
                                        display_str[start..end].to_string(),
                                    ));
                                    element.editor.delete_backward();
                                    emitted_msg = Some(element.editor.display_text().to_string());
                                }
                            }
                            handled = EventResult::Handled;
                            ctx.request_frame();
                        }
                        Key::Character(s) if ctrl_alt && (s == "z" || s == "Z") => {
                            if shift {
                                if element.editor.redo() {
                                    emitted_msg = Some(element.editor.display_text().to_string());
                                }
                            } else if element.editor.undo() {
                                emitted_msg = Some(element.editor.display_text().to_string());
                            }
                            handled = EventResult::Handled;
                            ctx.request_frame();
                        }
                        Key::Character(s) if ctrl_alt && (s == "y" || s == "Y") => {
                            if element.editor.redo() {
                                emitted_msg = Some(element.editor.display_text().to_string());
                            }
                            handled = EventResult::Handled;
                            ctx.request_frame();
                        }
                        Key::Character(s) if ctrl_alt && (s == "v" || s == "V") => {
                            if let Some(crate::ClipboardData::Text(pasted)) = ctx.clipboard_get() {
                                element.editor.insert(&pasted);
                                emitted_msg = Some(element.editor.display_text().to_string());
                            }
                            handled = EventResult::Handled;
                            ctx.request_frame();
                        }
                        _ => {
                            if let Some(ref text) = event.text {
                                if !text.is_empty() && !ctrl_alt {
                                    if text.chars().all(|c| !c.is_control()) {
                                        element.editor.insert(text.as_str());
                                        emitted_msg =
                                            Some(element.editor.display_text().to_string());
                                        handled = EventResult::Handled;
                                        ctx.request_frame();
                                    }
                                }
                            }
                        }
                    }
                }
            }
            _ => {}
        }

        self.sync_render_nodes(ctx, element);

        let cursor_after = element.editor.cursor();
        if cursor_before != cursor_after || emitted_msg.is_some() {
            if let Some(computed) = element.node.get_computed(ctx) {
                if computed.w > 0.0 {
                    let constraints = element.node.get_constraints(ctx).unwrap_or_default();
                    let inner_w =
                        (computed.w - constraints.padding.left - constraints.padding.right)
                            .max(0.0);
                    let inner_h =
                        (computed.h - constraints.padding.top - constraints.padding.bottom)
                            .max(0.0);

                    let mut text_style_for_scroll = TextStyle::default();
                    if let Some(info) = element.node.get_text_userdata::<TextRenderInfo>(ctx) {
                        text_style_for_scroll = info.style.clone();
                    } else if let Some(style) = element.node.get_text_userdata::<TextStyle>(ctx) {
                        text_style_for_scroll = style.clone();
                    }

                    let (cx, cy, ch) = crate::text::get_cursor_geometry(
                        &element.editor.display_text(),
                        &text_style_for_scroll,
                        inner_w,
                        cursor_after,
                        &ctx.text_context,
                    );

                    let measured = crate::text::measure_text(
                        &element.editor.display_text(),
                        &text_style_for_scroll,
                        inner_w,
                        inner_h,
                        &ctx.text_context,
                        &[],
                    );

                    let mut scroll_x = constraints.scroll.x;
                    let mut scroll_y = constraints.scroll.y;

                    let cursor_w = 1.0;

                    if cx < scroll_x {
                        scroll_x = cx;
                    } else if cx + cursor_w > scroll_x + inner_w {
                        scroll_x = cx + cursor_w - inner_w;
                    }

                    if cy < scroll_y {
                        scroll_y = cy;
                    } else if cy + ch > scroll_y + inner_h {
                        scroll_y = cy + ch - inner_h;
                    }

                    let max_scroll_x = (measured.computed_width + cursor_w - inner_w).max(0.0);
                    let max_scroll_y = (measured.computed_height - inner_h).max(0.0);

                    scroll_x = scroll_x.clamp(0.0, max_scroll_x);
                    scroll_y = scroll_y.clamp(0.0, max_scroll_y);

                    if scroll_x != constraints.scroll.x || scroll_y != constraints.scroll.y {
                        element.node.update_constraints(ctx, |c| {
                            c.scroll.x = scroll_x;
                            c.scroll.y = scroll_y;
                        });
                        ctx.request_frame();
                    }
                }
            }
        }

        self.apply_custom_style(ctx, element.node.clone());
        self.sync_render_nodes(ctx, element);

        (handled, emitted_msg)
    }
}
