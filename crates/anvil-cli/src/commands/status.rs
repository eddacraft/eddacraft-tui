use std::io::IsTerminal;
use std::path::Path;

use anvil_kernel_types::hooks::is_anvil_managed_command;
use anvil_tui::surfaces::status::{
    GateRunResult, HookStatus, ProfileInfo, StatusData, StatusState,
};
use clap::Args;
use serde::Serialize;

use crate::GlobalArgs;
use crate::activation;
use crate::commands::hooks::{config_hooks_enabled, list_config_hook_commands};

#[derive(Debug, Args)]
pub struct StatusArgs {
    /// Print only the activation diagnostic (LAUNCH-008 / LAUNCH-012).
    ///
    /// Equivalent to `anvil start --verify`'s status mode — a
    /// non-mutating layered probe of the activation pipeline that
    /// reports the literal protection state (`protecting`,
    /// `ready_restart_required`, `watching`, `needs_action`,
    /// `unsupported`, or `error`) without touching config.
    #[arg(long)]
    pub verify: bool,
}

pub fn run(args: &StatusArgs, global: &GlobalArgs) -> anyhow::Result<()> {
    if args.verify {
        return run_verify(global);
    }

    let data = gather_status_data(".");
    let activation = activation::verify(Path::new("."));

    if global.json {
        print_json(&data, &activation)?;
    } else if !global.no_tui && std::io::stdout().is_terminal() {
        let state = StatusState::new(data);
        crate::tui::run_surface(state)?;
    } else {
        print_plain(&data, &activation);
    }

    Ok(())
}

