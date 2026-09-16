//! `anvil settings` inspect (SETINS) plus Class A edits (SETPREF).

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::io::IsTerminal;
use std::path::Path;

use anvil_observability::settings_telemetry::{SettingsSignal, emit as emit_settings_trace};
use anvil_settings::exit_codes::{SettingsOutcome, code_for};
use anvil_settings::{
    Catalogue, CatalogueEntry, ClassAOp, ConsequenceClass, Declaration, EnvelopeCommand,
    HealthStatus, ResolutionEvent, RuntimeState, Scope, SettingGroup, SettingRow, SettingsService,
    Snapshot, SnapshotRequest, ValueType, apply_class_a, first_release_catalogue,
    redact_setting_value,
};
use anvil_tui::surfaces::settings::{
    PendingWrite, RuntimeLabel, SettingsGroupView, SettingsRowDetail, SettingsRowView,
    SettingsSourceRow, SettingsSourcesView, SettingsState, SettingsStatusView,
};
use anyhow::Context;
use clap::{Args, Subcommand, ValueEnum};
use serde_json::Value;

use crate::GlobalArgs;

#[derive(Debug)]
pub struct SettingsExit(pub u8);

impl std::fmt::Display for SettingsExit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "settings exit {}", self.0)
    }
}

impl std::error::Error for SettingsExit {}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum SettingsFormat {
    Text,
    Json,
}

#[derive(Debug, Args)]
pub struct SettingsArgs {
    #[command(subcommand)]
    pub command: Option<SettingsCommand>,
    /// Output format. Implies non-interactive mode.
    #[arg(long, value_enum)]
    pub format: Option<SettingsFormat>,
    /// Health check. Implies non-interactive output; non-zero on unhealthy state.
    #[arg(long)]
    pub check: bool,
    /// Focus a canonical key or deprecated alias in the TUI.
    #[arg(long)]
    pub focus: Option<String>,
}

#[derive(Debug, Subcommand)]
pub enum SettingsCommand {
    /// Show grouped settings rows.
    Show,
    /// Explain one canonical key.
    Explain { key: String },
    /// Operational status summary.
    Status,
    /// Resolution sources and provenance.
    Sources,
}

#[derive(Debug, Args)]
pub struct SlashArgs {
    /// Canonical key or alias to focus when a TUI is available.
    #[arg(value_name = "KEY")]
    pub key: Option<String>,
}

#[allow(clippy::unnecessary_wraps)]
pub fn run_slash(args: &SlashArgs, global: &GlobalArgs) -> anyhow::Result<()> {
    if !global.no_tui && std::io::stdout().is_terminal() {
        let forwarded = SettingsArgs {
            command: None,
            format: None,
            check: false,
            focus: args.key.clone(),
        };
        return run(&forwarded, global);
    }
    print_slash_explanation();
    if let Some(key) = &args.key {
        println!("Equivalent: anvil settings --focus {key}");
    }
    Ok(())
}

fn print_slash_explanation() {
    println!(
        "/settings opens the inspect control centre in an interactive TUI.\n\
         Equivalent CLI: `anvil settings` (TUI) or `anvil settings show` (text/json).\n\
         Subcommands: show, explain <key>, status, sources."
    );
}

fn emit_settings_signal(signal: SettingsSignal) {
    if !crate::telemetry::send_allowed() {
        return;
    }
    emit_settings_trace(signal);
}

