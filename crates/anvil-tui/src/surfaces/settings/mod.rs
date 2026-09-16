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

/// First-release inspect tabs (spec §8). `Audit` is SETGOV.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsTab {
    Settings,
    Status,
    Sources,
}

impl SettingsTab {
    pub(crate) const ALL: [Self; 3] = [Self::Settings, Self::Status, Self::Sources];

    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Settings => "Settings",
            Self::Status => "Status",
            Self::Sources => "Sources",
        }
    }

    fn shift(self, next: bool) -> Self {
        let idx = Self::ALL.iter().position(|tab| *tab == self).unwrap_or(0);
        let next_idx = if next {
            (idx + 1) % Self::ALL.len()
        } else {
            idx.checked_sub(1).unwrap_or(Self::ALL.len() - 1)
        };
        Self::ALL[next_idx]
    }
}

/// Command-supplied Status tab (SETINS-003). The surface formats; it does not
/// recompute health or posture.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SettingsStatusView {
    pub version: String,
    pub runtime: String,
    pub project: String,
    pub worktree: String,
    pub session: String,
    pub resolved_posture: String,
    pub attested_posture: String,
    pub health: String,
    pub health_reasons: Vec<String>,
    pub non_healthy: Vec<String>,
    pub integrations: Vec<String>,
    pub adapters: Vec<String>,
    pub pending: Option<String>,
    pub validation: String,
    pub attestation_age: String,
    pub attestation_source: String,
}

/// One discovered configuration source (SETINS-004).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingsSourceRow {
    pub scope: String,
    pub path_display: String,
    pub writable: bool,
    pub kind: String,
}

/// Command-supplied Sources tab.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SettingsSourcesView {
    pub revision: String,
    pub sources: Vec<SettingsSourceRow>,
    pub overridden: Vec<String>,
    pub winning: Vec<String>,
    pub field_provenance: Vec<String>,
    pub unknown_keys: Vec<String>,
    pub deprecated_keys: Vec<String>,
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
    /// SETPREF: Class A rows may be edited through the settings service.
    pub class_a: bool,
    pub write_scope: Option<String>,
    pub boolean_value: Option<bool>,
    pub enum_allowed: Vec<String>,
    pub inherited: bool,
    pub default_display: String,
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

/// Pending Class A write the CLI persists through the settings service.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PendingWrite {
    Toggle { key: String },
    Set { key: String, value: String },
    Reset { key: String },
}

type PersistFn = Box<dyn FnMut(PendingWrite) -> Result<(), String>>;