/// LAUNCH-012: verification surface. Stand-alone activation probe
/// suitable for `anvil status --verify`. Non-mutating: never writes
/// config, never spawns subprocesses outside read-only probes. The
/// `anvil start --verify` form forwards here once LAUNCH-006 promotes
/// the start command.
fn run_verify(global: &GlobalArgs) -> anyhow::Result<()> {
    let activation = activation::verify(Path::new("."));
    if global.json {
        let json = serde_json::to_string_pretty(&activation::render_json(&activation))?;
        println!("{json}");
    } else {
        print!("{}", activation::render_human(&activation));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Data gathering
// ---------------------------------------------------------------------------

fn gather_status_data(root: &str) -> StatusData {
    let root = Path::new(root);
    StatusData {
        hooks: gather_hooks(root),
        profile: gather_profile(root),
        recent_runs: gather_recent_runs(root),
    }
}

/// Check well-known hook locations and report whether each is active.
///
/// Reports two source types per event:
///
/// 1. **File-mode** — `.husky/<event>` and (for `pre-commit` only) the
///    bare `.git/hooks/pre-commit` fallback when no Husky entry is active.
/// 2. **Config-mode** — Git 2.54 native `hook.<event>.command` entries
///    surfaced via `git config --get-all`. Each entry produces its own
///    `HookStatus` row with the path set to a `git config hook.<event>.command`
///    label so the surface distinguishes config-mode from file-mode at a
///    glance. Anvil-managed entries are tagged with `(anvil-managed)` in
///    the path label so users can tell their custom commands apart from
///    Anvil's.
fn gather_hooks(root: &Path) -> Vec<HookStatus> {
    let candidates = [
        ("pre-commit", ".husky/pre-commit"),
        ("pre-push", ".husky/pre-push"),
        ("post-merge", ".husky/post-merge"),
    ];

    let mut hooks: Vec<HookStatus> = candidates
        .iter()
        .map(|(name, rel)| {
            let full = root.join(rel);
            let active = is_executable(&full);
            HookStatus {
                name: (*name).to_string(),
                active,
                path: if full.exists() {
                    rel.to_string()
                } else {
                    String::new()
                },
            }
        })
        .collect();

    // Fallback: bare git hook if no husky pre-commit was found.
    // Resolve worktree `.git` files (pointer to real git dir) so hooks are
    // found regardless of checkout type.
    let has_husky_precommit = hooks.iter().any(|h| h.name == "pre-commit" && h.active);

    if !has_husky_precommit {
        let git_dir = resolve_git_dir(root);
        let git_hook = git_dir.join("hooks/pre-commit");
        if git_hook.exists() {
            let rel = match git_hook.strip_prefix(root) {
                Ok(p) => p.to_string_lossy().into_owned(),
                Err(_) => git_hook.to_string_lossy().into_owned(),
            };
            let active = is_executable(&git_hook);
            hooks.push(HookStatus {
                name: "pre-commit".to_string(),
                active,
                path: rel,
            });
        }
    }

    // Append config-mode (`git config hook.<event>.command`) entries so
    // GHOOK-002-installed hooks are first-class in the dashboard. These
    // surface alongside file-mode rows; file mode remains the default
    // detection branch per the GHOOK-001 compatibility policy.
    //
    // `hook.<event>.enabled = false` flips Git's runtime behaviour off
    // even when commands are present, so the row reflects that — an
    // explicit-disabled config entry is reported `active: false` with a
    // `(disabled)` label, matching what Git will actually do.
    for event in ["pre-commit", "pre-push", "post-merge"] {
        let commands = list_config_hook_commands_safe(root, event);
        if commands.is_empty() {
            continue;
        }
        let enabled = config_hooks_enabled(root, event);
        for cmd in commands {
            let owner = if is_anvil_managed_command(&cmd) {
                " (anvil-managed)"
            } else {
                ""
            };
            let state = if enabled { "" } else { " (disabled)" };
            let label = format!("git config hook.{event}.command{owner}{state}");
            hooks.push(HookStatus {
                name: event.to_string(),
                active: enabled,
                path: label,
            });
        }
    }

    hooks
}

/// Wrapper that swallows `git config` failures so the status surface keeps
/// rendering even on hosts where `git` is missing or the repo's config is
/// transiently broken. Config-mode detection is best-effort: a missing
/// `git` should never gate the dashboard.
fn list_config_hook_commands_safe(root: &Path, event: &str) -> Vec<String> {
    list_config_hook_commands(root, event).unwrap_or_default()
}

/// Read `.anvilrc` for profile configuration.
fn gather_profile(root: &Path) -> ProfileInfo {
    let rc_path = root.join(".anvilrc");

    let Ok(contents) = std::fs::read_to_string(&rc_path) else {
        return ProfileInfo {
            name: "(no config)".to_string(),
            checks: vec![],
            path: ".anvilrc".to_string(),
        };
    };

    // Try JSON first, then fall back to simple YAML/TOML key extraction
    // (init can generate any of the three formats).
    let checks = if let Ok(value) = serde_json::from_str::<serde_json::Value>(&contents) {
        value
            .get("checks")
            .and_then(serde_json::Value::as_array)
            .map(|arr| {
                arr.iter()
                    .filter_map(serde_json::Value::as_str)
                    .map(String::from)
                    .collect()
            })
            .unwrap_or_default()
    } else if contents.contains("schemaVersion:") || contents.contains("schema_version =") {
        // Recognised YAML or TOML format — extract checks.
        parse_checks_from_text(&contents)
    } else {
        return ProfileInfo {
            name: "(invalid config)".to_string(),
            checks: vec![],
            path: ".anvilrc".to_string(),
        };
    };

    ProfileInfo {
        name: "default".to_string(),
        checks,
        path: ".anvilrc".to_string(),
    }
}

/// Read the most recent gate runs from the cache index.
fn gather_recent_runs(root: &Path) -> Vec<GateRunResult> {
    let index_path = root.join(".anvil/cache/index.json");
    let cache_dir = root.join(".anvil/cache");

    let Ok(contents) = std::fs::read_to_string(&index_path) else {
        return vec![];
    };

    let Ok(value) = serde_json::from_str::<serde_json::Value>(&contents) else {
        return vec![];
    };

    let Some(entries) = value.get("entries").and_then(serde_json::Value::as_object) else {
        return vec![];
    };

    let mut runs: Vec<GateRunResult> = entries
        .iter()
        .filter_map(|(key, val)| {
            // Try loading the actual entry file for full gate results.
            // The index entry only has metadata (file, created_at, expires_at, size_bytes).
            if let Some(file) = val.get("file").and_then(serde_json::Value::as_str)
                && !file.contains('/')
                && !file.contains('\\')
                && file != ".."
                && file != "."
            {
                // Try entries/ subdirectory (workspace FileCacheProvider format)
                // then cache root (standalone format).
                let entry_path = cache_dir.join("entries").join(file);
                let entry_path = if entry_path.exists() {
                    entry_path
                } else {
                    cache_dir.join(file)
                };
                if let Ok(entry_contents) = std::fs::read_to_string(entry_path) {
                    // Workspace FileCacheProvider writes entries as
                    // <64-hex-hmac>\n<json>. Skip the HMAC prefix.
                    let json_str = if entry_contents.len() > 65
                        && entry_contents.as_bytes()[64] == b'\n'
                        && entry_contents[..64].chars().all(|c| c.is_ascii_hexdigit())
                    {
                        &entry_contents[65..]
                    } else {
                        &entry_contents
                    };
                    if let Ok(entry_val) = serde_json::from_str::<serde_json::Value>(json_str) {
                        let gate_val = entry_val.get("value").unwrap_or(&entry_val);
                        return parse_gate_entry(key, gate_val);
                    }
                }
            }
            // Fall back to parsing the index entry directly.
            parse_gate_entry(key, val)
        })
        .collect();

    // Sort by timestamp descending, take 5 most recent.
    runs.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
    runs.truncate(5);
    runs
}

fn parse_gate_entry(key: &str, val: &serde_json::Value) -> Option<GateRunResult> {
    // Try timestamp from the last colon-separated segment of the key.
    // Falls back to `created_at` field in the entry metadata (used by the
    // runtime file-cache provider whose keys are `gate:check:<name>:<hash>`).
    let ts: i64 = key
        .rsplit(':')
        .next()
        .and_then(|s| s.parse().ok())
        .or_else(|| {
            // Runtime file-cache writes created_at as Date.now() (milliseconds).
            val.get("created_at")
                .and_then(serde_json::Value::as_i64)
                .map(|ms| ms / 1000)
        })?;

    let timestamp = format_unix_timestamp(ts);

    let passed = val
        .get("passed")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);
    let score = val
        .get("score")
        .and_then(serde_json::Value::as_f64)
        .unwrap_or(0.0);
    #[allow(clippy::cast_possible_truncation)]
    let checks_run = val
        .get("checksRun")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(0) as usize;
    #[allow(clippy::cast_possible_truncation)]
    let checks_passed = val
        .get("checksPassed")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(0) as usize;
    let duration_ms = val
        .get("durationMs")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(0);

    Some(GateRunResult {
        timestamp,
        passed,
        score,
        checks_run,
        checks_passed,
        duration_ms,
    })
}

/// Format a Unix timestamp as `YYYY-MM-DD HH:MM` (UTC, no external crate).
fn format_unix_timestamp(secs: i64) -> String {
    // Days from epoch algorithm (civil from days).
    let days_since_epoch = secs.div_euclid(86400);
    let time_of_day = secs.rem_euclid(86400);

    let hours = time_of_day / 3600;
    let minutes = (time_of_day % 3600) / 60;

    // Algorithm from Howard Hinnant's date library (public domain).
    let z = days_since_epoch + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };

    format!("{y:04}-{m:02}-{d:02} {hours:02}:{minutes:02}")
}

