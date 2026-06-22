use eddacraft_tui::keyboard::Action;
use eddacraft_tui::theme::{EddaCraftTheme, Theme};
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::surface::Surface;

// ---------------------------------------------------------------------------
// Summary data
// ---------------------------------------------------------------------------

/// Summary data passed to the completion screen.
#[derive(Default)]
pub struct OnboardingSummary {
    /// Number of findings from the discovery scan.
    pub findings_count: usize,
    /// Whether anvil config was created during guided init.
    pub config_created: bool,
    /// Whether git hooks were installed.
    pub hooks_installed: bool,
    /// Names of hooks that were installed (e.g., `["pre-commit", "pre-push"]`).
    pub hooks_names: Vec<String>,
    /// Whether a tutorial path was completed.
    pub tutorial_completed: bool,
    /// Which tutorial path was completed, if any.
    pub tutorial_path: Option<String>,
}

// ---------------------------------------------------------------------------
// Surface state
// ---------------------------------------------------------------------------

/// State for the onboarding completion / summary screen.
pub struct CompletionState {
    pub summary: OnboardingSummary,
    pub should_quit: bool,
    pub wants_back: bool,
    /// Set when user presses Enter to return to the welcome menu.
    pub wants_continue: bool,
}

impl CompletionState {
    #[must_use]
    pub fn new(summary: OnboardingSummary) -> Self {
        Self {
            summary,
            should_quit: false,
            wants_back: false,
            wants_continue: false,
        }
    }
}

impl Surface for CompletionState {
    fn surface_name(&self) -> &'static str {
        "Complete"
    }

    fn help_text(&self) -> &'static str {
        "enter continue  esc back  q quit"
    }

    fn handle_key(&mut self, action: Action) {
        match action {
            Action::Select => {
                self.wants_continue = true;
            }
            Action::Back => {
                self.wants_back = true;
            }
            Action::Quit => {
                self.should_quit = true;
            }
            _ => {}
        }
    }

    fn should_quit(&self) -> bool {
        self.should_quit || self.wants_continue
    }

    fn should_back(&self) -> bool {
        self.wants_back
    }

    fn reset(&mut self) {
        self.should_quit = false;
        self.wants_back = false;
        self.wants_continue = false;
    }

    fn render(&self, frame: &mut Frame, area: Rect, theme: &EddaCraftTheme) {
        render(frame, area, self, theme);
    }
}

// ---------------------------------------------------------------------------
// Rendering
// ---------------------------------------------------------------------------

const PAD: &str = "  ";

fn render(frame: &mut Frame, area: Rect, state: &CompletionState, theme: &EddaCraftTheme) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.success()))
        .title(" Setup Complete ");
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let summary = &state.summary;
    let mut lines: Vec<Line> = Vec::new();

    // ── Config status ───────────────────────────────────────────────────
    if summary.config_created {
        lines.push(status_line("\u{2713} Config created", theme.success()));
    } else {
        lines.push(status_line("\u{2013} Config: skipped", theme.muted()));
    }

    // ── Hooks status ────────────────────────────────────────────────────
    if summary.hooks_installed {
        let hook_list = summary.hooks_names.join(", ");
        let label = if hook_list.is_empty() {
            "\u{2713} Hooks installed".to_string()
        } else {
            format!("\u{2713} Hooks installed: {hook_list}")
        };
        lines.push(status_line(&label, theme.success()));
    } else {
        lines.push(status_line("\u{2013} Hooks: not installed", theme.muted()));
    }

    // ── Findings count ──────────────────────────────────────────────────
    if summary.findings_count > 0 {
        let label = format!(
            "Found {} warning{} in your project",
            summary.findings_count,
            if summary.findings_count == 1 { "" } else { "s" }
        );
        lines.push(status_line(&label, theme.warning()));
    } else {
        lines.push(status_line("No warnings found", theme.success()));
    }

    // ── Tutorial status ─────────────────────────────────────────────────
    if summary.tutorial_completed {
        let label = match &summary.tutorial_path {
            Some(path) => format!("\u{2713} Completed {path} tutorial"),
            None => "\u{2713} Tutorial completed".to_string(),
        };
        lines.push(status_line(&label, theme.success()));
    } else {
        lines.push(status_line("\u{2013} Tutorial: skipped", theme.muted()));
    }

    // ── Blank separator ─────────────────────────────────────────────────
    lines.push(Line::default());

    // ── Next steps ──────────────────────────────────────────────────────
    lines.push(Line::from(vec![
        Span::styled(PAD, Style::default()),
        Span::styled(
            "What\u{2019}s next:",
            Style::default().fg(theme.fg()).add_modifier(Modifier::BOLD),
        ),
    ]));

    for suggestion in NEXT_STEPS {
        lines.push(Line::from(vec![
            Span::styled(PAD, Style::default()),
            Span::styled("  \u{2022} ", Style::default().fg(theme.accent())),
            Span::styled(suggestion.label, Style::default().fg(theme.fg())),
            Span::styled(suggestion.command, Style::default().fg(theme.accent())),
            Span::styled(suggestion.suffix, Style::default().fg(theme.fg())),
        ]));
    }

    // ── Blank + prompt ──────────────────────────────────────────────────
    lines.push(Line::default());
    lines.push(Line::from(vec![
        Span::styled(PAD, Style::default()),
        Span::styled(
            "Press enter to continue to the welcome menu.",
            Style::default()
                .fg(theme.accent())
                .add_modifier(Modifier::BOLD),
        ),
    ]));

    frame.render_widget(Paragraph::new(Text::from(lines)), inner);
}

