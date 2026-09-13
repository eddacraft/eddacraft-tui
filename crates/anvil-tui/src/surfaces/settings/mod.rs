//! Settings inspect surface (SETINS-001 grouping/search/navigation,
//! SETINS-002 row honesty and expandable detail).
//!
//! The surface formats a command-supplied read model. It does not resolve
//! configuration, classify runtime state, or infer active enforcement.

pub mod render;

use eddacraft_tui::keyboard::Action;

/// Mutually exclusive runtime state labels from the SETCON read model.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeLabel {
    Active,
    Drift,
    Stale,
    Failed,
    Unknown,
}

impl RuntimeLabel {
    /// Word shown on the row. State is never colour-only.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Drift => "DRIFT",
            Self::Stale => "STALE",
            Self::Failed => "FAILED",
            Self::Unknown => "unknown",
        }
    }
}

/// One catalogue group in the Settings view.
#[derive(Debug, Clone)]
pub struct SettingsGroupView {
    pub id: String,
    pub label: String,
    pub rows: Vec<SettingsRowView>,
}

/// Expandable detail lines already formatted by the projector.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SettingsRowDetail {
    pub lines: Vec<String>,
}

/// One inspectable setting row. Display strings are pre-redacted.
#[derive(Debug, Clone)]
pub struct SettingsRowView {
    pub key: String,
    pub label: String,
    pub description: String,
    pub group: String,
    pub deprecated_aliases: Vec<String>,
    pub resolved_display: String,
    pub source_badge: String,
    pub active_display: Option<String>,
    pub active_source_badge: Option<String>,
    pub constraint_badge: Option<String>,
    pub runtime: RuntimeLabel,
    pub workflow_state: Option<String>,
    pub consequence_badge: Option<String>,
    /// True only when the projector reports resolved and active agree.
    pub compact: bool,
    pub detail: SettingsRowDetail,
}

impl SettingsRowView {
    fn matches(&self, query: &str) -> bool {
        if query.is_empty() {
            return true;
        }
        let q = query.to_ascii_lowercase();
        let fields = [
            self.label.as_str(),
            self.key.as_str(),
            self.description.as_str(),
            self.group.as_str(),
        ];
        fields
            .iter()
            .any(|field| field.to_ascii_lowercase().contains(&q))
            || self
                .deprecated_aliases
                .iter()
                .any(|alias| alias.to_ascii_lowercase().contains(&q))
    }
}

/// Interactive Settings view state. Inspect-only: no edit path.
#[allow(clippy::struct_excessive_bools)]
pub struct SettingsState {
    pub groups: Vec<SettingsGroupView>,
    pub selected: usize,
    pub searching: bool,
    pub search_query: String,
    pub expanded: bool,
    pub should_quit: bool,
    pub wants_back: bool,
}

impl SettingsState {
    #[must_use]
    pub fn new(groups: Vec<SettingsGroupView>) -> Self {
        Self {
            groups,
            selected: 0,
            searching: false,
            search_query: String::new(),
            expanded: false,
            should_quit: false,
            wants_back: false,
        }
    }

    #[must_use]
    pub fn group_labels(&self) -> Vec<&str> {
        self.groups
            .iter()
            .map(|group| group.label.as_str())
            .collect()
    }

    #[must_use]
    pub fn visible_rows(&self) -> Vec<(usize, usize)> {
        let mut out = Vec::new();
        for (group_idx, group) in self.groups.iter().enumerate() {
            for (row_idx, row) in group.rows.iter().enumerate() {
                if row.matches(&self.search_query) {
                    out.push((group_idx, row_idx));
                }
            }
        }
        out
    }

    #[must_use]
    pub fn current_row(&self) -> Option<&SettingsRowView> {
        let visible = self.visible_rows();
        let (group_idx, row_idx) = visible.get(self.selected).copied()?;
        self.groups.get(group_idx)?.rows.get(row_idx)
    }

