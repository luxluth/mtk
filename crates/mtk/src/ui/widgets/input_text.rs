use std::time::Instant;

use crate::ui::event::EventResult;
use crate::ui::widgets::editor::Editor;
use crate::ui::{Event, View};
use crate::{Context, Node, TextRenderInfo, TextStyle};
use winit::keyboard::{Key, NamedKey};

/// A single-line text input widget with built-in cursor, selection, and auto-scroll support.
///
/// [InputText] maps to a [String] state and emits [String] messages whenever the user types
/// or modifies the text. It should typically be wrapped in an `adapt` call to map its local
/// string state to your global application state.
pub struct InputText {
    pub(crate) captures_tab: bool,
}

// NOTE: Under the hood, this widget utilizes the `Editor` struct to manage text buffers, unicode
// boundaries, and cursor movements. The view's bounds use `Overflow::Hidden` and it calculates
// cursor layout geometry to smoothly clamp scroll constraints and keep the cursor in view when typing.

/// Creates a new `InputText` widget.
///
/// # Examples
/// ```rust,ignore
/// adapt(
///     input_text().style(Style::new().width(Size::Fixed(300))),
///     AppState::username,
///     AppMsg::UpdateUsername,
/// )
/// ```
pub fn input_text() -> InputText {
    InputText {
        captures_tab: false,
    }
}

impl InputText {
    /// Configures whether this input captures the `Tab` key to insert a tab character (4 spaces).
    ///
    /// If `false` (default), pressing `Tab` will bubble up and typically move focus to the
    /// next focusable widget. If `true`, pressing `Tab` inserts a tab character into the text.
    pub fn captures_tab(mut self, captures: bool) -> Self {
        self.captures_tab = captures;
        self
    }
}

pub struct InputInner {
    node: Node,
    editor: Editor,
    caret: Node,
    is_dragging: bool,
    last_click: Option<Instant>,
    click_count: u8,
}

impl InputInner {
    pub fn new(node: Node, editor: Editor, caret: Node) -> Self {
        Self {
            node,
            editor,
            caret,
            is_dragging: false,
            last_click: None,
            click_count: 0,
        }
    }
}

impl View<String> for InputText {
    type Element = InputInner; // Node, Editor, Caret, IsDragging, LastClick, ClickCount
    type Message = String;

    fn build(&self, ctx: &mut Context) -> Self::Element {
        let node = ctx.create_node();
        let mut editor = Editor::new();
        editor.set_text("");

        ctx.register_focusable(node.clone());

        let caret = ctx.create_node();
        node.append(ctx, caret.clone());
        node.update_constraints(ctx, |c| {
            c.overflow = crate::style::Overflow::Hidden;
        });

        InputInner::new(node, editor, caret)
    }

    fn rebuild(&self, _prev: &Self, ctx: &mut Context, element: &mut Self::Element) {
        // Preserve the user's TextStyle if they set one via StyleWrap
        let mut text_style = TextStyle::default();
        if let Some(info) = element.node.get_text_userdata::<TextRenderInfo>(ctx) {
            text_style = info.style.clone();
        } else if let Some(style) = element.node.get_text_userdata::<TextStyle>(ctx) {
            text_style = style.clone();
        }

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
        };
        element
            .node
            .set_text_with_userdata(ctx, &element.editor.display_text(), render_info);
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
        let mut emitted_msg = None;

