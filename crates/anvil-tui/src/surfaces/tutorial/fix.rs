use eddacraft_tui::keyboard::Action;
use eddacraft_tui::theme::EddaCraftTheme;
use eddacraft_tui::widgets::editor::EditorState;
use ratatui::Frame;
use ratatui::layout::Rect;

use crate::surface::Surface;

use super::discovery::Finding;
use super::fix_render;

// ---------------------------------------------------------------------------
// Phase
// ---------------------------------------------------------------------------

/// Current phase of the fix surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FixPhase {
    /// Waiting for the user to edit the file externally (or press 'e' for inline).
    Watching,
    /// Inline editor is active.
    Editing,
    /// The check passed after an edit — success state.
    Resolved,
    /// Timeout elapsed without a fix.
    TimedOut,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

/// Default timeout in ticks (600 ticks at ~100 ms/tick = 60 s).
const DEFAULT_TIMEOUT_TICKS: u32 = 600;

/// Pending inline-editor save: a line-range replacement, not whole-file content.
///
/// The inline editor only edits the displayed context window. Writing
/// [`Self::content`] alone to the source path would truncate every line
/// outside that window. Callers must apply the replacement with
/// [`Self::apply_to`] before writing to disk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingSave {
    /// 1-based start line of the replaced range in the original file.
    pub start_line: usize,
    /// Number of original context lines this edit replaces (the window size
    /// when the editor opened, not the post-edit line count).
    pub original_line_count: usize,
    /// Replacement text for that range (may contain a different line count).
    pub content: String,
}

impl PendingSave {
    /// Splice this range replacement into full `original` file text.
    ///
    /// Lines before [`Self::start_line`] and after the original window remain
    /// intact. The result is complete file content suitable for writing to disk.
    #[must_use]
    pub fn apply_to(&self, original: &str) -> String {
        let trailing_newline = !original.is_empty() && original.ends_with('\n');
        let lines: Vec<&str> = original.lines().collect();

        let start = self.start_line.saturating_sub(1).min(lines.len());
        let end = start
            .saturating_add(self.original_line_count)
            .min(lines.len());

        let replacement: Vec<&str> = if self.content.is_empty() {
            Vec::new()
        } else {
            // `split` keeps a trailing empty segment when content ends with \n,
            // matching EditorState::content() join behaviour.
            self.content.split('\n').collect()
        };

        let mut out =
            Vec::with_capacity(lines.len().saturating_sub(end - start) + replacement.len());
        out.extend_from_slice(&lines[..start]);
        out.extend(replacement.iter().copied());
        out.extend_from_slice(&lines[end..]);

        if out.is_empty() {
            return if trailing_newline {
                "\n".to_string()
            } else {
                String::new()
            };
        }

        let mut result = out.join("\n");
        if trailing_newline {
            result.push('\n');
        }
        result
    }
}

/// TUI surface that presents a single finding and lets the user fix it via
/// external editing, inline editing, or skip.
///
/// This is a pure state machine — file watching, I/O, and check execution are
/// driven externally. The caller must invoke [`FixState::set_context`],
/// [`FixState::notify_file_changed`], [`FixState::set_check_result`], and
/// [`FixState::tick`] as appropriate.
#[allow(clippy::struct_excessive_bools)]
pub struct FixState {
    /// Current phase of the fix flow.
    pub phase: FixPhase,
    /// The finding to fix.
    pub finding: Finding,
    /// Lines of file content around the warning for context display.
    pub context_lines: Vec<String>,
    /// 1-based line number where `context_lines` starts in the file.
    pub context_start_line: usize,
    /// Inline editor state, created when the user presses 'e'.
    pub editor: Option<EditorState>,
    /// Whether the check passed after the last edit.
    pub check_passed: bool,
    /// Ticks elapsed (caller calls `tick()` each cycle, ~100 ms per tick).
    pub ticks: u32,
    /// Tick limit for timeout.
    pub timeout_ticks: u32,
    pub should_quit: bool,
    pub wants_back: bool,
    /// Set when the user advances past this surface (fix resolved or skip).
    pub wants_advance: bool,
    /// Set when the user explicitly skips the fix.
    pub wants_skip: bool,
    /// Pending inline-editor save captured when the user saves.
    ///
    /// This is a **range replacement** for the context window, not whole-file
    /// content. Callers must use [`PendingSave::apply_to`] against the original
    /// file text before writing to disk, then re-run the check. Cleared after
    /// the caller processes it via [`Self::take_pending_save`].
    pub pending_save: Option<PendingSave>,
    /// When true, the inline editor ('e' key) is disabled. Set by callers
    /// that cannot drive the editor save/check loop (e.g. the welcome flow).
    pub editor_disabled: bool,
}