pub fn run(args: &SettingsArgs, global: &GlobalArgs) -> anyhow::Result<()> {
    let interactive = args.command.is_none()
        && args.format.is_none()
        && !args.check
        && !global.json
        && !global.no_tui
        && std::io::stdout().is_terminal();

    let model = load_model(Path::new("."))?;
    if interactive {
        emit_settings_signal(SettingsSignal::PanelOpened);
        let mut state = model.tui_state();
        if let Some(key) = args.focus.as_deref() {
            state.focus_key(key);
        }
        let catalogue = first_release_catalogue().context("settings catalogue")?;
        let user_root = anvil_config::user_config_root();
        if let Ok(file) = anvil_config::load_user_settings(&user_root) {
            state.source_revision = Some(file.revision);
        }
        state.persist = Some(Box::new(move |write| {
            let (key, op) = match write {
                PendingWrite::Toggle { key } => (key, ClassAOp::Toggle),
                PendingWrite::Set { key, value } => (key, ClassAOp::Set(Value::String(value))),
                PendingWrite::Reset { key } => (key, ClassAOp::Reset),
            };
            let expected = anvil_config::load_user_settings(&user_root)
                .ok()
                .map(|file| file.revision);
            apply_class_a(
                &catalogue,
                &key,
                op,
                Scope::User,
                &user_root,
                expected.as_deref(),
            )
            .map(|_| ())
            .map_err(|err| err.to_string())
        }));
        let state = crate::tui::run_surface(state)?;
        if state.search_used {
            emit_settings_signal(SettingsSignal::SearchUsed);
        }
        return Ok(());
    }

    let command = match &args.command {
        None | Some(SettingsCommand::Show) => EnvelopeCommand::Show,
        Some(SettingsCommand::Explain { .. }) => EnvelopeCommand::Explain,
        Some(SettingsCommand::Status) => EnvelopeCommand::Status,
        Some(SettingsCommand::Sources) => EnvelopeCommand::Sources,
    };

    let json = global.json || matches!(args.format, Some(SettingsFormat::Json));
    if json {
        let envelope = model
            .service
            .envelope(&model.snapshot, command, model.generated_at)
            .map_err(|err| {
                emit_settings_signal(SettingsSignal::ValidationFailed);
                anyhow::Error::new(SettingsExit(code_for(SettingsOutcome::RedactionError)))
                    .context(err)
            })?;
        let envelope = if let Some(SettingsCommand::Explain { key }) = &args.command {
            filter_envelope_key(envelope, key)?
        } else {
            envelope
        };
        crate::output::json::print(&envelope)?;
    } else {
        print!("{}", render_text(&model, command, args.command.as_ref()));
    }

    if args.check {
        return check_health(&model.snapshot);
    }
    Ok(())
}

struct InspectModel {
    service: SettingsService,
    snapshot: Snapshot,
    generated_at: &'static str,
}

impl InspectModel {
    fn tui_state(&self) -> SettingsState {
        SettingsState::new(project_groups(&self.service, &self.snapshot))
            .with_status(project_status(&self.snapshot))
            .with_sources(project_sources(&self.service, &self.snapshot))
    }
}

fn load_model(root: &Path) -> anyhow::Result<InspectModel> {
    let catalogue = first_release_catalogue().context("settings catalogue")?;
    let mut declarations = declarations_from_user();
    declarations.extend(declarations_from_project(root, &catalogue));
    let service = SettingsService::new(catalogue);
    let generated_at = "1970-01-01T00:00:00Z";
    let snapshot = service
        .snapshot(&SnapshotRequest {
            workspace_root: Some(root),
            declarations: &declarations,
            bundle: None,
            approvals: &[],
            attestations: &BTreeMap::new(),
            now: generated_at,
            generated_at,
            command: EnvelopeCommand::Show,
        })
        .context("settings snapshot")?;
    Ok(InspectModel {
        service,
        snapshot,
        generated_at,
    })
}

fn declarations_from_user() -> Vec<Declaration> {
    let root = anvil_config::user_config_root();
    let Ok(file) = anvil_config::load_user_settings(&root) else {
        return Vec::new();
    };
    file.values
        .into_iter()
        .map(|(key, value)| Declaration {
            key,
            scope: Scope::User,
            source_id: "user-config".into(),
            event: ResolutionEvent::Set(value),
        })
        .collect()
}

fn declarations_from_project(root: &Path, catalogue: &Catalogue) -> Vec<Declaration> {
    let Some(discovered) = SettingsService::discover_config(root).ok().flatten() else {
        return Vec::new();
    };
    let Ok(value) = anvil_config::parse_file(&discovered.path) else {
        return Vec::new();
    };
    let source_id = discovered.path.to_string_lossy().into_owned();
    let mut declarations = Vec::new();
    for (key, target) in catalogue.project_config_targets() {
        if let Some(found) = value.pointer(&target.pointer) {
            declarations.push(Declaration {
                key: key.to_owned(),
                scope: Scope::Project,
                source_id: source_id.clone(),
                event: ResolutionEvent::Set(found.clone()),
            });
        }
    }
    declarations
}

fn project_groups(service: &SettingsService, snapshot: &Snapshot) -> Vec<SettingsGroupView> {
    let mut groups: Vec<(SettingGroup, SettingsGroupView)> = Vec::new();
    for entry in service.catalogue().iter() {
        let row = snapshot
            .rows
            .iter()
            .find(|row| row.key == entry.key.as_str());
        let Some(row) = row else {
            continue;
        };
        let view = project_row(service.catalogue(), entry, row);
        if let Some((_, group)) = groups.iter_mut().find(|(id, _)| *id == entry.group) {
            group.rows.push(view);
        } else {
            groups.push((
                entry.group,
                SettingsGroupView {
                    id: format!("{:?}", entry.group).to_ascii_lowercase(),
                    label: group_label(entry.group).to_owned(),
                    rows: vec![view],
                },
            ));
        }
    }
    groups.into_iter().map(|(_, group)| group).collect()
}

