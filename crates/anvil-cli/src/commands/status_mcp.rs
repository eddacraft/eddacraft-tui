//! Status-local MCP process inventory and split readiness claims (MCPLH-005).
//!
//! Report-only: this module never signals a live MCP process. CIB-242
//! visibility is extended with CLI vs MCP version inventory, `mcp_skew`
//! aggregates, parent grouping, and split `protecting` / `agent_ready` /
//! `graph_ready` claims so operators do not conflate pre-write attach
//! with current tools or a ready graph.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use anvil_intercept_proto::protocol::{AssuranceState, WorkspaceAssurance};
use serde::Serialize;

use crate::commands::watch_save_time;

/// One same-user `anvil mcp serve` process. Inventory is report-only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct McpProcessRecord {
    pub pid: u32,
    pub parent_pid: Option<u32>,
    pub parent_command: String,
    pub version: Option<String>,
    /// Proven same binary identity as this CLI (inode or version match).
    pub current: bool,
    pub orphan: bool,
}

/// Spec §9.5 `mcp_processes` aggregate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct McpProcessInventory {
    pub total: usize,
    pub skewed: usize,
    pub current: usize,
    pub orphan: usize,
    pub by_parent: Vec<McpParentGroup>,
}

/// Parent grouping for residual skew reporting.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct McpParentGroup {
    pub command: String,
    pub parent_pids: Vec<u32>,
    pub skewed_children: usize,
}

/// Spec §9.5 graph readiness, reused from save-time assurance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum GraphState {
    Ready,
    Warming,
    Stale,
    Unavailable,
}

/// Graph claim derived from existing assurance — not a second probe.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct GraphReadiness {
    pub state: GraphState,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// OQ-4 split: `agent_ready` is pre-write attach plus current MCP, not graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SplitReadinessClaims {
    pub protecting: bool,
    pub agent_ready: bool,
    pub graph_ready: bool,
}

/// Additive `anvil.status.v1` fields. Every member is omitted when `None`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub(crate) struct StatusMcpJson {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cli_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mcp_skew: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mcp_processes: Option<McpProcessInventory>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub graph: Option<GraphReadiness>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_ready: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub graph_ready: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub protecting: Option<bool>,
}

/// Injectable listing so unit tests do not depend on `/proc`.
pub(crate) trait McpInventorySource {
    fn list_processes(&self) -> Option<Vec<McpProcessRecord>>;
}

/// Linux `/proc` best-effort scan. Other platforms report probe-unavailable.
pub(crate) struct ProcMcpInventorySource;

impl McpInventorySource for ProcMcpInventorySource {
    fn list_processes(&self) -> Option<Vec<McpProcessRecord>> {
        scan_mcp_processes()
    }
}

/// Classify records from `source`. Empty or failed probes yield `None`
/// so default `anvil status --json` stays omitted-when-empty.
pub(crate) fn inventory_from_source(
    cli_version: &str,
    source: &dyn McpInventorySource,
) -> Option<McpProcessInventory> {
    let records = source.list_processes()?;
    if records.is_empty() {
        return None;
    }
    Some(classify_inventory(cli_version, &records))
}

/// Production gather: same-user `mcp serve` processes, report-only.
pub(crate) fn gather_mcp_inventory(cli_version: &str) -> Option<McpProcessInventory> {
    inventory_from_source(cli_version, &ProcMcpInventorySource)
}

/// Map save-time assurance onto the spec §9.5 graph closed set.
#[must_use]
pub(crate) fn graph_from_assurance(
    assurance: Option<&WorkspaceAssurance>,
) -> Option<GraphReadiness> {
    let assurance = assurance?;
    let (state, reason) = match assurance.state {
        AssuranceState::Clean | AssuranceState::Bounded => (GraphState::Ready, None),
        AssuranceState::Pending | AssuranceState::Running => (GraphState::Warming, None),
        AssuranceState::Stale => (
            GraphState::Stale,
            assurance
                .reason
                .map(watch_save_time::stale_reason_str)
                .map(str::to_owned),
        ),
        AssuranceState::Unavailable | AssuranceState::Unknown => (
            GraphState::Unavailable,
            assurance
                .reason
                .map(watch_save_time::stale_reason_str)
                .map(str::to_owned),
        ),
    };
    Some(GraphReadiness { state, reason })
}

