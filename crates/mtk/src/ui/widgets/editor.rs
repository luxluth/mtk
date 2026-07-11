/// A headless text editor abstraction that manages the state of a single-line or multi-line text input.
///
/// It handles UTF-8 safe cursor navigation, text selection via an anchor system, and text mutations
/// such as insertions and backspace/delete operations. It is completely decoupled from rendering.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Editor {
    /// The underlying text buffer.
    text: String,
    /// The current byte-index of the cursor within the text buffer.
    cursor: usize,
    /// The byte-index where a text selection began. If `None`, no text is currently selected.
    /// When selecting text (e.g. holding Shift), this remains fixed while the `cursor` moves.
    selection_anchor: Option<usize>,
    /// Temporary text being composed by the OS Input Method Editor (IME).
    ime_preedit: Option<(String, Option<(usize, usize)>)>,
}

impl Default for Editor {
    fn default() -> Self {
        Self::new()
    }
}

impl Editor {
    /// Creates a new, empty `Editor` state.
    pub fn new() -> Self {
        Self {
            text: String::new(),
            cursor: 0,
            selection_anchor: None,
            ime_preedit: None,
        }
    }

    /// Returns the underlying text buffer without any temporary IME composition.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Returns the text to be displayed on screen, which includes any active IME preedit text
    /// temporarily inserted at the cursor position.
    pub fn display_text(&self) -> String {
        if let Some((preedit_text, _)) = &self.ime_preedit {
            let mut display = String::with_capacity(self.text.len() + preedit_text.len());
            display.push_str(&self.text[..self.cursor]);
            display.push_str(preedit_text);
            display.push_str(&self.text[self.cursor..]);
            display
        } else {
            self.text.clone()
        }
    }

    /// Sets the active IME preedit state. If `text` is empty, the preedit state is cleared.
    pub fn set_ime_preedit(&mut self, text: String, cursor_pos: Option<(usize, usize)>) {
        if text.is_empty() {
            self.ime_preedit = None;
        } else {
            self.ime_preedit = Some((text, cursor_pos));
        }
    }

    /// Commits the given text from the IME, inserting it into the buffer and clearing preedit.
    pub fn commit_ime(&mut self, text: &str) {
        self.ime_preedit = None;
        self.insert(text);
    }

    /// Returns the byte range `(start, end)` of the active IME preedit within the `display_text`.
    /// Returns `None` if there is no active IME composition.
    pub fn preedit_range(&self) -> Option<(usize, usize)> {
        self.ime_preedit
            .as_ref()
            .map(|(text, _)| (self.cursor, self.cursor + text.len()))
    }

    /// Returns the current byte-index position of the cursor relative to the underlying text buffer.
    pub fn cursor(&self) -> usize {
        self.cursor
    }

    /// Returns the cursor index mapped to the `display_text` string, accounting for
    /// any active IME preedit and its internal cursor position.
    pub fn display_cursor(&self) -> usize {
        if let Some((text, cursor_pos)) = &self.ime_preedit {
            if let Some((start, _)) = cursor_pos {
                self.cursor + start
            } else {
                self.cursor + text.len()
            }
        } else {
            self.cursor
        }
    }

    /// Returns the active text selection as a tuple of `(start, end)` byte indices.
    /// The `start` is guaranteed to be less than or equal to `end`.
    /// Returns `None` if no text is selected.
    pub fn selection(&self) -> Option<(usize, usize)> {
        self.selection_anchor.and_then(|anchor| {
            if anchor < self.cursor {
                Some((anchor, self.cursor))
            } else if anchor > self.cursor {
                Some((self.cursor, anchor))
            } else {
                None
            }
        })
    }

    /// Returns `true` if the text buffer is completely empty.
    pub fn is_empty(&self) -> bool {
        self.text.is_empty()
    }

    /// Replaces the entire text buffer with the given string, clearing any selection
    /// and moving the cursor to the end of the new text.
    pub fn set_text(&mut self, text: &str) {
        self.text = text.to_string();
        self.cursor = self.text.len();
        self.selection_anchor = None;
    }

