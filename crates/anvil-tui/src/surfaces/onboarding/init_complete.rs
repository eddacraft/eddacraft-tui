//! Post-init landing screen — the "you're set up, here's what happens next"
//! moment between guided init and the discovery / tutorial flow.
//!
//! POLISH-008 item 2: before this, guided init would write config and jump
//! straight into the discovery scan, leaving the user unsure what had just
//! happened to their project. This surface reports what was written and
//! previews the next step so the transition doesn't feel abrupt.

use eddacraft_tui::keyboard::Action;
use eddacraft_tui::theme::{EddaCraftTheme, Theme};
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Padding, Paragraph, Wrap};
use unicode_width::UnicodeWidthStr;

use crate::shell::inset_content;
use crate::surface::Surface;

/// Summary of what guided init wrote to disk. Fed directly to the landing
/// screen so the copy reflects the user's actual project state rather than
/// hardcoded strings.
#[derive(Debug, Clone)]
pub struct InitCompleteSummary {
    /// Filename of the config file written (e.g. `.anvilrc`).
    pub config_path: String,
    /// Planning directory created or confirmed (e.g. `plans/`).
    pub plans_dir: String,
    /// Cache directory created (e.g. `.anvil/cache/`).
    pub cache_dir: String,
    /// Whether we appended a new entry to the user's `.gitignore`.
    pub gitignore_updated: bool,
    /// Names of checks the user enabled during guided init.
    pub checks_enabled: Vec<String>,
}

impl Default for InitCompleteSummary {
    fn default() -> Self {
        Self {
            config_path: ".anvilrc".to_string(),
            plans_dir: "plans/".to_string(),
            cache_dir: ".anvil/cache/".to_string(),
            gitignore_updated: true,
            checks_enabled: Vec::new(),
        }
    }
}

pub struct InitCompleteState {
    pub summary: InitCompleteSummary,
    pub should_quit: bool,
    pub wants_continue: bool,
}

impl InitCompleteState {
    #[must_use]
    pub fn new(summary: InitCompleteSummary) -> Self {
        Self {
            summary,
            should_quit: false,
            wants_continue: false,
        }
    }
}

impl Surface for InitCompleteState {
    fn surface_name(&self) -> &'static str {
        "Setup"
    }

    fn help_text(&self) -> &'static str {
        "enter continue  q quit"
    }

    fn handle_key(&mut self, action: Action) {
        match action {
            Action::Select => self.wants_continue = true,
            Action::Quit | Action::Back => self.should_quit = true,
            _ => {}
        }
    }

    fn should_quit(&self) -> bool {
        self.should_quit || self.wants_continue
    }

    fn reset(&mut self) {
        self.should_quit = false;
        self.wants_continue = false;
    }

    fn render(&self, frame: &mut Frame, area: Rect, theme: &EddaCraftTheme) {
        render(frame, area, self, theme);
    }
}

// ---------------------------------------------------------------------------
// Rendering
// ---------------------------------------------------------------------------

