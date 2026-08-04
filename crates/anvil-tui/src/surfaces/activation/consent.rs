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
///
/// CIB-245: the kind also selects the consent **step**, so the flat multi-select
/// becomes Project → Hooks → Workflows → MCP clients.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ConsentKind {
    Project,
    Hooks,
    Workflow,
    Mcp,
}

impl ConsentKind {
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Mcp => "MCP",
            Self::Workflow => "Workflow",
            Self::Project => "Project",
            Self::Hooks => "Hooks",
        }
    }

    /// Section heading shown as the step title (CIB-245), mirroring the way the
    /// verdict groups evidence by section.
    #[must_use]
    pub fn section_title(self) -> &'static str {
        match self {
            Self::Project => "Project",
            Self::Hooks => "Hooks / git",
            Self::Workflow => "Workflows",
            Self::Mcp => "MCP clients",
        }
    }

    /// One-line framing for the whole step, above the rows.
    #[must_use]
    pub fn section_summary(self) -> &'static str {
        match self {
            Self::Project => "Files anvil wants to add to this repository.",
            Self::Hooks => "Changes to what git does when you commit or push.",
            Self::Workflow => "Jobs anvil wants to run in your CI provider.",
            Self::Mcp => "Editors and agent CLIs that should be able to call anvil.",
        }
    }

    /// Section order used to build steps.
    const ORDER: [Self; 4] = [Self::Project, Self::Hooks, Self::Workflow, Self::Mcp];
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
    /// CIB-245: plain-language "what is this" owned by the offer builder. Empty
    /// only for callers that predate the blurb (tests and legacy fixtures);
    /// rendering falls back to `description` so a row is never blank.
    pub blurb: String,
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
            blurb: String::new(),
            kind,
            repo_scoped: false,
            unsafe_drift: None,
            disabled: None,
        }
    }

    /// CIB-245: attach the plain-language explanation for this row.
    #[must_use]
    pub fn blurb(mut self, blurb: impl Into<String>) -> Self {
        self.blurb = blurb.into();
        self
    }

    /// What the row should say it *is*, falling back to the path/action detail
    /// when no blurb was supplied.
    #[must_use]
    pub fn explanation(&self) -> &str {
        if self.blurb.is_empty() {
            &self.description
        } else {
            &self.blurb
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
///
/// CIB-245: rows are grouped into ordered steps by [`ConsentKind`]. `items`
/// stays a single flat, section-sorted list so selections are global and
/// survive stepping back and forth; navigation is scoped to the current step.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConsentState {
    pub items: Vec<ConsentItem>,
    pub selected_index: usize,
    selected_ids: Vec<String>,
    /// Sections present in this run, in [`ConsentKind::ORDER`].
    steps: Vec<ConsentKind>,
    step_index: usize,
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
        // Stable sort: section order across kinds, construction order within.
        items.sort_by_key(|item| item.kind);
        let steps = ConsentKind::ORDER
            .into_iter()
            .filter(|kind| items.iter().any(|item| item.kind == *kind))
            .collect();
        Self {
            items,
            selected_index: 0,
            selected_ids: Vec::new(),
            steps,
            step_index: 0,
            unsafe_confirm_index: None,
            unsafe_confirmed: None,
            submitted: false,
        }
    }

    #[must_use]
    pub fn selected_ids(&self) -> &[String] {
        &self.selected_ids
    }

    /// Sections present in this run, in display order.
    #[must_use]
    pub fn steps(&self) -> &[ConsentKind] {
        &self.steps
    }

    /// The section currently on screen.
    #[must_use]
    pub fn current_step(&self) -> Option<ConsentKind> {
        self.steps.get(self.step_index).copied()
    }

    /// 1-based `(position, total)` progress cue for the step chrome.
    #[must_use]
    pub fn step_position(&self) -> (usize, usize) {
        (self.step_index + 1, self.steps.len())
    }

    /// Half-open `items` range covered by the current step.
    #[must_use]
    fn step_range(&self) -> (usize, usize) {
        let Some(kind) = self.current_step() else {
            return (0, 0);
        };
        let start = self
            .items
            .iter()
            .position(|item| item.kind == kind)
            .unwrap_or(0);
        let len = self.items.iter().filter(|item| item.kind == kind).count();
        (start, start + len)
    }

    /// Rows belonging to the current step.
    #[must_use]
    pub fn step_items(&self) -> &[ConsentItem] {
        let (start, end) = self.step_range();
        &self.items[start..end]
    }

    /// Index of the cursor within the current step's rows.
    #[must_use]
    pub fn step_cursor(&self) -> usize {
        let (start, _) = self.step_range();
        self.selected_index.saturating_sub(start)
    }

    /// Advance to the next section, keeping every selection made so far.
    pub fn next_step(&mut self) {
        if self.steps.len() < 2 {
            return;
        }
        self.step_index = (self.step_index + 1) % self.steps.len();
        self.selected_index = self.step_range().0;
    }

    /// Go back to the previous section, keeping every selection made so far.
    pub fn previous_step(&mut self) {
        if self.steps.len() < 2 {
            return;
        }
        self.step_index = self
            .step_index
            .checked_sub(1)
            .unwrap_or(self.steps.len() - 1);
        self.selected_index = self.step_range().0;
    }

    #[must_use]
    pub fn current(&self) -> Option<&ConsentItem> {
        self.items.get(self.selected_index)
    }

    pub fn next(&mut self) {
        let (start, end) = self.step_range();
        if end > start {
            self.selected_index = start + (self.selected_index + 1 - start) % (end - start);
        }
    }

    pub fn previous(&mut self) {
        let (start, end) = self.step_range();
        if end > start {
            self.selected_index = if self.selected_index > start {
                self.selected_index - 1
            } else {
                end - 1
            };
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
    // CIB-245: only the current section's rows are on screen, and each row
    // leads with the plain-language blurb — the path is the secondary detail,
    // not the whole explanation.
    let items: Vec<SelectItem> = state
        .step_items()
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
                    item.unsafe_drift.clone().unwrap_or_else(|| {
                        let explanation = item.explanation();
                        if explanation == item.description {
                            explanation.to_string()
                        } else {
                            format!("{explanation} ({})", item.description)
                        }
                    })
                },
                |reason| reason.label().to_string(),
            );
            SelectItem::new(format!("{prefix} {}", item.label), desc)
        })
        .collect();
    let mut select_state = SelectState {
        selected: state.step_cursor(),
        offset: 0,
    };
    let title = state.current_step().map_or_else(
        || " Consent ".to_string(),
        |kind| {
            let (position, total) = state.step_position();
            format!(" Consent — {} ({position}/{total}) ", kind.section_title())
        },
    );
    Select::new(items, theme)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(title)
                .border_style(Style::default().fg(theme.accent())),
        )
        .render(area, frame.buffer_mut(), &mut select_state);
}