    /// Sets the cursor position directly.
    pub fn set_cursor(&mut self, cursor: usize) {
        self.cursor = cursor.min(self.text.len());
    }

    /// Gets the selection anchor directly.
    pub fn selection_anchor(&self) -> Option<usize> {
        self.selection_anchor
    }

    /// Sets the selection anchor directly.
    pub fn set_selection_anchor(&mut self, anchor: Option<usize>) {
        if let Some(a) = anchor {
            self.selection_anchor = Some(a.min(self.text.len()));
        } else {
            self.selection_anchor = None;
        }
    }

    /// Inserts a string at the current cursor position.
    /// If there is an active selection, the selected text is deleted first.
    pub fn insert(&mut self, text: &str) {
        self.delete_selection();
        self.text.insert_str(self.cursor, text);
        self.cursor += text.len();
    }

    /// Inserts a single character at the current cursor position.
    /// If there is an active selection, the selected text is deleted first.
    pub fn insert_char(&mut self, ch: char) {
        self.delete_selection();
        self.text.insert(self.cursor, ch);
        self.cursor += ch.len_utf8();
    }

    /// Deletes the character immediately preceding the cursor (Backspace).
    /// If there is an active selection, deletes the selection instead.
    pub fn delete_backward(&mut self) {
        if self.delete_selection() {
            return;
        }
        if self.cursor > 0 {
            let prev_char = self.text[..self.cursor].chars().next_back().unwrap();
            let len = prev_char.len_utf8();
            self.cursor -= len;
            self.text.remove(self.cursor);
        }
    }

    /// Deletes the character immediately following the cursor (Delete).
    /// If there is an active selection, deletes the selection instead.
    pub fn delete_forward(&mut self) {
        if self.delete_selection() {
            return;
        }
        if self.cursor < self.text.len() {
            self.text.remove(self.cursor);
        }
    }

    /// Deletes the word immediately preceding the cursor (Ctrl+Backspace).
    /// If there is an active selection, deletes the selection instead.
    pub fn delete_word_backward(&mut self) {
        if self.delete_selection() {
            return;
        }
        let end = self.cursor;
        self.step_word_left();
        let start = self.cursor;
        if start < end {
            self.text.replace_range(start..end, "");
        }
    }

    /// Deletes the word immediately following the cursor (Ctrl+Delete).
    /// If there is an active selection, deletes the selection instead.
    pub fn delete_word_forward(&mut self) {
        if self.delete_selection() {
            return;
        }
        let start = self.cursor;
        self.step_word_right();
        let end = self.cursor;
        if start < end {
            self.text.replace_range(start..end, "");
            self.cursor = start; // Cursor stays at start, text shrinks
        }
    }

    /// Deletes the currently selected text.
    /// Returns `true` if text was actually deleted, or `false` if there was no selection.
    pub fn delete_selection(&mut self) -> bool {
        if let Some((start, end)) = self.selection() {
            if start != end {
                self.text.replace_range(start..end, "");
                self.cursor = start;
                self.selection_anchor = None;
                return true;
            }
        }
        self.selection_anchor = None;
        false
    }

    /// Selects the entire text buffer.
    pub fn select_all(&mut self) {
        self.selection_anchor = Some(0);
        self.cursor = self.text.len();
    }

    /// Moves the cursor one character to the left.
    /// If `shift` is true, extends the selection.
    /// If `shift` is false and there is a selection, jumps to the start of the selection.
    pub fn move_left(&mut self, shift: bool) {
        if shift {
            if self.selection_anchor.is_none() {
                self.selection_anchor = Some(self.cursor);
            }
            self.step_left();
        } else {
            if let Some((start, end)) = self.selection() {
                if start != end {
                    self.cursor = start;
                    self.selection_anchor = None;
                    return;
                }
            }
            self.selection_anchor = None;
            self.step_left();
        }
    }

