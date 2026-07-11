//! Structured verdict view for the activation TUI (ACTTUI-005).
//!
//! This module keeps the activation state vocabulary honest by rendering the
//! state label supplied by the CLI verbatim. It adds eddacraft-tui widgets around
//! that fixed truth: `StatusBadge` for the headline, `Tree` for collapsible
//! evidence, `Toast` for the smoke-test placeholder, and `HelpBar` for
//! contextual keys. (The `BigBanner` celebration treatment was deferred with
//! the first-run wow expansion; ACTTUI-012 removed the unused wiring.)

use eddacraft_tui::keyboard::{Action, Binding};
use eddacraft_tui::prelude::{
    BadgeStatus, HelpBar, StatusBadge, Toast, ToastPlacement, ToastStack, Tree, TreeNode, TreeState,
};
use eddacraft_tui::theme::{EddaCraftTheme, Theme};
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Paragraph, StatefulWidget, Widget};

/// Presentation tone for the verdict headline. This is styling only; it must
/// never invent or upgrade the fixed protection-state label.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerdictTone {
    Protecting,
    Watching,
    Attention,
    Unsupported,
    Blocked,
}

impl VerdictTone {
    /// Map the fixed protection-state vocabulary word to a tone. Unknown words
    /// deliberately map to attention rather than success.
    #[must_use]
    pub fn from_state_label(label: &str) -> Self {
        match label {
            "protecting" => Self::Protecting,
            "watching" => Self::Watching,
            "unsupported" => Self::Unsupported,
            "error" => Self::Blocked,
            _ => Self::Attention,
        }
    }

    #[must_use]
    fn badge(self) -> BadgeStatus {
        match self {
            Self::Protecting => BadgeStatus::Success,
            Self::Watching => BadgeStatus::Running,
            Self::Attention => BadgeStatus::Warning,
            Self::Unsupported => BadgeStatus::Skipped,
            Self::Blocked => BadgeStatus::Error,
        }
    }

    #[must_use]
    fn auto_expand(self) -> bool {
        !matches!(self, Self::Protecting)
    }
}

/// One collapsible section in the activation verdict tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerdictSection {
    pub id: String,
    pub title: String,
    pub rows: Vec<String>,
}

impl VerdictSection {
    #[must_use]
    pub fn new(id: impl Into<String>, title: impl Into<String>, rows: Vec<String>) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            rows,
        }
    }
}

/// Structured verdict handed to the surface by the CLI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerdictModel {
    pub state_label: String,
    pub headline: String,
    pub sections: Vec<VerdictSection>,
}

impl VerdictModel {
    #[must_use]
    pub fn new(
        state_label: impl Into<String>,
        headline: impl Into<String>,
        sections: Vec<VerdictSection>,
    ) -> Self {
        Self {
            state_label: state_label.into(),
            headline: headline.into(),
            sections,
        }
    }

    /// Conservative fallback for older call sites: parse the plain verdict into
    /// stable sections without changing any user-visible state words.
    #[must_use]
    pub fn from_plain(verdict: &str) -> Self {
        let state_label = verdict
            .lines()
            .find_map(|line| {
                let trimmed = line.trim();
                trimmed
                    .strip_prefix("state:")
                    .map(str::trim)
                    .map(str::to_string)
            })
            .unwrap_or_else(|| "needs_action".to_string());
        Self::new(
            state_label.clone(),
            format!("Activation state: {state_label}"),
            sections_from_plain(verdict),
        )
    }

    #[must_use]
    pub fn tone(&self) -> VerdictTone {
        VerdictTone::from_state_label(&self.state_label)
    }
}

fn sections_from_plain(verdict: &str) -> Vec<VerdictSection> {
    let mut activation = Vec::new();
    let mut layers = Vec::new();
    let mut install = Vec::new();
    let mut languages = Vec::new();
    let mut config = Vec::new();

    for line in verdict.lines() {
        let row = line.trim();
        if row.is_empty() || row.eq_ignore_ascii_case("activation") || row == "verify:" {
            continue;
        }
        let lower = row.to_ascii_lowercase();
        if lower.contains("l0 ")
            || lower.contains("l2 ")
            || lower.contains("l3/")
            || lower.contains("mcp")
            || lower.contains("watch")
            || lower.contains("daemon")
            || lower.contains("hook")
            || lower.contains("layer")
        {
            layers.push(row.to_string());
        } else if lower.contains("install") || lower.contains("workflow") || lower.contains("write")
        {
            install.push(row.to_string());
        } else if lower.contains("language")
            || lower.contains("coverage")
            || lower.contains("unsupported")
        {
            languages.push(row.to_string());
        } else if lower.contains("config") || lower.contains(".anvil") || lower.contains("rule") {
            config.push(row.to_string());
        } else {
            activation.push(row.to_string());
        }
    }

    vec![
        VerdictSection::new("activation", "Activation", activation),
        VerdictSection::new("layers", "Layers", layers),
        VerdictSection::new("install", "Install", install),
        VerdictSection::new("languages", "Languages", languages),
        VerdictSection::new("config", "Config", config),
    ]
}

