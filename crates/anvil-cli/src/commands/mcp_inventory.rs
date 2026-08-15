//! Report-only inventory of live `anvil mcp serve` children (MCPLH-003).
//!
//! Default refresh process mode is visibility only. This module never
//! signals or kills a process. Orphan reap lands in MCPLH-006.

use std::path::{Path, PathBuf};

use serde::Serialize;

/// How refresh should treat live MCP children.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ProcessMode {
    Report,
    None,
}

impl ProcessMode {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Report => "report",
            Self::None => "none",
        }
    }
}

/// Best-effort classification of one `anvil mcp serve` child.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum ProcessClass {
    Current,
    Skewed,
    Orphan,
}

/// One live MCP child, grouped later by parent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct McpProcess {
    pub pid: u32,
    pub ppid: u32,
    pub command: String,
    pub parent_command: String,
    pub class: ProcessClass,
}

/// Report-only snapshot of live MCP children.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ProcessInventory {
    pub mode: &'static str,
    pub signalled: u32,
    pub total: usize,
    pub skewed: usize,
    pub current: usize,
    pub orphan: usize,
    pub by_parent: Vec<ParentGroup>,
}

/// Parent-grouped residual skew for the operator report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ParentGroup {
    pub command: String,
    pub parent_pids: Vec<u32>,
    pub skewed_children: usize,
    pub current_children: usize,
    pub orphan_children: usize,
}

/// Sink used so tests can prove report mode never signals.
///
/// Production report/none modes must not call [`ProcessSignalSink::signal`].
/// MCPLH-006 will use this for orphan-reap.
pub(crate) trait ProcessSignalSink {
    #[allow(dead_code)] // reserved for MCPLH-006; report/none must not call it
    fn signal(&mut self, pid: u32);
}

/// No-op sink for the production refresh path.
pub(crate) struct NoopSignals;

impl ProcessSignalSink for NoopSignals {
    fn signal(&mut self, _pid: u32) {}
}

/// Apply the process policy. Report and none never call [`ProcessSignalSink::signal`].
#[must_use]
pub(crate) fn apply_process_mode(
    mode: ProcessMode,
    inventory: ProcessInventory,
    _sink: &mut dyn ProcessSignalSink,
) -> ProcessInventory {
    match mode {
        ProcessMode::Report | ProcessMode::None => inventory,
    }
}

/// Scan live processes, or return an empty report when mode is [`ProcessMode::None`].
#[must_use]
pub(crate) fn collect_inventory(mode: ProcessMode, preferred: Option<&Path>) -> ProcessInventory {
    match mode {
        ProcessMode::None => empty_inventory(ProcessMode::None),
        ProcessMode::Report => summarise(&scan_live(preferred), ProcessMode::Report),
    }
}

#[must_use]
pub(crate) fn empty_inventory(mode: ProcessMode) -> ProcessInventory {
    ProcessInventory {
        mode: mode.as_str(),
        signalled: 0,
        total: 0,
        skewed: 0,
        current: 0,
        orphan: 0,
        by_parent: Vec::new(),
    }
}

#[must_use]
pub(crate) fn summarise(processes: &[McpProcess], mode: ProcessMode) -> ProcessInventory {
    let total = processes.len();
    let skewed = processes
        .iter()
        .filter(|proc| proc.class == ProcessClass::Skewed)
        .count();
    let current = processes
        .iter()
        .filter(|proc| proc.class == ProcessClass::Current)
        .count();
    let orphan = processes
        .iter()
        .filter(|proc| proc.class == ProcessClass::Orphan)
        .count();
    ProcessInventory {
        mode: mode.as_str(),
        signalled: 0,
        total,
        skewed,
        current,
        orphan,
        by_parent: group_by_parent(processes),
    }
}

fn group_by_parent(processes: &[McpProcess]) -> Vec<ParentGroup> {
    let mut groups: Vec<ParentGroup> = Vec::new();
    for proc in processes {
        if let Some(existing) = groups
            .iter_mut()
            .find(|group| group.command == proc.parent_command)
        {
            if !existing.parent_pids.contains(&proc.ppid) {
                existing.parent_pids.push(proc.ppid);
            }
            match proc.class {
                ProcessClass::Skewed => existing.skewed_children += 1,
                ProcessClass::Current => existing.current_children += 1,
                ProcessClass::Orphan => existing.orphan_children += 1,
            }
        } else {
            groups.push(ParentGroup {
                command: proc.parent_command.clone(),
                parent_pids: vec![proc.ppid],
                skewed_children: usize::from(proc.class == ProcessClass::Skewed),
                current_children: usize::from(proc.class == ProcessClass::Current),
                orphan_children: usize::from(proc.class == ProcessClass::Orphan),
            });
        }
    }
    groups.sort_by(|left, right| left.command.cmp(&right.command));
    groups
}

fn scan_live(preferred: Option<&Path>) -> Vec<McpProcess> {
    #[cfg(unix)]
    {
        scan_proc(preferred)
    }
    #[cfg(not(unix))]
    {
        let _ = preferred;
        Vec::new()
    }
}