    /// Moves the cursor one character to the right.
    /// If `shift` is true, extends the selection.
    /// If `shift` is false and there is a selection, jumps to the end of the selection.
    pub fn move_right(&mut self, shift: bool) {
        if shift {
            if self.selection_anchor.is_none() {
                self.selection_anchor = Some(self.cursor);
            }
            self.step_right();
        } else {
            if let Some((start, end)) = self.selection() {
                if start != end {
                    self.cursor = end;
                    self.selection_anchor = None;
                    return;
                }
            }
            self.selection_anchor = None;
            self.step_right();
        }
    }

    /// Moves the cursor left by one word boundary.
    /// Skips over trailing non-alphanumeric characters, then skips all alphanumeric characters.
    pub fn move_word_left(&mut self, shift: bool) {
        if shift {
            if self.selection_anchor.is_none() {
                self.selection_anchor = Some(self.cursor);
            }
            self.step_word_left();
        } else {
            if let Some((start, end)) = self.selection() {
                if start != end {
                    self.cursor = start;
                    self.selection_anchor = None;
                    return;
                }
            }
            self.selection_anchor = None;
            self.step_word_left();
        }
    }

    /// Moves the cursor right by one word boundary.
    /// Skips all alphanumeric characters, then stops before the next alphanumeric character.
    pub fn move_word_right(&mut self, shift: bool) {
        if shift {
            if self.selection_anchor.is_none() {
                self.selection_anchor = Some(self.cursor);
            }
            self.step_word_right();
        } else {
            if let Some((start, end)) = self.selection() {
                if start != end {
                    self.cursor = end;
                    self.selection_anchor = None;
                    return;
                }
            }
            self.selection_anchor = None;
            self.step_word_right();
        }
    }

    /// Internal helper to step the cursor one character to the left.
    fn step_left(&mut self) {
        if self.cursor > 0 {
            let prev_char = self.text[..self.cursor].chars().next_back().unwrap();
            self.cursor -= prev_char.len_utf8();
        }
    }

    /// Internal helper to step the cursor one character to the right.
    fn step_right(&mut self) {
        if self.cursor < self.text.len() {
            let next_char = self.text[self.cursor..].chars().next().unwrap();
            self.cursor += next_char.len_utf8();
        }
    }

    /// Internal helper to step the cursor one word to the left.
    fn step_word_left(&mut self) {
        let mut chars = self.text[..self.cursor].chars().rev();
        let mut skipped_any = false;

        // Skip trailing non-alphanumeric chars
        while let Some(c) = chars.next() {
            if c.is_alphanumeric() {
                // Found alphanumeric, step back one to put it back conceptually
                // though we just break and handle below
                self.cursor -= c.len_utf8();
                break;
            } else {
                self.cursor -= c.len_utf8();
                skipped_any = true;
            }
        }

        // Now skip all alphanumeric chars
        for c in chars {
            if c.is_alphanumeric() {
                self.cursor -= c.len_utf8();
                skipped_any = true;
            } else {
                break;
            }
        }

        if !skipped_any {
            self.step_left(); // fallback
        }
    }

    /// Internal helper to step the cursor one word to the right.
    fn step_word_right(&mut self) {
        let mut chars = self.text[self.cursor..].chars();
        let mut skipped_any = false;

        // Skip alphanumeric chars first
        while let Some(c) = chars.next() {
            if c.is_alphanumeric() {
                self.cursor += c.len_utf8();
                skipped_any = true;
            } else {
                // If we found a non-alphanumeric, we stop and leave cursor before it
                // UNLESS we haven't skipped anything yet, in which case we skip non-alphanumeric
                if !skipped_any {
                    self.cursor += c.len_utf8();
                    skipped_any = true;
                    for next_c in chars.by_ref() {
                        if !next_c.is_alphanumeric() {
                            self.cursor += next_c.len_utf8();
                        } else {
                            break;
                        }
                    }
                }
                break;
            }
        }

        if !skipped_any {
            self.step_right(); // fallback
        }
    }