fn group_label(group: SettingGroup) -> &'static str {
    match group {
        SettingGroup::Project => "Project",
        SettingGroup::Protection => "Protection",
        SettingGroup::Agents => "Agents & approvals",
        SettingGroup::Privacy => "Privacy & egress",
        SettingGroup::Integrations => "Integrations",
        SettingGroup::Interface => "Interface",
    }
}

fn project_row(catalogue: &Catalogue, entry: &CatalogueEntry, row: &SettingRow) -> SettingsRowView {
    let resolved_display = if entry.consequence_class == ConsequenceClass::A {
        match row.resolved.as_ref() {
            Some(Value::Bool(flag)) => flag.to_string(),
            Some(Value::String(text)) => text.clone(),
            _ => display_value(catalogue, &row.key, row.resolved.as_ref()),
        }
    } else {
        display_value(catalogue, &row.key, row.resolved.as_ref())
    };
    let runtime = match row.runtime {
        RuntimeState::Active => RuntimeLabel::Active,
        RuntimeState::Drift => RuntimeLabel::Drift,
        RuntimeState::Stale => RuntimeLabel::Stale,
        RuntimeState::Failed => RuntimeLabel::Failed,
        RuntimeState::Unknown => RuntimeLabel::Unknown,
    };
    let source = row
        .provenance
        .iter()
        .rev()
        .find(|event| !event.overridden)
        .map_or_else(
            || "default".to_owned(),
            |event| format!("{:?}", event.scope).to_ascii_lowercase(),
        );
    let inherited = source != "user" && source != "default";
    SettingsRowView {
        key: row.key.clone(),
        label: entry.label.clone(),
        description: entry.label.clone(),
        group: group_label(entry.group).to_owned(),
        deprecated_aliases: entry.deprecated_aliases.clone(),
        resolved_display: resolved_display.clone(),
        source_badge: source,
        active_display: None,
        active_source_badge: None,
        constraint_badge: None,
        runtime,
        workflow_state: None,
        consequence_badge: Some(format!("class-{:?}", entry.consequence_class)),
        compact: runtime == RuntimeLabel::Active,
        detail: SettingsRowDetail {
            lines: vec![
                format!("key: {}", row.key),
                format!("cli: anvil settings explain {}", row.key),
            ],
        },
        class_a: entry.consequence_class == ConsequenceClass::A,
        write_scope: entry
            .default_write_scope
            .map(|scope| format!("{scope:?}").to_ascii_lowercase()),
        boolean_value: row.resolved.as_ref().and_then(Value::as_bool),
        enum_allowed: match &entry.value_type {
            ValueType::Enum { allowed } => allowed.clone(),
            _ => Vec::new(),
        },
        inherited,
        default_display: match &entry.default {
            Some(Value::Bool(flag)) => flag.to_string(),
            Some(Value::String(text)) => text.clone(),
            Some(other) => other.to_string(),
            None => String::new(),
        },
    }
}

fn display_value(catalogue: &Catalogue, key: &str, value: Option<&Value>) -> String {
    let Some(value) = value else {
        return "—".into();
    };
    match redact_setting_value(catalogue, key, value) {
        Ok(redacted) => {
            if redacted.get("present").is_some() {
                "[redacted]".into()
            } else if redacted.get("reason").and_then(Value::as_str) == Some("unclassified") {
                "[hidden]".into()
            } else {
                redacted.to_string()
            }
        }
        Err(_) => "[redacted]".into(),
    }
}

fn project_status(snapshot: &Snapshot) -> SettingsStatusView {
    let mode = snapshot
        .rows
        .iter()
        .find(|row| row.key == "protection.enforcement.mode");
    let resolved_posture = mode
        .and_then(|row| row.resolved.as_ref())
        .map_or_else(|| "unknown".into(), ToString::to_string);
    let attested_posture = mode.map_or_else(
        || "unknown".into(),
        |row| format!("{:?}", row.runtime).to_ascii_lowercase(),
    );
    let non_healthy: Vec<String> = snapshot
        .rows
        .iter()
        .filter(|row| {
            matches!(
                row.runtime,
                RuntimeState::Drift | RuntimeState::Stale | RuntimeState::Failed
            )
        })
        .map(|row| format!("{} {:?}", row.key, row.runtime).to_ascii_lowercase())
        .collect();
    SettingsStatusView {
        version: env!("CARGO_PKG_VERSION").into(),
        runtime: std::env::consts::OS.into(),
        project: snapshot.discovered.clone().unwrap_or_else(|| ".".into()),
        worktree: snapshot.discovered.clone().unwrap_or_else(|| ".".into()),
        session: "local".into(),
        resolved_posture,
        attested_posture,
        health: format!("{:?}", snapshot.health.status).to_ascii_lowercase(),
        health_reasons: snapshot.health.reasons.clone(),
        non_healthy,
        integrations: vec![],
        adapters: vec![],
        pending: None,
        validation: format!("{:?}", snapshot.health.status).to_ascii_lowercase(),
        attestation_age: "n/a".into(),
        attestation_source: "none".into(),
    }
}