#[cfg(unix)]
fn scan_proc(preferred: Option<&Path>) -> Vec<McpProcess> {
    let Ok(entries) = std::fs::read_dir("/proc") else {
        return Vec::new();
    };
    let mut found = Vec::new();
    for entry in entries.flatten() {
        let Ok(pid) = entry.file_name().to_string_lossy().parse::<u32>() else {
            continue;
        };
        let args = match read_cmdline(pid) {
            Some(args) if looks_like_anvil_mcp_serve(&args) => args,
            _ => continue,
        };
        let parent_pid = read_ppid(pid).unwrap_or(0);
        let parent_alive =
            parent_pid > 1 && std::path::Path::new(&format!("/proc/{parent_pid}")).exists();
        let parent_command = match read_cmdline(parent_pid).and_then(|args| args.into_iter().next())
        {
            Some(cmd) => command_basename(&cmd),
            None => "unknown".to_owned(),
        };
        let exe = std::fs::read_link(format!("/proc/{pid}/exe")).ok();
        let class = classify(parent_alive, exe.as_deref(), preferred);
        found.push(McpProcess {
            pid,
            ppid: parent_pid,
            command: args.first().cloned().unwrap_or_else(|| "anvil".to_owned()),
            parent_command,
            class,
        });
    }
    found.sort_by_key(|proc| proc.pid);
    found
}

#[cfg(any(unix, test))]
fn classify(parent_alive: bool, exe: Option<&Path>, preferred: Option<&Path>) -> ProcessClass {
    if !parent_alive {
        return ProcessClass::Orphan;
    }
    match (exe, preferred) {
        (_, None) => ProcessClass::Current,
        (Some(exe), Some(preferred)) if identities_match(exe, preferred) => ProcessClass::Current,
        _ => ProcessClass::Skewed,
    }
}

#[cfg(any(unix, test))]
fn identities_match(left: &Path, right: &Path) -> bool {
    let left = dunce::canonicalize(left).unwrap_or_else(|_| left.to_path_buf());
    let right = dunce::canonicalize(right).unwrap_or_else(|_| right.to_path_buf());
    left == right
}

#[cfg(any(unix, test))]
fn looks_like_anvil_mcp_serve(args: &[String]) -> bool {
    let has_mcp = args.iter().any(|arg| arg == "mcp");
    let has_serve = args.iter().any(|arg| arg == "serve");
    let has_stdio = args.iter().any(|arg| arg == "--stdio");
    if !(has_mcp && has_serve && has_stdio) {
        return false;
    }
    args.first().is_some_and(|command| {
        let name = command_basename(command);
        name.eq_ignore_ascii_case("anvil") || name.eq_ignore_ascii_case("anvil.exe")
    })
}

#[cfg(any(unix, test))]
fn command_basename(command: &str) -> String {
    match PathBuf::from(command).file_name() {
        Some(name) => name.to_string_lossy().into_owned(),
        None => command.to_owned(),
    }
}

#[cfg(unix)]
fn read_cmdline(pid: u32) -> Option<Vec<String>> {
    let raw = std::fs::read(format!("/proc/{pid}/cmdline")).ok()?;
    if raw.is_empty() {
        return None;
    }
    let args: Vec<String> = raw
        .split(|byte| *byte == 0)
        .filter(|part| !part.is_empty())
        .map(|part| String::from_utf8_lossy(part).into_owned())
        .collect();
    if args.is_empty() { None } else { Some(args) }
}

#[cfg(unix)]
fn read_ppid(pid: u32) -> Option<u32> {
    let status = std::fs::read_to_string(format!("/proc/{pid}/status")).ok()?;
    status.lines().find_map(|line| {
        line.strip_prefix("PPid:")
            .and_then(|rest| rest.trim().parse().ok())
    })
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{
        McpProcess, ProcessClass, ProcessMode, ProcessSignalSink, apply_process_mode, classify,
        looks_like_anvil_mcp_serve, summarise,
    };

    struct RecordingSink {
        pids: Vec<u32>,
    }

    impl ProcessSignalSink for RecordingSink {
        fn signal(&mut self, pid: u32) {
            self.pids.push(pid);
        }
    }

    fn sample_inventory() -> Vec<McpProcess> {
        vec![
            McpProcess {
                pid: 11,
                ppid: 10,
                command: "anvil".into(),
                parent_command: "grok".into(),
                class: ProcessClass::Skewed,
            },
            McpProcess {
                pid: 12,
                ppid: 10,
                command: "anvil".into(),
                parent_command: "grok".into(),
                class: ProcessClass::Current,
            },
        ]
    }

    #[test]
    fn processes_report_never_sends_a_signal() {
        let inventory = summarise(&sample_inventory(), ProcessMode::Report);
        let mut sink = RecordingSink { pids: Vec::new() };
        let reported = apply_process_mode(ProcessMode::Report, inventory, &mut sink);
        assert!(
            sink.pids.is_empty(),
            "report mode must not signal: {:?}",
            sink.pids
        );
        assert_eq!(reported.signalled, 0);
        assert_eq!(reported.total, 2);
        assert_eq!(reported.skewed, 1);
        assert_eq!(reported.by_parent[0].command, "grok");
        assert_eq!(reported.by_parent[0].parent_pids, vec![10]);
    }

    #[test]
    fn unknown_preferred_is_not_skewed() {
        assert_eq!(
            classify(true, Some(Path::new("/opt/homebrew/bin/anvil")), None),
            ProcessClass::Current,
            "missing preferred must match re-exec: not skewed"
        );
    }

    #[test]
    fn shape_check_requires_anvil_mcp_serve_stdio() {
        assert!(looks_like_anvil_mcp_serve(&[
            "/opt/homebrew/Cellar/anvil/0.9.2-beta/bin/anvil".into(),
            "mcp".into(),
            "serve".into(),
            "--stdio".into(),
        ]));
        assert!(
            !looks_like_anvil_mcp_serve(&["anvil".into(), "mcp".into(), "refresh".into(),]),
            "refresh itself is not an MCP child"
        );
        assert!(!looks_like_anvil_mcp_serve(&[
            "python".into(),
            "mcp".into(),
            "serve".into(),
            "--stdio".into(),
        ]));
        let _ = Path::new("/unused");
    }
}