impl FixState {
    /// Create a new fix surface for the given finding.
    pub fn new(finding: Finding) -> Self {
        Self {
            phase: FixPhase::Watching,
            finding,
            context_lines: Vec::new(),
            context_start_line: 1,
            editor: None,
            check_passed: false,
            ticks: 0,
            timeout_ticks: DEFAULT_TIMEOUT_TICKS,
            should_quit: false,
            wants_back: false,
            wants_advance: false,
            wants_skip: false,
            pending_save: None,
            editor_disabled: false,
        }
    }

    /// Set the file context lines displayed around the warning.
    ///
    /// `start_line` is the 1-based line number of the first line in `lines`.
    pub fn set_context(&mut self, lines: Vec<String>, start_line: usize) {
        self.context_lines = lines;
        self.context_start_line = start_line;
    }

    /// Take the pending save, clearing the field atomically.
    ///
    /// Returns `Some(PendingSave)` if the user saved from the inline editor
    /// since the last call. The caller must apply the range replacement to the
    /// original file via [`PendingSave::apply_to`] before writing to disk, then
    /// call [`Self::set_check_result`].
    pub fn take_pending_save(&mut self) -> Option<PendingSave> {
        self.pending_save.take()
    }

    /// Notify that the watched file changed on disk.
    ///
    /// Only meaningful in the `Watching` phase — triggers the caller to re-run
    /// the relevant check and call [`set_check_result`].
    pub fn notify_file_changed(&mut self) {
        // The notification itself is a no-op on state; the caller is expected
        // to re-run the check and call `set_check_result`. We intentionally
        // keep this method so the API surface is explicit.
    }

    /// Provide the result of a re-check after a file change or editor save.
    ///
    /// When `passed` is true the phase transitions to `Resolved`.
    pub fn set_check_result(&mut self, passed: bool) {
        self.check_passed = passed;
        if passed && matches!(self.phase, FixPhase::Watching | FixPhase::Editing) {
            self.phase = FixPhase::Resolved;
            // Close the editor if it was open.
            self.editor = None;
        }
    }

    /// Advance the timeout counter by one tick (~100 ms).
    ///
    /// Transitions to `TimedOut` when the limit is reached while still in
    /// `Watching` phase.
    pub fn tick(&mut self) {
        if matches!(self.phase, FixPhase::Watching) {
            self.ticks = self.ticks.saturating_add(1);
            if self.ticks >= self.timeout_ticks {
                self.phase = FixPhase::TimedOut;
            }
        }
    }

    /// Transition to the inline editing phase.
    ///
    /// Creates an [`EditorState`] from the current context lines.
    pub fn open_editor(&mut self) {
        if !matches!(self.phase, FixPhase::Watching) {
            return;
        }

        let content = self.context_lines.join("\n");
        let mut editor = EditorState::from_string(&content);

        // Position the cursor on the warning line within the context block.
        // Only reposition if the warning line falls within the context window.
        if let Some(warning_line) = self.finding.line
            && warning_line >= self.context_start_line
        {
            let target = warning_line - self.context_start_line;
            let total = editor.line_count();
            let clamped = target.min(total.saturating_sub(1));
            for _ in 0..clamped {
                editor.move_down();
            }
        }

        self.editor = Some(editor);
        self.phase = FixPhase::Editing;
    }

    /// Close the inline editor and return to watching.
    pub fn close_editor(&mut self) {
        if matches!(self.phase, FixPhase::Editing) {
            self.editor = None;
            self.phase = FixPhase::Watching;
        }
    }

    // ── Key handling helpers ────────────────────────────────────────────

    fn handle_watching(&mut self, action: Action) {
        match action {
            Action::Character('e') if !self.editor_disabled => self.open_editor(),
            Action::Character('s') => {
                self.wants_skip = true;
                self.wants_advance = true;
            }
            Action::Back => self.wants_back = true,
            Action::Quit => self.should_quit = true,
            _ => {}
        }
    }