    #[must_use]
    pub fn footer_commands(&self) -> &'static str {
        if self.searching {
            "type to filter  up/down results  esc cancel  ctrl+c quit"
        } else if self.expanded {
            "j/k navigate  enter collapse  esc close  q quit"
        } else {
            "/ search  j/k navigate  g/G jump  esc back  q quit"
        }
    }

    fn clamp_selected(&mut self) {
        let len = self.visible_rows().len();
        if len == 0 {
            self.selected = 0;
        } else if self.selected >= len {
            self.selected = len - 1;
        }
    }

    fn on_filter_changed(&mut self) {
        self.expanded = false;
        self.clamp_selected();
    }

    fn move_selection(&mut self, down: bool) {
        let len = self.visible_rows().len();
        if len == 0 {
            return;
        }
        let last = len - 1;
        self.selected = if down {
            self.selected.saturating_add(1).min(last)
        } else {
            self.selected.saturating_sub(1)
        };
        self.expanded = false;
    }

    fn jump_first(&mut self) {
        self.selected = 0;
        self.expanded = false;
    }

    fn jump_last(&mut self) {
        self.selected = self.visible_rows().len().saturating_sub(1);
        self.expanded = false;
    }

    pub fn handle_key(&mut self, action: Action) {
        if self.searching {
            self.handle_search_key(action);
        } else {
            self.handle_browse_key(action);
        }
    }

    fn handle_search_key(&mut self, action: Action) {
        match action {
            Action::Character(ch) => {
                self.search_query.push(ch);
                self.on_filter_changed();
            }
            Action::Backspace => {
                self.search_query.pop();
                self.on_filter_changed();
            }
            Action::Delete => {
                self.search_query.clear();
                self.on_filter_changed();
            }
            Action::Up => self.move_selection(false),
            Action::Down => self.move_selection(true),
            Action::Home => self.jump_first(),
            Action::End => self.jump_last(),
            Action::Select => self.expanded = !self.expanded,
            Action::Back => {
                self.searching = false;
                self.search_query.clear();
                self.on_filter_changed();
            }
            Action::Quit => self.should_quit = true,
            _ => {}
        }
    }

    fn handle_browse_key(&mut self, action: Action) {
        match action {
            Action::Character('/') => {
                self.searching = true;
                self.search_query.clear();
                self.expanded = false;
            }
            Action::Character('g') | Action::Home => self.jump_first(),
            Action::Character('G') | Action::End => self.jump_last(),
            Action::Up => self.move_selection(false),
            Action::Down => self.move_selection(true),
            Action::Select => {
                if self.current_row().is_some() {
                    self.expanded = !self.expanded;
                }
            }
            Action::Back => {
                if self.expanded {
                    self.expanded = false;
                } else {
                    self.wants_back = true;
                }
            }
            Action::Quit => self.should_quit = true,
            _ => {}
        }
    }
}

impl crate::surface::Surface for SettingsState {
    fn surface_name(&self) -> &'static str {
        "Settings"
    }

    fn help_text(&self) -> &str {
        self.footer_commands()
    }

    fn text_entry_active(&self) -> bool {
        self.searching
    }

    fn handle_key(&mut self, action: Action) {
        SettingsState::handle_key(self, action);
    }

    fn should_quit(&self) -> bool {
        self.should_quit
    }

    fn should_back(&self) -> bool {
        self.wants_back
    }

    fn reset(&mut self) {
        self.should_quit = false;
        self.wants_back = false;
        self.searching = false;
        self.search_query.clear();
        self.expanded = false;
        self.selected = 0;
    }

    fn render(
        &self,
        frame: &mut ratatui::Frame,
        area: ratatui::layout::Rect,
        theme: &eddacraft_tui::theme::EddaCraftTheme,
    ) {
        render::render(frame, area, self, theme);
    }
}