/// Interactive Settings view state. Class A edits go through `persist`.
#[allow(clippy::struct_excessive_bools)]
pub struct SettingsState {
    pub groups: Vec<SettingsGroupView>,
    pub selected: usize,
    pub searching: bool,
    pub search_query: String,
    pub expanded: bool,
    pub tab: SettingsTab,
    pub status: SettingsStatusView,
    pub sources: SettingsSourcesView,
    pub reduced_motion: bool,
    pub search_used: bool,
    pub should_quit: bool,
    pub wants_back: bool,
    pub pending_write: Option<PendingWrite>,
    pub pending_reset_key: Option<String>,
    pub reset_preview: Option<String>,
    pub last_error: Option<String>,
    pub source_revision: Option<String>,
    pub persist: Option<PersistFn>,
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
            tab: SettingsTab::Settings,
            status: SettingsStatusView::default(),
            sources: SettingsSourcesView::default(),
            reduced_motion: false,
            search_used: false,
            should_quit: false,
            wants_back: false,
            pending_write: None,
            pending_reset_key: None,
            reset_preview: None,
            last_error: None,
            source_revision: None,
            persist: None,
        }
    }

    #[must_use]
    pub fn with_status(mut self, status: SettingsStatusView) -> Self {
        self.status = status;
        self
    }

    #[must_use]
    pub fn with_sources(mut self, sources: SettingsSourcesView) -> Self {
        self.sources = sources;
        self
    }

    #[must_use]
    pub fn with_reduced_motion(mut self, reduced: bool) -> Self {
        self.reduced_motion = reduced;
        self
    }

    pub fn focus_key(&mut self, key: &str) {
        self.tab = SettingsTab::Settings;
        if let Some(idx) = self.visible_rows().iter().position(|&(g, r)| {
            self.groups
                .get(g)
                .and_then(|group| group.rows.get(r))
                .is_some_and(|row| {
                    row.key == key || row.deprecated_aliases.iter().any(|a| a == key)
                })
        }) {
            self.selected = idx;
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

    fn current_row_mut(&mut self) -> Option<&mut SettingsRowView> {
        let visible = self.visible_rows();
        let (group_idx, row_idx) = visible.get(self.selected).copied()?;
        self.groups.get_mut(group_idx)?.rows.get_mut(row_idx)
    }

    fn commit_write(&mut self, write: PendingWrite) {
        self.last_error = None;
        self.pending_write = Some(write.clone());
        if let Some(persist) = self.persist.as_mut()
            && let Err(err) = persist(write)
        {
            self.last_error = Some(err);
        }
    }

    fn toggle_class_a(&mut self) {
        let Some(row) = self.current_row() else {
            return;
        };
        if !row.class_a {
            self.last_error = Some("not a Class A setting".into());
            return;
        }
        let Some(current) = row.boolean_value else {
            return;
        };
        let key = row.key.clone();
        let next = !current;
        let inherited = row.inherited;
        if inherited && self.reset_preview.is_none() {
            self.reset_preview = Some(format!(
                "override {key} at user scope  enter confirm  esc cancel"
            ));
            self.pending_write = Some(PendingWrite::Toggle { key });
            return;
        }
        self.commit_write(PendingWrite::Toggle { key });
        if self.last_error.is_some() {
            return;
        }
        if let Some(row) = self.current_row_mut() {
            row.boolean_value = Some(next);
            row.resolved_display = next.to_string();
            row.source_badge = "user".into();
            row.inherited = false;
            if inherited {
                row.detail.lines.insert(
                    0,
                    "override: writing user scope will override the inherited value".into(),
                );
            }
        }
    }

    fn cycle_class_a_enum(&mut self) -> bool {
        let Some(row) = self.current_row() else {
            return false;
        };
        if !row.class_a || row.enum_allowed.is_empty() {
            return false;
        }
        let key = row.key.clone();
        let allowed = row.enum_allowed.clone();
        let current = row.resolved_display.clone();
        let idx = allowed
            .iter()
            .position(|item| item == &current)
            .unwrap_or(0);
        let next = allowed[(idx + 1) % allowed.len()].clone();
        if row.inherited && self.reset_preview.is_none() {
            self.reset_preview = Some(format!(
                "override {key} at user scope  enter confirm  esc cancel"
            ));
            self.pending_write = Some(PendingWrite::Set { key, value: next });
            return true;
        }
        self.commit_write(PendingWrite::Set {
            key,
            value: next.clone(),
        });
        if self.last_error.is_some() {
            return true;
        }
        if let Some(row) = self.current_row_mut() {
            row.resolved_display = next;
            row.source_badge = "user".into();
            row.inherited = false;
        }
        true
    }

    fn preview_reset(&mut self) {
        let Some(row) = self.current_row() else {
            return;
        };
        if !row.class_a {
            self.last_error =
                Some("reset stays read-only; Class B/C enter the proposal flow in SETGOV".into());
            return;
        }
        let key = row.key.clone();
        let next = if row.default_display.is_empty() {
            "catalogue default".to_owned()
        } else {
            row.default_display.clone()
        };
        self.reset_preview = Some(format!(
            "reset {key} -> {next} [default]  enter confirm  esc cancel"
        ));
        self.pending_reset_key = Some(key);
    }

    fn confirm_reset(&mut self) {
        let Some(key) = self.pending_reset_key.take() else {
            return;
        };
        self.reset_preview = None;
        let default_display = self
            .current_row()
            .map(|row| row.default_display.clone())
            .unwrap_or_default();
        self.commit_write(PendingWrite::Reset { key });
        if self.last_error.is_some() {
            return;
        }
        if let Some(row) = self.current_row_mut() {
            row.resolved_display = if default_display.is_empty() {
                "false".into()
            } else {
                default_display
            };
            row.source_badge = "default".into();
            if let Ok(flag) = row.resolved_display.parse() {
                row.boolean_value = Some(flag);
            }
            row.inherited = false;
        }
    }

    #[must_use]
    pub fn footer_commands(&self) -> &str {
        if let Some(err) = &self.last_error {
            return err.as_str();
        }
        if let Some(preview) = &self.reset_preview {
            return preview.as_str();
        }
        match self.tab {
            SettingsTab::Settings if self.searching => {
                "type to filter  up/down results  esc cancel  ctrl+c quit"
            }
            SettingsTab::Settings if self.pending_reset_key.is_some() => {
                "enter confirm reset  esc cancel  q quit"
            }
            SettingsTab::Settings if self.expanded => {
                "h/l tabs  j/k navigate  enter collapse  esc close  q quit"
            }
            SettingsTab::Settings => {
                "h/l tabs  / search  space toggle  d reset  j/k navigate  g/G jump  esc back  q quit"
            }
            SettingsTab::Status | SettingsTab::Sources => "h/l tabs  esc back  q quit",
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
            Action::Select => {
                self.searching = false;
            }
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
            Action::Left => {
                self.tab = self.tab.shift(false);
                self.expanded = false;
                self.searching = false;
            }
            Action::Right => {
                self.tab = self.tab.shift(true);
                self.expanded = false;
                self.searching = false;
            }
            Action::Character('/') if self.tab == SettingsTab::Settings => {
                self.searching = true;
                self.search_used = true;
                self.search_query.clear();
                self.expanded = false;
            }
            Action::Character('g') | Action::Home => self.jump_first(),
            Action::Character('G') | Action::End => self.jump_last(),
            Action::Up => self.move_selection(false),
            Action::Down => self.move_selection(true),
            Action::Toggle => self.toggle_class_a(),
            Action::Character('d') => self.preview_reset(),
            Action::Select => {
                if self.pending_reset_key.is_some() {
                    self.confirm_reset();
                } else if self.reset_preview.is_some() && self.pending_write.is_some() {
                    let write = self.pending_write.clone().expect("pending");
                    self.reset_preview = None;
                    self.commit_write(write);
                } else if self.cycle_class_a_enum() {
                    // Enum edit applied through the settings service.
                } else if self.current_row().is_some() {
                    self.expanded = !self.expanded;
                }
            }
            Action::Back => {
                if self.pending_reset_key.is_some() {
                    self.pending_reset_key = None;
                    self.reset_preview = None;
                } else if self.expanded {
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
        RuntimeLabel::Active => format_active_values(row, state_word, &resolved),
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
        None => false,
        Some(active) => active == row.resolved_display,
    }
}

fn format_active_values(row: &SettingsRowView, state_word: &str, resolved: &str) -> String {
    match row.active_display.as_deref() {
        Some(active) => {
            let src = row.active_source_badge.as_deref().unwrap_or("current");
            format!("{state_word}  resolved: {resolved}  active: {active} [{src}]")
        }
        None => match evidence_annotation(row) {
            Some(evidence) => format!("{state_word}  resolved: {resolved}  {evidence}"),
            None => format!("{state_word}  resolved: {resolved}  active: (not observed)"),
        },
    }
}

fn evidence_annotation(row: &SettingsRowView) -> Option<&str> {
    row.detail.lines.iter().find_map(|line| {
        let trimmed = line.trim();
        trimmed.starts_with("evidence:").then_some(trimmed)
    })
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
    if let Some(scope) = &row.write_scope {
        parts.push(scope.as_str());
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
            class_a: false,
            write_scope: None,
            boolean_value: None,
            enum_allowed: Vec::new(),
            inherited: false,
            default_display: String::new(),
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
            "interface.compact",
            "Compact display",
            "Reduce spacing in the TUI",
            "Interface",
            "true",
            "user",
            RuntimeLabel::Active,
            true,
        );
        compact_mode.class_a = true;
        compact_mode.write_scope = Some("user".into());
        compact_mode.boolean_value = Some(true);
        compact_mode.inherited = false;
        compact_mode.default_display = "false".into();
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
    fn settings_space_toggles_class_a_boolean() {
        let mut state = sample_state();
        state.focus_key("interface.compact");
        state.handle_key(Action::Toggle);
        let row = state.current_row().unwrap();
        assert_eq!(row.boolean_value, Some(false));
        assert_eq!(
            state.pending_write,
            Some(PendingWrite::Toggle {
                key: "interface.compact".into()
            })
        );
    }

    #[test]
    fn settings_reset_preview_names_default_source() {
        let mut state = sample_state();
        state.focus_key("interface.compact");
        state.handle_key(Action::Character('d'));
        let preview = state.reset_preview.as_deref().expect("preview");
        assert!(preview.contains("interface.compact"), "{preview}");
        assert!(preview.contains("false"), "{preview}");
        assert!(preview.contains("[default]"), "{preview}");
        assert!(state.footer_commands().contains("false"));
        state.handle_key(Action::Select);
        let row = state.current_row().unwrap();
        assert_eq!(row.source_badge, "default");
        assert_eq!(
            state.pending_write,
            Some(PendingWrite::Reset {
                key: "interface.compact".into()
            })
        );
    }

    #[test]
    fn settings_write_scope_is_visible() {
        let state = sample_state();
        let compact = state
            .groups
            .iter()
            .flat_map(|group| group.rows.iter())
            .find(|row| row.key == "interface.compact")
            .unwrap();
        let rendered = format_row_values(compact);
        assert!(rendered.contains("user"), "{rendered}");
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
            Some("interface.compact")
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
    fn settings_view_select_while_searching_leaves_search_without_expanding() {
        let mut state = sample_state();
        state.handle_key(Action::Character('/'));
        for ch in "enforcement".chars() {
            state.handle_key(Action::Character(ch));
        }
        assert!(state.searching);
        assert!(!state.expanded);
        state.handle_key(Action::Select);
        assert!(!state.searching);
        assert!(!state.expanded);
        assert_eq!(state.search_query, "enforcement");
        assert_eq!(
            state.current_row().map(|row| row.key.as_str()),
            Some("protection.enforcement.mode")
        );
        state.handle_key(Action::Select);
        assert!(state.expanded);
        assert!(!state.searching);
    }

    #[test]
    fn settings_view_select_empty_search_does_not_open_blank_detail() {
        let mut state = sample_state();
        state.handle_key(Action::Character('/'));
        state.handle_key(Action::Select);
        assert!(!state.searching);
        assert!(!state.expanded);
    }

    #[test]
    fn settings_view_footer_includes_class_a_edit_commands() {
        let state = sample_state();
        let footer = state.footer_commands();
        assert!(footer.contains("space toggle"), "{footer}");
        assert!(footer.contains("d reset"), "{footer}");
        assert!(!footer.to_ascii_lowercase().contains("audit"));
    }

    #[test]
    fn settings_view_toggle_does_not_edit_class_c() {
        let mut state = sample_state();
        state.focus_key("protection.enforcement.mode");
        let before = state.current_row().unwrap().resolved_display.clone();
        state.handle_key(Action::Toggle);
        assert_eq!(state.current_row().unwrap().resolved_display, before);
        assert!(state.last_error.is_some());
    }

    #[test]
    fn settings_row_compact_only_when_resolved_and_active_agree() {
        let state = sample_state();
        let compact = state
            .groups
            .iter()
            .flat_map(|group| group.rows.iter())
            .find(|row| row.key == "interface.compact")
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
            .find(|row| row.key == "interface.compact")
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
    fn settings_row_active_without_observed_value_does_not_echo_resolved() {
        let row = sample_state()
            .groups
            .iter()
            .flat_map(|group| group.rows.iter())
            .find(|row| row.key == "project.schema_version")
            .cloned()
            .expect("schema row");
        assert!(row.active_display.is_none());
        let rendered = format_row_values(&row);
        assert!(rendered.contains("resolved: 1.0.0 [default]"), "{rendered}");
        assert!(!rendered.contains("active: 1.0.0"), "{rendered}");
        assert!(rendered.contains("active: (not observed)"), "{rendered}");
    }

    #[test]
    fn settings_row_active_without_observed_value_shows_evidence() {
        let mut row = sample_state()
            .groups
            .iter()
            .flat_map(|group| group.rows.iter())
            .find(|row| row.key == "privacy.api_token")
            .cloned()
            .expect("token row");
        row.runtime = RuntimeLabel::Active;
        row.compact = false;
        row.active_display = None;
        let rendered = format_row_values(&row);
        assert!(
            rendered.contains("resolved: [redacted] [env]"),
            "{rendered}"
        );
        assert!(!rendered.contains("active: [redacted]"), "{rendered}");
        assert!(
            rendered.contains("evidence: classified digest — lower evidence than a revealed value"),
            "{rendered}"
        );
        assert!(!rendered.contains("active: (not observed)"), "{rendered}");
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
            "h/l tabs  j/k navigate  enter collapse  esc close  q quit"
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

    #[test]
    fn settings_status_reports_health_and_posture() {
        let state = sample_state().with_status(SettingsStatusView {
            version: "0.10.0-beta".into(),
            runtime: "linux".into(),
            project: "anvil-001".into(),
            worktree: "feat/setins".into(),
            session: "sess-1".into(),
            resolved_posture: "warn".into(),
            attested_posture: "unknown".into(),
            health: "indeterminate".into(),
            health_reasons: vec!["protection.checks is unknown".into()],
            non_healthy: vec!["protection.checks unknown".into()],
            integrations: vec!["mcp: anvil".into()],
            adapters: vec!["grok".into()],
            pending: Some("restart intercept".into()),
            validation: "ok".into(),
            attestation_age: "n/a".into(),
            attestation_source: "none".into(),
        });
        assert_eq!(state.status.health, "indeterminate");
        assert_eq!(state.status.resolved_posture, "warn");
        assert_eq!(state.status.attested_posture, "unknown");
        assert!(
            state
                .status
                .non_healthy
                .iter()
                .any(|row| row.contains("unknown"))
        );
    }

    #[test]
    fn settings_sources_lists_revision_and_precedence() {
        let state = sample_state().with_sources(SettingsSourcesView {
            revision: "rev-1".into(),
            sources: vec![
                SettingsSourceRow {
                    scope: "org".into(),
                    path_display: "[redacted]".into(),
                    writable: false,
                    kind: "constraint".into(),
                },
                SettingsSourceRow {
                    scope: "project".into(),
                    path_display: ".anvil.yaml".into(),
                    writable: true,
                    kind: "precedence".into(),
                },
            ],
            overridden: vec!["org warn".into()],
            winning: vec!["project block".into()],
            field_provenance: vec!["checks[0]=project".into()],
            unknown_keys: vec!["legacy.foo".into()],
            deprecated_keys: vec!["ui.compact".into()],
        });
        assert_eq!(state.sources.revision, "rev-1");
        assert_eq!(state.sources.sources[0].kind, "constraint");
        assert!(state.sources.sources[1].writable);
        assert!(state.sources.unknown_keys.contains(&"legacy.foo".into()));
    }

    #[test]
    fn settings_status_h_l_switches_tabs() {
        let mut state = sample_state();
        assert_eq!(state.tab, SettingsTab::Settings);
        state.handle_key(Action::Right);
        assert_eq!(state.tab, SettingsTab::Status);
        state.handle_key(Action::Right);
        assert_eq!(state.tab, SettingsTab::Sources);
        state.handle_key(Action::Left);
        assert_eq!(state.tab, SettingsTab::Status);
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