    fn handle_editing(&mut self, action: Action) {
        let Some(editor) = self.editor.as_mut() else {
            return;
        };

        match action {
            // Ctrl-S arrives as Character('\x13') if the caller passes raw bytes,
            // but the standard KeyHandler maps Ctrl+key (except Ctrl-C) to
            // Action::None. We also accept Select (Enter) as a save trigger
            // when in editor mode to ensure there is always a reachable path.
            Action::Character('\x13') | Action::Select => {
                // Capture a range replacement for the context window. The
                // editor only holds that window — writing its content alone
                // would truncate the rest of the file. Stash before
                // close_editor drops the EditorState.
                self.pending_save = Some(PendingSave {
                    start_line: self.context_start_line,
                    original_line_count: self.context_lines.len(),
                    content: editor.content(),
                });
                self.close_editor();
            }
            Action::Character(c) => editor.insert(c),
            Action::Backspace => editor.backspace(),
            Action::Delete => editor.delete(),
            Action::Up => editor.move_up(),
            Action::Down => editor.move_down(),
            Action::Left => editor.move_left(),
            Action::Right => editor.move_right(),
            Action::Home => editor.home(),
            Action::End => editor.end(),
            Action::PageUp => editor.page_up(20),
            Action::PageDown => editor.page_down(20),
            Action::Back => self.close_editor(),
            Action::Quit => self.should_quit = true,
            _ => {}
        }
    }

    fn handle_resolved(&mut self, action: Action) {
        match action {
            Action::Select => self.wants_advance = true,
            Action::Back => self.wants_back = true,
            Action::Quit => self.should_quit = true,
            _ => {}
        }
    }

    fn handle_timed_out(&mut self, action: Action) {
        match action {
            Action::Select | Action::Character('s') => {
                self.wants_skip = true;
                self.wants_advance = true;
            }
            Action::Back => self.wants_back = true,
            Action::Quit => self.should_quit = true,
            _ => {}
        }
    }

    /// Process a keyboard action, dispatching to the current phase handler.
    pub fn handle_key(&mut self, action: Action) {
        match self.phase {
            FixPhase::Watching => self.handle_watching(action),
            FixPhase::Editing => self.handle_editing(action),
            FixPhase::Resolved => self.handle_resolved(action),
            FixPhase::TimedOut => self.handle_timed_out(action),
        }
    }
}

