use std::io;
use std::path::{Path, PathBuf};

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, StatefulWidget, Widget};

use crate::theme::Theme;

/// Multi-line text editor state.
///
/// Tracks file content, cursor position, scroll, editable region, and dirty/saved flags.
#[derive(Debug, Clone)]
pub struct EditorState {
    /// All lines of content (including read-only context lines).
    lines: Vec<String>,
    /// Inclusive start, exclusive end of the editable region.
    editable_range: (usize, usize),
    /// Cursor position as `(line_index, byte_offset_in_line)`.
    cursor: (usize, usize),
    /// Index of the first visible line (scroll offset).
    scroll_offset: usize,
    /// File path for save operations.
    file_path: Option<PathBuf>,
    /// True if content has been modified since the last load or save.
    pub dirty: bool,
    /// True after the most recent successful save.
    pub saved: bool,
}

impl Default for EditorState {
    fn default() -> Self {
        Self::new()
    }
}

impl EditorState {
    /// Create an empty editor with a single empty editable line.
    pub fn new() -> Self {
        Self {
            lines: vec![String::new()],
            editable_range: (0, 1),
            cursor: (0, 0),
            scroll_offset: 0,
            file_path: None,
            dirty: false,
            saved: false,
        }
    }

    /// Load content from `path`. All lines are editable.
    pub fn from_file(path: &Path) -> io::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let mut state = Self::from_string(&content);
        state.file_path = Some(path.to_path_buf());
        Ok(state)
    }

    /// Load content from `path` with a read-only context window around a focus line.
    ///
    /// Lines that fall outside the `[focus_line - editable_above, focus_line + editable_below]`
    /// range (both inclusive) are rendered as read-only context.  The cursor is placed at the
    /// start of `focus_line`.
    pub fn from_file_with_context(
        path: &Path,
        focus_line: usize,
        editable_above: usize,
        editable_below: usize,
    ) -> io::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let lines: Vec<String> = content.lines().map(str::to_owned).collect();
        let total = lines.len().max(1);

        let start = focus_line.saturating_sub(editable_above).min(total);
        let end = (focus_line + editable_below + 1).min(total);

        let cursor_line = focus_line.min(total - 1);

        let state = Self {
            lines,
            editable_range: (start, end),
            cursor: (cursor_line, 0),
            scroll_offset: 0,
            file_path: Some(path.to_path_buf()),
            dirty: false,
            saved: false,
        };
        Ok(state)
    }

    /// Load content from a string slice. All lines are editable.
    pub fn from_string(content: &str) -> Self {
        let lines: Vec<String> = if content.is_empty() {
            vec![String::new()]
        } else {
            // Preserve a trailing newline as an extra empty line.
            let mut ls: Vec<String> = content.lines().map(str::to_owned).collect();
            if content.ends_with('\n') {
                ls.push(String::new());
            }
            ls
        };
        let count = lines.len();
        Self {
            lines,
            editable_range: (0, count),
            cursor: (0, 0),
            scroll_offset: 0,
            file_path: None,
            dirty: false,
            saved: false,
        }
    }

    // ── Queries ──────────────────────────────────────────────────────────────

    /// Returns true when `line` is within the editable range.
    pub fn is_editable(&self, line: usize) -> bool {
        let (start, end) = self.editable_range;
        line >= start && line < end
    }

    /// Return the entire content as a single string (lines joined with `\n`).
    pub fn content(&self) -> String {
        self.lines.join("\n")
    }

    /// Total number of lines.
    pub fn line_count(&self) -> usize {
        self.lines.len()
    }

    /// Zero-based index of the line the cursor is on.
    pub fn cursor_line(&self) -> usize {
        self.cursor.0
    }

    /// Zero-based character column of the cursor (not byte offset).
    pub fn cursor_col(&self) -> usize {
        let line = &self.lines[self.cursor.0];
        line[..self.cursor.1].chars().count()
    }

    // ── Internal helpers ──────────────────────────────────────────────────────

    /// Convert a character column on `line_idx` to a byte offset.
    fn col_to_byte(&self, line_idx: usize, col: usize) -> usize {
        let line = &self.lines[line_idx];
        line.char_indices().nth(col).map_or(line.len(), |(i, _)| i)
    }

    /// Clamp the cursor's byte offset so it does not exceed the line length
    /// after moving to a new line.
    fn clamp_cursor_to_line(&mut self) {
        let line_len = self.lines[self.cursor.0].len();
        if self.cursor.1 > line_len {
            self.cursor.1 = line_len;
        }
    }

    // ── Navigation ────────────────────────────────────────────────────────────

    /// Move the cursor to the start of the current line.
    pub fn home(&mut self) {
        self.cursor.1 = 0;
    }

    /// Move the cursor to the end of the current line.
    pub fn end(&mut self) {
        self.cursor.1 = self.lines[self.cursor.0].len();
    }

    /// Move the cursor one character to the left, wrapping to the previous line if at column 0.
    pub fn move_left(&mut self) {
        if self.cursor.1 > 0 {
            let line = &self.lines[self.cursor.0];
            self.cursor.1 = line[..self.cursor.1]
                .char_indices()
                .next_back()
                .map_or(0, |(i, _)| i);
        } else if self.cursor.0 > 0 {
            self.cursor.0 -= 1;
            self.cursor.1 = self.lines[self.cursor.0].len();
        }
    }

    /// Move the cursor one character to the right, wrapping to the next line if at end of line.
    pub fn move_right(&mut self) {
        let line = &self.lines[self.cursor.0];
        if self.cursor.1 < line.len() {
            self.cursor.1 = line[self.cursor.1..]
                .char_indices()
                .nth(1)
                .map_or(line.len(), |(i, _)| self.cursor.1 + i);
        } else if self.cursor.0 + 1 < self.lines.len() {
            self.cursor.0 += 1;
            self.cursor.1 = 0;
        }
    }

    /// Move the cursor up one line, clamping the column to the line length.
    pub fn move_up(&mut self) {
        if self.cursor.0 > 0 {
            let col = self.cursor_col();
            self.cursor.0 -= 1;
            self.cursor.1 = self.col_to_byte(self.cursor.0, col);
            self.clamp_cursor_to_line();
        }
    }

    /// Move the cursor down one line, clamping the column to the line length.
    pub fn move_down(&mut self) {
        if self.cursor.0 + 1 < self.lines.len() {
            let col = self.cursor_col();
            self.cursor.0 += 1;
            self.cursor.1 = self.col_to_byte(self.cursor.0, col);
            self.clamp_cursor_to_line();
        }
    }

    /// Move the cursor up by `viewport_height` lines.
    pub fn page_up(&mut self, viewport_height: usize) {
        let col = self.cursor_col();
        self.cursor.0 = self.cursor.0.saturating_sub(viewport_height);
        self.cursor.1 = self.col_to_byte(self.cursor.0, col);
        self.clamp_cursor_to_line();
    }

    /// Move the cursor down by `viewport_height` lines.
    pub fn page_down(&mut self, viewport_height: usize) {
        let col = self.cursor_col();
        let last = self.lines.len().saturating_sub(1);
        self.cursor.0 = (self.cursor.0 + viewport_height).min(last);
        self.cursor.1 = self.col_to_byte(self.cursor.0, col);
        self.clamp_cursor_to_line();
    }

    /// Adjust `scroll_offset` so the cursor line is visible within `viewport_height`.
    pub fn ensure_cursor_visible(&mut self, viewport_height: usize) {
        let height = viewport_height.max(1);
        if self.cursor.0 < self.scroll_offset {
            self.scroll_offset = self.cursor.0;
        } else if self.cursor.0 >= self.scroll_offset + height {
            self.scroll_offset = self.cursor.0 - height + 1;
        }
    }

    // ── Editing ───────────────────────────────────────────────────────────────

    /// Insert a character at the cursor position.
    ///
    /// A newline (`'\n'`) splits the current line.  Editing is rejected when the
    /// cursor is on a read-only line.
    pub fn insert(&mut self, c: char) {
        if !self.is_editable(self.cursor.0) {
            return;
        }
        self.saved = false;
        if c == '\n' {
            let rest = self.lines[self.cursor.0][self.cursor.1..].to_owned();
            self.lines[self.cursor.0].truncate(self.cursor.1);
            let new_line_idx = self.cursor.0 + 1;
            self.lines.insert(new_line_idx, rest);
            // Extend the editable range to cover the new line.
            let (start, end) = self.editable_range;
            if new_line_idx < end + 1 {
                self.editable_range = (start, end + 1);
            }
            self.cursor.0 = new_line_idx;
            self.cursor.1 = 0;
        } else {
            self.lines[self.cursor.0].insert(self.cursor.1, c);
            self.cursor.1 += c.len_utf8();
        }
        self.dirty = true;
    }

    /// Delete the character immediately before the cursor.
    ///
    /// When at the start of a line, joins with the previous line — but only if both
    /// lines are editable.  Read-only lines silently reject the operation.
    pub fn backspace(&mut self) {
        if !self.is_editable(self.cursor.0) {
            return;
        }
        if self.cursor.1 > 0 {
            let prev_byte = self.lines[self.cursor.0][..self.cursor.1]
                .char_indices()
                .next_back()
                .map_or(0, |(i, _)| i);
            self.lines[self.cursor.0].replace_range(prev_byte..self.cursor.1, "");
            self.cursor.1 = prev_byte;
            self.dirty = true;
        } else if self.cursor.0 > 0 && self.is_editable(self.cursor.0 - 1) {
            // Join current line onto the previous line.
            let current = self.lines.remove(self.cursor.0);
            let prev_len = self.lines[self.cursor.0 - 1].len();
            self.lines[self.cursor.0 - 1].push_str(&current);
            // Shrink the editable range by one line.
            let (start, end) = self.editable_range;
            self.editable_range = (start, end.saturating_sub(1));
            self.cursor.0 -= 1;
            self.cursor.1 = prev_len;
            self.dirty = true;
        }
        self.saved = false;
    }

    /// Delete the character at the cursor position.
    ///
    /// When at the end of a line, joins the next line onto the current one — but only
    /// if both lines are editable.
    pub fn delete(&mut self) {
        if !self.is_editable(self.cursor.0) {
            return;
        }
        let line_len = self.lines[self.cursor.0].len();
        if self.cursor.1 < line_len {
            let next_byte = self.lines[self.cursor.0][self.cursor.1..]
                .char_indices()
                .nth(1)
                .map_or(line_len, |(i, _)| self.cursor.1 + i);
            self.lines[self.cursor.0].replace_range(self.cursor.1..next_byte, "");
            self.dirty = true;
        } else if self.cursor.0 + 1 < self.lines.len() && self.is_editable(self.cursor.0 + 1) {
            // Join next line onto the current line.
            let next = self.lines.remove(self.cursor.0 + 1);
            self.lines[self.cursor.0].push_str(&next);
            let (start, end) = self.editable_range;
            self.editable_range = (start, end.saturating_sub(1));
            self.dirty = true;
        }
        self.saved = false;
    }

    // ── File I/O ──────────────────────────────────────────────────────────────

    /// Write all lines back to `file_path`.
    ///
    /// Returns an error if no file path was set.
    pub fn save(&mut self) -> io::Result<()> {
        let path = self.file_path.clone().ok_or_else(|| {
            io::Error::new(io::ErrorKind::NotFound, "no file path set on EditorState")
        })?;
        let content = self.content();
        std::fs::write(&path, content)?;
        self.dirty = false;
        self.saved = true;
        Ok(())
    }
}