/// Format the value/state portion of a row. Never emits `active` unless the
/// runtime label is [`RuntimeLabel::Active`].
#[must_use]
pub fn format_row_values(row: &SettingsRowView) -> String {
    let extras = extra_badges(row);
    let resolved = format!("{} [{}]", row.resolved_display, row.source_badge);
    let state_word = row.runtime.as_str();
    let body = match row.runtime {
        RuntimeLabel::Active if row.compact && displays_agree(row) => {
            format!("{state_word}  {resolved}")
        }
        RuntimeLabel::Active => {
            let active = row
                .active_display
                .as_deref()
                .unwrap_or(&row.resolved_display);
            let src = row.active_source_badge.as_deref().unwrap_or("current");
            format!("{state_word}  resolved: {resolved}  active: {active} [{src}]")
        }
        RuntimeLabel::Drift => {
            let active = row.active_display.as_deref().unwrap_or("");
            let src = row.active_source_badge.as_deref().unwrap_or("current");
            format!("{state_word}  resolved: {resolved}  active: {active} [{src}]")
        }
        RuntimeLabel::Stale => {
            let last = row.active_display.as_deref().unwrap_or("");
            let src = row.active_source_badge.as_deref().unwrap_or("stale");
            format!("{state_word}  resolved: {resolved}  last: {last} [{src}]")
        }
        RuntimeLabel::Failed | RuntimeLabel::Unknown => {
            format!("{state_word}  resolved: {resolved}")
        }
    };
    if extras.is_empty() {
        body
    } else {
        format!("{body}  {extras}")
    }
}

fn displays_agree(row: &SettingsRowView) -> bool {
    match row.active_display.as_deref() {
        None => true,
        Some(active) => active == row.resolved_display,
    }
}

