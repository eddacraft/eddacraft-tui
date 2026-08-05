//! Structured verdict view for the activation TUI (ACTTUI-005).
//!
//! This module keeps the activation state vocabulary honest by rendering the
//! state label supplied by the CLI verbatim. It adds eddacraft-tui widgets around
//! that fixed truth: `StatusBadge` for the headline, `Tree` for collapsible
//! evidence, and `Toast` for Prove feedback (ACTTUI-016).
//!
//! Keyboard help lives only on the shell `HelpBar` (`ActivationSurface::help_text`)
//! — CIB-275 / ACTTUI-015: a second in-pane bar with different keys (arrows vs
//! `j/k`) made the result screen unreadable. The `next:` guidance line is also
//! promoted out of the single-line tree leaf so it can wrap at typical console
//! widths instead of clipping mid-sentence.
//!
//! JOURNEY-008 re-introduces a single-beat `BigBanner` celebration on the
//! *first* protecting activation only (the run that establishes the LAUNCH-010
//! baseline). It is a decorative flourish above — never a replacement for — the
//! honesty-pinned `StatusBadge` headline, and it stays silent on healthy repeat
//! runs so it never adds noise. (ACTTUI-012 had removed the earlier unused
//! wiring; this reinstates it behind the once-per-local-environment gate.)

use std::sync::Arc;

use eddacraft_tui::keyboard::Action;
use eddacraft_tui::prelude::{
    BadgeStatus, BigBanner, StatusBadge, Toast, ToastPlacement, ToastStack, Tree, TreeNode,
    TreeState,
};
use eddacraft_tui::theme::{EddaCraftTheme, Theme};
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Paragraph, StatefulWidget, Widget, Wrap};
use unicode_width::UnicodeWidthChar;

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

/// Short, honesty-pinned banner text for the first-success celebration.
///
/// It deliberately reuses the fixed `protecting` state word rather than a
/// completion claim (`protected`, `secure`, `done`): the celebration must not
/// upgrade the protection vocabulary. The full honesty-pinned headline still
/// renders below via [`render_headline`].
pub const CELEBRATION_BANNER_TEXT: &str = "Protecting";

/// Rows the celebration banner occupies when shown (quadrant pixel glyphs are
/// four cells tall).
const CELEBRATION_BANNER_HEIGHT: u16 = 4;

/// Minimum verdict-area height before the banner is allowed to show. The body
/// needs 5 rows (headline 2 + tree 3); adding the banner needs 4 more. Below
/// this the banner silently degrades to a full, uncompressed verdict so it can
/// never squeeze the honesty-pinned headline off screen. Mirrors the
/// `hint_min_height` guard convention used by the welcome/onboarding surfaces.
/// CIB-275: the former +1 row was an in-pane help bar; keys now live only on
/// the shell chrome.
const CELEBRATION_MIN_HEIGHT: u16 = CELEBRATION_BANNER_HEIGHT + 5;

/// Cap for the wrapped `next:` guidance band so a long repair hint cannot
/// starve the collapsible verdict tree on short terminals.
const NEXT_GUIDANCE_MAX_HEIGHT: u16 = 6;

/// Structured verdict handed to the surface by the CLI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerdictModel {
    pub state_label: String,
    pub headline: String,
    pub sections: Vec<VerdictSection>,
    /// True only when this run first established protection (the LAUNCH-010
    /// baseline was written this run). Gates the JOURNEY-008 celebration so it
    /// fires once per local environment — the `.anvil/baseline.json` lifecycle,
    /// not a durable per-repo flag; deleting `.anvil/` legitimately re-fires it
    /// — and never on healthy repeat runs.
    pub first_success: bool,
    /// Full `next:` / `Next:` guidance, lifted out of the tree so it can wrap.
    /// Absent when the plain arbiter has no repair or closing next step.
    pub next_guidance: Option<String>,
}