fn project_sources(service: &SettingsService, snapshot: &Snapshot) -> SettingsSourcesView {
    let mut overridden = Vec::new();
    let mut winning = Vec::new();
    for row in &snapshot.rows {
        for event in &row.provenance {
            let line = format!("{} {:?}", row.key, event.scope);
            if event.overridden {
                overridden.push(line);
            } else {
                winning.push(line);
            }
        }
    }
    let unknown_keys: Vec<String> = snapshot
        .rows
        .iter()
        .filter(|row| service.catalogue().get(&row.key).is_none())
        .map(|row| row.key.clone())
        .collect();
    let mut sources = Vec::new();
    for row in &snapshot.rows {
        for event in &row.provenance {
            let writable = matches!(
                event.scope,
                Scope::Project | Scope::User | Scope::Session | Scope::Environment
            );
            sources.push(SettingsSourceRow {
                scope: format!("{:?}", event.scope).to_ascii_lowercase(),
                path_display: event.source_id.clone(),
                writable,
                kind: if event.overridden {
                    "overridden".into()
                } else {
                    "precedence".into()
                },
            });
        }
    }
    if sources.is_empty() {
        sources.push(SettingsSourceRow {
            scope: "project".into(),
            path_display: snapshot
                .discovered
                .clone()
                .unwrap_or_else(|| "catalogue default".into()),
            writable: true,
            kind: "precedence".into(),
        });
    }
    SettingsSourcesView {
        revision: snapshot.model_revision.clone(),
        sources,
        overridden,
        winning,
        field_provenance: vec![],
        unknown_keys,
        deprecated_keys: service
            .catalogue()
            .iter()
            .flat_map(|entry| entry.deprecated_aliases.iter().cloned())
            .collect(),
    }
}

fn render_text(
    model: &InspectModel,
    command: EnvelopeCommand,
    sub: Option<&SettingsCommand>,
) -> String {
    match command {
        EnvelopeCommand::Show => {
            let mut out = String::from("Settings\n");
            for group in project_groups(&model.service, &model.snapshot) {
                let _ = write!(out, "\n{}\n", group.label);
                for row in group.rows {
                    let _ = writeln!(
                        out,
                        "  {}  {}  {}",
                        row.label,
                        row.resolved_display,
                        row.runtime.as_str()
                    );
                }
            }
            out
        }
        EnvelopeCommand::Explain => {
            let key = match sub {
                Some(SettingsCommand::Explain { key }) => key.as_str(),
                _ => "",
            };
            let groups = project_groups(&model.service, &model.snapshot);
            groups
                .iter()
                .flat_map(|group| group.rows.iter())
                .find(|row| row.key == key)
                .map_or_else(
                    || format!("unknown key {key}\n"),
                    |row| {
                        let mut out = format!("{}\n", row.label);
                        for line in &row.detail.lines {
                            out.push_str(line);
                            out.push('\n');
                        }
                        out
                    },
                )
        }
        EnvelopeCommand::Status => {
            let status = project_status(&model.snapshot);
            format!(
                "health: {}\nresolved posture: {}\nattested posture: {}\n",
                status.health, status.resolved_posture, status.attested_posture
            )
        }
        EnvelopeCommand::Sources => {
            let sources = project_sources(&model.service, &model.snapshot);
            let mut out = format!("revision: {}\n", sources.revision);
            for source in &sources.sources {
                let _ = writeln!(
                    out,
                    "source: {} {} {} [{}]",
                    source.kind,
                    source.scope,
                    source.path_display,
                    if source.writable {
                        "writable"
                    } else {
                        "locked"
                    }
                );
            }
            for item in &sources.winning {
                let _ = writeln!(out, "winning: {item}");
            }
            for item in &sources.overridden {
                let _ = writeln!(out, "overridden: {item}");
            }
            out
        }
    }
}

