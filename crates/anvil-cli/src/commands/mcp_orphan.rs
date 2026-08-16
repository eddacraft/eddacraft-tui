//! Classify and reap orphan `anvil mcp serve --stdio` processes (CIB-344).
//!
//! A shim is **live** when its parent process still exists (`ppid > 1` and a
//! parent cmdline was observed). A shim is an **orphan** when it has been
//! reparented to init or its parent cmdline is gone. Doctor reports orphans;
//! `anvil doctor --fix` sends SIGTERM to orphans only. Live editor/agent
//! children are never signalled.

/// One observed `anvil mcp serve` process.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct McpServeProcess {
    pub pid: u32,
    pub ppid: u32,
    pub cmdline: String,
    pub parent_cmdline: Option<String>,
}

/// Whether `cmdline` is an anvil MCP stdio server (not intercept, not graph-base).
#[must_use]
pub(crate) fn is_anvil_mcp_serve(cmdline: &str) -> bool {
    let tokens: Vec<&str> = cmdline
        .split('\0')
        .flat_map(|s| s.split_whitespace())
        .collect();
    let has_anvil = tokens
        .iter()
        .any(|t| PathLeaf(t).ends_with("anvil") || *t == "anvil");
    has_anvil
        && tokens.windows(3).any(|w| w == ["mcp", "serve", "--stdio"])
        && !tokens
            .iter()
            .any(|t| *t == "intercept" || *t == "graph-base")
}

/// Orphan: parent is init / missing. Live parent (any cmdline) is kept.
#[must_use]
pub(crate) fn is_orphan(proc: &McpServeProcess) -> bool {
    proc.ppid <= 1 || proc.parent_cmdline.is_none()
}

/// Split a process table into (live, orphan) MCP shims.
#[must_use]
pub(crate) fn partition_mcp_shims(
    procs: &[McpServeProcess],
) -> (Vec<&McpServeProcess>, Vec<&McpServeProcess>) {
    let shims: Vec<&McpServeProcess> = procs
        .iter()
        .filter(|p| is_anvil_mcp_serve(&p.cmdline))
        .collect();
    let (orphans, live): (Vec<_>, Vec<_>) = shims.into_iter().partition(|p| is_orphan(p));
    (live, orphans)
}

/// List `anvil mcp serve --stdio` processes on this host. Empty on non-Unix.
#[must_use]
pub(crate) fn list_anvil_mcp_serve() -> Vec<McpServeProcess> {
    #[cfg(target_os = "linux")]
    {
        list_anvil_mcp_serve_linux()
    }
    #[cfg(not(target_os = "linux"))]
    {
        Vec::new()
    }
}

/// SIGTERM each orphan pid. Returns how many signals were sent.
pub(crate) fn reap_orphans(orphans: &[McpServeProcess]) -> usize {
    let mut sent = 0usize;
    for proc in orphans {
        if !is_orphan(proc) {
            continue;
        }
        if signal_term(proc.pid) {
            sent += 1;
        }
    }
    sent
}

fn signal_term(pid: u32) -> bool {
    #[cfg(unix)]
    {
        std::process::Command::new("kill")
            .arg("-TERM")
            .arg(pid.to_string())
            .status()
            .is_ok_and(|status| status.success())
    }
    #[cfg(not(unix))]
    {
        let _ = pid;
        false
    }
}

#[cfg(target_os = "linux")]
fn list_anvil_mcp_serve_linux() -> Vec<McpServeProcess> {
    let mut out = Vec::new();
    let Ok(entries) = std::fs::read_dir("/proc") else {
        return out;
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let Some(child_pid) = name.to_str().and_then(|s| s.parse::<u32>().ok()) else {
            continue;
        };
        let cmdline = match std::fs::read(entry.path().join("cmdline")) {
            Ok(bytes) if !bytes.is_empty() => String::from_utf8_lossy(&bytes).into_owned(),
            _ => continue,
        };
        if !is_anvil_mcp_serve(&cmdline) {
            continue;
        }
        let parent_pid = read_ppid(child_pid).unwrap_or(0);
        let parent_cmdline = if parent_pid > 1 {
            std::fs::read(format!("/proc/{parent_pid}/cmdline"))
                .ok()
                .filter(|b| !b.is_empty())
                .map(|b| String::from_utf8_lossy(&b).into_owned())
        } else {
            None
        };
        out.push(McpServeProcess {
            pid: child_pid,
            ppid: parent_pid,
            cmdline,
            parent_cmdline,
        });
    }
    out
}

#[cfg(target_os = "linux")]
fn read_ppid(pid: u32) -> Option<u32> {
    let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    // stat: pid (comm) state ppid ...
    let close = stat.rfind(')')?;
    let rest = stat.get(close + 2..)?;
    rest.split_whitespace().nth(1)?.parse().ok()
}

struct PathLeaf<'a>(&'a str);

impl PathLeaf<'_> {
    fn ends_with(&self, needle: &str) -> bool {
        self.0
            .rsplit(['/', '\\'])
            .next()
            .is_some_and(|leaf| leaf == needle)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn proc(pid: u32, parent_pid: u32, cmdline: &str, parent_cmd: Option<&str>) -> McpServeProcess {
        McpServeProcess {
            pid,
            ppid: parent_pid,
            cmdline: cmdline.to_string(),
            parent_cmdline: parent_cmd.map(str::to_string),
        }
    }

    #[test]
    fn detects_stdio_shim_and_ignores_intercept() {
        assert!(is_anvil_mcp_serve("anvil\0mcp\0serve\0--stdio"));
        assert!(is_anvil_mcp_serve(
            "/home/linuxbrew/.linuxbrew/Cellar/anvil/0.9.2-beta/bin/anvil mcp serve --stdio"
        ));
        assert!(!is_anvil_mcp_serve("anvil intercept start --foreground"));
        assert!(!is_anvil_mcp_serve("anvil graph-base build --repo /tmp/x"));
        assert!(!is_anvil_mcp_serve("anvil mcp install --client grok"));
    }

    #[test]
    fn live_parent_is_not_orphan() {
        let live = proc(10, 20, "anvil mcp serve --stdio", Some("grok"));
        assert!(!is_orphan(&live));
        let cursor = proc(11, 21, "anvil mcp serve --stdio", Some("/usr/bin/cursor"));
        assert!(!is_orphan(&cursor));
    }

    #[test]
    fn init_or_missing_parent_is_orphan() {
        let init = proc(10, 1, "anvil mcp serve --stdio", None);
        assert!(is_orphan(&init));
        let gone = proc(10, 99, "anvil mcp serve --stdio", None);
        assert!(is_orphan(&gone));
    }

    #[test]
    fn partition_keeps_live_and_names_orphans() {
        let table = [
            proc(1, 50, "anvil mcp serve --stdio", Some("grok")),
            proc(2, 1, "anvil mcp serve --stdio", None),
            proc(3, 8, "anvil intercept start --foreground", Some("1")),
        ];
        let (live, orphans) = partition_mcp_shims(&table);
        assert_eq!(live.iter().map(|p| p.pid).collect::<Vec<_>>(), vec![1]);
        assert_eq!(orphans.iter().map(|p| p.pid).collect::<Vec<_>>(), vec![2]);
    }

    #[test]
    fn reap_skips_live_parents() {
        let live = proc(
            std::process::id(),
            2,
            "anvil mcp serve --stdio",
            Some("grok"),
        );
        assert_eq!(reap_orphans(&[live]), 0);
    }
}