impl VerdictModel {
    #[must_use]
    pub fn new(
        state_label: impl Into<String>,
        headline: impl Into<String>,
        mut sections: Vec<VerdictSection>,
    ) -> Self {
        let next_guidance = take_next_guidance(&mut sections);
        Self {
            state_label: state_label.into(),
            headline: headline.into(),
            sections,
            first_success: false,
            next_guidance,
        }
    }

    /// Mark this verdict as the project's first successful activation. Callers
    /// pass `true` only when the baseline was written this run; the default is
    /// `false` so every existing call site stays a quiet repeat verdict.
    #[must_use]
    pub fn with_first_success(mut self, first_success: bool) -> Self {
        self.first_success = first_success;
        self
    }

    /// Whether to show the first-success celebration: the state must be
    /// `protecting` (never a repair or partial state) *and* this must be the
    /// first activation. Styling only — it never invents protection state.
    #[must_use]
    pub fn celebrates(&self) -> bool {
        self.first_success && self.tone() == VerdictTone::Protecting
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

/// Pull the first `next:` / `Next:` row out of section trees so the dedicated
/// wrapping band owns it (CIB-275). Case-insensitive prefix match after trim.
fn take_next_guidance(sections: &mut [VerdictSection]) -> Option<String> {
    for section in sections.iter_mut() {
        if let Some(idx) = section.rows.iter().position(is_next_guidance_row) {
            return Some(section.rows.remove(idx));
        }
    }
    None
}

fn is_next_guidance_row(row: impl AsRef<str>) -> bool {
    row.as_ref()
        .trim()
        .to_ascii_lowercase()
        .starts_with("next:")
}

/// Display-width-aware wrap height for the guidance band (ASCII-heavy copy;
/// still counts wide glyphs correctly when present).
fn next_guidance_height(text: &str, width: u16) -> u16 {
    if text.is_empty() || width == 0 {
        return 0;
    }
    let max_w = usize::from(width);
    let mut rows = 1u16;
    let mut col = 0usize;
    for ch in text.chars() {
        let cw = UnicodeWidthChar::width(ch).unwrap_or(0);
        if cw == 0 {
            continue;
        }
        if col + cw > max_w && col > 0 {
            rows = rows.saturating_add(1);
            col = 0;
        }
        col += cw;
    }
    rows.clamp(1, NEXT_GUIDANCE_MAX_HEIGHT)
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

/// Callback that runs in-surface Prove (ACTTUI-016). Returns toast copy only —
/// never invents protection-state claims.
pub type ProveRunner = Arc<dyn Fn() -> String + Send + Sync>;

/// Mutable verdict view state.
pub struct VerdictView {
    model: VerdictModel,
    tree: TreeState,
    toast: Option<String>,
    prove: Option<ProveRunner>,
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
            prove: None,
        }
    }

    /// Attach an in-surface Prove runner (ACTTUI-016). When absent, `t` explains
    /// that Prove is unavailable rather than claiming a future hardening slice.
    #[must_use]
    pub fn with_prove(mut self, prove: ProveRunner) -> Self {
        self.prove = Some(prove);
        self
    }

    /// Replace the Prove runner after construction (CLI wiring).
    pub fn set_prove(&mut self, prove: ProveRunner) {
        self.prove = Some(prove);
    }

    #[must_use]
    pub fn model(&self) -> &VerdictModel {
        &self.model
    }

    /// Whether this view shows the first-success celebration banner.
    #[must_use]
    pub fn celebrates(&self) -> bool {
        self.model.celebrates()
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
            // ACTTUI-016 Prove: run the attached check-pipeline callback when
            // present. ACTTUI-015 honesty: never claim a future "hardening slice"
            // or that `--no-tui` always shows a recipe (CIB-183 re-runs omit it).
            Action::Character('t' | 'T') => {
                self.toast = Some(match &self.prove {
                    Some(prove) => prove(),
                    None => {
                        "Prove unavailable here. Manual check-pipeline recipe: write a throwaway secret-shaped fixture, run `anvil check` on it, then delete the file. This does not prove MCP pre-write is live."
                            .to_string()
                    }
                });
            }
            Action::Quit | Action::Back => return true,
            _ => {}
        }
        false
    }
}