fn render_hint(frame: &mut Frame, area: Rect, state: &ConsentState, theme: &EddaCraftTheme) {
    // ACTTUI-015: key legends live only on the shell HelpBar (`help_text`), not
    // a second footer here — overlapping chrome garble on small terminals.
    // CIB-245: the section summary frames what this step is asking for, and
    // the count is stated across all steps so stepping never hides selections.
    let selected = state.selected_ids().len();
    let mut lines = Vec::new();
    if let Some(kind) = state.current_step() {
        lines.push(Line::from(Span::raw(kind.section_summary().to_string())));
    }
    lines.push(Line::from(vec![Span::styled(
        format!("{selected} selected in total"),
        Style::default()
            .fg(theme.accent())
            .add_modifier(Modifier::BOLD),
    )]));
    frame.render_widget(Paragraph::new(lines), area);
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

    /// Deliberately constructed out of section order (MCP, Workflow, MCP) so
    /// the grouping in `ConsentState::new` is exercised, not assumed.
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
            .blurb("Runs anvil's checks on every pull request.")
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

    fn render_to_string(state: &ConsentState) -> String {
        let backend = TestBackend::new(100, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let theme = EddaCraftTheme;
        terminal
            .draw(|frame| render(frame, frame.area(), state, &theme))
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
        out
    }

    #[test]
    fn consent_defaults_to_unticked_items() {
        let state = ConsentState::new(sample_items(), false);
        assert!(state.selected_ids().is_empty());
        // Section-sorted: Workflow, then the two MCP rows.
        assert_eq!(state.items[0].id, "workflow");
        assert!(state.items[0].selectable());
        assert!(state.items[1].selectable());
        assert!(!state.items[2].selectable());
    }

    /// CIB-245: rows group into ordered sections, one step per section.
    #[test]
    fn items_group_into_ordered_steps() {
        let state = ConsentState::new(sample_items(), false);
        assert_eq!(state.steps(), [ConsentKind::Workflow, ConsentKind::Mcp]);
        assert_eq!(state.current_step(), Some(ConsentKind::Workflow));
        assert_eq!(state.step_position(), (1, 2));
        assert_eq!(state.step_items().len(), 1);
    }

    /// CIB-245: stepping to the MCP screen retains earlier selections, and
    /// stepping back finds them still ticked.
    #[test]
    fn stepping_retains_prior_selections() {
        let mut state = ConsentState::new(sample_items(), false);
        state.toggle_current();
        assert_eq!(state.selected_ids(), ["workflow"]);

        state.next_step();
        assert_eq!(state.current_step(), Some(ConsentKind::Mcp));
        assert_eq!(state.step_position(), (2, 2));
        assert_eq!(state.step_items().len(), 2);
        state.toggle_current();
        assert_eq!(state.selected_ids(), ["workflow", "cursor"]);

        state.previous_step();
        assert_eq!(state.current_step(), Some(ConsentKind::Workflow));
        assert!(state.is_selected("workflow"));
        assert!(state.is_selected("cursor"));
    }

    /// CIB-245: `next`/`previous` wrap inside the current step so the cursor
    /// can never land on a row the operator cannot see.
    #[test]
    fn navigation_is_scoped_to_the_current_step() {
        let mut state = ConsentState::new(sample_items(), false);
        state.next();
        assert_eq!(state.current().unwrap().id, "workflow");
        state.previous();
        assert_eq!(state.current().unwrap().id, "workflow");

        state.next_step();
        assert_eq!(state.current().unwrap().id, "cursor");
        state.next();
        assert_eq!(state.current().unwrap().id, "unsafe");
        state.next();
        assert_eq!(state.current().unwrap().id, "cursor");
    }

    /// CIB-245: a run with a single section still works and shows no stepping.
    #[test]
    fn single_section_run_has_one_step() {
        let mut state = ConsentState::new(
            vec![ConsentItem::new(
                "cursor",
                "Cursor MCP",
                "write ~/.cursor/mcp.json",
                ConsentKind::Mcp,
            )],
            false,
        );
        assert_eq!(state.steps(), [ConsentKind::Mcp]);
        state.next_step();
        assert_eq!(state.current_step(), Some(ConsentKind::Mcp));
        assert_eq!(state.step_position(), (1, 1));
    }

    /// CIB-245: a blurb explains the row; without one the path detail stands in
    /// so a row is never blank.
    #[test]
    fn explanation_prefers_blurb_and_falls_back_to_description() {
        let state = ConsentState::new(sample_items(), false);
        assert_eq!(
            state.items[0].explanation(),
            "Runs anvil's checks on every pull request."
        );
        assert_eq!(state.items[1].explanation(), "write ~/.cursor/mcp.json");
    }

    #[test]
    fn toggles_only_selectable_items() {
        let mut state = ConsentState::new(sample_items(), false);
        state.next_step();
        state.toggle_current();
        assert_eq!(state.selected_ids(), ["cursor"]);
        state.next();
        state.toggle_current();
        assert_eq!(state.selected_ids(), ["cursor"]);
    }

    #[test]
    fn gated_project_writes_disable_repo_scoped_items() {
        let state = ConsentState::new(sample_items(), true);
        // The repo-scoped workflow row now sorts first.
        assert!(matches!(
            state.items[0].disabled,
            Some(ConsentDisabledReason::ProjectWritesGated)
        ));
        assert!(!state.items[0].selectable());
        assert!(state.items[1].selectable());
    }

    #[test]
    fn unsafe_select_opens_confirm_overlay_without_selecting() {
        let mut state = ConsentState::new(sample_items(), false);
        state.next_step();
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
        state.next_step();
        state.next();
        state.select_current();
        let out = render_to_string(&state);
        assert!(out.contains("Consent"));
        assert!(out.contains("Cursor MCP"));
        assert!(out.contains("Unsafe drift"));
    }

    /// CIB-245: the section title, progress cue, and framing line are on
    /// screen, and only the current section's rows are.
    #[test]
    fn render_shows_section_chrome_and_scopes_rows() {
        let state = ConsentState::new(sample_items(), false);
        let out = render_to_string(&state);
        assert!(out.contains("Consent — Workflows (1/2)"), "{out}");
        assert!(out.contains("Jobs anvil wants to run in your CI provider."));
        assert!(out.contains("Runs anvil's checks on every pull request."));
        assert!(out.contains("GitHub workflow"));
        // MCP rows belong to the next step, not this screen.
        assert!(!out.contains("Cursor MCP"), "{out}");
    }

    #[test]
    fn render_shows_gated_reason_on_the_section_that_owns_the_row() {
        let state = ConsentState::new(sample_items(), true);
        let out = render_to_string(&state);
        assert!(out.contains("ANVIL_HOME gates project writes"), "{out}");
    }
}
