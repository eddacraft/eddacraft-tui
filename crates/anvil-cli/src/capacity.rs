//! Linux-only preflight check for inotify watch headroom.
//!
//! `anvil watch` relies on inotify to detect file changes. When the kernel's
//! `fs.inotify.max_user_watches` limit is near-exhausted across the user's
//! session (commonly by language servers, `nx daemon`, and dev servers),
//! new watches fail silently per-directory — leading to the "some changes
//! are never detected" symptom.
//!
//! This module collects current usage so commands like `anvil init` can
//! print an actionable recommendation up-front, instead of surprising the
//! user later inside `watch`.

use std::path::Path;

/// Reference limits used by tests as "comfortable headroom" sentinel
/// values for a modern monorepo dev box. Not consulted at runtime —
/// `is_tight` only checks actual headroom against the project's needs.
#[cfg(test)]
const RECOMMENDED_MAX_WATCHES: u64 = 524_288;
#[cfg(test)]
const RECOMMENDED_MAX_INSTANCES: u64 = 512;

/// Top consumers we show when surfacing pressure — keep it small to stay
/// actionable rather than dumping a `ps`-style table. Only consumed by the
/// Linux collect path; on macOS / Windows the collector is a no-op stub
/// and this would otherwise show as dead code on the cross builds.
#[cfg(target_os = "linux")]
const TOP_CONSUMERS_SHOWN: usize = 3;

/// Snapshot of the host's inotify state plus an estimate of how many
/// directories anvil would want to watch for the given project.
#[derive(Debug)]
pub struct InotifyStatus {
    pub max_watches: u64,
    pub max_instances: u64,
    pub in_use_watches: u64,
    /// Total distinct processes holding at least one inotify fd (same-user only).
    pub consuming_processes: u64,
    /// Top few consumers by watch count: `(command, count)`.
    pub top_consumers: Vec<(String, u64)>,
    /// Approximate number of directories under `root` that anvil's default
    /// filter would register a watch for.
    pub project_dirs: u64,
}

impl InotifyStatus {
    /// Returns `true` when the user's session is close enough to the
    /// configured limit that `anvil watch` is at real risk of having
    /// degraded performance for this project.
    ///
    /// Only fires on a genuine headroom shortfall — a "below-recommended"
    /// kernel limit alone is not enough. A user on a default Ubuntu box
    /// (`max_user_watches = 65_536`) running `anvil init` against a small
    /// project should not see the warning when their actual headroom is
    /// fine. See issue #1109.
    pub fn is_tight(&self) -> bool {
        // Remaining headroom can't absorb this project plus a little
        // slack for the watcher's own metadata.
        let headroom = self.max_watches.saturating_sub(self.in_use_watches);
        let needed = self.project_dirs.saturating_add(self.project_dirs / 10);
        headroom < needed
    }
}

/// Collect the inotify headroom snapshot. Returns `None` on non-Linux
/// platforms or if the kernel's `/proc/sys/fs/inotify` entries aren't
/// readable (containers, hardened sysctls).
#[cfg(target_os = "linux")]
pub fn collect(root: &Path) -> Option<InotifyStatus> {
    let max_watches = read_u64("/proc/sys/fs/inotify/max_user_watches")?;
    let max_instances = read_u64("/proc/sys/fs/inotify/max_user_instances")?;
    let (in_use_watches, consuming_processes, top_consumers) = scan_proc();
    let project_dirs = count_project_dirs(root);
    Some(InotifyStatus {
        max_watches,
        max_instances,
        in_use_watches,
        consuming_processes,
        top_consumers,
        project_dirs,
    })
}

#[cfg(not(target_os = "linux"))]
pub fn collect(_root: &Path) -> Option<InotifyStatus> {
    None
}

/// Produce the human-readable recommendation block. Returns an empty `Vec`
/// when no warning is warranted — callers print only when non-empty so the
/// happy path stays quiet.
pub fn recommendation_lines(status: &InotifyStatus) -> Vec<String> {
    if !status.is_tight() {
        return Vec::new();
    }

    let mut out = Vec::new();
    out.push(String::new());
    out.push(
        "Inotify headroom is tight — watch performance may be degraded on this machine."
            .to_string(),
    );
    out.push(format!(
        "  watches in use:        {} / {}",
        status.in_use_watches, status.max_watches
    ));
    out.push(format!("  instances limit:       {}", status.max_instances));
    out.push(format!("  this project's dirs:   ~{}", status.project_dirs));
    out.push(format!(
        "  processes holding fds: {}",
        status.consuming_processes
    ));
    if !status.top_consumers.is_empty() {
        let consumers: Vec<String> = status
            .top_consumers
            .iter()
            .map(|(name, count)| format!("{name} x{count}"))
            .collect();
        out.push(format!("  top consumers:         {}", consumers.join(", ")));
    }
    // Stay in app-side / user-action territory — closing watch-heavy
    // processes is the lever a CLI user actually has without root.
    // Issue #1109: don't prescribe sudo / sysctl from the anvil binary.
    out.push(
        "Tip: closing watch-heavy processes (language servers, nx daemon, dev servers)"
            .to_string(),
    );
    out.push("     frees up watches for `anvil watch`.".to_string());
    out
}