pub fn render(frame: &mut Frame, area: Rect, view: &VerdictView, theme: &EddaCraftTheme) {
    // JOURNEY-008: on the first protecting run only, a single celebration banner
    // sits above the verdict body. Everywhere else the body fills the whole area
    // exactly as before, so repeat runs are byte-for-byte unchanged. The banner
    // is suppressed when the area is too short to hold it *and* an uncompressed
    // verdict, so it never squeezes the honesty-pinned headline off screen.
    let body = if view.celebrates() && area.height >= CELEBRATION_MIN_HEIGHT {
        let split = Layout::vertical([
            Constraint::Length(CELEBRATION_BANNER_HEIGHT),
            Constraint::Min(3),
        ])
        .split(area);
        render_celebration_banner(frame, split[0], theme);
        split[1]
    } else {
        area
    };

    // CIB-275: no in-pane HelpBar — shell chrome owns keys. The free row goes
    // to a wrapping `next:` band so the one line that tells the user what to do
    // is fully readable at typical console widths.
    let next_height = view
        .model
        .next_guidance
        .as_deref()
        .map_or(0, |text| next_guidance_height(text, body.width));

    let chunks = Layout::vertical([
        Constraint::Length(2),
        Constraint::Length(next_height),
        Constraint::Min(3),
    ])
    .split(body);

    render_headline(frame, chunks[0], &view.model, theme);
    if next_height > 0
        && let Some(text) = view.model.next_guidance.as_deref()
    {
        render_next_guidance(frame, chunks[1], text, theme);
    }
    render_tree(frame, chunks[2], view, theme);

    if let Some(message) = view.toast() {
        render_toast(frame, area, message, theme);
    }
}

/// Render the first-success celebration banner. Short, accent-styled pixel text
/// reusing the honesty-pinned protecting vocabulary; the fixed headline still
/// renders below it via [`render_headline`].
fn render_celebration_banner(frame: &mut Frame, area: Rect, theme: &EddaCraftTheme) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    frame.render_widget(
        BigBanner::new(theme, CELEBRATION_BANNER_TEXT).centered(),
        area,
    );
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