fn render(frame: &mut Frame, area: Rect, state: &InitCompleteState, theme: &EddaCraftTheme) {
    let area = inset_content(area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.success()))
        .padding(Padding::new(2, 2, 1, 1))
        .title(" anvil is ready ");
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let s = &state.summary;

    // Determine the longest path so the descriptions align. We use terminal
    // cell width (via unicode-width) rather than char count so CJK glyphs
    // and emoji-wide paths line up with the description column instead of
    // bleeding into it. Include `.gitignore` only when it was actually
    // mutated so an unchanged ignore file never claims space in the blast
    // radius list (CIB-263). Computed without a temporary Vec — render runs
    // every frame.
    let mut widest = UnicodeWidthStr::width(s.config_path.as_str())
        .max(UnicodeWidthStr::width(s.plans_dir.as_str()))
        .max(UnicodeWidthStr::width(s.cache_dir.as_str()));
    if s.gitignore_updated {
        widest = widest.max(UnicodeWidthStr::width(".gitignore"));
    }
    let gap = widest + 2;

    let mut lines: Vec<Line> = vec![
        Line::from(Span::styled(
            "\u{2713}  anvil is ready",
            Style::default()
                .fg(theme.success())
                .add_modifier(Modifier::BOLD),
        )),
        Line::default(),
        Line::from(Span::styled(
            "We wrote these to your project:",
            Style::default().fg(theme.fg()),
        )),
        Line::default(),
        path_line(&s.config_path, "configuration", gap, theme),
        path_line(
            &s.plans_dir,
            "where your architecture plans live",
            gap,
            theme,
        ),
        path_line(&s.cache_dir, "local cache", gap, theme),
    ];
    // CIB-263: list `.gitignore` as its own path row when ignore rules were
    // appended. The cache description stays plain "local cache" so we do not
    // double-announce the gitignore change on the cache line.
    if s.gitignore_updated {
        lines.push(path_line(
            ".gitignore",
            "appended ignore entries",
            gap,
            theme,
        ));
    }

    if !s.checks_enabled.is_empty() {
        lines.push(Line::default());
        lines.push(Line::from(vec![
            Span::styled("Enabled checks: ", Style::default().fg(theme.muted())),
            Span::styled(s.checks_enabled.join(", "), Style::default().fg(theme.fg())),
        ]));
    }

    lines.push(Line::default());
    lines.push(Line::from(Span::styled(
        "Next: a quick scan for issues in your code, then a short tutorial on how anvil works. Takes about 5 minutes.",
        Style::default().fg(theme.fg()),
    )));
    lines.push(Line::default());
    lines.push(Line::from(vec![
        Span::styled(
            "[enter] ",
            Style::default()
                .fg(theme.accent())
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("continue    ", Style::default().fg(theme.fg())),
        Span::styled(
            "[q] ",
            Style::default()
                .fg(theme.accent())
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("quit", Style::default().fg(theme.fg())),
    ]));

    frame.render_widget(
        Paragraph::new(Text::from(lines)).wrap(Wrap { trim: false }),
        inner,
    );
}

fn path_line<'a>(
    path: &'a str,
    description: &'a str,
    gap: usize,
    theme: &EddaCraftTheme,
) -> Line<'a> {
    let pad = gap.saturating_sub(UnicodeWidthStr::width(path));
    let padding: String = " ".repeat(pad);
    Line::from(vec![
        Span::styled("  ", Style::default()),
        Span::styled(
            path,
            Style::default()
                .fg(theme.accent())
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(padding, Style::default()),
        Span::styled(description, Style::default().fg(theme.muted())),
    ])
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    /// Plain-text (style-stripped) buffer dump with row separators for
    /// substring assertions that must not span line boundaries.
    fn plain(buf: &ratatui::buffer::Buffer) -> String {
        let area = buf.area;
        let mut s = String::new();
        for y in area.y..area.y + area.height {
            for x in area.x..area.x + area.width {
                s.push_str(buf[(x, y)].symbol());
            }
            s.push('\n');
        }
        s
    }

    fn summary_with_checks() -> InitCompleteSummary {
        InitCompleteSummary {
            checks_enabled: vec!["secret-scan".to_string(), "anti-pattern".to_string()],
            ..Default::default()
        }
    }

    #[test]
    fn select_sets_wants_continue() {
        let mut state = InitCompleteState::new(InitCompleteSummary::default());
        state.handle_key(Action::Select);
        assert!(state.wants_continue);
        assert!(Surface::should_quit(&state));
    }

    #[test]
    fn quit_sets_should_quit() {
        let mut state = InitCompleteState::new(InitCompleteSummary::default());
        state.handle_key(Action::Quit);
        assert!(state.should_quit);
        assert!(!state.wants_continue);
    }

    #[test]
    fn back_exits_without_continue() {
        let mut state = InitCompleteState::new(InitCompleteSummary::default());
        state.handle_key(Action::Back);
        assert!(state.should_quit);
        assert!(!state.wants_continue);
    }

    #[test]
    fn reset_clears_flags() {
        let mut state = InitCompleteState::new(InitCompleteSummary::default());
        state.should_quit = true;
        state.wants_continue = true;
        Surface::reset(&mut state);
        assert!(!state.should_quit);
        assert!(!state.wants_continue);
    }

    #[test]
    fn default_summary_has_expected_paths() {
        let s = InitCompleteSummary::default();
        assert_eq!(s.config_path, ".anvilrc");
        assert_eq!(s.plans_dir, "plans/");
        assert_eq!(s.cache_dir, ".anvil/cache/");
        assert!(s.gitignore_updated);
        assert!(s.checks_enabled.is_empty());
    }

    #[test]
    fn renders_default_without_panic() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let state = InitCompleteState::new(InitCompleteSummary::default());
        let theme = EddaCraftTheme;
        terminal
            .draw(|frame| render(frame, frame.area(), &state, &theme))
            .unwrap();
    }

    #[test]
    fn renders_with_checks_without_panic() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let state = InitCompleteState::new(summary_with_checks());
        let theme = EddaCraftTheme;
        terminal
            .draw(|frame| render(frame, frame.area(), &state, &theme))
            .unwrap();
    }

    #[test]
    fn renders_without_gitignore_note() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let state = InitCompleteState::new(InitCompleteSummary {
            gitignore_updated: false,
            ..Default::default()
        });
        let theme = EddaCraftTheme;
        terminal
            .draw(|frame| render(frame, frame.area(), &state, &theme))
            .unwrap();

        let rendered = plain(terminal.backend().buffer());
        assert!(
            !rendered.contains(".gitignore"),
            "unchanged gitignore must not appear as a path row (CIB-263): {rendered}"
        );
        assert!(
            rendered.contains("local cache") && !rendered.contains("added to .gitignore"),
            "cache line stays plain when gitignore was not updated: {rendered}"
        );
    }

    // CIB-263: when ignore entries were appended, the landing list names
    // `.gitignore` as its own path — matching the tutorial blast-radius honesty
    // tags and the CLI plain summary.
    #[test]
    fn renders_gitignore_as_own_path_row_when_updated() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let state = InitCompleteState::new(InitCompleteSummary {
            gitignore_updated: true,
            ..Default::default()
        });
        let theme = EddaCraftTheme;
        terminal
            .draw(|frame| render(frame, frame.area(), &state, &theme))
            .unwrap();

        let rendered = plain(terminal.backend().buffer());
        assert!(
            rendered.contains(".gitignore"),
            "updated gitignore must appear as its own path row (CIB-263): {rendered}"
        );
        assert!(
            rendered.contains("appended ignore entries"),
            "gitignore row should describe the ignore-rule update: {rendered}"
        );
        assert!(
            rendered.contains("local cache") && !rendered.contains("added to .gitignore"),
            "cache description must not double-announce gitignore: {rendered}"
        );
    }

    #[test]
    fn renders_in_tiny_terminal() {
        let cases: &[(u16, u16)] = &[(40, 12), (20, 8), (5, 5)];
        for &(w, h) in cases {
            let backend = TestBackend::new(w, h);
            let mut terminal = Terminal::new(backend).unwrap();
            let state = InitCompleteState::new(summary_with_checks());
            let theme = EddaCraftTheme;
            terminal
                .draw(|frame| render(frame, frame.area(), &state, &theme))
                .unwrap_or_else(|e| panic!("panicked at {w}x{h}: {e}"));
        }
    }

    #[test]
    fn snapshot_default_state() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let state = InitCompleteState::new(summary_with_checks());
        let theme = EddaCraftTheme;

        terminal
            .draw(|frame| {
                let content = crate::shell::render_shell(
                    frame,
                    frame.area(),
                    Surface::surface_name(&state),
                    Surface::help_text(&state),
                    &theme,
                );
                render(frame, content, &state, &theme);
            })
            .unwrap();

        let buf = terminal.backend().buffer().clone();
        insta::assert_snapshot!(crate::test_utils::snapshot::buffer_to_string(&buf));
    }
}
