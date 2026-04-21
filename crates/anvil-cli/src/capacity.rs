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

/// Recommended limits for a modern monorepo dev box.
const RECOMMENDED_MAX_WATCHES: u64 = 524_288;
const RECOMMENDED_MAX_INSTANCES: u64 = 512;

/// Top consumers we show when surfacing pressure — keep it small to stay
/// actionable rather than dumping a `ps`-style table.
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
    /// configured limit that `anvil watch` is at real risk of missing
    /// changes in this project.
    pub fn is_tight(&self) -> bool {
        // Treat the limit itself being below the recommended value as
        // tight — most modern monorepos pay for it eventually even if
        // usage looks OK at this moment.
        if self.max_watches < RECOMMENDED_MAX_WATCHES {
            return true;
        }
        if self.max_instances < RECOMMENDED_MAX_INSTANCES {
            return true;
        }
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
    out.push("Inotify headroom is tight — `anvil watch` may miss file changes.".to_string());
    out.push(format!(
        "  watches in use:        {} / {}",
        status.in_use_watches, status.max_watches
    ));
    out.push(format!(
        "  instances limit:       {} (recommended: {})",
        status.max_instances, RECOMMENDED_MAX_INSTANCES
    ));
    out.push(format!(
        "  this project's dirs:   ~{}",
        status.project_dirs
    ));
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
    out.push(String::new());
    out.push("To raise the limits (survives reboot, takes effect immediately):".to_string());
    out.push(format!(
        "  echo 'fs.inotify.max_user_watches={RECOMMENDED_MAX_WATCHES}' | sudo tee /etc/sysctl.d/99-inotify.conf"
    ));
    out.push(format!(
        "  echo 'fs.inotify.max_user_instances={RECOMMENDED_MAX_INSTANCES}' | sudo tee -a /etc/sysctl.d/99-inotify.conf"
    ));
    out.push("  sudo sysctl --system".to_string());
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
        let s = status(RECOMMENDED_MAX_WATCHES, RECOMMENDED_MAX_INSTANCES, 10_000, 500);
        assert!(!s.is_tight());
    }

    #[test]
    fn below_recommended_watch_limit_is_tight() {
        // Default Ubuntu limit of 65_536 is below recommended — flag it
        // even if current usage is fine.
        let s = status(65_536, RECOMMENDED_MAX_INSTANCES, 1_000, 500);
        assert!(s.is_tight());
    }

    #[test]
    fn below_recommended_instance_limit_is_tight() {
        let s = status(RECOMMENDED_MAX_WATCHES, 128, 10_000, 500);
        assert!(s.is_tight());
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
        let s = status(RECOMMENDED_MAX_WATCHES, RECOMMENDED_MAX_INSTANCES, 1_000, 500);
        assert!(recommendation_lines(&s).is_empty());
    }

    #[test]
    fn recommendation_lines_mentions_sysctl_when_tight() {
        let s = status(65_536, 128, 40_000, 8_000);
        let lines = recommendation_lines(&s);
        assert!(!lines.is_empty());
        let joined = lines.join("\n");
        assert!(joined.contains("max_user_watches=524288"));
        assert!(joined.contains("max_user_instances=512"));
        assert!(joined.contains("sudo sysctl --system"));
        assert!(joined.contains("tsserver x20000"));
    }

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