/// Split claims. `agent_ready` ignores graph (OQ-4).
#[must_use]
pub(crate) fn split_readiness_claims(
    protecting: bool,
    inventory: Option<&McpProcessInventory>,
    graph: Option<&GraphReadiness>,
) -> SplitReadinessClaims {
    let mcp_skew = inventory.is_some_and(|inv| inv.skewed > 0);
    SplitReadinessClaims {
        protecting,
        agent_ready: protecting && !mcp_skew,
        graph_ready: graph.is_some_and(|g| g.state == GraphState::Ready),
    }
}

/// Additive JSON section. `None` when there is nothing to emit.
#[must_use]
pub(crate) fn status_mcp_json(
    cli_version: &str,
    protecting: bool,
    inventory: Option<&McpProcessInventory>,
    graph: Option<&GraphReadiness>,
) -> Option<StatusMcpJson> {
    if inventory.is_none() && graph.is_none() {
        return None;
    }
    let claims = split_readiness_claims(protecting, inventory, graph);
    Some(StatusMcpJson {
        cli_version: inventory.map(|_| cli_version.to_owned()),
        mcp_skew: inventory.map(|inv| inv.skewed > 0),
        mcp_processes: inventory.cloned(),
        graph: graph.cloned(),
        agent_ready: Some(claims.agent_ready),
        graph_ready: graph.map(|g| g.state == GraphState::Ready),
        protecting: Some(claims.protecting),
    })
}

/// Human MCP + split-claim block. Matching versions stay quiet.
#[must_use]
pub(crate) fn render_status_mcp_plain(
    cli_version: &str,
    protecting: bool,
    inventory: Option<&McpProcessInventory>,
    graph: Option<&GraphReadiness>,
) -> String {
    let claims = split_readiness_claims(protecting, inventory, graph);
    let mut out = String::new();
    let graph_not_ready = graph.is_some_and(|g| g.state != GraphState::Ready);
    if graph_not_ready {
        let attach = if claims.protecting {
            "protecting"
        } else {
            "not protecting"
        };
        let _ = writeln!(out, "  Attach: {attach}");
        if let Some(graph) = graph {
            let _ = writeln!(out, "  Graph: {}", render_graph_label(graph));
        }
    }
    if let Some(inventory) = inventory {
        out.push_str(&render_mcp_skew_guidance(cli_version, inventory));
    }
    out
}

fn render_graph_label(graph: &GraphReadiness) -> String {
    match graph.state {
        GraphState::Ready => "ready".to_owned(),
        GraphState::Warming => "warming".to_owned(),
        GraphState::Stale => match graph.reason.as_deref() {
            Some(reason) => format!("stale ({reason})"),
            None => "stale".to_owned(),
        },
        GraphState::Unavailable => match graph.reason.as_deref() {
            Some(reason) => format!("unavailable ({reason})"),
            None => "unavailable".to_owned(),
        },
    }
}

/// Skew copy for humans. Empty when every listed process matches the CLI.
#[must_use]
pub(crate) fn render_mcp_skew_guidance(
    cli_version: &str,
    inventory: &McpProcessInventory,
) -> String {
    if inventory.skewed == 0 {
        return String::new();
    }
    let mut out = String::new();
    let _ = writeln!(
        out,
        "  MCP: {} of {} processes differ from CLI version {cli_version}",
        inventory.skewed, inventory.total
    );
    for group in &inventory.by_parent {
        if group.skewed_children == 0 {
            continue;
        }
        let pids = format_parent_pids(&group.parent_pids);
        let child = if group.skewed_children == 1 {
            "child"
        } else {
            "children"
        };
        let _ = writeln!(
            out,
            "    parent {} ({pids}): {} skewed {child}",
            group.command, group.skewed_children
        );
    }
    out.push_str(
        "    Reconnect MCP for this client, or retry a tool call after: anvil mcp refresh\n",
    );
    out
}