// ── Widget ────────────────────────────────────────────────────────────────────

/// A multi-line editor widget.
///
/// Renders the content of an [`EditorState`] with a line-number gutter, current-line
/// highlight, and muted styling for read-only context lines.
pub struct Editor<'a, T: Theme> {
    theme: &'a T,
    block: Option<Block<'a>>,
    show_line_numbers: bool,
}

impl<'a, T: Theme> Editor<'a, T> {
    /// Create a new editor widget backed by `theme`.
    pub fn new(theme: &'a T) -> Self {
        Self {
            theme,
            block: None,
            show_line_numbers: true,
        }
    }

    /// Set an optional surrounding block (border + title).
    #[must_use]
    pub fn block(mut self, block: Block<'a>) -> Self {
        self.block = Some(block);
        self
    }

    /// Control whether the line-number gutter is rendered.
    #[must_use]
    pub fn show_line_numbers(mut self, show: bool) -> Self {
        self.show_line_numbers = show;
        self
    }
}

impl<T: Theme> StatefulWidget for Editor<'_, T> {
    type State = EditorState;

    #[allow(clippy::too_many_lines)]
    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        if area.width == 0 || area.height == 0 {
            return;
        }

        // Render the surrounding block (if any) and obtain the inner area.
        let inner = if let Some(block) = self.block {
            let styled = block.border_style(self.theme.border_focused());
            let inner = styled.inner(area);
            styled.render(area, buf);
            inner
        } else {
            area
        };

        if inner.width == 0 || inner.height == 0 {
            return;
        }

        let viewport_height = inner.height as usize;

        // Keep the cursor visible.
        state.ensure_cursor_visible(viewport_height);

        // Reserve the bottom row for a status bar showing cursor position and dirty flag.
        let (editor_area, status_area) = if inner.height > 1 {
            let h = inner.height - 1;
            let ed = Rect::new(inner.x, inner.y, inner.width, h);
            let st = Rect::new(inner.x, inner.y + h, inner.width, 1);
            (ed, Some(st))
        } else {
            (inner, None)
        };

        let lines_area_height = editor_area.height as usize;

        // Gutter width: enough digits for the total line count, minimum 3.
        let gutter_width: u16 = if self.show_line_numbers {
            let digits = state.lines.len().to_string().len();
            #[allow(clippy::cast_possible_truncation)]
            let w = (digits + 1).max(3) as u16;
            w
        } else {
            0
        };

        let text_x = editor_area.x + gutter_width;
        let text_width = editor_area.width.saturating_sub(gutter_width);

        let visible_start = state.scroll_offset;
        let visible_end = (visible_start + lines_area_height).min(state.lines.len());

        for (row_idx, line_idx) in (visible_start..visible_end).enumerate() {
            #[allow(clippy::cast_possible_truncation)]
            let y = editor_area.y + row_idx as u16;
            let is_cursor_line = line_idx == state.cursor.0;
            let editable = state.is_editable(line_idx);

            // ── Gutter ────────────────────────────────────────────────────
            if gutter_width > 0 {
                let gutter_area = Rect::new(editor_area.x, y, gutter_width, 1);
                let num_str = format!(
                    "{:>width$} ",
                    line_idx + 1,
                    width = (gutter_width - 1) as usize
                );
                let gutter_style = if is_cursor_line {
                    Style::default()
                        .fg(self.theme.accent())
                        .add_modifier(Modifier::BOLD)
                } else {
                    self.theme.disabled()
                };
                Line::styled(num_str, gutter_style).render(gutter_area, buf);
            }

            // ── Line content ──────────────────────────────────────────────
            if text_width == 0 {
                continue;
            }
            let text_area = Rect::new(text_x, y, text_width, 1);

            let line_content = state.lines[line_idx].as_str();

            let line_style = if is_cursor_line {
                // Current line: highlighted background, keep text colour from editable/RO.
                Style::default().bg(self.theme.border()).fg(if editable {
                    self.theme.fg()
                } else {
                    self.theme.muted()
                })
            } else if editable {
                self.theme.base()
            } else {
                self.theme.disabled()
            };

            Line::styled(line_content, line_style).render(text_area, buf);
        }

        // ── Status bar ────────────────────────────────────────────────────
        if let Some(st_area) = status_area {
            let row = state.cursor.0 + 1;
            let col = state.cursor_col() + 1;
            let dirty_marker = if state.dirty { " [modified]" } else { "" };
            let status = format!("Ln {row}, Col {col}{dirty_marker}  Ctrl-S: save");
            Line::styled(status, self.theme.disabled()).render(st_area, buf);
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use std::io::Write;

    use super::*;

    // ── Helpers ───────────────────────────────────────────────────────────────

    fn state_from(content: &str) -> EditorState {
        EditorState::from_string(content)
    }

    fn temp_file(content: &str) -> tempfile::NamedTempFile {
        let mut f = tempfile::NamedTempFile::new().expect("create temp file");
        f.write_all(content.as_bytes()).expect("write temp file");
        f
    }

    // ── from_string ───────────────────────────────────────────────────────────

    #[test]
    fn from_string_splits_on_newlines() {
        let state = state_from("foo\nbar\nbaz");
        assert_eq!(state.line_count(), 3);
        assert_eq!(state.lines[0], "foo");
        assert_eq!(state.lines[2], "baz");
    }

    #[test]
    fn from_string_empty_gives_one_empty_line() {
        let state = state_from("");
        assert_eq!(state.line_count(), 1);
        assert_eq!(state.lines[0], "");
    }

    #[test]
    fn from_string_trailing_newline_preserved() {
        let state = state_from("a\nb\n");
        // "a\nb\n".lines() yields ["a", "b"], then we add a trailing empty line.
        assert_eq!(state.line_count(), 3);
        assert_eq!(state.lines[2], "");
    }

    #[test]
    fn from_string_all_lines_editable() {
        let state = state_from("x\ny");
        assert!(state.is_editable(0));
        assert!(state.is_editable(1));
    }

    // ── from_file ─────────────────────────────────────────────────────────────

    #[test]
    fn from_file_loads_content() {
        let f = temp_file("hello\nworld");
        let state = EditorState::from_file(f.path()).expect("load file");
        assert_eq!(state.line_count(), 2);
        assert_eq!(state.lines[0], "hello");
    }

    #[test]
    fn from_file_missing_returns_error() {
        let result = EditorState::from_file(Path::new("/nonexistent/path/to/file.txt"));
        assert!(result.is_err());
    }

    // ── from_file_with_context ────────────────────────────────────────────────

    #[test]
    fn from_file_with_context_sets_editable_range() {
        let f = temp_file("0\n1\n2\n3\n4\n5\n6");
        // focus=3, editable_above=1, editable_below=1  → editable [2,5)
        let state = EditorState::from_file_with_context(f.path(), 3, 1, 1).expect("load file");
        assert!(!state.is_editable(0));
        assert!(!state.is_editable(1));
        assert!(state.is_editable(2));
        assert!(state.is_editable(3));
        assert!(state.is_editable(4));
        assert!(!state.is_editable(5));
        assert!(!state.is_editable(6));
    }

    #[test]
    fn from_file_with_context_cursor_on_focus_line() {
        let f = temp_file("a\nb\nc\nd\ne");
        let state = EditorState::from_file_with_context(f.path(), 2, 1, 1).expect("load file");
        assert_eq!(state.cursor_line(), 2);
    }

    // ── Insert ────────────────────────────────────────────────────────────────

    #[test]
    fn insert_char_appends_to_line() {
        let mut state = state_from("ab");
        state.cursor.1 = 2;
        state.insert('c');
        assert_eq!(state.lines[0], "abc");
        assert_eq!(state.cursor.1, 3);
    }

    #[test]
    fn insert_char_in_middle_of_line() {
        let mut state = state_from("ac");
        state.cursor.1 = 1;
        state.insert('b');
        assert_eq!(state.lines[0], "abc");
    }

    #[test]
    fn insert_newline_splits_line() {
        let mut state = state_from("hello world");
        state.cursor.1 = 5;
        state.insert('\n');
        assert_eq!(state.line_count(), 2);
        assert_eq!(state.lines[0], "hello");
        assert_eq!(state.lines[1], " world");
        assert_eq!(state.cursor.0, 1);
        assert_eq!(state.cursor.1, 0);
    }

    #[test]
    fn insert_sets_dirty() {
        let mut state = state_from("x");
        assert!(!state.dirty);
        state.insert('y');
        assert!(state.dirty);
    }

    #[test]
    fn insert_rejected_on_readonly_line() {
        let mut state = state_from("ro\neditable");
        state.editable_range = (1, 2);
        // cursor on line 0 (read-only)
        state.cursor = (0, 0);
        state.insert('x');
        assert_eq!(state.lines[0], "ro", "read-only line must not be modified");
        assert!(!state.dirty);
    }

    #[test]
    fn insert_unicode_multibyte() {
        let mut state = state_from("hello");
        state.cursor.1 = 5;
        state.insert('é');
        assert_eq!(state.lines[0], "helloé");
        assert_eq!(state.cursor.1, 5 + 'é'.len_utf8());
    }

    // ── Backspace ─────────────────────────────────────────────────────────────

    #[test]
    fn backspace_removes_preceding_char() {
        let mut state = state_from("abc");
        state.cursor.1 = 3;
        state.backspace();
        assert_eq!(state.lines[0], "ab");
        assert_eq!(state.cursor.1, 2);
    }

    #[test]
    fn backspace_at_start_of_line_joins_previous_editable_line() {
        let mut state = state_from("foo\nbar");
        state.cursor = (1, 0);
        state.backspace();
        assert_eq!(state.line_count(), 1);
        assert_eq!(state.lines[0], "foobar");
        assert_eq!(state.cursor.0, 0);
        assert_eq!(state.cursor.1, 3);
    }

    #[test]
    fn backspace_at_start_of_first_line_is_noop() {
        let mut state = state_from("abc");
        state.cursor = (0, 0);
        state.backspace();
        assert_eq!(state.lines[0], "abc");
    }

    #[test]
    fn backspace_rejected_on_readonly_line() {
        let mut state = state_from("ro\neditable");
        state.editable_range = (1, 2);
        state.cursor = (0, 2);
        state.backspace();
        assert_eq!(state.lines[0], "ro");
        assert!(!state.dirty);
    }

    #[test]
    fn backspace_at_start_of_editable_line_does_not_join_readonly_previous() {
        let mut state = state_from("context\neditable");
        // line 0 is read-only, line 1 is editable
        state.editable_range = (1, 2);
        state.cursor = (1, 0);
        state.backspace();
        // Should not join because previous line is read-only.
        assert_eq!(state.line_count(), 2);
        assert!(!state.dirty);
    }

    // ── Delete ────────────────────────────────────────────────────────────────

    #[test]
    fn delete_removes_char_at_cursor() {
        let mut state = state_from("abc");
        state.cursor.1 = 0;
        state.delete();
        assert_eq!(state.lines[0], "bc");
    }

    #[test]
    fn delete_at_end_of_line_joins_next_editable_line() {
        let mut state = state_from("foo\nbar");
        state.cursor = (0, 3);
        state.delete();
        assert_eq!(state.line_count(), 1);
        assert_eq!(state.lines[0], "foobar");
        assert_eq!(state.cursor.1, 3);
    }

    #[test]
    fn delete_at_end_of_last_line_is_noop() {
        let mut state = state_from("abc");
        state.cursor.1 = 3;
        state.delete();
        assert_eq!(state.lines[0], "abc");
    }

    #[test]
    fn delete_does_not_join_readonly_next_line() {
        let mut state = state_from("editable\ncontext");
        state.editable_range = (0, 1);
        state.cursor = (0, 8);
        state.delete();
        assert_eq!(state.line_count(), 2);
        assert!(!state.dirty);
    }

    // ── Navigation ────────────────────────────────────────────────────────────

    #[test]
    fn move_left_decrements_column() {
        let mut state = state_from("abc");
        state.cursor.1 = 2;
        state.move_left();
        assert_eq!(state.cursor.1, 1);
    }

    #[test]
    fn move_left_at_start_goes_to_prev_line() {
        let mut state = state_from("ab\ncd");
        state.cursor = (1, 0);
        state.move_left();
        assert_eq!(state.cursor.0, 0);
        assert_eq!(state.cursor.1, 2);
    }

    #[test]
    fn move_left_at_origin_is_noop() {
        let mut state = state_from("abc");
        state.cursor = (0, 0);
        state.move_left();
        assert_eq!(state.cursor, (0, 0));
    }

    #[test]
    fn move_right_increments_column() {
        let mut state = state_from("abc");
        state.cursor.1 = 0;
        state.move_right();
        assert_eq!(state.cursor.1, 1);
    }

    #[test]
    fn move_right_at_end_goes_to_next_line() {
        let mut state = state_from("ab\ncd");
        state.cursor = (0, 2);
        state.move_right();
        assert_eq!(state.cursor.0, 1);
        assert_eq!(state.cursor.1, 0);
    }

    #[test]
    fn move_right_at_end_of_last_line_is_noop() {
        let mut state = state_from("abc");
        state.cursor = (0, 3);
        state.move_right();
        assert_eq!(state.cursor, (0, 3));
    }

    #[test]
    fn move_up_clamps_column() {
        let mut state = state_from("ab\nhello world");
        state.cursor = (1, 11);
        state.move_up();
        assert_eq!(state.cursor.0, 0);
        // "ab" has length 2, so column must be clamped to 2.
        assert_eq!(state.cursor.1, 2);
    }

    #[test]
    fn move_down_clamps_column() {
        let mut state = state_from("hello world\nab");
        state.cursor = (0, 11);
        state.move_down();
        assert_eq!(state.cursor.0, 1);
        assert_eq!(state.cursor.1, 2);
    }

    #[test]
    fn home_moves_to_start_of_line() {
        let mut state = state_from("abc");
        state.cursor.1 = 3;
        state.home();
        assert_eq!(state.cursor.1, 0);
    }

    #[test]
    fn end_moves_to_end_of_line() {
        let mut state = state_from("abc");
        state.cursor.1 = 0;
        state.end();
        assert_eq!(state.cursor.1, 3);
    }

    #[test]
    fn page_up_moves_cursor_multiple_lines() {
        let lines: Vec<&str> = (0..20).map(|_| "line").collect();
        let content = lines.join("\n");
        let mut state = state_from(&content);
        state.cursor = (15, 0);
        state.page_up(10);
        assert_eq!(state.cursor.0, 5);
    }

    #[test]
    fn page_down_moves_cursor_multiple_lines() {
        let lines: Vec<&str> = (0..20).map(|_| "line").collect();
        let content = lines.join("\n");
        let mut state = state_from(&content);
        state.cursor = (3, 0);
        state.page_down(10);
        assert_eq!(state.cursor.0, 13);
    }

    #[test]
    fn page_up_does_not_go_before_first_line() {
        let mut state = state_from("a\nb\nc");
        state.cursor = (1, 0);
        state.page_up(10);
        assert_eq!(state.cursor.0, 0);
    }

    #[test]
    fn page_down_does_not_go_past_last_line() {
        let mut state = state_from("a\nb\nc");
        state.cursor = (1, 0);
        state.page_down(100);
        assert_eq!(state.cursor.0, 2);
    }

    // ── Read-only navigation ──────────────────────────────────────────────────

    #[test]
    fn navigation_into_readonly_lines_is_allowed() {
        let mut state = state_from("context\neditable\ncontext2");
        state.editable_range = (1, 2);
        state.cursor = (1, 0);
        // Navigate into read-only context above.
        state.move_up();
        assert_eq!(
            state.cursor.0, 0,
            "navigation into read-only line should work"
        );
    }

    // ── Save ──────────────────────────────────────────────────────────────────

    #[test]
    fn save_writes_content_to_file() {
        let f = temp_file("original");
        let mut state = EditorState::from_file(f.path()).expect("load");
        state.cursor.1 = 8;
        state.insert('!');
        state.save().expect("save");
        let written = std::fs::read_to_string(f.path()).expect("read back");
        assert_eq!(written, "original!");
    }

    #[test]
    fn save_clears_dirty_flag() {
        let f = temp_file("hi");
        let mut state = EditorState::from_file(f.path()).expect("load");
        state.insert('!');
        assert!(state.dirty);
        state.save().expect("save");
        assert!(!state.dirty);
        assert!(state.saved);
    }

    #[test]
    fn save_without_path_returns_error() {
        let mut state = state_from("data");
        let result = state.save();
        assert!(result.is_err());
    }

    // ── Dirty flag ────────────────────────────────────────────────────────────

    #[test]
    fn dirty_not_set_on_load() {
        let f = temp_file("content");
        let state = EditorState::from_file(f.path()).expect("load");
        assert!(!state.dirty);
    }

    #[test]
    fn dirty_set_on_insert() {
        let mut state = state_from("x");
        state.insert('y');
        assert!(state.dirty);
    }

    #[test]
    fn dirty_set_on_backspace() {
        let mut state = state_from("xy");
        state.cursor.1 = 2;
        state.backspace();
        assert!(state.dirty);
    }

    #[test]
    fn dirty_set_on_delete() {
        let mut state = state_from("xy");
        state.cursor.1 = 0;
        state.delete();
        assert!(state.dirty);
    }

    // ── Scroll / cursor visibility ────────────────────────────────────────────

    #[test]
    fn ensure_cursor_visible_scrolls_down() {
        let lines: Vec<&str> = (0..30).map(|_| "line").collect();
        let content = lines.join("\n");
        let mut state = state_from(&content);
        state.cursor.0 = 25;
        state.scroll_offset = 0;
        state.ensure_cursor_visible(10);
        assert!(state.scroll_offset > 0);
        assert!(state.cursor.0 < state.scroll_offset + 10);
    }

    #[test]
    fn ensure_cursor_visible_scrolls_up() {
        let lines: Vec<&str> = (0..30).map(|_| "line").collect();
        let content = lines.join("\n");
        let mut state = state_from(&content);
        state.cursor.0 = 2;
        state.scroll_offset = 15;
        state.ensure_cursor_visible(10);
        assert_eq!(state.scroll_offset, 2);
    }

    // ── content() and line_count() ────────────────────────────────────────────

    #[test]
    fn content_joins_lines_with_newline() {
        let state = state_from("a\nb\nc");
        assert_eq!(state.content(), "a\nb\nc");
    }

    #[test]
    fn cursor_col_counts_chars_not_bytes() {
        let mut state = state_from("héllo");
        // 'é' is 2 bytes; cursor at byte 3 (after "hé") should report col 2.
        state.cursor.1 = 3; // byte offset after 'h'(1) + 'é'(2)
        assert_eq!(state.cursor_col(), 2);
    }

    // ── Edge cases ────────────────────────────────────────────────────────────

    #[test]
    fn empty_file_via_from_file() {
        let f = temp_file("");
        let state = EditorState::from_file(f.path()).expect("load");
        assert_eq!(state.line_count(), 1);
        assert_eq!(state.lines[0], "");
    }

    #[test]
    fn single_line_no_newline() {
        let state = state_from("abc");
        assert_eq!(state.line_count(), 1);
        assert_eq!(state.cursor_line(), 0);
    }
}
