//! Dashboard list surface with live previews (TUIDASH-012).
//!
//! A two-pane picker: a list of dashboards on the left, and a mini-preview of
//! the highlighted one on the right rendered through the json-render engine.
//! Arrow keys browse; Enter records the choice (the CLI then opens it
//! full-screen); `q`/`esc` quit. Native dashboards have no spec to preview, so
//! their pane shows a short description card instead.

use eddacraft_tui::keyboard::Action;
use eddacraft_tui::theme::{EddaCraftTheme, Theme};
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, BorderType, Borders, Paragraph, Wrap};

use super::spec::SpecDashboardState;
use crate::surface::Surface;

/// One row in the dashboard list.
pub struct ListEntry {
    /// Machine name (recorded as the choice).
    pub name: String,
    /// Human title shown in the list and preview header.
    pub title: String,
    /// One-line description shown when there is no spec preview.
    pub description: String,
    /// A bound spec surface used to render the preview pane. `None` for native
    /// dashboards, which have no json-render spec.
    pub preview: Option<SpecDashboardState>,
}

impl ListEntry {
    /// A native dashboard entry (no spec preview).
    #[must_use]
    pub fn native(
        name: impl Into<String>,
        title: impl Into<String>,
        description: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            title: title.into(),
            description: description.into(),
            preview: None,
        }
    }

    /// A saved-spec dashboard entry whose preview renders `surface`.
    #[must_use]
    pub fn spec(
        name: impl Into<String>,
        title: impl Into<String>,
        surface: SpecDashboardState,
    ) -> Self {
        Self {
            name: name.into(),
            title: title.into(),
            description: String::new(),
            preview: Some(surface),
        }
    }
}

/// Two-pane dashboard list with previews.
pub struct DashboardListState {
    entries: Vec<ListEntry>,
    selected: usize,
    should_quit: bool,
    /// Machine name the user chose with Enter, inspected by the CLI after exit.
    pub chosen: Option<String>,
}

impl DashboardListState {
    /// Create a list over `entries`.
    #[must_use]
    pub fn new(entries: Vec<ListEntry>) -> Self {
        Self {
            entries,
            selected: 0,
            should_quit: false,
            chosen: None,
        }
    }

    /// The highlighted entry, if any.
    #[must_use]
    pub fn selected_entry(&self) -> Option<&ListEntry> {
        self.entries.get(self.selected)
    }

    // Clamp at the ends rather than wrapping — a short fixed list reads more
    // predictably stopping at the boundary.
    fn move_up(&mut self) {
        self.selected = self.selected.saturating_sub(1);
    }

    fn move_down(&mut self) {
        if self.selected + 1 < self.entries.len() {
            self.selected += 1;
        }
    }

    fn choose(&mut self) {
        if let Some(entry) = self.entries.get(self.selected) {
            self.chosen = Some(entry.name.clone());
            self.should_quit = true;
        }
    }

    fn render_list(&self, frame: &mut Frame, area: Rect, theme: &EddaCraftTheme) {
        let lines: Vec<Line> = self
            .entries
            .iter()
            .enumerate()
            .map(|(i, entry)| {
                if i == self.selected {
                    Line::styled(
                        format!("▶ {}", entry.title),
                        Style::default()
                            .fg(theme.accent())
                            .add_modifier(Modifier::BOLD),
                    )
                } else {
                    Line::styled(format!("  {}", entry.title), theme.base())
                }
            })
            .collect();
        frame.render_widget(Paragraph::new(lines), area);
    }

    fn render_preview(&self, frame: &mut Frame, area: Rect, theme: &EddaCraftTheme) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.border()));
        let inner = block.inner(area);
        frame.render_widget(block, area);
        if inner.width == 0 || inner.height == 0 {
            return;
        }
        if let Some(entry) = self.selected_entry() {
            match &entry.preview {
                // Render the spec preview through the engine.
                Some(preview) => preview.render(frame, inner, theme),
                // Native dashboard: no spec, show a description card.
                None => frame.render_widget(
                    Paragraph::new(entry.description.clone())
                        .style(theme.base())
                        .wrap(Wrap { trim: true }),
                    inner,
                ),
            }
        }
    }
}

