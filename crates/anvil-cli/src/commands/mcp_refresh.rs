//! `anvil mcp refresh` — config rewrite, daemon recycle, generation poke.
//!
//! Cascade (spec §9.2): CONFIG → DAEMON → SIGNAL → REPORT. Default process
//! mode is report-only. `--processes orphan-reap` SIGTERMs same-user orphans
//! whose parent is gone. Live parents' MCP children are never signalled.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use clap::{Args, ValueEnum};
use serde::Serialize;
use serde_json::Value;

use crate::GlobalArgs;
use crate::activation::agent_registry::{AgentClientId, InstallScope, McpConfigKind};
use crate::activation::detect_agents::RealDetectionEnv;
use crate::activation::mcp_client::preferred_mcp_command;
use crate::commands::daemon_recycle::{
    DaemonRecycleHooks, DaemonRecycleOutcome, recycle_daemon_if_version_skew,
};
use crate::commands::mcp_config::default_client_config_root;
use crate::commands::mcp_generation::{bump_generation, generation_path, read_generation};
use crate::commands::mcp_installer;
use crate::commands::mcp_inventory::{
    NoopSignals, ProcessInventory, ProcessMode, ProcessSignalSink, apply_process_mode,
    collect_inventory,
};

#[cfg(unix)]
use crate::commands::mcp_inventory::UnixTermSignals;
use crate::mcp::reexec::resolve_preferred_executable;

const CLI_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Args)]
pub struct McpRefreshArgs {
    /// Preview config, daemon, and generation actions without mutating them.
    #[arg(long)]
    pub dry_run: bool,

    /// Clients to refresh: `all`, `detected`, or one or more registry ids.
    #[arg(long, value_delimiter = ',', num_args = 1.., default_value = "all")]
    pub clients: Vec<String>,

    /// How to treat a running intercept daemon.
    #[arg(long, value_enum, default_value_t = DaemonMode::Auto)]
    pub daemon: DaemonMode,

    /// Live MCP child policy. `report` lists by parent (default); `orphan-reap`
    /// SIGTERMs same-user orphans (parent gone); `none` skips the scan.
    #[arg(long, default_value = "report")]
    pub processes: String,