/// Build a single status line with consistent padding and colour.
fn status_line(label: &str, colour: ratatui::style::Color) -> Line<'static> {
    Line::from(vec![
        Span::styled(PAD, Style::default()),
        Span::styled(label.to_string(), Style::default().fg(colour)),
    ])
}

struct NextStep {
    label: &'static str,
    command: &'static str,
    suffix: &'static str,
}

const NEXT_STEPS: &[NextStep] = &[
    NextStep {
        label: "Run ",
        command: "anvil watch",
        suffix: " to monitor continuously",
    },
    NextStep {
        label: "Run ",
        command: "anvil gate",
        suffix: " before pushing",
    },
    NextStep {
        label: "See ",
        command: "anvil --help",
        suffix: " for all commands",
    },
];

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    fn default_summary() -> OnboardingSummary {
        OnboardingSummary::default()
    }

    fn full_summary() -> OnboardingSummary {
        OnboardingSummary {
            findings_count: 7,
            config_created: true,
            hooks_installed: true,
            hooks_names: vec!["pre-commit".to_string(), "pre-push".to_string()],
            tutorial_completed: true,
            tutorial_path: Some("Policy".to_string()),
        }
    }

    // ── Initial state ───────────────────────────────────────────────────

    #[test]
    fn initial_flags_are_false() {
        let state = CompletionState::new(default_summary());
        assert!(!state.should_quit);
        assert!(!state.wants_back);
        assert!(!state.wants_continue);
    }

    #[test]
    fn surface_name_is_complete() {
        let state = CompletionState::new(default_summary());
        assert_eq!(state.surface_name(), "Complete");
    }

    #[test]
    fn help_text_mentions_enter_and_quit() {
        let state = CompletionState::new(default_summary());
        let help = state.help_text();
        assert!(help.contains("enter"), "help should mention enter");
        assert!(help.contains("quit"), "help should mention quit");
    }

    // ── Key handling ────────────────────────────────────────────────────

    #[test]
    fn select_sets_wants_continue() {
        let mut state = CompletionState::new(default_summary());
        state.handle_key(Action::Select);
        assert!(state.wants_continue);
        assert!(!state.should_quit);
        assert!(!state.wants_back);
    }

    #[test]
    fn back_sets_wants_back() {
        let mut state = CompletionState::new(default_summary());
        state.handle_key(Action::Back);
        assert!(state.wants_back);
        assert!(!state.should_quit);
        assert!(!state.wants_continue);
    }

    #[test]
    fn quit_sets_should_quit() {
        let mut state = CompletionState::new(default_summary());
        state.handle_key(Action::Quit);
        assert!(state.should_quit);
        assert!(!state.wants_back);
        assert!(!state.wants_continue);
    }

    #[test]
    fn other_keys_are_noop() {
        let mut state = CompletionState::new(default_summary());
        state.handle_key(Action::Up);
        state.handle_key(Action::Down);
        state.handle_key(Action::Toggle);
        state.handle_key(Action::Character('x'));
        assert!(!state.should_quit);
        assert!(!state.wants_back);
        assert!(!state.wants_continue);
    }

    // ── should_quit includes wants_continue ─────────────────────────────

    #[test]
    fn should_quit_true_when_wants_continue() {
        let mut state = CompletionState::new(default_summary());
        assert!(!Surface::should_quit(&state));
        state.handle_key(Action::Select);
        assert!(Surface::should_quit(&state));
    }

    #[test]
    fn should_quit_true_when_quit_pressed() {
        let mut state = CompletionState::new(default_summary());
        state.handle_key(Action::Quit);
        assert!(Surface::should_quit(&state));
    }

    #[test]
    fn should_quit_false_initially() {
        let state = CompletionState::new(default_summary());
        assert!(!Surface::should_quit(&state));
    }

    // ── should_back ─────────────────────────────────────────────────────

    #[test]
    fn should_back_true_when_back_pressed() {
        let mut state = CompletionState::new(default_summary());
        assert!(!Surface::should_back(&state));
        state.handle_key(Action::Back);
        assert!(Surface::should_back(&state));
    }

    // ── reset ───────────────────────────────────────────────────────────

    #[test]
    fn reset_clears_all_flags() {
        let mut state = CompletionState::new(default_summary());
        state.should_quit = true;
        state.wants_back = true;
        state.wants_continue = true;
        Surface::reset(&mut state);
        assert!(!state.should_quit);
        assert!(!state.wants_back);
        assert!(!state.wants_continue);
    }

    #[test]
    fn reset_preserves_summary() {
        let mut state = CompletionState::new(full_summary());
        state.should_quit = true;
        Surface::reset(&mut state);
        assert_eq!(state.summary.findings_count, 7);
        assert!(state.summary.config_created);
    }

    // ── OnboardingSummary default ───────────────────────────────────────

    #[test]
    fn default_summary_is_empty() {
        let s = OnboardingSummary::default();
        assert_eq!(s.findings_count, 0);
        assert!(!s.config_created);
        assert!(!s.hooks_installed);
        assert!(s.hooks_names.is_empty());
        assert!(!s.tutorial_completed);
        assert!(s.tutorial_path.is_none());
    }

    // ── Render smoke tests ──────────────────────────────────────────────

    #[test]
    fn renders_default_summary_without_panic() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let state = CompletionState::new(default_summary());
        let theme = EddaCraftTheme;
        terminal
            .draw(|frame| render(frame, frame.area(), &state, &theme))
            .unwrap();
    }

    #[test]
    fn renders_full_summary_without_panic() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let state = CompletionState::new(full_summary());
        let theme = EddaCraftTheme;
        terminal
            .draw(|frame| render(frame, frame.area(), &state, &theme))
            .unwrap();
    }

    #[test]
    fn renders_in_small_area() {
        let backend = TestBackend::new(40, 12);
        let mut terminal = Terminal::new(backend).unwrap();
        let state = CompletionState::new(full_summary());
        let theme = EddaCraftTheme;
        terminal
            .draw(|frame| render(frame, frame.area(), &state, &theme))
            .unwrap();
    }

    #[test]
    fn renders_config_only_summary() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let state = CompletionState::new(OnboardingSummary {
            config_created: true,
            ..default_summary()
        });
        let theme = EddaCraftTheme;
        terminal
            .draw(|frame| render(frame, frame.area(), &state, &theme))
            .unwrap();
    }

    #[test]
    fn renders_hooks_without_names() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let state = CompletionState::new(OnboardingSummary {
            hooks_installed: true,
            hooks_names: vec![],
            ..default_summary()
        });
        let theme = EddaCraftTheme;
        terminal
            .draw(|frame| render(frame, frame.area(), &state, &theme))
            .unwrap();
    }

    #[test]
    fn renders_single_finding() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let state = CompletionState::new(OnboardingSummary {
            findings_count: 1,
            ..default_summary()
        });
        let theme = EddaCraftTheme;
        terminal
            .draw(|frame| render(frame, frame.area(), &state, &theme))
            .unwrap();
    }

    #[test]
    fn renders_tutorial_without_path_name() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let state = CompletionState::new(OnboardingSummary {
            tutorial_completed: true,
            tutorial_path: None,
            ..default_summary()
        });
        let theme = EddaCraftTheme;
        terminal
            .draw(|frame| render(frame, frame.area(), &state, &theme))
            .unwrap();
    }

    #[test]
    fn renders_many_findings() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let state = CompletionState::new(OnboardingSummary {
            findings_count: 42,
            ..default_summary()
        });
        let theme = EddaCraftTheme;
        terminal
            .draw(|frame| render(frame, frame.area(), &state, &theme))
            .unwrap();
    }
}