impl Surface for FixState {
    fn surface_name(&self) -> &'static str {
        "Fix"
    }

    fn help_text(&self) -> &'static str {
        match self.phase {
            FixPhase::Watching if self.editor_disabled => "s skip  esc back  q quit",
            FixPhase::Watching => "e editor  s skip  esc back  q quit",
            FixPhase::Editing => "type to edit  enter save  esc cancel  q quit",
            FixPhase::Resolved => "enter continue  esc back  q quit",
            FixPhase::TimedOut => "enter/s skip  esc back  q quit",
        }
    }

    fn handle_key(&mut self, action: Action) {
        self.handle_key(action);
    }

    fn should_quit(&self) -> bool {
        self.should_quit || self.wants_advance
    }

    fn should_back(&self) -> bool {
        self.wants_back
    }

    fn reset(&mut self) {
        self.phase = FixPhase::Watching;
        self.context_lines.clear();
        self.context_start_line = 1;
        self.editor = None;
        self.check_passed = false;
        self.ticks = 0;
        self.should_quit = false;
        self.wants_back = false;
        self.wants_advance = false;
        self.wants_skip = false;
        self.pending_save = None;
    }

    fn render(&self, frame: &mut Frame, area: Rect, theme: &EddaCraftTheme) {
        fix_render::render(frame, area, self, theme);
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::super::discovery::{FindingSeverity, FindingSource};
    use super::*;

    fn make_finding() -> Finding {
        Finding {
            file: "src/main.rs".to_string(),
            line: Some(10),
            severity: FindingSeverity::Error,
            source: FindingSource::AntiPattern,
            title: "hardcoded secret".to_string(),
            message: "API key found in source".to_string(),
            suggestion: "Move the secret to an environment variable".to_string(),
            warning_id: None,
        }
    }

    fn make_context_lines() -> Vec<String> {
        vec![
            "fn main() {".to_string(),
            "    let config = load_config();".to_string(),
            "    let db = connect(&config);".to_string(),
            "    let key = \"sk-1234567890\";".to_string(),
            "    let client = Client::new(key);".to_string(),
            "    start_server(client);".to_string(),
            "}".to_string(),
        ]
    }

    fn state_with_context() -> FixState {
        let mut state = FixState::new(make_finding());
        state.set_context(make_context_lines(), 7);
        state
    }

    // ── Initial state ────────────────────────────────────────────────────

    #[test]
    fn new_starts_in_watching_phase() {
        let state = FixState::new(make_finding());
        assert_eq!(state.phase, FixPhase::Watching);
        assert!(!state.should_quit);
        assert!(!state.wants_back);
        assert!(!state.wants_advance);
        assert!(!state.wants_skip);
        assert!(!state.check_passed);
        assert!(state.editor.is_none());
        assert_eq!(state.ticks, 0);
        assert_eq!(state.timeout_ticks, DEFAULT_TIMEOUT_TICKS);
    }

    #[test]
    fn new_stores_finding() {
        let finding = make_finding();
        let state = FixState::new(finding.clone());
        assert_eq!(state.finding.title, "hardcoded secret");
        assert_eq!(state.finding.file, "src/main.rs");
        assert_eq!(state.finding.line, Some(10));
    }

    // ── set_context ─────────────────────────────────────────────────────

    #[test]
    fn set_context_stores_lines_and_start() {
        let mut state = FixState::new(make_finding());
        let lines = make_context_lines();
        state.set_context(lines.clone(), 7);
        assert_eq!(state.context_lines, lines);
        assert_eq!(state.context_start_line, 7);
    }

    // ── tick / timeout ──────────────────────────────────────────────────

    #[test]
    fn tick_advances_counter_in_watching() {
        let mut state = FixState::new(make_finding());
        state.tick();
        assert_eq!(state.ticks, 1);
        state.tick();
        assert_eq!(state.ticks, 2);
    }

    #[test]
    fn tick_transitions_to_timed_out() {
        let mut state = FixState::new(make_finding());
        state.timeout_ticks = 5;
        for _ in 0..5 {
            state.tick();
        }
        assert_eq!(state.phase, FixPhase::TimedOut);
        assert_eq!(state.ticks, 5);
    }

    #[test]
    fn tick_noop_in_editing_phase() {
        let mut state = state_with_context();
        state.open_editor();
        assert_eq!(state.phase, FixPhase::Editing);
        state.tick();
        assert_eq!(state.ticks, 0);
        assert_eq!(state.phase, FixPhase::Editing);
    }

    #[test]
    fn tick_noop_in_resolved_phase() {
        let mut state = FixState::new(make_finding());
        state.set_check_result(true);
        assert_eq!(state.phase, FixPhase::Resolved);
        state.tick();
        assert_eq!(state.ticks, 0);
    }

    #[test]
    fn tick_noop_in_timed_out_phase() {
        let mut state = FixState::new(make_finding());
        state.timeout_ticks = 2;
        state.tick();
        state.tick();
        assert_eq!(state.phase, FixPhase::TimedOut);
        let ticks_before = state.ticks;
        state.tick();
        assert_eq!(state.ticks, ticks_before);
    }

    // ── set_check_result ────────────────────────────────────────────────

    #[test]
    fn check_passed_transitions_to_resolved() {
        let mut state = FixState::new(make_finding());
        state.set_check_result(true);
        assert!(state.check_passed);
        assert_eq!(state.phase, FixPhase::Resolved);
    }

    #[test]
    fn check_failed_stays_in_watching() {
        let mut state = FixState::new(make_finding());
        state.set_check_result(false);
        assert!(!state.check_passed);
        assert_eq!(state.phase, FixPhase::Watching);
    }

    #[test]
    fn check_passed_in_editing_closes_editor() {
        let mut state = state_with_context();
        state.open_editor();
        assert!(state.editor.is_some());

        state.set_check_result(true);
        assert_eq!(state.phase, FixPhase::Resolved);
        assert!(state.editor.is_none());
    }

    #[test]
    fn check_result_noop_in_resolved() {
        let mut state = FixState::new(make_finding());
        state.set_check_result(true);
        assert_eq!(state.phase, FixPhase::Resolved);

        // Another call should not change anything.
        state.set_check_result(false);
        assert_eq!(state.phase, FixPhase::Resolved);
    }

    #[test]
    fn check_result_noop_in_timed_out() {
        let mut state = FixState::new(make_finding());
        state.timeout_ticks = 1;
        state.tick();
        assert_eq!(state.phase, FixPhase::TimedOut);

        state.set_check_result(true);
        assert_eq!(state.phase, FixPhase::TimedOut);
    }

    // ── open_editor / close_editor ──────────────────────────────────────

    #[test]
    fn open_editor_transitions_to_editing() {
        let mut state = state_with_context();
        state.open_editor();
        assert_eq!(state.phase, FixPhase::Editing);
        assert!(state.editor.is_some());
    }

    #[test]
    fn open_editor_positions_cursor_on_warning_line() {
        let mut state = state_with_context();
        // Finding line = 10, context starts at line 7 -> offset 3 (0-based).
        state.open_editor();
        let editor = state.editor.as_ref().unwrap();
        assert_eq!(editor.cursor_line(), 3);
    }

    #[test]
    fn open_editor_clamps_cursor_when_past_end() {
        let mut state = FixState::new(Finding {
            file: "src/main.rs".to_string(),
            line: Some(100),
            severity: FindingSeverity::Error,
            source: FindingSource::AntiPattern,
            title: "test".to_string(),
            message: "test".to_string(),
            suggestion: "test".to_string(),
            warning_id: None,
        });
        state.set_context(vec!["line1".to_string(), "line2".to_string()], 1);
        state.open_editor();
        let editor = state.editor.as_ref().unwrap();
        // Line 100 is far past 2 lines — should clamp to last line (index 1).
        assert!(editor.cursor_line() <= 1);
    }

    #[test]
    fn open_editor_noop_when_not_watching() {
        let mut state = FixState::new(make_finding());
        state.set_check_result(true);
        assert_eq!(state.phase, FixPhase::Resolved);
        state.open_editor();
        assert_eq!(state.phase, FixPhase::Resolved);
        assert!(state.editor.is_none());
    }

    #[test]
    fn close_editor_returns_to_watching() {
        let mut state = state_with_context();
        state.open_editor();
        state.close_editor();
        assert_eq!(state.phase, FixPhase::Watching);
        assert!(state.editor.is_none());
    }

    #[test]
    fn close_editor_noop_when_not_editing() {
        let mut state = FixState::new(make_finding());
        state.close_editor();
        assert_eq!(state.phase, FixPhase::Watching);
    }

    // ── Key handling: Watching ──────────────────────────────────────────

    #[test]
    fn watching_e_opens_editor() {
        let mut state = state_with_context();
        state.handle_key(Action::Character('e'));
        assert_eq!(state.phase, FixPhase::Editing);
    }

    #[test]
    fn watching_s_skips() {
        let mut state = FixState::new(make_finding());
        state.handle_key(Action::Character('s'));
        assert!(state.wants_skip);
        assert!(state.wants_advance);
        assert!(Surface::should_quit(&state));
    }

    #[test]
    fn watching_back_sets_wants_back() {
        let mut state = FixState::new(make_finding());
        state.handle_key(Action::Back);
        assert!(state.wants_back);
    }

    #[test]
    fn watching_quit_sets_should_quit() {
        let mut state = FixState::new(make_finding());
        state.handle_key(Action::Quit);
        assert!(state.should_quit);
    }

    #[test]
    fn watching_other_keys_noop() {
        let mut state = FixState::new(make_finding());
        state.handle_key(Action::Down);
        assert_eq!(state.phase, FixPhase::Watching);
        assert!(!state.should_quit);
        assert!(!state.wants_back);
    }

    // ── Key handling: Editing ──────────────────────────────────────────

    #[test]
    fn editing_character_inserts() {
        let mut state = state_with_context();
        state.open_editor();
        state.handle_key(Action::Character('x'));
        let editor = state.editor.as_ref().unwrap();
        assert!(editor.dirty);
    }

    #[test]
    fn editing_backspace_deletes() {
        let mut state = state_with_context();
        state.open_editor();
        // Insert then backspace.
        state.handle_key(Action::Character('x'));
        state.handle_key(Action::Backspace);
        let editor = state.editor.as_ref().unwrap();
        assert!(editor.dirty);
    }

    #[test]
    fn editing_navigation_keys() {
        let mut state = state_with_context();
        state.open_editor();
        // These should not panic.
        state.handle_key(Action::Up);
        state.handle_key(Action::Down);
        state.handle_key(Action::Left);
        state.handle_key(Action::Right);
        state.handle_key(Action::Home);
        state.handle_key(Action::End);
        state.handle_key(Action::PageUp);
        state.handle_key(Action::PageDown);
        assert_eq!(state.phase, FixPhase::Editing);
    }

    #[test]
    fn editing_back_closes_editor() {
        let mut state = state_with_context();
        state.open_editor();
        state.handle_key(Action::Back);
        assert_eq!(state.phase, FixPhase::Watching);
        assert!(state.editor.is_none());
    }

    #[test]
    fn editing_quit_sets_should_quit() {
        let mut state = state_with_context();
        state.open_editor();
        state.handle_key(Action::Quit);
        assert!(state.should_quit);
    }

    #[test]
    fn editing_select_saves_and_closes() {
        let mut state = state_with_context();
        state.open_editor();
        // Select (Enter) triggers save+close in editor mode.
        state.handle_key(Action::Select);
        assert_eq!(state.phase, FixPhase::Watching);
        assert!(state.editor.is_none());
        assert!(state.pending_save.is_some());
    }

    #[test]
    fn editing_ctrl_s_saves_and_closes() {
        let mut state = state_with_context();
        state.open_editor();
        state.handle_key(Action::Character('\x13'));
        assert_eq!(state.phase, FixPhase::Watching);
        assert!(state.editor.is_none());
        assert!(state.pending_save.is_some());
    }

    #[test]
    fn pending_save_captures_editor_text() {
        let mut state = state_with_context();
        state.open_editor();
        // Insert a character to modify the content.
        state.handle_key(Action::Character('x'));
        state.handle_key(Action::Select);
        let save = state.pending_save.as_ref().unwrap();
        assert!(save.content.contains('x'));
        assert_eq!(save.start_line, 7);
        assert_eq!(save.original_line_count, make_context_lines().len());
    }

    // ── Key handling: Resolved ──────────────────────────────────────────

    #[test]
    fn resolved_select_advances() {
        let mut state = FixState::new(make_finding());
        state.set_check_result(true);
        state.handle_key(Action::Select);
        assert!(state.wants_advance);
        assert!(Surface::should_quit(&state));
    }

    #[test]
    fn resolved_back_sets_wants_back() {
        let mut state = FixState::new(make_finding());
        state.set_check_result(true);
        state.handle_key(Action::Back);
        assert!(state.wants_back);
    }

    #[test]
    fn resolved_quit_sets_should_quit() {
        let mut state = FixState::new(make_finding());
        state.set_check_result(true);
        state.handle_key(Action::Quit);
        assert!(state.should_quit);
    }

    // ── Key handling: TimedOut ──────────────────────────────────────────

    #[test]
    fn timed_out_select_skips() {
        let mut state = FixState::new(make_finding());
        state.timeout_ticks = 1;
        state.tick();
        state.handle_key(Action::Select);
        assert!(state.wants_skip);
        assert!(state.wants_advance);
    }

    #[test]
    fn timed_out_s_skips() {
        let mut state = FixState::new(make_finding());
        state.timeout_ticks = 1;
        state.tick();
        state.handle_key(Action::Character('s'));
        assert!(state.wants_skip);
        assert!(state.wants_advance);
    }

    #[test]
    fn timed_out_back_sets_wants_back() {
        let mut state = FixState::new(make_finding());
        state.timeout_ticks = 1;
        state.tick();
        state.handle_key(Action::Back);
        assert!(state.wants_back);
    }

    #[test]
    fn timed_out_quit_sets_should_quit() {
        let mut state = FixState::new(make_finding());
        state.timeout_ticks = 1;
        state.tick();
        state.handle_key(Action::Quit);
        assert!(state.should_quit);
    }

    // ── Surface trait ───────────────────────────────────────────────────

    #[test]
    fn surface_name_returns_fix() {
        let state = FixState::new(make_finding());
        assert_eq!(state.surface_name(), "Fix");
    }

    #[test]
    fn help_text_changes_per_phase() {
        let mut state = state_with_context();
        assert!(state.help_text().contains("editor"));

        state.open_editor();
        assert!(state.help_text().contains("save"));

        state.close_editor();
        state.set_check_result(true);
        assert!(state.help_text().contains("continue"));
    }

    #[test]
    fn help_text_timed_out() {
        let mut state = FixState::new(make_finding());
        state.timeout_ticks = 1;
        state.tick();
        assert!(state.help_text().contains("skip"));
    }

    #[test]
    fn should_quit_true_when_advance() {
        let mut state = FixState::new(make_finding());
        state.set_check_result(true);
        state.handle_key(Action::Select);
        assert!(Surface::should_quit(&state));
    }

    #[test]
    fn should_quit_true_when_quit() {
        let mut state = FixState::new(make_finding());
        state.handle_key(Action::Quit);
        assert!(Surface::should_quit(&state));
    }

    #[test]
    fn should_back_true_when_wants_back() {
        let mut state = FixState::new(make_finding());
        state.handle_key(Action::Back);
        assert!(Surface::should_back(&state));
    }

    #[test]
    fn reset_returns_to_initial_state() {
        let mut state = state_with_context();
        state.open_editor();
        state.handle_key(Action::Select); // triggers save, sets pending_save
        state.ticks = 100;
        state.should_quit = true;
        state.wants_advance = true;
        state.wants_skip = true;
        state.check_passed = true;

        state.reset();

        assert_eq!(state.phase, FixPhase::Watching);
        assert!(state.context_lines.is_empty());
        assert_eq!(state.context_start_line, 1);
        assert!(state.editor.is_none());
        assert!(!state.check_passed);
        assert_eq!(state.ticks, 0);
        assert!(!state.should_quit);
        assert!(!state.wants_back);
        assert!(!state.wants_advance);
        assert!(!state.wants_skip);
        assert!(state.pending_save.is_none());
    }

    #[test]
    fn reset_clears_pending_save() {
        let mut state = state_with_context();
        state.open_editor();
        state.handle_key(Action::Select);
        assert!(state.pending_save.is_some());

        state.reset();
        assert!(state.pending_save.is_none());
    }

    #[test]
    fn take_pending_save_clears_field() {
        let mut state = state_with_context();
        state.open_editor();
        state.handle_key(Action::Select);

        let save = state.take_pending_save();
        assert!(save.is_some());
        assert!(state.pending_save.is_none());

        // Second call returns None.
        assert!(state.take_pending_save().is_none());
    }

    // ── PendingSave range apply (data-loss regression) ──────────────────

    #[test]
    fn pending_save_preserves_content_outside_context_window() {
        // Full source with content before and after the displayed window.
        let original = "prefix_a
prefix_b
window_1
window_2_SECRET
window_3
suffix_a
suffix_b
";
        // Context window is lines 3-5 only (the window_* block).
        let window = vec![
            "window_1".to_string(),
            "window_2_SECRET".to_string(),
            "window_3".to_string(),
        ];
        let mut state = FixState::new(Finding {
            file: "src/main.rs".to_string(),
            line: Some(4),
            severity: FindingSeverity::Error,
            source: FindingSource::AntiPattern,
            title: "hardcoded secret".to_string(),
            message: "API key found in source".to_string(),
            suggestion: "Move the secret to an environment variable".to_string(),
            warning_id: None,
        });
        state.set_context(window, 3);

        // Open editor, move to the secret line, replace it via delete+type.
        state.open_editor();
        // Cursor is on warning line (line 4 => offset 1 within window).
        assert_eq!(state.editor.as_ref().unwrap().cursor_line(), 1);

        // Clear the secret line and type a safe replacement.
        // End then backspace until empty, then type the fix.
        state.handle_key(Action::End);
        for _ in 0.."window_2_SECRET".len() {
            state.handle_key(Action::Backspace);
        }
        for ch in "window_2_SAFE".chars() {
            state.handle_key(Action::Character(ch));
        }
        state.handle_key(Action::Select);

        let save = state
            .take_pending_save()
            .expect("pending save after inline edit");
        // Contract: payload is a range, not a whole-file dump of the window.
        assert_eq!(save.start_line, 3);
        assert_eq!(save.original_line_count, 3);
        assert!(save.content.contains("window_2_SAFE"));
        assert!(!save.content.contains("SECRET"));

        // Naive whole-file write of the window alone would drop prefix/suffix.
        assert!(
            !save.content.contains("prefix_a"),
            "window-only payload must not look like a full file"
        );

        let applied = save.apply_to(original);
        assert!(
            applied.starts_with("prefix_a\nprefix_b\n"),
            "prefix must be preserved: {applied:?}"
        );
        assert!(
            applied.contains("window_2_SAFE"),
            "edit must be present: {applied:?}"
        );
        assert!(
            !applied.contains("SECRET"),
            "old secret must be gone: {applied:?}"
        );
        assert!(
            applied.ends_with("suffix_a\nsuffix_b\n") || applied.contains("\nsuffix_a\nsuffix_b\n"),
            "suffix must be preserved: {applied:?}"
        );
        // Byte-stable regions outside the window.
        let prefix = "prefix_a\nprefix_b\n";
        assert_eq!(&applied[..prefix.len()], prefix);
        assert!(applied.ends_with("suffix_a\nsuffix_b\n"));
        assert_eq!(
            applied.matches('\n').count(),
            original.matches('\n').count()
        );
    }

    #[test]
    fn pending_save_apply_to_round_trips_unchanged_window() {
        let original = "a\nb\nc\nd\ne\n";
        let mut state = FixState::new(make_finding());
        state.set_context(vec!["b".to_string(), "c".to_string(), "d".to_string()], 2);
        state.open_editor();
        state.handle_key(Action::Select); // save without edits
        let save = state.take_pending_save().expect("save");
        assert_eq!(save.apply_to(original), original);
    }

    // ── Full flow: external edit ────────────────────────────────────────

    #[test]
    fn full_flow_external_edit_resolves() {
        let mut state = state_with_context();
        assert_eq!(state.phase, FixPhase::Watching);

        // Simulate a few ticks while waiting.
        state.tick();
        state.tick();
        assert_eq!(state.ticks, 2);

        // Caller detects file change and re-runs check.
        state.notify_file_changed();
        state.set_check_result(true);
        assert_eq!(state.phase, FixPhase::Resolved);

        // User presses Enter to advance.
        state.handle_key(Action::Select);
        assert!(state.wants_advance);
        assert!(Surface::should_quit(&state));
    }

    // ── Full flow: inline editor resolves ───────────────────────────────

    #[test]
    fn full_flow_inline_edit_resolves() {
        let mut state = state_with_context();

        // Press 'e' to open editor.
        state.handle_key(Action::Character('e'));
        assert_eq!(state.phase, FixPhase::Editing);
        assert!(state.editor.is_some());

        // Type something.
        state.handle_key(Action::Character('x'));

        // Save (Select/Enter).
        state.handle_key(Action::Select);
        assert_eq!(state.phase, FixPhase::Watching);

        // Caller re-runs check after editor save.
        state.set_check_result(true);
        assert_eq!(state.phase, FixPhase::Resolved);

        // Advance.
        state.handle_key(Action::Select);
        assert!(state.wants_advance);
    }

    // ── Full flow: timeout ──────────────────────────────────────────────

    #[test]
    fn full_flow_timeout_then_skip() {
        let mut state = FixState::new(make_finding());
        state.timeout_ticks = 3;

        state.tick();
        state.tick();
        assert_eq!(state.phase, FixPhase::Watching);

        state.tick();
        assert_eq!(state.phase, FixPhase::TimedOut);

        // Skip from timeout.
        state.handle_key(Action::Select);
        assert!(state.wants_skip);
        assert!(state.wants_advance);
    }

    // ── Full flow: skip immediately ─────────────────────────────────────

    #[test]
    fn full_flow_skip_immediately() {
        let mut state = state_with_context();
        state.handle_key(Action::Character('s'));
        assert!(state.wants_skip);
        assert!(state.wants_advance);
        assert!(Surface::should_quit(&state));
    }

    // ── Edge cases ──────────────────────────────────────────────────────

    #[test]
    fn open_editor_with_no_context_lines() {
        let mut state = FixState::new(make_finding());
        // No context set — open_editor should still work with empty content.
        state.open_editor();
        assert_eq!(state.phase, FixPhase::Editing);
        let editor = state.editor.as_ref().unwrap();
        assert_eq!(editor.line_count(), 1); // single empty line
    }

    #[test]
    fn finding_without_line_number() {
        let mut state = FixState::new(Finding {
            file: "src/lib.rs".to_string(),
            line: None,
            severity: FindingSeverity::Warning,
            source: FindingSource::Architecture,
            title: "boundary violation".to_string(),
            message: "cross-module import".to_string(),
            suggestion: "use the public API".to_string(),
            warning_id: None,
        });
        state.set_context(make_context_lines(), 1);
        state.open_editor();
        let editor = state.editor.as_ref().unwrap();
        // Without a line number, cursor stays at line 0.
        assert_eq!(editor.cursor_line(), 0);
    }

    #[test]
    fn multiple_check_results_before_resolution() {
        let mut state = FixState::new(make_finding());
        state.set_check_result(false);
        assert_eq!(state.phase, FixPhase::Watching);
        state.set_check_result(false);
        assert_eq!(state.phase, FixPhase::Watching);
        state.set_check_result(true);
        assert_eq!(state.phase, FixPhase::Resolved);
    }
}