    /// Override the client config root. Defaults to the user home (global)
    /// and the current directory (project).
    #[arg(long)]
    pub workspace: Option<PathBuf>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum DaemonMode {
    Auto,
    Restart,
    Reuse,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ConfigAction {
    client: String,
    scope: String,
    path: String,
    command: String,
    drifted: bool,
    action: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct DaemonReport {
    cli_version: String,
    version: Option<String>,
    skew: bool,
    action: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct GenerationReport {
    path: String,
    value: u64,
    bumped: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct RefreshReport {
    ok: bool,
    dry_run: bool,
    config: Vec<ConfigAction>,
    daemon: DaemonReport,
    processes: ProcessInventory,
    generation: GenerationReport,
}

pub fn run(args: &McpRefreshArgs, global: &GlobalArgs) -> Result<()> {
    let process_mode = parse_process_mode(&args.processes)?;
    #[cfg(any(unix, windows))]
    let hooks = crate::commands::daemon_recycle::LiveDaemonRecycleHooks;
    #[cfg(not(any(unix, windows)))]
    let hooks = UnsupportedDaemonHooks;
    let mut signals = production_signal_sink(process_mode, args.dry_run);
    let report = refresh(args, process_mode, CLI_VERSION, &hooks, signals.as_mut())?;
    emit_report(&report, global.json)?;
    if report.ok {
        Ok(())
    } else {
        bail!("mcp refresh completed with errors")
    }
}

fn production_signal_sink(mode: ProcessMode, dry_run: bool) -> Box<dyn ProcessSignalSink> {
    match (mode, dry_run) {
        (ProcessMode::OrphanReap, false) => {
            #[cfg(unix)]
            {
                Box::new(UnixTermSignals)
            }
            #[cfg(not(unix))]
            {
                Box::new(NoopSignals)
            }
        }
        _ => Box::new(NoopSignals),
    }
}

fn parse_process_mode(raw: &str) -> Result<ProcessMode> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "report" => Ok(ProcessMode::Report),
        "none" => Ok(ProcessMode::None),
        "orphan-reap" => Ok(ProcessMode::OrphanReap),
        "force-skewed" => {
            bail!(
                "--processes force-skewed is not offered (forbidden as a default; \
                 it would signal children of live parents)"
            )
        }
        other => {
            bail!("unknown --processes value `{other}`; expected report, orphan-reap, or none")
        }
    }
}

fn refresh(
    args: &McpRefreshArgs,
    process_mode: ProcessMode,
    cli_version: &str,
    hooks: &dyn DaemonRecycleHooks,
    signals: &mut dyn ProcessSignalSink,
) -> Result<RefreshReport> {
    let clients = resolve_clients(&args.clients)?;
    let (home, project) = config_roots(args.workspace.as_deref())?;
    let selected = select_clients(&clients, &home, &project);
    let command = preferred_mcp_command(None);

    let mut ok = true;
    let mut config = Vec::new();
    let mut seen_paths = BTreeSet::new();
    for client in selected {
        let adapter = client.entry();
        let Some(kind) = adapter.mcp_kind else {
            continue;
        };
        for (scope, root) in [
            (InstallScope::Global, home.as_path()),
            (InstallScope::Project, project.as_path()),
        ] {
            let Some(path) = adapter.mcp_path(scope, root) else {
                continue;
            };
            if !seen_paths.insert(path.clone()) {
                continue;
            }
            if let Some(action) =
                refresh_client(client, scope, kind, &path, root, command, args.dry_run)
            {
                if action.action.starts_with("error") {
                    ok = false;
                }
                config.push(action);
            }
        }
    }

    let daemon = refresh_daemon(args.daemon, args.dry_run, cli_version, hooks);
    if daemon.action.starts_with("failed") {
        ok = false;
    }

    let generation = refresh_generation(args.dry_run)?;

    let preferred = resolve_preferred_executable(None, std::env::var_os("PATH").as_deref());
    let scanned = collect_inventory(process_mode, preferred.as_deref());
    let processes = apply_process_mode(process_mode, scanned, signals, args.dry_run);

    Ok(RefreshReport {
        ok,
        dry_run: args.dry_run,
        config,
        daemon,
        processes,
        generation,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ClientSelection {
    All,
    Detected,
    Explicit,
}

fn resolve_clients(raw: &[String]) -> Result<(ClientSelection, Vec<AgentClientId>)> {
    let tokens: Vec<String> = raw
        .iter()
        .flat_map(|value| value.split(','))
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_ascii_lowercase)
        .collect();
    if tokens.is_empty() || tokens.iter().all(|token| token == "all") {
        return Ok((ClientSelection::All, Vec::new()));
    }
    if tokens.iter().all(|token| token == "detected") {
        return Ok((ClientSelection::Detected, Vec::new()));
    }
    if tokens
        .iter()
        .any(|token| token == "all" || token == "detected")
    {
        bail!("--clients cannot mix all/detected with client ids");
    }
    let mut ids = Vec::new();
    for token in tokens {
        ids.push(parse_client_id(&token)?);
    }
    Ok((ClientSelection::Explicit, ids))
}

fn parse_client_id(raw: &str) -> Result<AgentClientId> {
    use clap::ValueEnum;
    AgentClientId::from_str(raw, true).map_err(|_| anyhow::anyhow!("unknown MCP client `{raw}`"))
}

fn config_roots(workspace: Option<&Path>) -> Result<(PathBuf, PathBuf)> {
    if let Some(workspace) = workspace {
        return Ok((workspace.to_path_buf(), workspace.to_path_buf()));
    }
    let home = default_client_config_root()?;
    let project = std::env::current_dir().context("resolving current directory")?;
    Ok((home, project))
}

fn select_clients(
    selection: &(ClientSelection, Vec<AgentClientId>),
    home: &Path,
    project: &Path,
) -> Vec<AgentClientId> {
    let env = RealDetectionEnv;
    AgentClientId::all()
        .iter()
        .filter(|entry| entry.mcp_kind.is_some())
        .filter(|entry| match selection.0 {
            ClientSelection::All => true,
            ClientSelection::Detected => {
                entry.detected_for_mcp(&env, InstallScope::Global, home)
                    || entry.detected_for_mcp(&env, InstallScope::Project, project)
            }
            ClientSelection::Explicit => selection.1.contains(&entry.id),
        })
        .map(|entry| entry.id)
        .collect()
}

fn refresh_client(
    client: AgentClientId,
    scope: InstallScope,
    kind: McpConfigKind,
    path: &Path,
    root: &Path,
    command: &str,
    dry_run: bool,
) -> Option<ConfigAction> {
    if !has_anvil_entry(kind, path) {
        return None;
    }
    match mcp_installer::install(client, scope, root, command, false, dry_run) {
        Ok(report) => {
            let action = if report.changed && dry_run {
                "would-rewrite"
            } else if report.wrote {
                "rewrote"
            } else {
                "unchanged"
            };
            Some(ConfigAction {
                client: client.label().to_owned(),
                scope: scope.label().to_owned(),
                path: report.path.display().to_string(),
                command: command.to_owned(),
                drifted: report.drifted,
                action: action.to_owned(),
            })
        }
        Err(error) => {
            let message = error.to_string();
            if message.contains("user-owned") || message.contains("foreign") {
                return Some(ConfigAction {
                    client: client.label().to_owned(),
                    scope: scope.label().to_owned(),
                    path: path.display().to_string(),
                    command: command.to_owned(),
                    drifted: true,
                    action: "skipped-foreign".to_owned(),
                });
            }
            Some(ConfigAction {
                client: client.label().to_owned(),
                scope: scope.label().to_owned(),
                path: path.display().to_string(),
                command: command.to_owned(),
                drifted: false,
                action: format!("error: {message}"),
            })
        }
    }
}

fn has_anvil_entry(kind: McpConfigKind, path: &Path) -> bool {
    let Ok(raw) = fs::read_to_string(path) else {
        return false;
    };
    if raw.trim().is_empty() {
        return false;
    }
    match kind {
        McpConfigKind::CodexToml | McpConfigKind::GrokToml => toml::from_str::<toml::Value>(&raw)
            .ok()
            .and_then(|parsed| parsed.get("mcp_servers")?.get("anvil").cloned())
            .is_some(),
        _ => serde_json::from_str::<Value>(&raw)
            .ok()
            .and_then(|parsed| json_anvil_entry(kind, &parsed).cloned())
            .is_some(),
    }
}

fn json_anvil_entry(kind: McpConfigKind, root: &Value) -> Option<&Value> {
    match kind {
        McpConfigKind::McpServersJson => root.pointer("/mcpServers/anvil"),
        McpConfigKind::ServersJson => root.pointer("/servers/anvil"),
        McpConfigKind::OpenCodeJson => root.pointer("/mcp/anvil"),
        McpConfigKind::ZedContextServersJson => root.pointer("/context_servers/anvil"),
        McpConfigKind::OpenClawJson => root.pointer("/mcp/servers/anvil"),
        McpConfigKind::CodexToml | McpConfigKind::GrokToml => None,
    }
}

fn refresh_daemon(
    mode: DaemonMode,
    dry_run: bool,
    cli_version: &str,
    hooks: &dyn DaemonRecycleHooks,
) -> DaemonReport {
    let running = hooks.running_daemon();
    let version = running.as_ref().map(|daemon| daemon.version.clone());
    let skew = version
        .as_deref()
        .is_some_and(|running_version| running_version != cli_version);

    match mode {
        DaemonMode::Reuse => DaemonReport {
            cli_version: cli_version.to_owned(),
            version: version.clone(),
            skew,
            action: if version.is_some() {
                "reused".to_owned()
            } else {
                "not-running".to_owned()
            },
        },
        DaemonMode::Auto if dry_run => DaemonReport {
            cli_version: cli_version.to_owned(),
            version: version.clone(),
            skew,
            action: if version.is_none() {
                "not-running".to_owned()
            } else if skew {
                "would-recycle".to_owned()
            } else {
                "reused".to_owned()
            },
        },
        DaemonMode::Restart if dry_run => DaemonReport {
            cli_version: cli_version.to_owned(),
            version: version.clone(),
            skew,
            action: if version.is_some() {
                "would-recycle".to_owned()
            } else {
                "not-running".to_owned()
            },
        },
        DaemonMode::Auto => report_recycle_outcome(
            cli_version,
            recycle_daemon_if_version_skew(cli_version, hooks),
        ),
        DaemonMode::Restart => report_recycle_outcome(cli_version, force_recycle(hooks)),
    }
}

fn force_recycle(hooks: &dyn DaemonRecycleHooks) -> DaemonRecycleOutcome {
    let Some(running) = hooks.running_daemon() else {
        return DaemonRecycleOutcome::NotRunning;
    };
    // Force stop → wait → start even when versions already match.
    force_recycle_running(hooks, running.version)
}

fn force_recycle_running(hooks: &dyn DaemonRecycleHooks, before: String) -> DaemonRecycleOutcome {
    let pid = match hooks.stop_daemon() {
        Ok(Some(pid)) => pid,
        Ok(None) => {
            return if hooks.running_daemon().is_none() {
                DaemonRecycleOutcome::NotRunning
            } else {
                DaemonRecycleOutcome::Failed {
                    before: Some(before),
                    recovery: "could not stop the daemon (no PID file); \
                               run `anvil intercept stop` then `anvil start`"
                        .to_owned(),
                }
            };
        }
        Err(recovery) => {
            return DaemonRecycleOutcome::Failed {
                before: Some(before),
                recovery,
            };
        }
    };
    if let Err(recovery) = hooks.wait_for_pid_exit(pid) {
        return DaemonRecycleOutcome::Failed {
            before: Some(before),
            recovery,
        };
    }
    match hooks.start_current_binary() {
        Ok(after) => DaemonRecycleOutcome::Recycled { before, after },
        Err(recovery) => DaemonRecycleOutcome::Failed {
            before: Some(before),
            recovery,
        },
    }
}

fn report_recycle_outcome(cli_version: &str, outcome: DaemonRecycleOutcome) -> DaemonReport {
    match outcome {
        DaemonRecycleOutcome::Skipped { version } => DaemonReport {
            cli_version: cli_version.to_owned(),
            version: Some(version),
            skew: false,
            action: "reused".to_owned(),
        },
        DaemonRecycleOutcome::NotRunning => DaemonReport {
            cli_version: cli_version.to_owned(),
            version: None,
            skew: false,
            action: "not-running".to_owned(),
        },
        DaemonRecycleOutcome::Recycled { before, after } => DaemonReport {
            cli_version: cli_version.to_owned(),
            version: Some(after.clone()),
            skew: false,
            action: format!("recycled ({before} → {after})"),
        },
        DaemonRecycleOutcome::Failed { before, recovery } => DaemonReport {
            cli_version: cli_version.to_owned(),
            version: before,
            skew: true,
            action: format!("failed:{recovery}"),
        },
    }
}

fn refresh_generation(dry_run: bool) -> Result<GenerationReport> {
    let path = generation_path()?;
    let current = read_generation(&path)?;
    if dry_run {
        return Ok(GenerationReport {
            path: path.display().to_string(),
            value: current,
            bumped: false,
        });
    }
    let value = bump_generation(&path)?;
    Ok(GenerationReport {
        path: path.display().to_string(),
        value,
        bumped: true,
    })
}

fn emit_report(report: &RefreshReport, json_mode: bool) -> Result<()> {
    if json_mode {
        println!("{}", serde_json::to_string_pretty(report)?);
        return Ok(());
    }
    println!(
        "MCP refresh{}",
        if report.dry_run { " (dry-run)" } else { "" }
    );
    println!("Config:");
    if report.config.is_empty() {
        println!("  no Anvil-owned MCP entries found");
    } else {
        for action in &report.config {
            println!(
                "  {} ({}): {} {} [{}]",
                action.client, action.scope, action.action, action.path, action.command
            );
        }
    }
    let daemon_version = report.daemon.version.as_deref().unwrap_or("none");
    println!(
        "Daemon: {} (cli {}, daemon {daemon_version})",
        report.daemon.action, report.daemon.cli_version
    );
    if report.generation.bumped {
        println!(
            "Generation: bumped to {} ({})",
            report.generation.value, report.generation.path
        );
    } else if report.dry_run {
        println!(
            "Generation: would bump from {} ({})",
            report.generation.value, report.generation.path
        );
    } else {
        println!(
            "Generation: {} ({})",
            report.generation.value, report.generation.path
        );
    }
    println!(
        "Processes ({}): {} total, {} skewed, {} current, {} orphan; signalled {}",
        report.processes.mode,
        report.processes.total,
        report.processes.skewed,
        report.processes.current,
        report.processes.orphan,
        report.processes.signalled
    );
    for group in &report.processes.by_parent {
        println!(
            "  {}: pids {:?} — {} skewed, {} current, {} orphan",
            group.command,
            group.parent_pids,
            group.skewed_children,
            group.current_children,
            group.orphan_children
        );
    }
    if report.processes.skewed > 0 {
        println!(
            "Anvil tried to recycle MCP in place. Reconnect MCP only for a parent \
             that still runs a stale image after the next tool call."
        );
    }
    Ok(())
}

#[cfg(not(any(unix, windows)))]
struct UnsupportedDaemonHooks;

#[cfg(not(any(unix, windows)))]
impl DaemonRecycleHooks for UnsupportedDaemonHooks {
    fn running_daemon(&self) -> Option<crate::commands::daemon_recycle::RunningDaemon> {
        None
    }
    fn stop_daemon(&self) -> Result<Option<u32>, String> {
        Ok(None)
    }
    fn wait_for_pid_exit(&self, _pid: u32) -> Result<(), String> {
        Ok(())
    }
    fn start_current_binary(&self) -> Result<String, String> {
        Err("daemon recycle is not available on this platform".into())
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use crate::commands::daemon_recycle::{DaemonRecycleHooks, RunningDaemon};
    use crate::commands::mcp_inventory::{ProcessMode, ProcessSignalSink};

    use super::{DaemonMode, refresh_daemon};

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum RecycleCall {
        Stop,
        Wait(u32),
        Start,
    }

    struct RecordingHooks {
        running: Option<RunningDaemon>,
        stop_pid: Option<u32>,
        start_after: Result<String, String>,
        calls: RefCell<Vec<RecycleCall>>,
    }

    impl RecordingHooks {
        fn skewed() -> Self {
            Self {
                running: Some(RunningDaemon {
                    version: "0.5.1-beta".into(),
                }),
                stop_pid: Some(4242),
                start_after: Ok("0.9.2-beta".into()),
                calls: RefCell::new(Vec::new()),
            }
        }

        fn matching() -> Self {
            Self {
                running: Some(RunningDaemon {
                    version: "0.9.2-beta".into(),
                }),
                stop_pid: Some(4242),
                start_after: Ok("0.9.2-beta".into()),
                calls: RefCell::new(Vec::new()),
            }
        }

        fn calls(&self) -> Vec<RecycleCall> {
            self.calls.borrow().clone()
        }
    }

    impl DaemonRecycleHooks for RecordingHooks {
        fn running_daemon(&self) -> Option<RunningDaemon> {
            self.running.clone()
        }

        fn stop_daemon(&self) -> Result<Option<u32>, String> {
            self.calls.borrow_mut().push(RecycleCall::Stop);
            Ok(self.stop_pid)
        }

        fn wait_for_pid_exit(&self, pid: u32) -> Result<(), String> {
            self.calls.borrow_mut().push(RecycleCall::Wait(pid));
            Ok(())
        }

        fn start_current_binary(&self) -> Result<String, String> {
            self.calls.borrow_mut().push(RecycleCall::Start);
            self.start_after.clone()
        }
    }

    struct RecordingSink {
        pids: Vec<u32>,
    }

    impl ProcessSignalSink for RecordingSink {
        fn signal(&mut self, pid: u32) {
            self.pids.push(pid);
        }
    }

    #[test]
    fn daemon_auto_recycles_on_skew() {
        let hooks = RecordingHooks::skewed();
        let report = refresh_daemon(DaemonMode::Auto, false, "0.9.2-beta", &hooks);
        assert!(report.action.starts_with("recycled"), "{:?}", report.action);
        assert_eq!(
            hooks.calls(),
            vec![
                RecycleCall::Stop,
                RecycleCall::Wait(4242),
                RecycleCall::Start
            ]
        );
    }

    #[test]
    fn daemon_auto_skips_when_versions_match() {
        let hooks = RecordingHooks::matching();
        let report = refresh_daemon(DaemonMode::Auto, false, "0.9.2-beta", &hooks);
        assert_eq!(report.action, "reused");
        assert!(
            hooks.calls().is_empty(),
            "matching versions must not recycle: {:?}",
            hooks.calls()
        );
    }

    #[test]
    fn daemon_dry_run_never_recycles_on_skew() {
        let hooks = RecordingHooks::skewed();
        let report = refresh_daemon(DaemonMode::Auto, true, "0.9.2-beta", &hooks);
        assert_eq!(report.action, "would-recycle");
        assert!(hooks.calls().is_empty());
    }

    #[test]
    fn processes_report_sink_stays_empty() {
        let inventory = crate::commands::mcp_inventory::empty_inventory(ProcessMode::Report);
        let mut sink = RecordingSink { pids: Vec::new() };
        let reported = crate::commands::mcp_inventory::apply_process_mode(
            ProcessMode::Report,
            inventory,
            &mut sink,
            false,
        );
        assert!(sink.pids.is_empty());
        assert_eq!(reported.signalled, 0);
    }

    #[test]
    fn parse_process_mode_accepts_orphan_reap() {
        assert_eq!(
            super::parse_process_mode("orphan-reap").unwrap(),
            ProcessMode::OrphanReap
        );
        assert_eq!(
            super::parse_process_mode("REPORT").unwrap(),
            ProcessMode::Report
        );
    }

    #[test]
    fn parse_process_mode_rejects_force_skewed() {
        let err = super::parse_process_mode("force-skewed").unwrap_err();
        let message = err.to_string();
        assert!(
            message.contains("force-skewed"),
            "error should name the rejected mode: {message}"
        );
        assert!(
            message.contains("not offered") || message.contains("forbidden"),
            "error should say force-skewed is not offered: {message}"
        );
    }
}
