use std::io::IsTerminal;
use std::path::Path;

use anvil_tui::surfaces::status::{
    GateRunResult, HookStatus, ProfileInfo, StatusData, StatusState,
};
use clap::Args;
use serde::Serialize;

use crate::GlobalArgs;

#[derive(Debug, Args)]
pub struct StatusArgs {}

pub fn run(_args: &StatusArgs, global: &GlobalArgs) -> anyhow::Result<()> {
    let data = gather_status_data(".");

    if global.json {
        print_json(&data)?;
    } else if !global.no_tui && std::io::stdout().is_terminal() {
        let state = StatusState::new(data);
        crate::tui::run_surface(state)?;
    } else {
        print_plain(&data);
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
    let has_husky_precommit = hooks.iter().any(|h| h.name == "pre-commit" && h.active);

    if !has_husky_precommit {
        let git_hook = root.join(".git/hooks/pre-commit");
        if git_hook.exists() {
            let active = is_executable(&git_hook);
            hooks.push(HookStatus {
                name: "pre-commit".to_string(),
                active,
                path: ".git/hooks/pre-commit".to_string(),
            });
        }
    }

    hooks
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

    let Ok(value) = serde_json::from_str::<serde_json::Value>(&contents) else {
        return ProfileInfo {
            name: "(invalid config)".to_string(),
            checks: vec![],
            path: ".anvilrc".to_string(),
        };
    };

    let checks = value
        .get("checks")
        .and_then(serde_json::Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(|v| {
                    // Check objects: { "name": "...", "enabled": true, ... }
                    if let Some(obj) = v.as_object() {
                        let name = obj.get("name")?.as_str()?;
                        let enabled = obj
                            .get("enabled")
                            .and_then(serde_json::Value::as_bool)
                            .unwrap_or(true);
                        if enabled {
                            return Some(name.to_string());
                        }
                        return None;
                    }
                    // Bare string fallback
                    v.as_str().map(String::from)
                })
                .collect()
        })
        .unwrap_or_default();

    ProfileInfo {
        name: "default".to_string(),
        checks,
        path: ".anvilrc".to_string(),
    }
}

/// Read the most recent gate runs from the cache index.
fn gather_recent_runs(root: &Path) -> Vec<GateRunResult> {
    let index_path = root.join(".anvil/cache/index.json");

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
        .filter_map(|(key, val)| parse_gate_entry(key, val))
        .collect();

    // Sort by timestamp descending, take 5 most recent.
    runs.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
    runs.truncate(5);
    runs
}

fn parse_gate_entry(key: &str, val: &serde_json::Value) -> Option<GateRunResult> {
    // Only process gate cache entries (key format: "gate:check:{name}:{hash}").
    if !key.starts_with("gate:") {
        return None;
    }

    // Timestamp comes from the index metadata, not the key.
    let created_at = val.get("created_at").and_then(serde_json::Value::as_f64)?;
    #[allow(clippy::cast_possible_truncation)]
    let timestamp = format_unix_timestamp(created_at as i64);

    // The index entry points to a result file; read summary fields if present,
    // otherwise fall back to the index entry's own fields.
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
// Platform helpers
// ---------------------------------------------------------------------------

#[cfg(unix)]
fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    path.metadata()
        .map(|m| m.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

#[cfg(not(unix))]
fn is_executable(path: &Path) -> bool {
    path.exists()
}

// ---------------------------------------------------------------------------
// Output: plain text
// ---------------------------------------------------------------------------

fn print_plain(data: &StatusData) {
    println!("ANVIL STATUS\n");

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

fn print_json(data: &StatusData) -> anyhow::Result<()> {
    let output = StatusOutput {
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
    fn gather_with_anvilrc_bare_strings() {
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
    fn gather_with_anvilrc_check_objects() {
        let dir = make_temp_dir();
        std::fs::write(
            dir.join(".anvilrc"),
            r#"{"checks": [
                {"name": "secret-detection", "enabled": true},
                {"name": "import-boundaries"},
                {"name": "disabled-check", "enabled": false}
            ]}"#,
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
                    "gate:check:secret-detection:abc123def456": {
                        "file": "abc123.json",
                        "created_at": 1710000000,
                        "passed": true,
                        "score": 0.95,
                        "checksRun": 8,
                        "checksPassed": 8,
                        "durationMs": 1850
                    },
                    "gate:check:import-boundaries:fed654cba321": {
                        "file": "fed654.json",
                        "created_at": 1709990000,
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
                "\"gate:check:chk{i}:hash{i}\":{{\"file\":\"h{i}.json\",\"created_at\":{ts},\"passed\":true,\"score\":0.9,\"checksRun\":1,\"checksPassed\":1,\"durationMs\":100}}"
            );
        }
        entries.push_str("}}");
        std::fs::write(cache_dir.join("index.json"), &entries).unwrap();

        let data = gather_status_data(dir.to_str().unwrap());
        assert_eq!(data.recent_runs.len(), 5);

        cleanup(&dir);
    }
}