// ---------------------------------------------------------------------------
// Config parsing helpers
// ---------------------------------------------------------------------------

/// Extract check names from YAML or TOML config text.
fn parse_checks_from_text(text: &str) -> Vec<String> {
    let mut checks = Vec::new();
    let mut in_checks = false;
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("checks") {
            in_checks = true;
            // TOML inline: checks = ["a", "b"]
            if let Some(bracket) = trimmed.find('[') {
                for item in trimmed[bracket..].split('"') {
                    let item = item
                        .trim()
                        .trim_matches(|c| c == '[' || c == ']' || c == ',');
                    if !item.is_empty()
                        && item != ","
                        && !item.starts_with('[')
                        && !item.starts_with(']')
                    {
                        checks.push(item.to_string());
                    }
                }
                in_checks = false;
            }
            continue;
        }
        if in_checks {
            // YAML list item: `  - "name"` or `  - name`
            if let Some(rest) = trimmed.strip_prefix("- ") {
                let name = rest.trim().trim_matches('"');
                if !name.is_empty() {
                    checks.push(name.to_string());
                }
            } else if !trimmed.starts_with('-') && !trimmed.is_empty() {
                in_checks = false;
            }
        }
    }
    checks
}

// ---------------------------------------------------------------------------
// Git helpers
// ---------------------------------------------------------------------------