fn format_parent_pids(pids: &[u32]) -> String {
    if pids.is_empty() {
        return "parent pids unknown".to_owned();
    }
    let joined = pids
        .iter()
        .map(u32::to_string)
        .collect::<Vec<_>>()
        .join(", ");
    format!("pids {joined}")
}

/// Fold records into spec §9.5 aggregates.
#[must_use]
pub(crate) fn classify_inventory(
    cli_version: &str,
    processes: &[McpProcessRecord],
) -> McpProcessInventory {
    let mut skewed = 0;
    let mut current = 0;
    let mut orphan = 0;
    let mut groups: BTreeMap<String, ParentAcc> = BTreeMap::new();

    for process in processes {
        if process.orphan {
            orphan += 1;
        }
        let is_skewed = process_is_skewed(cli_version, process);
        if is_skewed {
            skewed += 1;
        } else {
            current += 1;
        }
        let acc = groups.entry(process.parent_command.clone()).or_default();
        if let Some(ppid) = process.parent_pid {
            acc.parent_pids.insert(ppid);
        }
        if is_skewed {
            acc.skewed_children += 1;
        }
    }

    McpProcessInventory {
        total: processes.len(),
        skewed,
        current,
        orphan,
        by_parent: groups
            .into_iter()
            .map(|(command, acc)| McpParentGroup {
                command,
                parent_pids: acc.parent_pids.into_iter().collect(),
                skewed_children: acc.skewed_children,
            })
            .collect(),
    }
}

#[derive(Default)]
struct ParentAcc {
    parent_pids: BTreeSet<u32>,
    skewed_children: usize,
}

fn process_is_skewed(cli_version: &str, process: &McpProcessRecord) -> bool {
    if process.version.as_deref() == Some(cli_version) || process.current {
        return false;
    }
    true
}

/// Shape-check argv for `anvil mcp serve` (any later flags allowed).
#[must_use]
pub(crate) fn is_anvil_mcp_serve_cmdline(args: &[String]) -> bool {
    let Some(argv0) = args.first() else {
        return false;
    };
    if !looks_like_anvil(argv0) {
        return false;
    }
    let mut seen_mcp = false;
    for arg in args.iter().skip(1) {
        if !seen_mcp {
            if arg == "mcp" {
                seen_mcp = true;
            }
            continue;
        }
        if arg == "serve" {
            return true;
        }
        if arg.starts_with('-') {
            continue;
        }
        return false;
    }
    false
}

fn looks_like_anvil(argv0: &str) -> bool {
    let name = Path::new(argv0)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or(argv0);
    let name = name.strip_suffix(".exe").unwrap_or(name);
    name == "anvil"
}

/// Best-effort Homebrew Cellar version from an absolute path.
#[must_use]
pub(crate) fn version_hint_from_path(path: &Path) -> Option<String> {
    let parts: Vec<_> = path.iter().map(|s| s.to_string_lossy()).collect();
    for window in parts.windows(3) {
        if window[0] == "Cellar" && window[1] == "anvil" {
            let version = window[2].as_ref();
            if !version.is_empty() && version != "bin" {
                return Some(version.to_owned());
            }
        }
    }
    None
}