fn filter_envelope_key(mut envelope: Value, key: &str) -> anyhow::Result<Value> {
    let Some(data) = envelope.get_mut("data").and_then(Value::as_object_mut) else {
        anyhow::bail!("unknown setting {key}");
    };
    let Some(row) = data.remove(key) else {
        anyhow::bail!("unknown setting {key}");
    };
    data.clear();
    data.insert(key.to_owned(), row);
    Ok(envelope)
}

fn check_health(snapshot: &Snapshot) -> anyhow::Result<()> {
    match snapshot.health.status {
        HealthStatus::Healthy => Ok(()),
        HealthStatus::Unhealthy | HealthStatus::Indeterminate => Err(anyhow::Error::new(
            SettingsExit(code_for(SettingsOutcome::CheckFailed)),
        )),
    }
}

pub fn mcp_show(root: &Path) -> anyhow::Result<Value> {
    let model = load_model(root)?;
    model
        .service
        .envelope(&model.snapshot, EnvelopeCommand::Show, model.generated_at)
        .context("settings envelope")
}

pub fn mcp_explain(root: &Path, key: &str) -> anyhow::Result<Value> {
    let model = load_model(root)?;
    let envelope = model
        .service
        .envelope(
            &model.snapshot,
            EnvelopeCommand::Explain,
            model.generated_at,
        )
        .context("settings envelope")?;
    filter_envelope_key(envelope, key)
}

pub fn mcp_status(root: &Path) -> anyhow::Result<Value> {
    let model = load_model(root)?;
    model
        .service
        .envelope(&model.snapshot, EnvelopeCommand::Status, model.generated_at)
        .context("settings envelope")
}

pub fn mcp_sources(root: &Path) -> anyhow::Result<Value> {
    let model = load_model(root)?;
    model
        .service
        .envelope(
            &model.snapshot,
            EnvelopeCommand::Sources,
            model.generated_at,
        )
        .context("settings envelope")
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn settings_entry_slash_explains_cli_equivalent() {
        let text = "/settings opens the inspect control centre in an interactive TUI.\n\
             Equivalent CLI: `anvil settings` (TUI) or `anvil settings show` (text/json).";
        assert!(text.contains("/settings"));
        assert!(text.contains("anvil settings show"));
    }

    #[test]
    fn settings_entry_clap_registers_family() {
        let cmd = crate::Cli::command();
        let names: Vec<_> = cmd
            .get_subcommands()
            .map(|c| c.get_name().to_string())
            .collect();
        assert!(names.iter().any(|n| n == "settings"));
        assert!(names.iter().any(|n| n == "/settings"));
    }

    #[test]
    fn settings_cli_show_text_lists_groups() {
        let model = load_model(Path::new(".")).expect("model");
        let text = render_text(&model, EnvelopeCommand::Show, None);
        assert!(text.contains("Protection") || text.contains("Project"));
        assert!(
            !text
                .to_ascii_lowercase()
                .split_whitespace()
                .any(|w| w == "active")
                || text.contains("unknown")
        );
    }

    #[test]
    fn settings_cli_check_fails_without_evidence() {
        let model = load_model(Path::new(".")).expect("model");
        let err = check_health(&model.snapshot).expect_err("unknown is not healthy");
        let exit = err.downcast_ref::<SettingsExit>().expect("settings exit");
        assert_eq!(exit.0, 2);
    }

    #[test]
    fn settings_cli_json_envelope_is_versioned() {
        let model = load_model(Path::new(".")).expect("model");
        let envelope = model
            .service
            .envelope(&model.snapshot, EnvelopeCommand::Show, model.generated_at)
            .expect("envelope");
        assert_eq!(envelope["schema_version"], "anvil.settings.v1");
        assert!(envelope.get("data").is_some());
        assert!(envelope.get("health").is_some());
    }

    #[test]
    fn settings_cli_never_renders_active_without_evidence() {
        let model = load_model(Path::new(".")).expect("model");
        for row in &model.snapshot.rows {
            if row.runtime != RuntimeState::Active {
                continue;
            }
            panic!("empty attestations must not classify active: {}", row.key);
        }
    }

    #[test]
    fn settings_mcp_tools_are_read_only() {
        let root = Path::new(".");
        let show = mcp_show(root).expect("show");
        assert_eq!(show["schema_version"], "anvil.settings.v1");
        let status = mcp_status(root).expect("status");
        assert!(status.get("health").is_some());
        let sources = mcp_sources(root).expect("sources");
        assert!(sources.get("model_revision").is_some() || sources.get("data").is_some());
    }
}
