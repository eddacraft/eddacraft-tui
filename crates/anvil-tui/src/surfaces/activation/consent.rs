//! Consent model and renderer for ACTTUI-004.
//!
//! The orchestrator already defers write consent on the TUI path (ACTTUI-002).
//! This module owns the shared TUI chrome that replaces the legacy `demand`
//! pickers: unticked-by-default `Select` rows, gated repo-scoped write
//! suppression, and unsafe-drift confirmation through an overlay.

use eddacraft_tui::prelude::{
    Confirm, ConfirmState, Layer, OverlayStack, Placement, Select, SelectItem, SelectState,
};
use eddacraft_tui::theme::{EddaCraftTheme, Theme};
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, StatefulWidget};

/// Category of write being offered for explicit consent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConsentKind {
    Mcp,
    Workflow,
    Project,
}

impl ConsentKind {
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Mcp => "MCP",
            Self::Workflow => "Workflow",
            Self::Project => "Project",
        }
    }
}

/// Why a consent row is not currently selectable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConsentDisabledReason {
    /// Repo-scoped write suppressed by a gated `ANVIL_HOME` candidate run.
    ProjectWritesGated,
}

impl ConsentDisabledReason {
    #[must_use]
    pub fn label(&self) -> &'static str {
        match self {
            Self::ProjectWritesGated => "disabled — ANVIL_HOME gates project writes",
        }
    }
}

/// One consent offer rendered by the activation surface.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConsentItem {
    pub id: String,
    pub label: String,
    pub description: String,
    pub kind: ConsentKind,
    /// True for repo-scoped writes (workflows, hooks, project seeding).
    pub repo_scoped: bool,
    /// Unsafe drift is never part of the normal multi-select; selecting it opens
    /// a confirm overlay that explains why it cannot be auto-applied.
    pub unsafe_drift: Option<String>,
    pub disabled: Option<ConsentDisabledReason>,
}

impl ConsentItem {
    #[must_use]
    pub fn new(
        id: impl Into<String>,
        label: impl Into<String>,
        description: impl Into<String>,
        kind: ConsentKind,
    ) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            description: description.into(),
            kind,
            repo_scoped: false,
            unsafe_drift: None,
            disabled: None,
        }
    }

    #[must_use]
    pub fn repo_scoped(mut self) -> Self {
        self.repo_scoped = true;
        self
    }

    #[must_use]
    pub fn unsafe_drift(mut self, reason: impl Into<String>) -> Self {
        self.unsafe_drift = Some(reason.into());
        self
    }

    #[must_use]
    pub fn selectable(&self) -> bool {
        self.disabled.is_none() && self.unsafe_drift.is_none()
    }
}

/// Mutable consent state held by the activation surface.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConsentState {
    pub items: Vec<ConsentItem>,
    pub selected_index: usize,
    selected_ids: Vec<String>,
    pub unsafe_confirm_index: Option<usize>,
    pub unsafe_confirmed: Option<bool>,
    submitted: bool,
}

impl ConsentState {
    /// Construct consent state with **nothing selected** (CIB-165).
    #[must_use]
    pub fn new(mut items: Vec<ConsentItem>, project_writes_gated: bool) -> Self {
        if project_writes_gated {
            for item in &mut items {
                if item.repo_scoped && item.disabled.is_none() {
                    item.disabled = Some(ConsentDisabledReason::ProjectWritesGated);
                }
            }
        }
        Self {
            items,
            selected_index: 0,
            selected_ids: Vec::new(),
            unsafe_confirm_index: None,
            unsafe_confirmed: None,
            submitted: false,
        }
    }

    #[must_use]
    pub fn selected_ids(&self) -> &[String] {
        &self.selected_ids
    }

    #[must_use]
    pub fn current(&self) -> Option<&ConsentItem> {
        self.items.get(self.selected_index)
    }

    pub fn next(&mut self) {
        if !self.items.is_empty() {
            self.selected_index = (self.selected_index + 1) % self.items.len();
        }
    }