#[cfg(target_os = "linux")]
fn scan_mcp_processes() -> Option<Vec<McpProcessRecord>> {
    let self_uid = current_uid()?;
    let current_exe = std::env::current_exe().ok();
    let cli_version = env!("CARGO_PKG_VERSION");
    let mut out = Vec::new();
    let dir = std::fs::read_dir("/proc").ok()?;
    for entry in dir.flatten() {
        let name = entry.file_name();
        let Some(name) = name.to_str() else {
            continue;
        };
        let Ok(pid) = name.parse::<u32>() else {
            continue;
        };
        if let Some(record) = inspect_proc_pid(pid, self_uid, current_exe.as_deref(), cli_version) {
            out.push(record);
        }
    }
    Some(out)
}

#[cfg(not(target_os = "linux"))]
fn scan_mcp_processes() -> Option<Vec<McpProcessRecord>> {
    None
}

#[cfg(target_os = "linux")]
fn current_uid() -> Option<u32> {
    read_proc_status(std::process::id()).map(|status| status.uid)
}

#[cfg(target_os = "linux")]
fn inspect_proc_pid(
    pid: u32,
    self_uid: u32,
    current_exe: Option<&Path>,
    cli_version: &str,
) -> Option<McpProcessRecord> {
    let status = read_proc_status(pid)?;
    if status.uid != self_uid {
        return None;
    }
    let args = read_cmdline(pid)?;
    if !is_anvil_mcp_serve_cmdline(&args) {
        return None;
    }
    let exe = read_exe(pid);
    let parent_alive = status
        .ppid
        .is_some_and(|ppid| ppid > 1 && proc_exists(ppid));
    let orphan = !parent_alive;
    let parent_command = status
        .ppid
        .filter(|_| parent_alive)
        .and_then(process_command)
        .unwrap_or_else(|| "(gone)".to_owned());
    let version = exe.as_deref().and_then(version_hint_from_path).or_else(|| {
        exe.as_deref().and_then(|path| {
            current_exe
                .filter(|cur| same_identity(path, cur))
                .map(|_| cli_version.to_owned())
        })
    });
    let current = match (exe.as_deref(), current_exe) {
        (Some(path), Some(cur)) => same_identity(path, cur),
        _ => false,
    };
    Some(McpProcessRecord {
        pid,
        parent_pid: status.ppid,
        parent_command,
        version,
        current,
        orphan,
    })
}

#[cfg(target_os = "linux")]
struct ProcStatus {
    ppid: Option<u32>,
    uid: u32,
}

#[cfg(target_os = "linux")]
fn read_proc_status(pid: u32) -> Option<ProcStatus> {
    let text = std::fs::read_to_string(format!("/proc/{pid}/status")).ok()?;
    let mut parent = None;
    let mut uid = None;
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("PPid:") {
            parent = rest.trim().parse().ok();
        } else if let Some(rest) = line.strip_prefix("Uid:") {
            uid = rest.split_whitespace().next().and_then(|s| s.parse().ok());
        }
    }
    Some(ProcStatus {
        ppid: parent,
        uid: uid?,
    })
}

#[cfg(target_os = "linux")]
fn read_cmdline(pid: u32) -> Option<Vec<String>> {
    let bytes = std::fs::read(format!("/proc/{pid}/cmdline")).ok()?;
    if bytes.is_empty() {
        return None;
    }
    let args: Vec<String> = bytes
        .split(|b| *b == 0)
        .filter(|part| !part.is_empty())
        .map(|part| String::from_utf8_lossy(part).into_owned())
        .collect();
    if args.is_empty() { None } else { Some(args) }
}

#[cfg(target_os = "linux")]
fn read_exe(pid: u32) -> Option<PathBuf> {
    std::fs::read_link(format!("/proc/{pid}/exe")).ok()
}

#[cfg(target_os = "linux")]
fn proc_exists(pid: u32) -> bool {
    Path::new(&format!("/proc/{pid}")).exists()
}

#[cfg(target_os = "linux")]
fn process_command(pid: u32) -> Option<String> {
    if let Some(name) = read_cmdline(pid)
        .as_ref()
        .and_then(|args| args.first())
        .map(Path::new)
        .and_then(Path::file_name)
        .and_then(|s| s.to_str())
        .filter(|name| !name.is_empty())
    {
        return Some(name.to_owned());
    }
    let comm = std::fs::read_to_string(format!("/proc/{pid}/comm")).ok()?;
    let comm = comm.trim();
    if comm.is_empty() {
        None
    } else {
        Some(comm.to_owned())
    }
}