/// Mutable verdict view state.
#[derive(Debug, Clone)]
pub struct VerdictView {
    model: VerdictModel,
    tree: TreeState,
    toast: Option<String>,
}

impl VerdictView {
    #[must_use]
    pub fn new(model: VerdictModel) -> Self {
        let mut tree = TreeState::default();
        if model.tone().auto_expand() {
            for section in &model.sections {
                tree.expand(section.id.clone());
            }
        }
        Self {
            model,
            tree,
            toast: None,
        }
    }

    #[must_use]
    pub fn model(&self) -> &VerdictModel {
        &self.model
    }

    #[must_use]
    pub fn toast(&self) -> Option<&str> {
        self.toast.as_deref()
    }

    #[must_use]
    pub fn is_expanded(&self, id: &str) -> bool {
        self.tree.is_expanded(id)
    }

    fn nodes(&self) -> Vec<TreeNode> {
        self.model
            .sections
            .iter()
            .map(|section| {
                if section.rows.is_empty() {
                    TreeNode::leaf(section.id.clone(), section.title.clone())
                } else {
                    let children = section
                        .rows
                        .iter()
                        .enumerate()
                        .map(|(idx, row)| {
                            TreeNode::leaf(format!("{}::{idx}", section.id), row.clone())
                        })
                        .collect();
                    TreeNode::branch(section.id.clone(), section.title.clone(), children)
                }
            })
            .collect()
    }

    fn visible_count(&self) -> usize {
        let nodes = self.nodes();
        let theme = EddaCraftTheme;
        Tree::new(&theme, &nodes).visible_count(&self.tree)
    }

    fn selected_id(&self) -> Option<String> {
        let nodes = self.nodes();
        let theme = EddaCraftTheme;
        Tree::new(&theme, &nodes).selected_id(&self.tree)
    }

    /// Handle verdict-view keys. Returns `true` when the surface should quit.
    pub fn handle_key(&mut self, action: Action) -> bool {
        match action {
            Action::Up => self.tree.move_up(self.visible_count()),
            Action::Down => self.tree.move_down(self.visible_count()),
            Action::Select | Action::Toggle => {
                if let Some(id) = self.selected_id()
                    && self.model.sections.iter().any(|section| section.id == id)
                {
                    self.tree.toggle(&id);
                }
            }
            // Thin-v1 smoke path: honest feedback without pretending the TUI has
            // executed the recipe. The textual recipe remains on the plain path.
            Action::Character('t' | 'T') => {
                self.toast = Some("Smoke test: run `anvil start --no-tui` for the copy/paste recipe; in-surface execution lands in the contract-hardening slice.".to_string());
            }
            Action::Quit | Action::Back => return true,
            _ => {}
        }
        false
    }
}

pub fn render(frame: &mut Frame, area: Rect, view: &VerdictView, theme: &EddaCraftTheme) {
    let chunks = Layout::vertical([
        Constraint::Length(2),
        Constraint::Min(3),
        Constraint::Length(1),
    ])
    .split(area);

    render_headline(frame, chunks[0], &view.model, theme);
    render_tree(frame, chunks[1], view, theme);
    render_help(frame, chunks[2], theme);

    if let Some(message) = view.toast() {
        render_toast(frame, area, message, theme);
    }
}

fn render_headline(frame: &mut Frame, area: Rect, model: &VerdictModel, theme: &EddaCraftTheme) {
    if area.height == 0 {
        return;
    }
    let chunks = Layout::vertical([Constraint::Length(1), Constraint::Length(1)]).split(area);
    let badge_label = format!("state: {}", model.state_label);
    StatusBadge::new(model.tone().badge(), theme)
        .label(&badge_label)
        .render(chunks[0], frame.buffer_mut());
    frame.render_widget(
        Paragraph::new(Line::styled(
            model.headline.clone(),
            Style::default().fg(theme.muted()),
        )),
        chunks[1],
    );
}

fn render_tree(frame: &mut Frame, area: Rect, view: &VerdictView, theme: &EddaCraftTheme) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.accent()))
        .title(" Activation verdict ");
    let inner = block.inner(area);
    frame.render_widget(block, area);
    if inner.width == 0 || inner.height == 0 {
        return;
    }
    let nodes = view.nodes();
    let mut tree_state = view.tree.clone();
    Tree::new(theme, &nodes).render(inner, frame.buffer_mut(), &mut tree_state);
}