#[cfg(target_os = "linux")]
fn read_u64(path: &str) -> Option<u64> {
    std::fs::read_to_string(path).ok()?.trim().parse().ok()
}

/// Walk `/proc` summing inotify watches per pid. Only this user's processes
/// will be readable; that's fine — the kernel's watch limit is per-uid, so
/// cross-user counts wouldn't be actionable anyway.
#[cfg(target_os = "linux")]
fn scan_proc() -> (u64, u64, Vec<(String, u64)>) {
    let mut per_pid: Vec<(String, u64)> = Vec::new();
    let Ok(entries) = std::fs::read_dir("/proc") else {
        return (0, 0, Vec::new());
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let Some(name_str) = name.to_str() else {
            continue;
        };
        if !name_str.chars().all(|c| c.is_ascii_digit()) {
            continue;
        }
        let count = count_inotify_watches_for_pid(&entry.path());
        if count > 0 {
            let comm = std::fs::read_to_string(entry.path().join("comm"))
                .map_or_else(|_| name_str.to_string(), |s| s.trim().to_string());
            per_pid.push((comm, count));
        }
    }
    let total_watches: u64 = per_pid.iter().map(|(_, c)| c).sum();
    let total_procs = per_pid.len() as u64;
    per_pid.sort_by(|a, b| b.1.cmp(&a.1));
    per_pid.truncate(TOP_CONSUMERS_SHOWN);
    (total_watches, total_procs, per_pid)
}

/// A single inotify fd's fdinfo file contains one `inotify wd:N ...` line
/// per watch it holds, so the count is just a line-prefix count.
#[cfg(target_os = "linux")]
fn count_inotify_watches_for_pid(pid_dir: &Path) -> u64 {
    let Ok(fdinfos) = std::fs::read_dir(pid_dir.join("fdinfo")) else {
        return 0;
    };
    let mut total = 0u64;
    for fd in fdinfos.flatten() {
        if let Ok(content) = std::fs::read_to_string(fd.path()) {
            total += content
                .lines()
                .filter(|line| line.starts_with("inotify "))
                .count() as u64;
        }
    }
    total
}

/// Count directories anvil's default watcher filter would register. Uses
/// the same filter so the estimate matches what `watch` will actually do.
/// Linux-only — only the Linux collect path consults inotify headroom.
#[cfg(target_os = "linux")]
fn count_project_dirs(root: &Path) -> u64 {
    let filter = anvil_kernel::watcher::filter::FileFilter::default();
    let mut count = 0u64;
    for entry in walkdir::WalkDir::new(root)
        .into_iter()
        .filter_entry(|e| {
            // Prune ignored directories so we don't descend into them —
            // mirrors what start_watcher does.
            if !e.file_type().is_dir() {
                return true;
            }
            !filter.should_ignore(e.path())
        })
        .flatten()
    {
        if entry.file_type().is_dir() {
            count += 1;
        }
    }
    count
}

#[cfg(test)]
mod tests {
    use super::*;

    fn status(
        max_watches: u64,
        max_instances: u64,
        in_use_watches: u64,
        project_dirs: u64,
    ) -> InotifyStatus {
        InotifyStatus {
            max_watches,
            max_instances,
            in_use_watches,
            consuming_processes: 5,
            top_consumers: vec![
                ("tsserver".to_string(), 20_000),
                ("node".to_string(), 10_000),
            ],
            project_dirs,
        }
    }

    #[test]
    fn healthy_status_is_not_tight() {
        let s = status(
            RECOMMENDED_MAX_WATCHES,
            RECOMMENDED_MAX_INSTANCES,
            10_000,
            500,
        );
        assert!(!s.is_tight());
    }