/// Resolve the actual git directory. In worktrees, `.git` is a file containing
/// `gitdir: <path>` rather than a directory.
fn resolve_git_dir(root: &Path) -> std::path::PathBuf {
    let dot_git = root.join(".git");
    if dot_git.is_file()
        && let Ok(content) = std::fs::read_to_string(&dot_git)
        && let Some(path) = content.strip_prefix("gitdir: ")
    {
        let path = path.trim();
        return if Path::new(path).is_absolute() {
            std::path::PathBuf::from(path)
        } else {
            root.join(path)
        };
    }
    dot_git
}

// ---------------------------------------------------------------------------
// Platform helpers
// ---------------------------------------------------------------------------

#[cfg(unix)]
fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    path.metadata()
        .is_ok_and(|m| m.permissions().mode() & 0o111 != 0)
}

#[cfg(not(unix))]
fn is_executable(path: &Path) -> bool {
    path.exists()
}

// ---------------------------------------------------------------------------
// Output: plain text
// ---------------------------------------------------------------------------

fn print_plain(data: &StatusData, activation_diag: &activation::ActivationDiagnostic) {
    println!("ANVIL STATUS\n");

    // Activation header is rendered first so the literal protection
    // claim is the first thing the user sees — they should not have
    // to scroll past hooks/profile/runs to learn whether Anvil is
    // protecting their repo.
    print!("{}", activation::render_human(activation_diag));
    println!();

    println!("HOOKS");
    for hook in &data.hooks {
        let (icon, label) = if hook.active {
            ("\u{2713}", "active")
        } else if hook.path.is_empty() {
            ("\u{25cb}", "missing")
        } else {
            ("\u{2717}", "inactive")
        };

        if hook.path.is_empty() {
            println!("  {icon} {:<14} {label}", hook.name);
        } else {
            println!("  {icon} {:<14} {:<10} {}", hook.name, label, hook.path);
        }
    }

    println!();
    println!("PROFILE: {}", data.profile.name);
    if data.profile.checks.is_empty() {
        println!("  Checks: (none)");
    } else {
        println!("  Checks: {}", data.profile.checks.join(", "));
    }
    println!("  Config: {}", data.profile.path);

    println!();
    println!("RECENT RUNS");
    if data.recent_runs.is_empty() {
        println!("  (no recent runs)");
    } else {
        for run in &data.recent_runs {
            let icon = if run.passed { "\u{2713}" } else { "\u{2717}" };
            println!(
                "  {icon} {}  {}/{} checks  {:.2}  {}ms",
                run.timestamp, run.checks_passed, run.checks_run, run.score, run.duration_ms,
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Output: JSON
// ---------------------------------------------------------------------------

#[derive(Serialize)]
struct StatusOutput {
    activation: serde_json::Value,
    hooks: Vec<HookOutput>,
    profile: ProfileOutput,
    recent_runs: Vec<RunOutput>,
}

#[derive(Serialize)]
struct HookOutput {
    name: String,
    active: bool,
    path: String,
}

#[derive(Serialize)]
struct ProfileOutput {
    name: String,
    checks: Vec<String>,
    path: String,
}

#[derive(Serialize)]
struct RunOutput {
    timestamp: String,
    passed: bool,
    score: f64,
    checks_run: usize,
    checks_passed: usize,
    duration_ms: u64,
}

fn print_json(
    data: &StatusData,
    activation_diag: &activation::ActivationDiagnostic,
) -> anyhow::Result<()> {
    let output = StatusOutput {
        activation: activation::render_json(activation_diag),
        hooks: data
            .hooks
            .iter()
            .map(|h| HookOutput {
                name: h.name.clone(),
                active: h.active,
                path: h.path.clone(),
            })
            .collect(),
        profile: ProfileOutput {
            name: data.profile.name.clone(),
            checks: data.profile.checks.clone(),
            path: data.profile.path.clone(),
        },
        recent_runs: data
            .recent_runs
            .iter()
            .map(|r| RunOutput {
                timestamp: r.timestamp.clone(),
                passed: r.passed,
                score: r.score,
                checks_run: r.checks_run,
                checks_passed: r.checks_passed,
                duration_ms: r.duration_ms,
            })
            .collect(),
    };

    let json = serde_json::to_string_pretty(&output)?;
    println!("{json}");
    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    use std::sync::atomic::{AtomicU64, Ordering};

    static COUNTER: AtomicU64 = AtomicU64::new(0);

    fn make_temp_dir() -> std::path::PathBuf {
        let id = COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir =
            std::env::temp_dir().join(format!("anvil-status-test-{}-{id}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }

    fn cleanup(dir: &Path) {
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn gather_empty_directory() {
        let dir = make_temp_dir();
        let data = gather_status_data(dir.to_str().unwrap());
        // All hooks should be inactive/missing.
        assert!(data.hooks.iter().all(|h| !h.active));
        // Profile should show no config.
        assert_eq!(data.profile.name, "(no config)");
        assert!(data.profile.checks.is_empty());
        // No recent runs.
        assert!(data.recent_runs.is_empty());

        cleanup(&dir);
    }

    #[test]
    fn gather_with_anvilrc() {
        let dir = make_temp_dir();
        std::fs::write(
            dir.join(".anvilrc"),
            r#"{"checks": ["secret-detection", "import-boundaries"], "format": "yaml"}"#,
        )
        .unwrap();

        let data = gather_status_data(dir.to_str().unwrap());
        assert_eq!(data.profile.name, "default");
        assert_eq!(data.profile.checks.len(), 2);
        assert_eq!(data.profile.checks[0], "secret-detection");
        assert_eq!(data.profile.checks[1], "import-boundaries");

        cleanup(&dir);
    }

    #[test]
    fn gather_with_invalid_anvilrc() {
        let dir = make_temp_dir();
        std::fs::write(dir.join(".anvilrc"), "not json at all").unwrap();

        let data = gather_status_data(dir.to_str().unwrap());
        assert_eq!(data.profile.name, "(invalid config)");
        assert!(data.profile.checks.is_empty());

        cleanup(&dir);
    }

    #[test]
    fn gather_with_cache_index() {
        let dir = make_temp_dir();
        let cache_dir = dir.join(".anvil/cache");
        std::fs::create_dir_all(&cache_dir).unwrap();
        std::fs::write(
            cache_dir.join("index.json"),
            r#"{
                "entries": {
                    "gate:plan.md:1710000000": {
                        "passed": true,
                        "score": 0.95,
                        "checksRun": 8,
                        "checksPassed": 8,
                        "durationMs": 1850
                    },
                    "gate:plan.md:1709990000": {
                        "passed": false,
                        "score": 0.75,
                        "checksRun": 8,
                        "checksPassed": 6,
                        "durationMs": 2100
                    }
                }
            }"#,
        )
        .unwrap();

        let data = gather_status_data(dir.to_str().unwrap());
        assert_eq!(data.recent_runs.len(), 2);
        // Most recent first.
        assert!(data.recent_runs[0].passed);
        assert!(!data.recent_runs[1].passed);
        assert_eq!(data.recent_runs[0].checks_passed, 8);
        assert_eq!(data.recent_runs[1].checks_passed, 6);

        cleanup(&dir);
    }

    #[test]
    fn gather_with_file_cache_entries_subdir() {
        let dir = make_temp_dir();
        let cache_dir = dir.join(".anvil/cache");
        let entries_dir = cache_dir.join("entries");
        std::fs::create_dir_all(&entries_dir).unwrap();
        std::fs::write(
            cache_dir.join("index.json"),
            r#"{
                "entries": {
                    "gate:plan.md:1710000000": {
                        "file": "abc123.json",
                        "created_at": 1710000000000,
                        "size_bytes": 128
                    }
                }
            }"#,
        )
        .unwrap();
        std::fs::write(
            entries_dir.join("abc123.json"),
            format!(
                "{}\n{}",
                "a".repeat(64),
                r#"{"value":{"passed":true,"score":0.9,"checksRun":3,"checksPassed":3,"durationMs":1200}}"#
            ),
        )
        .unwrap();

        let data = gather_status_data(dir.to_str().unwrap());
        assert_eq!(data.recent_runs.len(), 1);
        assert!(data.recent_runs[0].passed);
        assert_eq!(data.recent_runs[0].checks_run, 3);
        assert_eq!(data.recent_runs[0].checks_passed, 3);

        cleanup(&dir);
    }

    #[test]
    fn gather_with_file_cache_entries_root() {
        let dir = make_temp_dir();
        let cache_dir = dir.join(".anvil/cache");
        std::fs::create_dir_all(&cache_dir).unwrap();
        std::fs::write(
            cache_dir.join("index.json"),
            r#"{
                "entries": {
                    "gate:plan.md:1710001000": {
                        "file": "root-entry.json",
                        "created_at": 1710001000000,
                        "size_bytes": 128
                    }
                }
            }"#,
        )
        .unwrap();
        std::fs::write(
            cache_dir.join("root-entry.json"),
            format!(
                "{}\n{}",
                "b".repeat(64),
                r#"{"value":{"passed":false,"score":0.5,"checksRun":4,"checksPassed":2,"durationMs":900}}"#
            ),
        )
        .unwrap();

        let data = gather_status_data(dir.to_str().unwrap());
        assert_eq!(data.recent_runs.len(), 1);
        assert!(!data.recent_runs[0].passed);
        assert_eq!(data.recent_runs[0].checks_run, 4);
        assert_eq!(data.recent_runs[0].checks_passed, 2);

        cleanup(&dir);
    }

    #[cfg(unix)]
    #[test]
    fn gather_hooks_with_executable() {
        use std::os::unix::fs::PermissionsExt;

        let dir = make_temp_dir();
        let husky_dir = dir.join(".husky");
        std::fs::create_dir_all(&husky_dir).unwrap();

        let hook_path = husky_dir.join("pre-commit");
        std::fs::write(&hook_path, "#!/bin/sh\nexit 0").unwrap();
        std::fs::set_permissions(&hook_path, std::fs::Permissions::from_mode(0o755)).unwrap();

        let hooks = gather_hooks(&dir);
        let pre_commit = hooks.iter().find(|h| h.name == "pre-commit").unwrap();
        assert!(pre_commit.active);
        assert_eq!(pre_commit.path, ".husky/pre-commit");

        cleanup(&dir);
    }

    #[test]
    fn format_timestamp_known_value() {
        // 2024-03-10 00:00:00 UTC = 1710028800
        let formatted = format_unix_timestamp(1_710_028_800);
        assert_eq!(formatted, "2024-03-10 00:00");
    }

    // --- parse_checks_from_text ---

    #[test]
    fn parse_checks_yaml_list() {
        let text = "schemaVersion: 1\nchecks:\n  - secret-detection\n  - import-boundaries\n";
        let checks = parse_checks_from_text(text);
        assert_eq!(checks, vec!["secret-detection", "import-boundaries"]);
    }

    #[test]
    fn parse_checks_yaml_quoted() {
        let text = "schemaVersion: 1\nchecks:\n  - \"secret-detection\"\n  - \"antipattern\"\n";
        let checks = parse_checks_from_text(text);
        assert_eq!(checks, vec!["secret-detection", "antipattern"]);
    }

    #[test]
    fn parse_checks_toml_inline() {
        let text = "schema_version = 1\nchecks = [\"secret-detection\", \"antipattern\"]\n";
        let checks = parse_checks_from_text(text);
        assert_eq!(checks, vec!["secret-detection", "antipattern"]);
    }

    #[test]
    fn parse_checks_empty_list() {
        let text = "schemaVersion: 1\nchecks:\nother: value\n";
        let checks = parse_checks_from_text(text);
        assert!(checks.is_empty());
    }

    #[test]
    fn parse_checks_toml_unquoted_mixed() {
        // Mixed quoting: "a" is quoted, b is bare. The split-on-quote parser
        // extracts both but preserves a leading space on the unquoted segment
        // because only bracket/comma chars are trimmed from the edges, not
        // whitespace between the comma and the bare value.
        let text = "schema_version = 1\nchecks = [\"a\", b]\n";
        let checks = parse_checks_from_text(text);
        assert_eq!(checks, vec!["a", " b"]);
    }

    #[test]
    fn parse_checks_no_checks_section() {
        let text = "schemaVersion: 1\nprofile: dev\n";
        let checks = parse_checks_from_text(text);
        assert!(checks.is_empty());
    }

    // --- parse_gate_entry ---

    #[test]
    fn parse_gate_entry_from_key_timestamp() {
        let val = serde_json::json!({
            "passed": true,
            "score": 0.95,
            "checksRun": 5,
            "checksPassed": 5,
            "durationMs": 1200
        });
        let result = parse_gate_entry("gate:plan.md:1710000000", &val).unwrap();
        assert!(result.passed);
        assert!((result.score - 0.95).abs() < f64::EPSILON);
        assert_eq!(result.checks_run, 5);
        assert_eq!(result.checks_passed, 5);
        assert_eq!(result.duration_ms, 1200);
    }

    #[test]
    fn parse_gate_entry_fallback_to_created_at() {
        let val = serde_json::json!({
            "created_at": 1_710_000_000_000_i64,
            "passed": false,
            "score": 0.5
        });
        // Key has no parseable timestamp suffix
        let result = parse_gate_entry("gate:check:secret:abcdef", &val).unwrap();
        assert!(!result.passed);
        assert!((result.score - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn parse_gate_entry_defaults_missing_fields() {
        let val = serde_json::json!({
            "created_at": 1_710_000_000_000_i64
        });
        let result = parse_gate_entry("gate:no-data:xxx", &val).unwrap();
        assert!(!result.passed);
        assert!(result.score.abs() < f64::EPSILON);
        assert_eq!(result.checks_run, 0);
        assert_eq!(result.checks_passed, 0);
        assert_eq!(result.duration_ms, 0);
    }

    #[test]
    fn parse_gate_entry_no_timestamp_returns_none() {
        let val = serde_json::json!({"passed": true});
        let result = parse_gate_entry("no-timestamp-key", &val);
        assert!(result.is_none());
    }

    // --- format_unix_timestamp ---

    #[test]
    fn format_epoch_zero() {
        assert_eq!(format_unix_timestamp(0), "1970-01-01 00:00");
    }

    #[test]
    fn format_known_date() {
        // 2024-01-01 12:00:00 UTC = 1704110400
        assert_eq!(format_unix_timestamp(1_704_110_400), "2024-01-01 12:00");
    }

    // --- resolve_git_dir ---

    #[test]
    fn resolve_git_dir_standard_directory() {
        let dir = make_temp_dir();
        std::fs::create_dir_all(dir.join(".git")).unwrap();
        let result = resolve_git_dir(&dir);
        assert_eq!(result, dir.join(".git"));
        cleanup(&dir);
    }

    #[test]
    fn resolve_git_dir_worktree_file() {
        let dir = make_temp_dir();
        let git_dir = dir.join("real-git-dir");
        std::fs::create_dir_all(&git_dir).unwrap();
        std::fs::write(dir.join(".git"), format!("gitdir: {}", git_dir.display())).unwrap();
        let result = resolve_git_dir(&dir);
        assert_eq!(result, git_dir);
        cleanup(&dir);
    }

    #[test]
    fn resolve_git_dir_missing_returns_dot_git() {
        let dir = make_temp_dir();
        let result = resolve_git_dir(&dir);
        assert_eq!(result, dir.join(".git"));
        cleanup(&dir);
    }

    #[test]
    fn truncates_to_five_runs() {
        use std::fmt::Write;

        let dir = make_temp_dir();
        let cache_dir = dir.join(".anvil/cache");
        std::fs::create_dir_all(&cache_dir).unwrap();

        let mut entries = String::from("{\"entries\":{");
        for i in 0..8 {
            if i > 0 {
                entries.push(',');
            }
            let ts = 1_710_000_000 + i * 1000;
            let _ = write!(
                entries,
                "\"gate:f.md:{ts}\":{{\"passed\":true,\"score\":0.9,\"checksRun\":1,\"checksPassed\":1,\"durationMs\":100}}"
            );
        }
        entries.push_str("}}");
        std::fs::write(cache_dir.join("index.json"), &entries).unwrap();

        let data = gather_status_data(dir.to_str().unwrap());
        assert_eq!(data.recent_runs.len(), 5);

        cleanup(&dir);
    }

    // --- GHOOK-003: config-mode hook detection ---

    /// Initialise a real git repo at `dir`. Returns `Ok(())` on success;
    /// callers should skip the test (not fail) when `git` is missing or the
    /// init fails — config-mode detection is not exercisable without a real
    /// `.git/config` to write into.
    fn try_git_init(dir: &Path) -> bool {
        std::process::Command::new("git")
            .args(["init", "-q"])
            .current_dir(dir)
            .status()
            .is_ok_and(|s| s.success())
    }

    fn add_config_hook(dir: &Path, event: &str, command: &str) {
        let key = format!("hook.{event}.command");
        let status = std::process::Command::new("git")
            .arg("-C")
            .arg(dir)
            .args(["config", "--add", &key, command])
            .status()
            .expect("git config --add");
        assert!(
            status.success(),
            "failed to seed config-mode hook for tests"
        );
    }

    /// (a) Config-mode-only repo reports the hook as installed.
    #[test]
    fn gather_hooks_reports_config_mode_only_install() {
        let dir = make_temp_dir();
        if !try_git_init(&dir) {
            eprintln!("skipping: git init unavailable");
            cleanup(&dir);
            return;
        }
        add_config_hook(&dir, "pre-commit", "ANVIL_HOOK=1 anvil gate --progress");

        let hooks = gather_hooks(&dir);
        let pre_commit_config: Vec<&HookStatus> = hooks
            .iter()
            .filter(|h| h.name == "pre-commit" && h.path.contains("git config"))
            .collect();
        assert_eq!(
            pre_commit_config.len(),
            1,
            "expected exactly one config-mode pre-commit row, got {hooks:?}",
        );
        assert!(pre_commit_config[0].active);
        assert!(
            pre_commit_config[0].path.contains("(anvil-managed)"),
            "anvil-managed entries must be tagged in the path label, got: {}",
            pre_commit_config[0].path,
        );

        cleanup(&dir);
    }

    /// (b) File-mode + config-mode both present reports both rows.
    #[test]
    #[cfg(unix)]
    fn gather_hooks_reports_both_file_and_config_modes() {
        use std::os::unix::fs::PermissionsExt;

        let dir = make_temp_dir();
        if !try_git_init(&dir) {
            eprintln!("skipping: git init unavailable");
            cleanup(&dir);
            return;
        }

        // File-mode hook in .husky/.
        let husky_dir = dir.join(".husky");
        std::fs::create_dir_all(&husky_dir).unwrap();
        let husky_hook = husky_dir.join("pre-commit");
        std::fs::write(&husky_hook, "#!/bin/sh\nexit 0").unwrap();
        std::fs::set_permissions(&husky_hook, std::fs::Permissions::from_mode(0o755)).unwrap();

        // Config-mode hook alongside it.
        add_config_hook(&dir, "pre-commit", "ANVIL_HOOK=1 anvil gate --progress");

        let hooks = gather_hooks(&dir);

        // File-mode row: path is `.husky/pre-commit`, active.
        let file_row = hooks
            .iter()
            .find(|h| h.name == "pre-commit" && h.path == ".husky/pre-commit")
            .expect("file-mode row must be present");
        assert!(file_row.active);

        // Config-mode row alongside it.
        let config_row = hooks
            .iter()
            .find(|h| h.name == "pre-commit" && h.path.contains("git config"))
            .expect("config-mode row must be present");
        assert!(config_row.active);
        assert!(config_row.path.contains("(anvil-managed)"));

        cleanup(&dir);
    }

    /// User-authored config-mode entries surface without the `(anvil-managed)`
    /// tag — the surface must distinguish the two so users can tell their
    /// own commands from Anvil's.
    #[test]
    fn gather_hooks_does_not_tag_user_authored_config_entries() {
        let dir = make_temp_dir();
        if !try_git_init(&dir) {
            eprintln!("skipping: git init unavailable");
            cleanup(&dir);
            return;
        }
        add_config_hook(&dir, "pre-commit", "npm run my-gate");

        let hooks = gather_hooks(&dir);
        let row = hooks
            .iter()
            .find(|h| h.name == "pre-commit" && h.path.contains("git config"))
            .expect("user-authored config-mode row must be reported");
        assert!(
            !row.path.contains("(anvil-managed)"),
            "user-authored entries must not be tagged as anvil-managed: {}",
            row.path,
        );

        cleanup(&dir);
    }
}