fn render_help(frame: &mut Frame, area: Rect, theme: &EddaCraftTheme) {
    const BINDINGS: &[Binding] = &[
        Binding {
            keys: "↑/↓",
            action: Action::Up,
            label: "Move",
        },
        Binding {
            keys: "enter/space",
            action: Action::Select,
            label: "Expand",
        },
        Binding {
            keys: "t",
            action: Action::Character('t'),
            label: "Smoke",
        },
        Binding {
            keys: "e",
            action: Action::Character('e'),
            label: "Evidence",
        },
        Binding {
            keys: "esc/q",
            action: Action::Quit,
            label: "Quit",
        },
    ];
    HelpBar::new(theme)
        .bindings(BINDINGS)
        .render(area, frame.buffer_mut());
}

fn render_toast(frame: &mut Frame, area: Rect, message: &str, theme: &EddaCraftTheme) {
    ToastStack::new(theme)
        .placement(ToastPlacement::BottomRight)
        .width(area.width.saturating_sub(4).clamp(10, 60))
        .push(Toast::new(theme, message).severity(BadgeStatus::Info))
        .render(area, frame.buffer_mut());
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    fn sections() -> Vec<VerdictSection> {
        vec![
            VerdictSection::new(
                "layers",
                "Active layers",
                vec!["L0 mcp pre-write (pending — restart required)".to_string()],
            ),
            VerdictSection::new("install", "Install", vec!["cursor: written".to_string()]),
        ]
    }

    fn render_symbols(view: &VerdictView, width: u16, height: u16) -> String {
        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).unwrap();
        let theme = EddaCraftTheme;
        terminal
            .draw(|frame| render(frame, frame.area(), view, &theme))
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
    fn tone_maps_state_labels_without_upgrading() {
        assert_eq!(
            VerdictTone::from_state_label("protecting"),
            VerdictTone::Protecting
        );
        assert_eq!(
            VerdictTone::from_state_label("watching"),
            VerdictTone::Watching
        );
        assert_eq!(
            VerdictTone::from_state_label("ready_restart_required"),
            VerdictTone::Attention,
        );
        assert_eq!(
            VerdictTone::from_state_label("future"),
            VerdictTone::Attention
        );
    }

    #[test]
    fn model_from_plain_keeps_state_label_verbatim() {
        let model = VerdictModel::from_plain(
            "ACTIVATION\n  state: ready_restart_required\n  next: restart\n",
        );
        assert_eq!(model.state_label, "ready_restart_required");
        assert_eq!(model.tone(), VerdictTone::Attention);
    }

    #[test]
    fn protecting_rerun_collapses_sections() {
        let view = VerdictView::new(VerdictModel::new(
            "protecting",
            "Protecting — live.",
            sections(),
        ));
        assert!(!view.is_expanded("layers"));
        assert!(!view.is_expanded("install"));
    }

    #[test]
    fn repair_state_auto_expands_sections() {
        let view = VerdictView::new(VerdictModel::new(
            "ready_restart_required",
            "Ready, restart required.",
            sections(),
        ));
        assert!(view.is_expanded("layers"));
        assert!(view.is_expanded("install"));
    }

    #[test]
    fn select_toggles_section_expansion() {
        let mut view = VerdictView::new(VerdictModel::new(
            "protecting",
            "Protecting — live.",
            sections(),
        ));
        assert!(!view.is_expanded("layers"));
        assert!(!view.handle_key(Action::Select));
        assert!(view.is_expanded("layers"));
        assert!(!view.handle_key(Action::Select));
        assert!(!view.is_expanded("layers"));
    }

    #[test]
    fn quit_and_back_request_exit() {
        let model = VerdictModel::new("watching", "Watching.", sections());
        assert!(VerdictView::new(model.clone()).handle_key(Action::Quit));
        assert!(VerdictView::new(model).handle_key(Action::Back));
    }

    #[test]
    fn smoke_key_sets_honest_toast_without_claiming_a_finding() {
        let mut view = VerdictView::new(VerdictModel::new(
            "protecting",
            "Protecting — live.",
            sections(),
        ));
        assert!(view.toast().is_none());
        assert!(!view.handle_key(Action::Character('t')));
        let toast = view.toast().unwrap();
        assert!(toast.contains("Smoke test"));
        assert!(!toast.contains("finding detected"));
    }

    #[test]
    fn renders_state_label_and_headline_verbatim() {
        let view = VerdictView::new(VerdictModel::new(
            "ready_restart_required",
            "Ready, restart required — restart your editor.",
            sections(),
        ));
        let out = render_symbols(&view, 100, 24);
        assert!(out.contains("state: ready_restart_required"));
        assert!(out.contains("Ready, restart required"));
        assert!(out.contains("Active layers"));
        assert!(out.contains("pending — restart required"));
        assert!(out.contains("Smoke"));
    }
}