impl Surface for DashboardListState {
    fn surface_name(&self) -> &'static str {
        "Dashboards"
    }

    fn help_text(&self) -> &'static str {
        "↑/↓ browse  enter open  esc/q quit"
    }

    fn handle_key(&mut self, action: Action) {
        match action {
            Action::Up => self.move_up(),
            Action::Down => self.move_down(),
            Action::Select => self.choose(),
            Action::Back | Action::Quit => self.should_quit = true,
            _ => {}
        }
    }

    fn should_quit(&self) -> bool {
        self.should_quit
    }

    fn render(&self, frame: &mut Frame, area: Rect, theme: &EddaCraftTheme) {
        // Left list ~40%, preview pane ~60%.
        let cols = Layout::horizontal([Constraint::Percentage(40), Constraint::Percentage(60)])
            .split(area);
        self.render_list(frame, cols[0], theme);
        self.render_preview(frame, cols[1], theme);
    }
}

#[cfg(test)]
mod tests {
    use eddacraft_tui::json_render::parse;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    use super::*;

    fn entries() -> Vec<ListEntry> {
        let tmp = tempfile::tempdir().expect("tempdir");
        let spec = parse(
            r#"{ "title": "Gate", "version": "1.0", "root": "h",
                 "elements": { "h": { "type": "Heading",
                     "props": { "children": "PREVIEW-HEADING" }, "children": [] } } }"#,
        )
        .expect("parse");
        vec![
            ListEntry::native(
                "architecture",
                "Architecture Health",
                "layers and violations",
            ),
            ListEntry::spec(
                "gate",
                "Gate",
                SpecDashboardState::new(spec, tmp.path().to_path_buf()),
            ),
        ]
    }

    #[test]
    fn navigation_clamps_and_select_records_choice() {
        let mut state = DashboardListState::new(entries());
        assert_eq!(state.selected, 0);
        state.handle_key(Action::Up); // clamp at top
        assert_eq!(state.selected, 0);
        state.handle_key(Action::Down);
        assert_eq!(state.selected, 1);
        state.handle_key(Action::Down); // clamp at bottom
        assert_eq!(state.selected, 1);
        state.handle_key(Action::Select);
        assert_eq!(state.chosen.as_deref(), Some("gate"));
        assert!(state.should_quit());
    }

    #[test]
    fn preview_pane_renders_the_selected_spec() {
        let theme = EddaCraftTheme;
        let mut state = DashboardListState::new(entries());
        state.handle_key(Action::Down); // select the spec entry
        let mut terminal = Terminal::new(TestBackend::new(80, 12)).expect("backend");
        terminal
            .draw(|frame| state.render(frame, frame.area(), &theme))
            .expect("draw");
        let text: String = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(ratatui::buffer::Cell::symbol)
            .collect();
        // The preview pane renders the spec's heading; the list shows both titles.
        assert!(
            text.contains("PREVIEW-HEADING"),
            "spec preview rendered: {text:?}"
        );
        assert!(text.contains("Architecture Health") && text.contains("Gate"));
    }

    #[test]
    fn native_entry_preview_shows_description_without_panic() {
        let theme = EddaCraftTheme;
        let state = DashboardListState::new(entries()); // selected = native
        let mut terminal = Terminal::new(TestBackend::new(80, 12)).expect("backend");
        terminal
            .draw(|frame| state.render(frame, frame.area(), &theme))
            .expect("draw");
        let text: String = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(ratatui::buffer::Cell::symbol)
            .collect();
        assert!(
            text.contains("layers and violations"),
            "native description: {text:?}"
        );
    }
}