/// CIB-275: dedicated wrapping band for the arbiter's next-step line.
fn render_next_guidance(frame: &mut Frame, area: Rect, text: &str, theme: &EddaCraftTheme) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    frame.render_widget(
        Paragraph::new(Line::styled(
            text.to_string(),
            Style::default()
                .fg(theme.accent())
                .add_modifier(Modifier::BOLD),
        ))
        .wrap(Wrap { trim: false }),
        area,
    );
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
    fn prove_key_without_runner_is_honest_and_claims_no_finding() {
        let mut view = VerdictView::new(VerdictModel::new(
            "protecting",
            "Protecting — live.",
            sections(),
        ));
        assert!(view.toast().is_none());
        assert!(!view.handle_key(Action::Character('t')));
        let toast = view.toast().unwrap();
        assert!(toast.contains("Prove unavailable") || toast.contains("Manual check-pipeline"));
        assert!(!toast.contains("contract-hardening"));
        assert!(!toast.contains("finding detected"));
        assert!(
            toast.contains("does not prove MCP pre-write")
                || toast.contains("not MCP pre-write")
                || toast.contains("Manual check-pipeline")
        );
    }

    #[test]
    fn prove_key_with_runner_surfaces_callback_result() {
        let mut view = VerdictView::new(VerdictModel::new(
            "protecting",
            "Protecting — live.",
            sections(),
        ))
        .with_prove(std::sync::Arc::new(|| {
            "Prove: secret-detection caught 1 finding(s) on the built-in sample fixture (check pipeline only — not MCP pre-write)."
                .to_string()
        }));
        assert!(!view.handle_key(Action::Character('t')));
        let toast = view.toast().unwrap();
        assert!(toast.contains("secret-detection caught"));
        assert!(toast.contains("check pipeline only"));
        assert!(toast.contains("built-in sample fixture"));
        assert!(
            !toast.contains("on the fixture"),
            "ambiguous 'the fixture' invites a live-repo misread (CIB-276): {toast}"
        );
        assert!(!toast.contains("contract-hardening"));
    }

    #[test]
    fn first_protecting_run_celebrates() {
        let model = VerdictModel::new("protecting", "Protecting — live.", sections())
            .with_first_success(true);
        assert!(model.celebrates());
        assert!(VerdictView::new(model).celebrates());
    }

    #[test]
    fn repeat_protecting_run_does_not_celebrate() {
        // Default `first_success` is false, so every existing protecting call
        // site stays a quiet repeat verdict.
        let model = VerdictModel::new("protecting", "Protecting — live.", sections());
        assert!(!model.celebrates());
    }

    #[test]
    fn repair_and_watching_states_never_celebrate_even_on_first_run() {
        for label in ["ready_restart_required", "watching", "error", "unsupported"] {
            let model = VerdictModel::new(label, "headline", sections()).with_first_success(true);
            assert!(
                !model.celebrates(),
                "only protecting may celebrate, got: {label}"
            );
        }
    }

    #[test]
    fn celebration_banner_text_stays_honest() {
        // The big-text celebration must reuse the fixed `protecting` vocabulary
        // and never upgrade it into a completion or guarantee claim.
        assert_eq!(CELEBRATION_BANNER_TEXT, "Protecting");
        let lower = CELEBRATION_BANNER_TEXT.to_ascii_lowercase();
        for forbidden in [
            "protected",
            "secure",
            "safe",
            "guaranteed",
            "done",
            "complete",
        ] {
            assert!(
                !lower.contains(forbidden),
                "celebration banner overclaims protection: {CELEBRATION_BANNER_TEXT}"
            );
        }
    }

    #[test]
    fn celebration_banner_sits_above_the_honesty_headline_only_on_first_run() {
        let celebrate = VerdictView::new(
            VerdictModel::new(
                "protecting",
                "Protecting — pre-write validation is live in this repo.",
                sections(),
            )
            .with_first_success(true),
        );
        let repeat = VerdictView::new(VerdictModel::new(
            "protecting",
            "Protecting — pre-write validation is live in this repo.",
            sections(),
        ));

        let out_celebrate = render_symbols(&celebrate, 100, 24);
        let out_repeat = render_symbols(&repeat, 100, 24);

        // The honesty-pinned headline renders in BOTH — the banner is additive,
        // never a replacement.
        assert!(out_celebrate.contains("state: protecting"));
        assert!(out_repeat.contains("state: protecting"));
        assert_ne!(out_celebrate, out_repeat);

        // On the first run the top band carries the painted banner and the
        // headline is pushed below it; the repeat run leads with the headline.
        let top_band: String = out_celebrate.lines().take(4).collect::<Vec<_>>().join("\n");
        assert!(
            top_band.chars().any(|c| !c.is_whitespace()),
            "celebration banner band should be painted"
        );
        assert!(
            !top_band.contains("state:"),
            "headline must sit below the celebration banner"
        );
        assert!(
            out_repeat
                .lines()
                .next()
                .unwrap_or_default()
                .contains("state: protecting"),
            "repeat run leads with the headline, no banner"
        );
    }

    #[test]
    fn celebration_degrades_on_short_terminals_preserving_the_headline() {
        // On a pane too short to hold both the banner and an uncompressed
        // verdict, the banner is suppressed so the honesty-pinned headline
        // always survives — the banner never squeezes the state line off
        // screen. CIB-275: keys live only on the shell chrome, so no in-pane
        // help bar is required to survive.
        let celebrate = VerdictView::new(
            VerdictModel::new(
                "protecting",
                "Protecting — pre-write validation is live in this repo.",
                sections(),
            )
            .with_first_success(true),
        );
        assert!(celebrate.celebrates());

        let out = render_symbols(&celebrate, 80, 8);
        assert!(
            out.contains("state: protecting"),
            "headline must survive on a short terminal"
        );
        assert!(
            out.lines()
                .next()
                .unwrap_or_default()
                .contains("state: protecting"),
            "short terminal leads with the headline; banner is suppressed"
        );
        assert!(
            !out.contains("[↑/↓]"),
            "in-pane help bar must stay gone on short terminals"
        );
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
        // CIB-275: Prove is advertised on the shell help bar only.
        assert!(!out.contains("[t] Prove"));
        assert!(!out.contains("[↑/↓]"));
    }

    #[test]
    fn cib275_no_inline_help_bar_on_verdict() {
        let view = VerdictView::new(VerdictModel::new(
            "ready_restart_required",
            "Ready, restart required — restart your editor.",
            sections(),
        ));
        let out = render_symbols(&view, 100, 22);
        assert!(
            !out.contains("[↑/↓]"),
            "arrows help bar must not render inside the verdict pane"
        );
        assert!(
            !out.contains("[enter/space]"),
            "second key legend must not fight the shell j/k help"
        );
        assert!(
            !out.contains("[esc/q] Quit"),
            "quit keys live on the shell HelpBar only"
        );
    }

    #[test]
    fn cib275_long_next_guidance_wraps_at_typical_console_width() {
        // Real repair hints from the plain arbiter are long enough that a
        // single tree leaf clips mid-sentence at ~80 columns. The dedicated
        // band must keep the full text readable by wrapping.
        let next = "next: no intercept daemon is answering for this worktree, so another editor restart will not help; run `anvil start` in a real terminal (not piped) to auto-start the daemon — for headless recovery use `anvil intercept start --foreground` — then re-run `anvil start --verify`.";
        assert!(
            next.chars().count() > 80,
            "fixture must exceed a typical console width"
        );
        let view = VerdictView::new(VerdictModel::new(
            "ready_restart_required",
            "Ready, restart required — restart your editor.",
            vec![VerdictSection::new(
                "activation",
                "Activation",
                vec![
                    "state: ready_restart_required".to_string(),
                    next.to_string(),
                ],
            )],
        ));
        assert_eq!(view.model().next_guidance.as_deref(), Some(next));
        assert!(
            view.model()
                .sections
                .iter()
                .flat_map(|s| s.rows.iter())
                .all(|row| !is_next_guidance_row(row)),
            "next: must leave the tree once promoted to the guidance band"
        );

        let out = render_symbols(&view, 80, 24);
        // Collapse whitespace so wrap-induced line breaks do not break the
        // full-text assertion.
        let compact: String = out.split_whitespace().collect::<Vec<_>>().join(" ");
        let next_compact: String = next.split_whitespace().collect::<Vec<_>>().join(" ");
        assert!(
            compact.contains(&next_compact),
            "full next: guidance must be readable at width 80; got:\n{out}"
        );
        assert!(
            !out.contains("[↑/↓]"),
            "wrapping next: must not re-introduce the dual help bar"
        );
    }

    #[test]
    fn cib275_from_plain_promotes_next_out_of_tree() {
        let model = VerdictModel::from_plain(
            "ACTIVATION\n  state: ready_restart_required\n  next: restart your editor or agent so the MCP server attaches, then re-run `anvil start --verify`.\n",
        );
        assert!(
            model
                .next_guidance
                .as_deref()
                .is_some_and(|n| n.starts_with("next: restart")),
            "from_plain must promote next: into next_guidance"
        );
        assert!(
            model
                .sections
                .iter()
                .flat_map(|s| s.rows.iter())
                .all(|row| !is_next_guidance_row(row))
        );
    }
}
