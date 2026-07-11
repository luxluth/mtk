use std::time::Instant;

use crate::ui::event::EventResult;
use crate::ui::widgets::editor::Editor;
use crate::ui::{Event, View};
use crate::{Context, Node, TextRenderInfo, TextStyle};
use winit::keyboard::{Key, NamedKey};

pub struct TextInput {
    pub(crate) captures_tab: bool,
}

pub fn text_input() -> TextInput {
    TextInput {
        captures_tab: false,
    }
}

impl TextInput {
    pub fn captures_tab(mut self, captures: bool) -> Self {
        self.captures_tab = captures;
        self
    }
}

impl View<String> for TextInput {
    type Element = (Node, Editor, Node, bool, Option<std::time::Instant>, u8); // Node, Editor, Caret, Hover, LastClick, ClickCount
    type Message = String;

    fn build(&self, ctx: &mut Context) -> Self::Element {
        let node = ctx.create_node();
        let mut editor = Editor::new();
        editor.set_text("");

        ctx.register_focusable(node.clone());

        let caret = ctx.create_node();
        node.append(ctx, caret.clone());
        (node, editor, caret, false, None, 0)
    }

    fn rebuild(&self, _prev: &Self, ctx: &mut Context, element: &mut Self::Element) {
        let (node, editor, _caret, _hover, _, _) = element;

        // Preserve the user's TextStyle if they set one via StyleWrap
        let mut text_style = TextStyle::default();
        if let Some(info) = node.get_text_userdata::<TextRenderInfo>(ctx) {
            text_style = info.style.clone();
        } else if let Some(style) = node.get_text_userdata::<TextStyle>(ctx) {
            text_style = style.clone();
        }

        let is_focused = Some(node.clone()) == ctx.focused_node();

        let render_info = TextRenderInfo {
            style: text_style,
            cursor: if is_focused {
                Some(editor.display_cursor())
            } else {
                None
            },
            selection: if is_focused { editor.selection() } else { None },
            preedit_range: editor.preedit_range(),
        };
        node.set_text_with_userdata(ctx, &editor.display_text(), render_info);
    }

    fn teardown(&self, ctx: &mut Context, element: &mut Self::Element) {
        let (node, _, caret, _, _, _) = element;
        ctx.unregister_focusable(*node);
        caret.remove(ctx);
        ctx.destroy_node(*caret);
        node.remove(ctx);
        ctx.destroy_node(*node);
    }

    fn get_node(&self, element: &Self::Element) -> Node {
        element.0.clone()
    }