        // Initialize editor state if this is the first tick and the text is not empty
        if element.editor.text() != *state {
            if let Event::Tick { dt: _ } = event {
                element.editor.set_text(&state);
                ctx.request_frame();
            }
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
                                element.editor.select_all(); // For single line text input, select all is fine
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
                    } else if is_focused {
                        ctx.clear_focus();
                    }
                    if is_hit {
                        element.is_dragging = true;
                    }
                } else {
                    element.is_dragging = false;
                }
            }
            Event::CursorMoved { x, y, hit_nodes: _ } => {
                if element.is_dragging {
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
            Event::KeyboardInput {
                event: key_event,
                is_synthetic: _,
            } => {
                if ctx.focused_node() == Some(element.node.clone()) && key_event.state.is_pressed()
                {
                    handled = EventResult::Handled;
                    let shift = ctx.modifiers().shift_key();
                    let ctrl_alt = ctx.modifiers().control_key() || ctx.modifiers().alt_key();

                    let mut text_changed = false;

                    match key_event.logical_key.as_ref() {
                        Key::Named(NamedKey::ArrowLeft) => {
                            if ctrl_alt {
                                element.editor.move_word_left(shift);
                            } else {
                                element.editor.move_left(shift);
                            }
                            ctx.request_frame();
                        }
                        Key::Named(NamedKey::ArrowRight) => {
                            if ctrl_alt {
                                element.editor.move_word_right(shift);
                            } else {
                                element.editor.move_right(shift);
                            }
                            ctx.request_frame();
                        }
                        Key::Named(NamedKey::Backspace) => {
                            if ctrl_alt {
                                element.editor.delete_word_backward();
                            } else {
                                element.editor.delete_backward();
                            }
                            text_changed = true;
                        }
                        Key::Named(NamedKey::Delete) => {
                            if ctrl_alt {
                                element.editor.delete_word_forward();
                            } else {
                                element.editor.delete_forward();
                            }
                            text_changed = true;
                        }
                        Key::Named(NamedKey::Home) => {
                            element.editor.move_to_start(shift);
                            ctx.request_frame();
                        }
                        Key::Named(NamedKey::End) => {
                            element.editor.move_to_end(shift);
                            ctx.request_frame();
                        }
                        Key::Named(NamedKey::Tab) => {
                            if ctrl_alt {
                                // Ignore Ctrl+Tab so it bubbles (or just do nothing here)
                                handled = EventResult::Ignored;
                            } else if self.captures_tab {
                                element.editor.insert("    ");
                                text_changed = true;
                            } else {
                                if shift {
                                    ctx.focus_prev();
                                } else {
                                    ctx.focus_next();
                                }
                            }
                        }
                        Key::Character(s) if ctrl_alt && (s == "a" || s == "A") => {
                            element.editor.select_all();
                            ctx.request_frame();
                        }
                        _ => {
                            if let Some(text) = &key_event.text {
                                if !text.is_empty() && !ctrl_alt {
                                    // NOTE: we avoid inserting control chars
                                    if text.chars().all(|c| !c.is_control()) {
                                        element.editor.insert(text.as_str());
                                        text_changed = true;
                                    }
                                }
                            }
                        }
                    }

                    if text_changed {
                        emitted_msg = Some(element.editor.display_text().to_string());
                        ctx.request_frame();
                    }
                }
            }
            Event::Ime(ime) => {
                if ctx.focused_node() == Some(element.node.clone()) {
                    use winit::event::Ime;
                    match ime {
                        Ime::Enabled => {}
                        Ime::Preedit(text, cursor_pos) => {
                            element.editor.set_ime_preedit(text, cursor_pos);
                            ctx.request_frame();
                            handled = EventResult::Handled;
                        }
                        Ime::Commit(text) => {
                            element.editor.commit_ime(&text);
                            emitted_msg = Some(element.editor.display_text().to_string());
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
            _ => {}
        }

        let mut text_style = TextStyle::default();
        if let Some(info) = element.node.get_text_userdata::<TextRenderInfo>(ctx) {
            text_style = info.style.clone();
        } else if let Some(style) = element.node.get_text_userdata::<TextStyle>(ctx) {
            text_style = style.clone();
        }

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
        };
        element
            .node
            .set_text_with_userdata(ctx, &element.editor.display_text(), render_info);

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

                    // We need to fetch text_style again since it was moved above.
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

                    // Clamp to max bounds to prevent empty space when deleting text
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

        (handled, emitted_msg)
    }
}