#[cfg(unix)]
fn same_identity(left: &Path, right: &Path) -> bool {
    use std::os::unix::fs::MetadataExt;
    if let (Ok(left_meta), Ok(right_meta)) = (left.metadata(), right.metadata()) {
        return left_meta.dev() == right_meta.dev() && left_meta.ino() == right_meta.ino();
    }
    dunce::canonicalize(left).ok() == dunce::canonicalize(right).ok()
}

#[cfg(not(unix))]
fn same_identity(left: &Path, right: &Path) -> bool {
    dunce::canonicalize(left).ok() == dunce::canonicalize(right).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use anvil_intercept_proto::protocol::StaleReason;

    const CLI: &str = "0.9.5-beta";
    const OLD: &str = "0.9.2-beta";

    struct FixtureSource(Vec<McpProcessRecord>);

    impl McpInventorySource for FixtureSource {
        fn list_processes(&self) -> Option<Vec<McpProcessRecord>> {
            Some(self.0.clone())
        }
    }

    struct FailedSource;

    impl McpInventorySource for FailedSource {
        fn list_processes(&self) -> Option<Vec<McpProcessRecord>> {
            None
        }
    }

    fn skewed_child() -> McpProcessRecord {
        McpProcessRecord {
            pid: 200,
            parent_pid: Some(100),
            parent_command: "grok".to_owned(),
            version: Some(OLD.to_owned()),
            current: false,
            orphan: false,
        }
    }

    fn current_child() -> McpProcessRecord {
        McpProcessRecord {
            pid: 201,
            parent_pid: Some(100),
            parent_command: "grok".to_owned(),
            version: Some(CLI.to_owned()),
            current: true,
            orphan: false,
        }
    }

    fn stale_graph() -> GraphReadiness {
        GraphReadiness {
            state: GraphState::Stale,
            reason: Some("scan-timeout".to_owned()),
        }
    }

    fn ready_graph() -> GraphReadiness {
        GraphReadiness {
            state: GraphState::Ready,
            reason: None,
        }
    }

    fn stale_assurance() -> WorkspaceAssurance {
        WorkspaceAssurance {
            state: AssuranceState::Stale,
            reason: Some(StaleReason::ScanTimeout),
            generation: 2,
            last_full_scan: None,
            scan_coverage: None,
        }
    }

    #[test]
    fn mismatched_versions_print_skew_guidance_without_false_agent_ready() {
        let inventory = classify_inventory(CLI, &[skewed_child()]);
        let rendered = render_mcp_skew_guidance(CLI, &inventory);
        assert!(
            rendered.contains("differ from CLI version 0.9.5-beta"),
            "skewed fixture must name the CLI version: {rendered}"
        );
        assert!(
            rendered.contains("parent grok"),
            "skewed fixture must group by parent: {rendered}"
        );
        assert!(
            rendered.contains("anvil mcp refresh") || rendered.contains("Reconnect MCP"),
            "guidance must offer per-parent reconnect or refresh, got: {rendered}"
        );
        assert!(
            !rendered
                .to_ascii_lowercase()
                .contains("restart all your agents"),
            "must not lead with mass session restart: {rendered}"
        );

        let claims = split_readiness_claims(true, Some(&inventory), Some(&ready_graph()));
        assert!(
            claims.protecting,
            "pre-write attach stays independent of MCP skew"
        );
        assert!(
            !claims.agent_ready,
            "skewed MCP must not claim agent_ready / current-tools"
        );
        assert!(
            claims.graph_ready,
            "ready graph stays ready while MCP is skewed"
        );
        let json = status_mcp_json(CLI, true, Some(&inventory), Some(&ready_graph())).unwrap();
        assert_eq!(json.mcp_skew, Some(true));
        assert_eq!(json.agent_ready, Some(false));
        assert_eq!(json.protecting, Some(true));
    }

    #[test]
    fn matching_versions_stay_quiet() {
        let inventory = classify_inventory(CLI, &[current_child()]);
        assert_eq!(inventory.skewed, 0);
        assert_eq!(inventory.current, 1);
        let rendered = render_mcp_skew_guidance(CLI, &inventory);
        assert!(
            rendered.is_empty(),
            "matching versions must not emit skew guidance: {rendered:?}"
        );
        let plain = render_status_mcp_plain(CLI, true, Some(&inventory), Some(&ready_graph()));
        assert!(
            !plain.contains("differ"),
            "matching + ready graph must stay quiet: {plain:?}"
        );
        assert!(
            !plain.contains("skew"),
            "matching versions must not claim false skew: {plain:?}"
        );
        let json = status_mcp_json(CLI, true, Some(&inventory), Some(&ready_graph())).unwrap();
        assert_eq!(json.mcp_skew, Some(false));
        assert_eq!(json.agent_ready, Some(true));
    }

    #[test]
    fn graph_scan_timeout_does_not_collapse_into_agent_ready() {
        let inventory = classify_inventory(CLI, &[current_child()]);
        let graph = stale_graph();
        let claims = split_readiness_claims(true, Some(&inventory), Some(&graph));
        assert!(
            claims.agent_ready,
            "OQ-4: agent_ready is attach + current MCP, not graph"
        );
        assert!(
            !claims.graph_ready,
            "scan-timeout must keep graph_ready false"
        );
        let plain = render_status_mcp_plain(CLI, true, Some(&inventory), Some(&graph));
        assert!(
            plain.contains("protecting"),
            "human attach claim must remain visible: {plain}"
        );
        assert!(
            plain.contains("scan-timeout"),
            "human graph claim must name scan-timeout: {plain}"
        );
        assert!(
            !plain.to_ascii_lowercase().contains("agent ready"),
            "must not collapse into a single agent-ready line: {plain}"
        );
        assert!(
            !plain.to_ascii_lowercase().contains("current tools"),
            "must not claim current-tools while graph is stale: {plain}"
        );
        let json = status_mcp_json(CLI, true, Some(&inventory), Some(&graph)).unwrap();
        assert_eq!(json.agent_ready, Some(true));
        assert_eq!(json.graph_ready, Some(false));
        assert_eq!(
            json.graph.as_ref().map(|g| g.state),
            Some(GraphState::Stale)
        );
        assert_eq!(
            json.graph.as_ref().and_then(|g| g.reason.as_deref()),
            Some("scan-timeout")
        );
    }

    #[test]
    fn protecting_is_independent_of_graph_ready() {
        let claims = split_readiness_claims(true, None, Some(&stale_graph()));
        assert!(claims.protecting);
        assert!(
            claims.agent_ready,
            "protecting with no MCP skew is agent_ready"
        );
        assert!(!claims.graph_ready);
        let json = status_mcp_json(CLI, true, None, Some(&stale_graph())).unwrap();
        assert_eq!(json.protecting, Some(true));
        assert_eq!(json.graph_ready, Some(false));
        assert!(
            json.mcp_processes.is_none(),
            "no inventory: omit mcp_processes"
        );
    }

    #[test]
    fn inventory_is_report_only_and_has_no_signal_helpers() {
        let src = include_str!("status_mcp.rs");
        let production = src
            .split("#[cfg(test)]")
            .next()
            .expect("production source precedes tests");
        for banned in [
            "libc::kill",
            "nix::sys::signal",
            "kill(",
            "SIGTERM",
            "SIGKILL",
        ] {
            assert!(
                !production.contains(banned),
                "MCPLH-005 inventory must stay report-only; found {banned}"
            );
        }
        let source = FixtureSource(vec![skewed_child()]);
        let inventory = inventory_from_source(CLI, &source).expect("fixture inventory");
        assert_eq!(inventory.total, 1);
        assert_eq!(inventory.skewed, 1);
        assert!(inventory_from_source(CLI, &FailedSource).is_none());
        assert!(inventory_from_source(CLI, &FixtureSource(vec![])).is_none());
    }

    #[test]
    fn json_includes_split_claim_fields_when_inventory_or_graph_present() {
        let inventory = classify_inventory(CLI, &[skewed_child(), current_child()]);
        let json = status_mcp_json(CLI, true, Some(&inventory), Some(&stale_graph())).unwrap();
        let value = serde_json::to_value(&json).expect("serialize");
        assert_eq!(value["cli_version"], CLI);
        assert_eq!(value["mcp_skew"], true);
        assert_eq!(value["mcp_processes"]["total"], 2);
        assert_eq!(value["mcp_processes"]["skewed"], 1);
        assert_eq!(value["mcp_processes"]["current"], 1);
        assert_eq!(value["mcp_processes"]["orphan"], 0);
        assert_eq!(value["mcp_processes"]["by_parent"][0]["command"], "grok");
        assert_eq!(value["mcp_processes"]["by_parent"][0]["skewed_children"], 1);
        assert_eq!(value["graph"]["state"], "stale");
        assert_eq!(value["graph"]["reason"], "scan-timeout");
        assert_eq!(value["agent_ready"], false);
        assert_eq!(value["graph_ready"], false);
        assert_eq!(value["protecting"], true);
        assert!(status_mcp_json(CLI, true, None, None).is_none());
    }

    #[test]
    fn graph_from_assurance_reuses_scan_timeout_signal() {
        let graph = graph_from_assurance(Some(&stale_assurance())).expect("graph");
        assert_eq!(graph.state, GraphState::Stale);
        assert_eq!(graph.reason.as_deref(), Some("scan-timeout"));
        assert!(graph_from_assurance(None).is_none());
        let clean = WorkspaceAssurance {
            state: AssuranceState::Clean,
            reason: None,
            generation: 1,
            last_full_scan: None,
            scan_coverage: None,
        };
        assert_eq!(
            graph_from_assurance(Some(&clean)).map(|g| g.state),
            Some(GraphState::Ready)
        );
        let warming = WorkspaceAssurance {
            state: AssuranceState::Running,
            reason: None,
            generation: 1,
            last_full_scan: None,
            scan_coverage: None,
        };
        assert_eq!(
            graph_from_assurance(Some(&warming)).map(|g| g.state),
            Some(GraphState::Warming)
        );
    }

    #[test]
    fn cmdline_shape_accepts_anvil_mcp_serve_only() {
        assert!(is_anvil_mcp_serve_cmdline(&[
            "anvil".into(),
            "mcp".into(),
            "serve".into(),
            "--stdio".into(),
        ]));
        assert!(is_anvil_mcp_serve_cmdline(&[
            "/opt/homebrew/Cellar/anvil/0.9.2-beta/bin/anvil".into(),
            "mcp".into(),
            "serve".into(),
        ]));
        assert!(!is_anvil_mcp_serve_cmdline(&[
            "anvil".into(),
            "mcp".into(),
            "install".into(),
        ]));
        assert!(!is_anvil_mcp_serve_cmdline(&[
            "python".into(),
            "mcp".into(),
            "serve".into(),
        ]));
        assert!(!is_anvil_mcp_serve_cmdline(&[
            "anvil".into(),
            "status".into()
        ]));
    }

    #[test]
    fn cellar_path_yields_version_hint() {
        let path = Path::new("/opt/homebrew/Cellar/anvil/0.9.2-beta/bin/anvil");
        assert_eq!(version_hint_from_path(path).as_deref(), Some(OLD));
        assert!(version_hint_from_path(Path::new("/usr/bin/anvil")).is_none());
    }
}