    fn handle_event(
        &self,
        element: &mut Self::Element,
        state: &String,
        event: Event,
        ctx: &mut Context,
    ) -> (EventResult, Option<Self::Message>) {
        let (node, editor, _caret, _, _, _) = element;

        let mut handled = EventResult::Ignored;
        let mut emitted_msg = None;

        // Initialize editor state if this is the first tick and the text is not empty
        if editor.text() != *state {
            if let Event::Tick { dt: _ } = event {
                editor.set_text(&state);
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
                let is_hit = hit_nodes.iter().any(|n| n == node);
                let is_focused = Some(node.clone()) == ctx.focused_node();

                if pressed {
                    if is_hit {
                        ctx.request_focus(node.clone());
                        handled = EventResult::Handled;

                        if let Some(computed) = node.get_computed(ctx) {
                            let constraints = node.get_constraints(ctx).unwrap_or_default();
                            let rel_x = x - computed.x - constraints.padding.left;
                            let rel_y = y - computed.y - constraints.padding.top;
                            let shift = ctx.modifiers().shift_key();

                            let inner_w =
                                (computed.w - constraints.padding.left - constraints.padding.right)
                                    .max(0.0);
                            let inner_h =
                                (computed.h - constraints.padding.top - constraints.padding.bottom)
                                    .max(0.0);

                            let mut text_style = TextStyle::default();
                            if let Some(info) = node.get_text_userdata::<TextRenderInfo>(ctx) {
                                text_style = info.style.clone();
                            } else if let Some(style) = node.get_text_userdata::<TextStyle>(ctx) {
                                text_style = style.clone();
                            }

                            let index = crate::text::hit_test_text(
                                &editor.display_text(),
                                &text_style,
                                inner_w,
                                inner_h,
                                rel_x,
                                rel_y,
                                &ctx.text_context,
                            );

                            let now = Instant::now();
                            let mut click_count = 1;
                            if let Some(last_click) = element.4 {
                                if now.duration_since(last_click).as_millis() < 500 {
                                    click_count = element.5 + 1;
                                }
                            }
                            element.4 = Some(now);
                            element.5 = click_count;

                            if click_count == 2 {
                                editor.set_cursor(index);
                                editor.set_selection_anchor(Some(index));
                                editor.move_word_left(false);
                                let start = editor.cursor();
                                editor.move_word_right(true);
                                let end = editor.cursor();
                                editor.set_selection_anchor(Some(start));
                                editor.set_cursor(end);
                            } else if click_count >= 3 {
                                editor.select_all(); // For single line text input, select all is fine
                            } else {
                                if shift {
                                    if editor.selection_anchor().is_none() {
                                        editor.set_selection_anchor(Some(editor.cursor()));
                                    }
                                    editor.set_cursor(index);
                                } else {
                                    editor.set_selection_anchor(None);
                                    editor.set_cursor(index);
                                }
                            }
                            ctx.request_frame();
                        }
                    } else if is_focused {
                        ctx.clear_focus();
                    }
                }
            }
            Event::KeyboardInput {
                event: key_event,
                is_synthetic: _,
            } => {
                if ctx.focused_node() == Some(node.clone()) && key_event.state.is_pressed() {
                    handled = EventResult::Handled;
                    let shift = ctx.modifiers().shift_key();
                    let ctrl_alt = ctx.modifiers().control_key() || ctx.modifiers().alt_key();

                    let mut text_changed = false;

                    match key_event.logical_key.as_ref() {
                        Key::Named(NamedKey::ArrowLeft) => {
                            if ctrl_alt {
                                editor.move_word_left(shift);
                            } else {
                                editor.move_left(shift);
                            }
                            ctx.request_frame();
                        }
                        Key::Named(NamedKey::ArrowRight) => {
                            if ctrl_alt {
                                editor.move_word_right(shift);
                            } else {
                                editor.move_right(shift);
                            }
                            ctx.request_frame();
                        }
                        Key::Named(NamedKey::Backspace) => {
                            if ctrl_alt {
                                editor.delete_word_backward();
                            } else {
                                editor.delete_backward();
                            }
                            text_changed = true;
                        }
                        Key::Named(NamedKey::Delete) => {
                            if ctrl_alt {
                                editor.delete_word_forward();
                            } else {
                                editor.delete_forward();
                            }
                            text_changed = true;
                        }
                        Key::Named(NamedKey::Home) => {
                            editor.move_to_start(shift);
                            ctx.request_frame();
                        }
                        Key::Named(NamedKey::End) => {
                            editor.move_to_end(shift);
                            ctx.request_frame();
                        }
                        Key::Named(NamedKey::Tab) => {
                            if ctrl_alt {
                                // Ignore Ctrl+Tab so it bubbles (or just do nothing here)
                                handled = EventResult::Ignored;
                            } else if self.captures_tab {
                                editor.insert("\t");
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
                            editor.select_all();
                            ctx.request_frame();
                        }
                        _ => {
                            if let Some(text) = &key_event.text {
                                if !text.is_empty() && !ctrl_alt {
                                    // NOTE: we avoid inserting control chars
                                    if text.chars().all(|c| !c.is_control()) {
                                        editor.insert(text.as_str());
                                        text_changed = true;
                                    }
                                }
                            }
                        }
                    }

                    if text_changed {
                        emitted_msg = Some(editor.display_text().to_string());
                        ctx.request_frame();
                    }
                }
            }
            Event::Ime(ime) => {
                if ctx.focused_node() == Some(node.clone()) {
                    use winit::event::Ime;
                    match ime {
                        Ime::Enabled => {}
                        Ime::Preedit(text, cursor_pos) => {
                            editor.set_ime_preedit(text, cursor_pos);
                            ctx.request_frame();
                            handled = EventResult::Handled;
                        }
                        Ime::Commit(text) => {
                            editor.commit_ime(&text);
                            emitted_msg = Some(editor.display_text().to_string());
                            ctx.request_frame();
                            handled = EventResult::Handled;
                        }
                        Ime::Disabled => {
                            editor.set_ime_preedit(String::new(), None);
                            ctx.request_frame();
                        }
                    }
                }
            }
            _ => {}
        }

        let mut text_style = TextStyle::default();
        if let Some(info) = node.get_text_userdata::<TextRenderInfo>(ctx) {
            text_style = info.style.clone();
        } else if let Some(style) = node.get_text_userdata::<TextStyle>(ctx) {
            text_style = style.clone();
        }

        let is_focused = Some(node.clone()) == ctx.focused_node();

        let render_info = TextRenderInfo {
            style: text_style,
            cursor: if is_focused {
                Some(editor.display_cursor())
            } else {
                None
            },
            selection: if is_focused { editor.selection() } else { None },
            preedit_range: editor.preedit_range(),
        };
        node.set_text_with_userdata(ctx, &editor.display_text(), render_info);

        (handled, emitted_msg)
    }
}