    /// Moves the cursor to the very beginning of the text buffer.
    pub fn move_to_start(&mut self, shift: bool) {
        if shift {
            if self.selection_anchor.is_none() {
                self.selection_anchor = Some(self.cursor);
            }
        } else {
            self.selection_anchor = None;
        }
        self.cursor = 0;
    }

    /// Moves the cursor to the very end of the text buffer.
    pub fn move_to_end(&mut self, shift: bool) {
        if shift {
            if self.selection_anchor.is_none() {
                self.selection_anchor = Some(self.cursor);
            }
        } else {
            self.selection_anchor = None;
        }
        self.cursor = self.text.len();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_editor_insert() {
        let mut editor = Editor::new();
        editor.insert("hello");
        assert_eq!(editor.text(), "hello");
        assert_eq!(editor.cursor(), 5);
    }

    #[test]
    fn test_editor_movements() {
        let mut editor = Editor::new();
        editor.insert("abc");
        editor.move_left(false);
        assert_eq!(editor.cursor(), 2); // after 'b'
        editor.move_left(false);
        assert_eq!(editor.cursor(), 1); // after 'a'
        editor.move_right(false);
        assert_eq!(editor.cursor(), 2);
    }

    #[test]
    fn test_editor_delete() {
        let mut editor = Editor::new();
        editor.insert("hello");
        editor.delete_backward();
        assert_eq!(editor.text(), "hell");
        editor.move_left(false); // cursor after 'l'
        editor.move_left(false); // cursor after 'e'
        editor.delete_forward(); // deletes 'l'
        assert_eq!(editor.text(), "hel");
    }

    #[test]
    fn test_editor_selection() {
        let mut editor = Editor::new();
        editor.insert("hello");
        editor.move_left(true);
        editor.move_left(true);
        assert_eq!(editor.selection(), Some((3, 5)));
        editor.delete_backward();
        assert_eq!(editor.text(), "hel");
        assert_eq!(editor.cursor(), 3);
        assert_eq!(editor.selection(), None);
    }

    #[test]
    fn test_unicode_boundaries() {
        let mut editor = Editor::new();
        editor.insert("🦀rust");
        assert_eq!(editor.cursor(), 8); // 🦀 is 4 bytes + rust is 4 bytes
        editor.move_to_start(false);
        assert_eq!(editor.cursor(), 0);
        editor.move_right(false);
        assert_eq!(editor.cursor(), 4);
        editor.delete_forward();
        assert_eq!(editor.text(), "🦀ust");
    }

    #[test]
    fn test_word_boundaries() {
        let mut editor = Editor::new();
        editor.insert("hello, world!");

        // cursor is at the end (13)
        editor.move_word_left(false);
        // jumps back over "world!" -> stops before "world!"
        assert_eq!(editor.cursor(), 7);

        editor.move_word_left(false);
        // jumps back over "hello, " -> stops before "hello"
        assert_eq!(editor.cursor(), 0);

        editor.move_word_right(false);
        // jumps over "hello", stops before ","
        assert_eq!(editor.cursor(), 5);

        editor.move_word_right(false);
        // jumps over ", ", stops before "world"
        assert_eq!(editor.cursor(), 7);

        editor.move_word_right(false);
        // jumps over "world", stops before "!"
        assert_eq!(editor.cursor(), 12);
    }

    #[test]
    fn test_editor_ime() {
        let mut editor = Editor::new();
        editor.insert("hello ");
        assert_eq!(editor.cursor(), 6);

        // Start composing "world"
        editor.set_ime_preedit("worl".to_string(), None);
        assert_eq!(editor.display_text(), "hello worl");
        assert_eq!(editor.preedit_range(), Some((6, 10)));
        assert_eq!(editor.text(), "hello "); // Underlying text unchanged
        assert_eq!(editor.cursor(), 6); // Cursor unchanged

        // Commit the composition
        editor.commit_ime("world");
        assert_eq!(editor.display_text(), "hello world");
        assert_eq!(editor.text(), "hello world");
        assert_eq!(editor.preedit_range(), None);
        assert_eq!(editor.cursor(), 11);
    }
}