fn extra_badges(row: &SettingsRowView) -> String {
    let mut parts = Vec::new();
    if let Some(constraint) = &row.constraint_badge {
        parts.push(constraint.as_str());
    }
    if let Some(workflow) = &row.workflow_state {
        parts.push(workflow.as_str());
    }
    if let Some(consequence) = &row.consequence_badge {
        parts.push(consequence.as_str());
    }
    parts.join("  ")
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::surface::Surface;

    #[allow(clippy::too_many_arguments)]
    fn row(
        key: &str,
        label: &str,
        description: &str,
        group: &str,
        resolved: &str,
        source: &str,
        runtime: RuntimeLabel,
        compact: bool,
    ) -> SettingsRowView {
        SettingsRowView {
            key: key.to_owned(),
            label: label.to_owned(),
            description: description.to_owned(),
            group: group.to_owned(),
            deprecated_aliases: Vec::new(),
            resolved_display: resolved.to_owned(),
            source_badge: source.to_owned(),
            active_display: None,
            active_source_badge: None,
            constraint_badge: None,
            runtime,
            workflow_state: None,
            consequence_badge: None,
            compact,
            detail: SettingsRowDetail::default(),
        }
    }

    #[allow(clippy::too_many_lines)]
    fn sample_groups() -> Vec<SettingsGroupView> {
        let mut enforcement = row(
            "protection.enforcement.mode",
            "Enforcement mode",
            "How anvil treats policy violations",
            "Protection",
            "block",
            "repo",
            RuntimeLabel::Drift,
            false,
        );
        enforcement.active_display = Some("warn".into());
        enforcement.active_source_badge = Some("current".into());
        enforcement.constraint_badge = Some("policy-constrained".into());
        enforcement.consequence_badge = Some("class-C".into());
        enforcement.detail = SettingsRowDetail {
            lines: vec![
                "key: protection.enforcement.mode".into(),
                "declarations: project=block [repo]".into(),
                "merge: replace".into(),
                "requested: block".into(),
                "constraints: min posture block".into(),
                "active: warn  instance intercept#1  age 12m".into(),
                "provenance: org=warn overridden; project=block winning".into(),
                "accepted: warn, block".into(),
                "default: warn".into(),
                "consequence: class-C".into(),
                "affected: intercept".into(),
                "findings: none".into(),
                "restart: none".into(),
                "cli: anvil settings explain protection.enforcement.mode".into(),
            ],
        };

        let mut compact_mode = row(
            "interface.compact_mode",
            "Compact mode",
            "Reduce spacing in the TUI",
            "Interface",
            "true",
            "user",
            RuntimeLabel::Active,
            true,
        );
        compact_mode.deprecated_aliases = vec!["ui.compact".into()];
        compact_mode.active_display = Some("true".into());
        compact_mode.detail = SettingsRowDetail {
            lines: vec!["key: interface.compact_mode".into()],
        };

        let mut token = row(
            "privacy.api_token",
            "API token",
            "Outbound API credential",
            "Privacy & egress",
            "[redacted]",
            "env",
            RuntimeLabel::Unknown,
            false,
        );
        token.detail = SettingsRowDetail {
            lines: vec![
                "key: privacy.api_token".into(),
                "value: [redacted]".into(),
                "evidence: classified digest — lower evidence than a revealed value".into(),
            ],
        };

        let mut locked = row(
            "integrations.mcp",
            "MCP registration",
            "Registered MCP servers",
            "Integrations",
            "anvil",
            "repo",
            RuntimeLabel::Unknown,
            false,
        );
        locked.workflow_state = Some("locked".into());

        let mut stale = row(
            "agents.approvals.default",
            "Default approval posture",
            "Session approval default",
            "Agents & approvals",
            "prompt",
            "user",
            RuntimeLabel::Stale,
            false,
        );
        stale.active_display = Some("allow".into());
        stale.active_source_badge = Some("stale 12m".into());

        let mut failed = row(
            "privacy.telemetry",
            "Telemetry",
            "Product telemetry export",
            "Privacy & egress",
            "off",
            "repo",
            RuntimeLabel::Failed,
            false,
        );
        failed.active_display = Some("on".into());

        vec![
            SettingsGroupView {
                id: "project".into(),
                label: "Project".into(),
                rows: vec![row(
                    "project.schema_version",
                    "Schema version",
                    "Project config schema version",
                    "Project",
                    "1.0.0",
                    "default",
                    RuntimeLabel::Active,
                    true,
                )],
            },
            SettingsGroupView {
                id: "protection".into(),
                label: "Protection".into(),
                rows: vec![enforcement],
            },
            SettingsGroupView {
                id: "agents".into(),
                label: "Agents & approvals".into(),
                rows: vec![stale],
            },
            SettingsGroupView {
                id: "privacy".into(),
                label: "Privacy & egress".into(),
                rows: vec![failed, token],
            },
            SettingsGroupView {
                id: "integrations".into(),
                label: "Integrations".into(),
                rows: vec![locked],
            },
            SettingsGroupView {
                id: "interface".into(),
                label: "Interface".into(),
                rows: vec![compact_mode],
            },
        ]
    }

    pub(crate) fn sample_state() -> SettingsState {
        SettingsState::new(sample_groups())
    }

    #[test]
    fn settings_view_renders_groups_in_catalogue_order() {
        let state = sample_state();
        assert_eq!(
            state.group_labels(),
            [
                "Project",
                "Protection",
                "Agents & approvals",
                "Privacy & egress",
                "Integrations",
                "Interface",
            ]
        );
    }

    #[test]
    fn settings_view_slash_focuses_search() {
        let mut state = sample_state();
        assert!(!state.text_entry_active());
        state.handle_key(Action::Character('/'));
        assert!(state.searching);
        assert!(state.text_entry_active());
        assert_eq!(
            state.footer_commands(),
            "type to filter  up/down results  esc cancel  ctrl+c quit"
        );
    }

    #[test]
    fn settings_view_search_matches_label_key_description_group_and_alias() {
        let mut state = sample_state();
        state.handle_key(Action::Character('/'));

        state.search_query = "Enforcement".into();
        assert_eq!(state.visible_rows().len(), 1);

        state.search_query = "protection.enforcement".into();
        assert_eq!(state.visible_rows().len(), 1);

        state.search_query = "policy violations".into();
        assert_eq!(state.visible_rows().len(), 1);

        state.search_query = "Agents & approvals".into();
        assert_eq!(state.visible_rows().len(), 1);

        state.search_query = "ui.compact".into();
        assert_eq!(state.visible_rows().len(), 1);
        assert_eq!(
            state.current_row().map(|row| row.key.as_str()),
            Some("interface.compact_mode")
        );
    }

    #[test]
    fn settings_view_search_keeps_group_source_constraint_and_state() {
        let mut state = sample_state();
        state.handle_key(Action::Character('/'));
        for ch in "enforcement".chars() {
            state.handle_key(Action::Character(ch));
        }
        let row = state.current_row().expect("filtered row");
        assert_eq!(row.group, "Protection");
        assert_eq!(row.source_badge, "repo");
        assert_eq!(row.constraint_badge.as_deref(), Some("policy-constrained"));
        assert_eq!(row.runtime, RuntimeLabel::Drift);
        let rendered = format_row_values(row);
        assert!(rendered.contains("repo"), "{rendered}");
        assert!(rendered.contains("policy-constrained"), "{rendered}");
        assert!(rendered.contains("DRIFT"), "{rendered}");
    }

    #[test]
    fn settings_view_j_k_and_g_g_navigate() {
        let mut state = sample_state();
        let last = state.visible_rows().len() - 1;
        state.handle_key(Action::Down);
        assert_eq!(state.selected, 1);
        state.handle_key(Action::Up);
        assert_eq!(state.selected, 0);
        state.handle_key(Action::Character('G'));
        assert_eq!(state.selected, last);
        state.handle_key(Action::Character('g'));
        assert_eq!(state.selected, 0);
    }

    #[test]
    fn settings_view_esc_backs_out_without_mutation() {
        let mut state = sample_state();
        state.handle_key(Action::Back);
        assert!(state.wants_back);
        assert!(!state.expanded);
        assert!(!state.searching);
    }

    #[test]
    fn settings_view_esc_cancels_search_before_leaving() {
        let mut state = sample_state();
        state.handle_key(Action::Character('/'));
        state.handle_key(Action::Character('e'));
        state.handle_key(Action::Back);
        assert!(!state.searching);
        assert!(state.search_query.is_empty());
        assert!(!state.wants_back);
    }

    #[test]
    fn settings_view_footer_omits_edit_commands() {
        let state = sample_state();
        let footer = state.footer_commands();
        assert!(!footer.to_ascii_lowercase().contains("edit"));
        assert!(!footer.contains("space"));
        assert!(!footer.contains("toggle"));
    }

    #[test]
    fn settings_view_toggle_does_not_edit() {
        let mut state = sample_state();
        let before = (
            state.selected,
            state.expanded,
            state.searching,
            state.search_query.clone(),
        );
        state.handle_key(Action::Toggle);
        assert_eq!(
            before,
            (
                state.selected,
                state.expanded,
                state.searching,
                state.search_query.clone()
            )
        );
    }

    #[test]
    fn settings_row_compact_only_when_resolved_and_active_agree() {
        let state = sample_state();
        let compact = state
            .groups
            .iter()
            .flat_map(|group| group.rows.iter())
            .find(|row| row.key == "interface.compact_mode")
            .expect("compact row");
        let rendered = format_row_values(compact);
        assert!(!rendered.contains("resolved:"), "{rendered}");
        assert!(rendered.contains("active"), "{rendered}");
        assert!(!rendered.contains("DRIFT"), "{rendered}");
    }

    #[test]
    fn settings_row_compact_flag_cannot_hide_disagreement() {
        let mut row = sample_state()
            .groups
            .iter()
            .flat_map(|group| group.rows.iter())
            .find(|row| row.key == "interface.compact_mode")
            .cloned()
            .expect("compact row");
        row.compact = true;
        row.runtime = RuntimeLabel::Active;
        row.resolved_display = "block".into();
        row.active_display = Some("warn".into());
        let rendered = format_row_values(&row);
        assert!(rendered.contains("resolved:"), "{rendered}");
        assert!(rendered.contains("warn"), "{rendered}");
        assert!(rendered.contains("block"), "{rendered}");
    }

    #[test]
    fn settings_row_drift_keeps_resolved_and_active_separate() {
        let state = sample_state();
        let row = state.current_row_by_key("protection.enforcement.mode");
        let rendered = format_row_values(row);
        assert!(rendered.contains("resolved: block [repo]"), "{rendered}");
        assert!(rendered.contains("active: warn [current]"), "{rendered}");
        assert!(rendered.contains("DRIFT"), "{rendered}");
        assert!(rendered.contains("policy-constrained"), "{rendered}");
        assert!(rendered.contains("class-C"), "{rendered}");
    }

    #[test]
    fn settings_row_unknown_never_says_active() {
        let state = sample_state();
        let row = state.current_row_by_key("privacy.api_token");
        let rendered = format_row_values(row);
        assert!(rendered.contains("unknown"), "{rendered}");
        assert!(
            !rendered.split_whitespace().any(|word| word == "active"),
            "{rendered}"
        );
    }

    #[test]
    fn settings_row_stale_is_not_drift() {
        let state = sample_state();
        let row = state.current_row_by_key("agents.approvals.default");
        let rendered = format_row_values(row);
        assert!(rendered.contains("last:"), "{rendered}");
        assert!(rendered.contains("STALE"), "{rendered}");
        assert!(!rendered.contains("DRIFT"), "{rendered}");
    }

    #[test]
    fn settings_row_failed_is_not_unknown() {
        let state = sample_state();
        let row = state.current_row_by_key("privacy.telemetry");
        let rendered = format_row_values(row);
        assert!(rendered.contains("FAILED"), "{rendered}");
        assert!(!rendered.contains("unknown"), "{rendered}");
        assert!(
            !rendered.split_whitespace().any(|word| word == "active"),
            "{rendered}"
        );
    }

    #[test]
    fn settings_row_workflow_state_stays_separate_from_runtime() {
        let state = sample_state();
        let row = state.current_row_by_key("integrations.mcp");
        let rendered = format_row_values(row);
        assert!(rendered.contains("locked"), "{rendered}");
        assert!(rendered.contains("unknown"), "{rendered}");
    }

    #[test]
    fn settings_row_expand_shows_detail_lines() {
        let mut state = sample_state();
        while state.current_row().map(|row| row.key.as_str()) != Some("protection.enforcement.mode")
        {
            let before = state.selected;
            state.handle_key(Action::Down);
            assert_ne!(state.selected, before, "row not found");
        }
        assert!(!state.expanded);
        state.handle_key(Action::Select);
        assert!(state.expanded);
        let detail = &state.current_row().expect("row").detail.lines;
        assert!(
            detail
                .iter()
                .any(|line| line.contains("key: protection.enforcement.mode"))
        );
        assert!(
            detail
                .iter()
                .any(|line| line.contains("cli: anvil settings explain"))
        );
        assert_eq!(
            state.footer_commands(),
            "j/k navigate  enter collapse  esc close  q quit"
        );
        state.handle_key(Action::Back);
        assert!(!state.expanded);
        assert!(!state.wants_back);
    }

    #[test]
    fn settings_row_reduced_evidence_explains_level() {
        let state = sample_state();
        let row = state.current_row_by_key("privacy.api_token");
        assert!(
            row.detail
                .lines
                .iter()
                .any(|line| line.contains("classified digest"))
        );
        assert_eq!(row.resolved_display, "[redacted]");
        assert!(!format_row_values(row).contains("s3cret"));
    }

    impl SettingsState {
        fn current_row_by_key(&self, key: &str) -> &SettingsRowView {
            self.groups
                .iter()
                .flat_map(|group| group.rows.iter())
                .find(|row| row.key == key)
                .unwrap_or_else(|| panic!("missing {key}"))
        }
    }
}