    pub fn previous(&mut self) {
        if !self.items.is_empty() {
            self.selected_index = self
                .selected_index
                .checked_sub(1)
                .unwrap_or(self.items.len() - 1);
        }
    }

    pub fn toggle_current(&mut self) {
        let Some(item) = self.current() else {
            return;
        };
        if !item.selectable() {
            return;
        }
        if let Some(pos) = self.selected_ids.iter().position(|id| id == &item.id) {
            self.selected_ids.remove(pos);
        } else {
            self.selected_ids.push(item.id.clone());
        }
    }

    pub fn select_current(&mut self) {
        let Some(item) = self.current() else {
            return;
        };
        if item.unsafe_drift.is_some() {
            self.unsafe_confirm_index = Some(self.selected_index);
            self.unsafe_confirmed = None;
        } else {
            self.toggle_current();
        }
    }

    pub fn confirm_unsafe(&mut self, confirmed: bool) {
        self.unsafe_confirmed = Some(confirmed);
        self.unsafe_confirm_index = None;
    }

    #[must_use]
    pub fn is_selected(&self, id: &str) -> bool {
        self.selected_ids.iter().any(|selected| selected == id)
    }

    pub fn submit(&mut self) {
        self.submitted = true;
    }

    #[must_use]
    pub fn submitted(&self) -> bool {
        self.submitted
    }
}

pub fn render(frame: &mut Frame, area: Rect, state: &ConsentState, theme: &EddaCraftTheme) {
    let chunks = Layout::vertical([Constraint::Min(5), Constraint::Length(3)]).split(area);
    render_select(frame, chunks[0], state, theme);
    render_hint(frame, chunks[1], state, theme);

    if let Some(index) = state.unsafe_confirm_index
        && let Some(item) = state.items.get(index)
    {
        render_unsafe_overlay(frame, area, item, theme);
    }
}

fn render_select(frame: &mut Frame, area: Rect, state: &ConsentState, theme: &EddaCraftTheme) {
    let items: Vec<SelectItem> = state
        .items
        .iter()
        .map(|item| {
            let prefix = if item.selectable() {
                if state.is_selected(&item.id) {
                    "[x]"
                } else {
                    "[ ]"
                }
            } else if item.unsafe_drift.is_some() {
                "[!]"
            } else {
                "[-]"
            };
            let desc = item.disabled.as_ref().map_or_else(
                || {
                    item.unsafe_drift
                        .clone()
                        .unwrap_or_else(|| item.description.clone())
                },
                |reason| reason.label().to_string(),
            );
            SelectItem::new(
                format!("{prefix} {}", item.label),
                format!("{} — {desc}", item.kind.label()),
            )
        })
        .collect();
    let mut select_state = SelectState {
        selected: state.selected_index,
        offset: 0,
    };
    Select::new(items, theme)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Consent ")
                .border_style(Style::default().fg(theme.accent())),
        )
        .render(area, frame.buffer_mut(), &mut select_state);
}