    #[test]
    fn below_recommended_watch_limit_with_room_is_not_tight() {
        // Issue #1109: default Ubuntu limit of 65_536 is below the
        // recommended value, but a fresh-install user with a small
        // project has plenty of actual headroom — don't warn them just
        // because the kernel limit is low.
        let s = status(65_536, RECOMMENDED_MAX_INSTANCES, 1_000, 500);
        assert!(!s.is_tight());
    }

    #[test]
    fn below_recommended_instance_limit_with_room_is_not_tight() {
        // Same shape as above for `max_user_instances`. The instance
        // ceiling alone doesn't predict per-watch exhaustion, so don't
        // warn unless the watch headroom genuinely can't fit the project.
        let s = status(RECOMMENDED_MAX_WATCHES, 128, 10_000, 500);
        assert!(!s.is_tight());
    }

    #[test]
    fn fresh_install_default_ubuntu_does_not_warn() {
        // Issue #1109 regression: `anvil init` on a stock Ubuntu box
        // (max_user_watches = 8_192 historically, 65_536 today) with a
        // small project that fits comfortably in headroom must NOT
        // surface the inotify recommendation block.
        let s = status(8_192, 128, 200, 300);
        assert!(!s.is_tight(), "fresh-install default should be silent");
        assert!(
            recommendation_lines(&s).is_empty(),
            "fresh-install default should produce no recommendation lines"
        );
    }

    #[test]
    fn headroom_too_small_for_project_is_tight() {
        // Generous limits but real usage leaves no room for the project.
        let s = status(
            RECOMMENDED_MAX_WATCHES,
            RECOMMENDED_MAX_INSTANCES,
            RECOMMENDED_MAX_WATCHES - 50,
            10_000,
        );
        assert!(s.is_tight());
    }

    #[test]
    fn recommendation_lines_empty_for_healthy_host() {
        let s = status(
            RECOMMENDED_MAX_WATCHES,
            RECOMMENDED_MAX_INSTANCES,
            1_000,
            500,
        );
        assert!(recommendation_lines(&s).is_empty());
    }

    #[test]
    fn recommendation_lines_show_user_actionable_tip_when_tight() {
        // Headroom = 65_536 - 60_000 = 5_536; needed = 8_000 + 800 = 8_800.
        // Headroom < needed → genuinely tight, recommendation should fire.
        let s = status(65_536, 128, 60_000, 8_000);
        let lines = recommendation_lines(&s);
        assert!(!lines.is_empty());
        let joined = lines.join("\n");
        // Tip targets the lever the user actually has without root —
        // closing watch-heavy processes — and surfaces the top consumers
        // so they know what to close.
        assert!(
            joined.contains("closing watch-heavy processes"),
            "expected non-sudo tip, got:\n{joined}"
        );
        assert!(joined.contains("tsserver x20000"));
    }

    #[test]
    fn recommendation_lead_uses_soft_language() {
        // Issue #1109: the lead message must not claim `anvil watch`
        // "may miss file changes" — the user feedback was that this
        // overstates the impact and reads as alarming. Use the softer
        // "watch performance may be degraded on this machine" wording.
        let s = status(65_536, 128, 60_000, 8_000);
        let lines = recommendation_lines(&s);
        let joined = lines.join("\n");
        assert!(
            joined.contains("watch performance may be degraded on this machine"),
            "expected softened lead language, got:\n{joined}"
        );
        assert!(
            !joined.contains("may miss file changes"),
            "old alarming wording must not return, got:\n{joined}"
        );
    }

    #[test]
    fn recommendation_does_not_prescribe_sudo_or_sysctl() {
        // Issue #1109: the anvil binary must not tell the user to run
        // `sudo`, edit `/etc/sysctl.d/`, or call `sysctl` — that's a
        // host-management concern outside the CLI's remit. The tip
        // should stay in user-actionable territory.
        let s = status(65_536, 128, 60_000, 8_000);
        let lines = recommendation_lines(&s);
        let joined = lines.join("\n");
        for forbidden in ["sudo", "sysctl", "/etc/sysctl", "max_user_watches="] {
            assert!(
                !joined.contains(forbidden),
                "recommendation must not mention `{forbidden}`, got:\n{joined}"
            );
        }
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn count_project_dirs_skips_ignored_subtrees() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join("src/inner")).unwrap();
        std::fs::create_dir_all(tmp.path().join("node_modules/bad/deep")).unwrap();
        std::fs::create_dir_all(tmp.path().join(".git/refs")).unwrap();

        let count = count_project_dirs(tmp.path());
        // root + src + src/inner = 3; the node_modules and .git trees
        // should be pruned by the default filter.
        assert_eq!(count, 3, "got {count}");
    }
}