fn render_hint(frame: &mut Frame, area: Rect, state: &ConsentState, theme: &EddaCraftTheme) {
    let selected = state.selected_ids().len();
    let line = Line::from(vec![
        Span::styled(
            format!("{selected} selected"),
            Style::default()
                .fg(theme.accent())
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
        Span::styled(
            "space toggle  enter toggle/inspect  a apply  esc/q cancel",
            theme.disabled(),
        ),
    ]);
    frame.render_widget(Paragraph::new(line), area);
}

fn render_unsafe_overlay(
    frame: &mut Frame,
    area: Rect,
    item: &ConsentItem,
    theme: &EddaCraftTheme,
) {
    let reason = item
        .unsafe_drift
        .as_deref()
        .unwrap_or("foreign or unrecognised MCP entry");
    let title = item.label.clone();
    OverlayStack::new(theme)
        .push(
            Layer::new(move |frame, area, theme: &EddaCraftTheme| {
                let block = Block::default()
                    .borders(Borders::ALL)
                    .title(" Unsafe drift ")
                    .border_style(Style::default().fg(theme.warning()));
                let chunks = Layout::vertical([Constraint::Length(2), Constraint::Length(1)])
                    .split(block.inner(area));
                frame.render_widget(block, area);
                frame.render_widget(
                    Paragraph::new(format!(
                        "{title}: {reason}\nOpen the editor config manually before overwriting."
                    ))
                    .wrap(ratatui::widgets::Wrap { trim: false }),
                    chunks[0],
                );
                let mut confirm_state = ConfirmState {
                    selected: false,
                    ..Default::default()
                };
                Confirm::new("Acknowledge unsafe drift?", theme).render(
                    chunks[1],
                    frame.buffer_mut(),
                    &mut confirm_state,
                );
            })
            .placement(Placement::CenterPercent {
                width: 70,
                height: 30,
            })
            .scrim(true),
        )
        .render_to_frame(frame, area);
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    fn sample_items() -> Vec<ConsentItem> {
        vec![
            ConsentItem::new(
                "cursor",
                "Cursor MCP",
                "write ~/.cursor/mcp.json",
                ConsentKind::Mcp,
            ),
            ConsentItem::new(
                "workflow",
                "GitHub workflow",
                "write .github/workflows/anvil.yml",
                ConsentKind::Workflow,
            )
            .repo_scoped(),
            ConsentItem::new(
                "unsafe",
                "Claude Code MCP",
                "foreign config",
                ConsentKind::Mcp,
            )
            .unsafe_drift("existing foreign mcpServers.anvil entry"),
        ]
    }

    #[test]
    fn consent_defaults_to_unticked_items() {
        let state = ConsentState::new(sample_items(), false);
        assert!(state.selected_ids().is_empty());
        assert!(state.items[0].selectable());
        assert!(state.items[1].selectable());
        assert!(!state.items[2].selectable());
    }

    #[test]
    fn toggles_only_selectable_items() {
        let mut state = ConsentState::new(sample_items(), false);
        state.toggle_current();
        assert_eq!(state.selected_ids(), ["cursor"]);
        state.next();
        state.next();
        state.toggle_current();
        assert_eq!(state.selected_ids(), ["cursor"]);
    }

    #[test]
    fn gated_project_writes_disable_repo_scoped_items() {
        let state = ConsentState::new(sample_items(), true);
        assert!(state.items[0].selectable());
        assert!(matches!(
            state.items[1].disabled,
            Some(ConsentDisabledReason::ProjectWritesGated)
        ));
        assert!(!state.items[1].selectable());
    }

    #[test]
    fn unsafe_select_opens_confirm_overlay_without_selecting() {
        let mut state = ConsentState::new(sample_items(), false);
        state.next();
        state.next();
        state.select_current();
        assert_eq!(state.unsafe_confirm_index, Some(2));
        assert!(state.selected_ids().is_empty());
        state.confirm_unsafe(false);
        assert_eq!(state.unsafe_confirmed, Some(false));
        assert_eq!(state.unsafe_confirm_index, None);
    }

    #[test]
    fn render_uses_shared_select_and_overlay_copy() {
        let mut state = ConsentState::new(sample_items(), true);
        state.next();
        state.next();
        state.select_current();
        let backend = TestBackend::new(100, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let theme = EddaCraftTheme;
        terminal
            .draw(|frame| render(frame, frame.area(), &state, &theme))
            .unwrap();
        let buf = terminal.backend().buffer();
        let area = buf.area;
        let mut out = String::new();
        for y in area.y..area.y + area.height {
            for x in area.x..area.x + area.width {
                out.push_str(buf[(x, y)].symbol());
            }
            out.push('\n');
        }
        assert!(out.contains("Consent"));
        assert!(out.contains("Cursor MCP"));
        assert!(out.contains("ANVIL_HOME gates project writes"));
        assert!(out.contains("Unsafe drift"));
    }
}
