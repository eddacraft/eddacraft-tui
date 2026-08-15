use std::io::IsTerminal;
use std::path::Path;
use std::time::Duration;

use anvil_intercept_proto::protocol::{AssuranceState, WorkspaceAssurance};
use anvil_intercept_proto::status::{DaemonStatusV1, SaveTimeDriverStatusV1};
use anvil_kernel_types::hooks::is_anvil_managed_command;
use anvil_kernel_types::protection_claim::{ProtectionClaim, WorktreeClaimState};
use anvil_tui::surfaces::status::{
    GateRunResult, HookStatus, ProfileInfo, StatusData, StatusState,
};
use clap::Args;
use serde::Serialize;

use crate::GlobalArgs;
use crate::activation;
use crate::commands::hooks::{
    config_hooks_enabled, is_config_mode_hook_path, list_config_hook_commands,
};
use crate::commands::protection_claim_section;
use crate::commands::status_mcp;
use crate::commands::watch_save_time;
use crate::config_summary::render_rule_mode_summary;

#[derive(Debug, Args)]
pub struct StatusArgs {
    /// Run a non-mutating activation probe — reports the current
    /// protection state (`protecting`, `ready_restart_required`,
    /// `watching`, `needs_action`, `unsupported`, or `error`)
    /// without touching config. Equivalent to `anvil start --verify`.
    #[arg(long)]
    pub verify: bool,
    /// Print per-tier activation evidence to stderr alongside the
    /// normal verdict on stdout. Only meaningful with `--verify`
    /// (the TUI and `--json` paths have their own diagnostic
    /// surfaces). Mirrors the same flag on `anvil start`.
    #[arg(long, requires = "verify")]
    pub why: bool,
}

pub fn run(args: &StatusArgs, global: &GlobalArgs) -> anyhow::Result<()> {
    if args.verify {
        return run_verify(args, global);
    }

    let mut data = gather_status_data(".");
    let activation = activation::verify(Path::new("."));
    // DSV-007 / UJ-005: best-effort save-time posture. A live daemon renders
    // its assurance + confinement; default-on routing with no daemon states
    // the off posture explicitly; explicit opt-in preserves the older
    // daemon-absent fallback surface; operator opt-out hides the line.
    let save_time = gather_save_time();
    let cli_version = env!("CARGO_PKG_VERSION");
    let graph = status_mcp::graph_from_assurance(save_time.assurance().map(|s| &s.assurance));
    // /proc inventory is only rendered on JSON and plain surfaces.
    // Skip the scan on the interactive TUI path (Copilot review).
    let mcp_inventory = if global.json || !status_prefers_tui(global) {
        status_mcp::gather_mcp_inventory(cli_version)
    } else {
        None
    };
    let protecting = matches!(
        activation.protection_state(),
        activation::state::ProtectionState::Protecting,
    );
    let mcp = status_mcp::status_mcp_json(
        cli_version,
        protecting,
        mcp_inventory.as_ref(),
        graph.as_ref(),
    );

    // DISTRIB-002: surface an update-available hint when one is
    // detected and the 24h rate-limit gate allows it. `--json` is
    // excluded from the rate-limit accounting because JSON consumers
    // are tooling, not humans, and consume the underlying
    // `anvil version --check` output directly. The hint is opt-out
    // via env var so a noisy CI does not have to add a flag everywhere.
    //
    // `include_advisories: true` so the hint names any
    // `Security-Advisory: GHSA-…` tag attached to the running
    // version's release body — the spec explicitly requires this
    // ("explicitly names any advisory tag attached to the running
    // version"). The second probe shares the same 3s timeout as the
    // latest-version probe; worst-case cold-start cost is 6s on a
    // dead network, capped at one hit per 24h by the rate-limit gate.
    if !global.json && std::env::var_os("ANVIL_DISABLE_UPDATE_HINT").is_none() {
        data.update_hint = crate::commands::version::compute_update_hint(true);
    }

    // INSIGHTS-004: first-week nudge (local-only, 14d window from
    // project-id created_at, once-per-week, suppressed after running
    // the default insights summary). Independent of the update rate
    // limit; opt-out is not provided (low noise by design).
    // Use workspace_root (best-effort git toplevel, fallback .) so that
    // subdir invocations still read/write the project .anvil/ state and
    // project-id consistently with `anvil watch` (and with gather for
    // witness etc).
    if !global.json {
        use chrono::Utc;
        let hint_root =
            crate::util::workspace_root().unwrap_or_else(|_| Path::new(".").to_path_buf());
        data.insights_hint = crate::insights::first_week_hint::first_week_insights_hint(
            &hint_root,
            Utc::now(),
            crate::install_root::project_writes_gated(),
        );
        // UJ-010: post-upgrade what's-new one-liner, once per version change.
        // Gathered ONLY when the plain surface will render (the same predicate
        // as the dispatch below, inverted): computing it consumes the
        // exactly-once marker, so gathering on the TUI path — which does not
        // render this hint — would silently burn the announcement.
        if global.no_tui || !std::io::stdout().is_terminal() {
            data.whats_new_hint =
                crate::whats_new::post_upgrade_hint(&hint_root, env!("CARGO_PKG_VERSION"));
        }
    }

    if global.json {
        // MLP2-048: IPC query_status for real ProtectionClaim surfaces; on
        // failure fall back to local-only (empty surfaces). Canonicalise paths
        // the same way the daemon does on register.
        let daemon_snapshot = match crate::commands::intercept::query_daemon_status() {
            Ok(snapshot) => Some(snapshot),
            Err(err) => {
                tracing::debug!(
                    error = %err,
                    "anvil status --json: daemon IPC unavailable; falling back to local-only protection claim",
                );
                None
            }
        };
        let worktree = std::fs::canonicalize(".").unwrap_or_else(|err| {
            tracing::warn!(
                error = %err,
                "anvil status --json: cwd canonicalise failed; protection claim will not match any daemon-registered session",
            );
            Path::new(".").to_path_buf()
        });
        print_json(
            &data,
            &activation,
            daemon_snapshot.as_ref(),
            &worktree,
            save_time.assurance(),
            mcp,
        )?;
    } else if status_prefers_tui(global) {
        let state = StatusState::new(data);
        crate::tui::run_surface(state)?;
    } else {
        warn_if_status_tui_unavailable(global);
        print_plain(
            &data,
            &activation,
            save_time,
            cli_version,
            mcp_inventory.as_ref(),
            graph.as_ref(),
        );
    }

    Ok(())
}

/// LAUNCH-012: verification surface. Stand-alone activation probe
/// suitable for `anvil status --verify`. Non-mutating: never writes
/// config, never spawns subprocesses outside read-only probes. The
/// `anvil start --verify` form forwards here once LAUNCH-006 promotes
/// the start command.
fn run_verify(args: &StatusArgs, global: &GlobalArgs) -> anyhow::Result<()> {
    let activation = activation::verify(Path::new("."));
    let cli_version = env!("CARGO_PKG_VERSION");
    let mcp_inventory = status_mcp::gather_mcp_inventory(cli_version);
    let save_time = gather_save_time();
    let graph = status_mcp::graph_from_assurance(save_time.assurance().map(|s| &s.assurance));
    let protecting = matches!(
        activation.protection_state(),
        activation::state::ProtectionState::Protecting,
    );
    if global.json {
        let mut value = activation::render_json(&activation);
        if let Some(mcp) = status_mcp::status_mcp_json(
            cli_version,
            protecting,
            mcp_inventory.as_ref(),
            graph.as_ref(),
        ) {
            merge_status_mcp_json(&mut value, &mcp);
        }
        let json = serde_json::to_string_pretty(&value)?;
        println!("{json}");
    } else {
        print!("{}", activation::render_human(&activation));
        print!("{}", render_rule_mode_summary(Path::new(".")));
        print!(
            "{}",
            status_mcp::render_status_mcp_plain(
                cli_version,
                protecting,
                mcp_inventory.as_ref(),
                graph.as_ref(),
            )
        );
        // MLP2-051g — verbose tier-evidence on stderr. Suppressed
        // under `--json` (consumers expect a single JSON document on
        // stdout; the stderr block does not change that contract,
        // but printing a block of free-form text alongside a machine
        // surface invites brittle parsers downstream).
        if args.why {
            eprint!("{}", activation::render_human_verbose(&activation));
        }
    }
    Ok(())
}

fn merge_status_mcp_json(value: &mut serde_json::Value, mcp: &status_mcp::StatusMcpJson) {
    let Ok(extra) = serde_json::to_value(mcp) else {
        return;
    };
    let Some(obj) = value.as_object_mut() else {
        return;
    };
    let Some(map) = extra.as_object() else {
        return;
    };
    for (key, extra_value) in map {
        obj.insert(key.clone(), extra_value.clone());
    }
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
        // DISTRIB-002: probe + rate-limit wiring is done by
        // [`gather_status_data_with_update_hint`]. Callers that want
        // the hint should use that wrapper; the bare gather stays
        // None so existing call sites (tests, --json) are unaffected.
        update_hint: None,
        // INSIGHTS-004: populated by caller (status run) after gather,
        // same pattern as update_hint so tests and --json paths stay
        // unaffected by the nudge.
        insights_hint: None,
        // UJ-010: populated by caller (status run) after gather, same
        // pattern as the hints above.
        whats_new_hint: None,
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
///    glance. anvil-managed entries are tagged with `(anvil-managed)` in
///    the path label so users can tell their custom commands apart from
///    anvil's.
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
            // CIB-251: config-mode rows never claim verified fire. Disabled
            // stays explicit; enabled-but-unverified is the honest default.
            let state = if enabled {
                " (fire not verified)"
            } else {
                " (disabled)"
            };
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

/// Read the project config (canonical `.anvil.<ext>` first, legacy
/// `.anvilrc` fallback — UCFG-001) for profile configuration.
fn gather_profile(root: &Path) -> ProfileInfo {
    // `discover` builds its candidate names from ASCII literals, so the
    // non-UTF-8 fallback below is unreachable for discovered paths.
    let (config_path, label) = match anvil_config::discover(root, ".anvil") {
        Ok(Some(discovered)) => {
            let label = discovered
                .path
                .file_name()
                .and_then(std::ffi::OsStr::to_str)
                .unwrap_or(".anvil.yaml")
                .to_string();
            (discovered.path, label)
        }
        _ => (root.join(".anvilrc"), ".anvilrc".to_string()),
    };

    let Ok(contents) = std::fs::read_to_string(&config_path) else {
        return ProfileInfo {
            name: "(no config)".to_string(),
            checks: vec![],
            path: label.clone(),
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
    } else if contents.contains("schema_version:")
        || contents.contains("schemaVersion:")
        || contents.contains("schema_version =")
        || contents.contains("schemaVersion =")
    {
        // Recognised YAML or TOML format — canonical snake_case (ADR-120)
        // or a legacy camelCase writer's output — extract checks.
        parse_checks_from_text(&contents)
    } else {
        return ProfileInfo {
            name: "(invalid config)".to_string(),
            checks: vec![],
            path: label.clone(),
        };
    };

    ProfileInfo {
        name: "default".to_string(),
        checks,
        path: label,
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

fn print_plain(
    data: &StatusData,
    activation_diag: &activation::ActivationDiagnostic,
    save_time: SaveTimePosture,
    cli_version: &str,
    mcp_inventory: Option<&status_mcp::McpProcessInventory>,
    graph: Option<&status_mcp::GraphReadiness>,
) {
    // Resolve the repo root once so the witness chain at
    // `<repo-root>/anvil/witness/active.ndjson` is found even when
    // `anvil status` is invoked from a subdirectory. Falls back to
    // the CWD when `git rev-parse` is unavailable or the directory
    // is not a git repo — the legible surface still renders, the
    // witness line just reports "none yet" instead of pointing at
    // the wrong tree.
    let root = resolve_repo_root().unwrap_or_else(|| Path::new(".").to_path_buf());
    let snapshot = build_legible_snapshot(data, activation_diag, &root, save_time);
    print!("{}", render_plain_legible(&snapshot));
    // Rule-mode summary line is appended as advisory context. The
    // 24-row budget is for the FULL `anvil status` plain output, so
    // the legible block reserves up to two extra lines for the rule-
    // mode summary (a single line in the common case, three when the
    // config is invalid).
    print!("{}", render_rule_mode_summary(&root));
    // ACTMO-017: surface the durably-registered worktrees and whether the
    // current directory is among them. Best-effort: a daemon-down query renders
    // a degraded line rather than omitting the section.
    let registered_snapshot = crate::commands::intercept::query_daemon_status().ok();
    let cwd = std::fs::canonicalize(".").ok();
    print!(
        "{}",
        render_registered_worktrees(registered_snapshot.as_ref(), cwd.as_deref())
    );
    let protecting = matches!(
        activation_diag.protection_state(),
        activation::state::ProtectionState::Protecting,
    );
    print!(
        "{}",
        status_mcp::render_status_mcp_plain(cli_version, protecting, mcp_inventory, graph)
    );
    // DISTRIB-002 update hint, INSIGHTS-004 nudge, and the UJ-010
    // what's-new line share the single footer line in plain output.
    // INSIGHTS-004 takes priority (matching the TUI watch strip, which
    // renders insights over update); the what's-new line outranks the
    // update hint because it fires exactly once per version while the
    // update hint repeats daily. The what's-new line is plain-only by
    // design — its gather is coupled to this surface so the
    // exactly-once marker is never consumed without rendering.
    if let Some(hint) = &data.insights_hint {
        println!("{hint}");
    } else if let Some(hint) = &data.whats_new_hint {
        println!("{hint}");
    } else if let Some(hint) = &data.update_hint {
        println!("{}", hint.render_line());
    }
}

/// ACTMO-017: render the registered-worktrees section for plain `anvil status`.
///
/// The **membership** axis (`registered` / `fenced` / `cascaded`) is derived
/// from the daemon's per-session `WorktreeStatusV1` overlay; the current
/// directory is flagged. The `protecting` vs `watching` (assurance) split is a
/// soft-dependency on DSV-046's driver and is not asserted here, per ADR-094
/// decision 6 — this surface shows membership truthfully without over-claiming.
fn render_registered_worktrees(snapshot: Option<&DaemonStatusV1>, cwd: Option<&Path>) -> String {
    use std::fmt::Write as _;

    let norm = |path: &Path| dunce::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let cwd = cwd.map(norm);
    let mut out = String::new();
    let Some(snapshot) = snapshot else {
        let _ = writeln!(out, "Registered worktrees: (daemon unavailable)");
        return out;
    };
    let registered = snapshot.registered_worktrees();
    if registered.is_empty() {
        let _ = writeln!(out, "Registered worktrees: (none)");
        return out;
    }
    let _ = writeln!(out, "Registered worktrees:");
    let mut cwd_listed = false;
    for worktree in &registered {
        let display = norm(worktree);
        let label = membership_label(snapshot, worktree);
        let driver = driver_segment(snapshot, worktree);
        let is_cwd = cwd.as_deref() == Some(display.as_path());
        cwd_listed |= is_cwd;
        let marker = if is_cwd { " (current)" } else { "" };
        let _ = writeln!(out, "  {} [{label}]{driver}{marker}", display.display());
    }
    if cwd.is_some() && !cwd_listed {
        let _ = writeln!(out, "  (current directory is not registered)");
    }
    out
}

/// DSV-049: the save-time driver segment appended to a registered worktree's
/// plain line. Silent for `Absent`/`Unknown` so the common case (driver
/// supervision off, or nothing attached) stays byte-identical to the
/// pre-DSV-049 surface; surfaced only when there is real attachment evidence
/// (`attached`) or an honest failure (`failed`). `Unknown` — a driver state
/// from a newer daemon — folds to silent, matching the wire contract's
/// "treat unknown fail-safe as absent" rule.
fn driver_segment(snapshot: &DaemonStatusV1, worktree: &Path) -> &'static str {
    let overlay = snapshot
        .worktrees
        .iter()
        .find(|entry| entry.worktree == worktree);
    match overlay.map(|entry| entry.save_time_driver) {
        Some(SaveTimeDriverStatusV1::Attached) => " driver: attached",
        Some(SaveTimeDriverStatusV1::Failed) => " driver: failed",
        _ => "",
    }
}

/// ACTMO-017 membership axis: a registered worktree is `cascaded` or `fenced`
/// when the daemon's overlay says so, else plain `registered`. "unregistered"
/// is the absence of an entry, so it is never a label here.
fn membership_label(snapshot: &DaemonStatusV1, worktree: &Path) -> &'static str {
    let overlay = snapshot
        .worktrees
        .iter()
        .find(|entry| entry.worktree == worktree);
    match overlay {
        Some(entry) if entry.cascaded => "cascaded",
        Some(entry) if entry.fenced => "fenced",
        _ => "registered",
    }
}

// ---------------------------------------------------------------------------
// DSV-007 Task 17: save-time assurance + confinement surface
// ---------------------------------------------------------------------------

/// The save-time assurance snapshot rendered by `anvil status`, plus the
/// operator confinement size. Built when daemon routing is explicitly forced on,
/// or when default-on routing finds a live daemon; `None` keeps the default
/// status output unchanged for non-daemon users.
#[derive(Debug, Clone)]
struct SaveTimeRender {
    /// The current assurance. A daemon-absent query is folded to
    /// `unavailable{daemon-absent}` so status never renders a stale cached
    /// `clean` when the daemon is gone.
    assurance: WorkspaceAssurance,
    /// Number of operator allow entries when the daemon is in `Allowlist`
    /// (confined) mode; `None` in open mode.
    confined: Option<usize>,
}

/// JSON projection of [`SaveTimeRender`] for the `--json` surface. Additive
/// (`additionalProperties: true`), so it does not bump the status schema.
#[derive(Serialize)]
struct SaveTimeOutput {
    state: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    reason: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    confined: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    last_full_scan: Option<String>,
}

/// The save-time posture `anvil status` renders (UJ-005). Status is the home
/// screen, so the posture is always stated — except under an explicit operator
/// opt-out, which keeps the pre-DSV surface.
#[derive(Debug, Clone)]
enum SaveTimePosture {
    /// A daemon assurance to render: the daemon answered, or routing is forced
    /// on and absence folds to `unavailable{daemon-absent}`.
    Assurance(SaveTimeRender),
    /// Default-on routing with no live daemon: render an explicit off-state
    /// line naming `anvil start` instead of omitting the line (UJ-005 revisits
    /// the DSV-021 omission under the beta guide-users posture).
    Off,
    /// Operator opt-out (`ANVIL_WATCH_DAEMON=0`): no save-time line at all.
    Hidden,
}

impl SaveTimePosture {
    /// The assurance render when one exists. The `--json` surface stays
    /// additive: `save_time` is emitted only for assurance postures, exactly
    /// as before UJ-005.
    fn assurance(&self) -> Option<&SaveTimeRender> {
        match self {
            SaveTimePosture::Assurance(render) => Some(render),
            SaveTimePosture::Off | SaveTimePosture::Hidden => None,
        }
    }
}

/// Gather the save-time posture for the status surfaces. Queries the daemon
/// unless routing is disabled, then classifies via [`classify_save_time`].
fn gather_save_time() -> SaveTimePosture {
    let mode = watch_save_time::daemon_routing_mode();
    if mode == watch_save_time::DaemonRoutingMode::Disabled {
        return SaveTimePosture::Hidden;
    }
    let workspace = crate::util::workspace_root().unwrap_or_else(|_| Path::new(".").to_path_buf());
    let queried = watch_save_time::query_workspace_status(&workspace);
    let confined = if queried.is_some() {
        confinement_allow_count()
    } else {
        None
    };
    classify_save_time(mode, queried, confined)
}

/// Pure classifier for the routing-mode × daemon-presence posture matrix.
/// Total over the matrix: `Disabled` folds to `Hidden` here too, even though
/// [`gather_save_time`] short-circuits it before querying. `confined` is the
/// confinement allow-count to render alongside a live daemon's assurance
/// (callers only gather it when the daemon answered).
fn classify_save_time(
    mode: watch_save_time::DaemonRoutingMode,
    queried: Option<WorkspaceAssurance>,
    confined: Option<usize>,
) -> SaveTimePosture {
    if mode == watch_save_time::DaemonRoutingMode::Disabled {
        return SaveTimePosture::Hidden;
    }
    match queried {
        // Daemon answered: report assurance + the confinement size it is
        // enforcing.
        Some(assurance) => SaveTimePosture::Assurance(SaveTimeRender {
            assurance,
            confined,
        }),
        // Explicit opt-in with no daemon keeps the preview fallback, because
        // the operator asked to diagnose daemon routing.
        None if mode == watch_save_time::DaemonRoutingMode::ForcedOn => {
            SaveTimePosture::Assurance(SaveTimeRender {
                assurance: watch_save_time::daemon_absent_assurance(),
                confined: None,
            })
        }
        // Default-on routing with no live daemon: state the off posture.
        None => SaveTimePosture::Off,
    }
}

/// The operator confinement allow-list size when the daemon is in `Allowlist`
/// mode, else `None` (open mode, or no readable config). Read directly from the
/// owner-only operator config (DSV-008); status runs as the same uid.
fn confinement_allow_count() -> Option<usize> {
    use anvil_intercept::confinement::{self, AdmissionModeFile};
    match confinement::load() {
        Ok(c) if c.mode() == AdmissionModeFile::Allowlist => Some(c.allow_count()),
        _ => None,
    }
}

/// Render the one-line save-time block for the legible plain surface. Absence is
/// `unavailable{daemon-absent} (daemon not running)` — never a stale `clean`.
fn render_save_time_line(render: &SaveTimeRender) -> String {
    use std::fmt::Write as _;
    let mut line = format!(
        "  Save-time: {}",
        watch_save_time::assurance_label(&render.assurance),
    );
    if render.assurance.state == AssuranceState::Unavailable {
        line.push_str(" (daemon not running)");
    }
    if let Some(n) = render.confined {
        let _ = write!(line, " \u{00b7} confined: {n}");
    }
    line
}

/// Closed-set wire string for an [`AssuranceState`] (matches the proto
/// serialiser; used for the JSON surface).
fn assurance_state_str(state: AssuranceState) -> &'static str {
    match state {
        AssuranceState::Clean => "clean",
        AssuranceState::Stale => "stale",
        AssuranceState::Pending => "pending",
        AssuranceState::Running => "running",
        AssuranceState::Bounded => "bounded",
        AssuranceState::Unavailable => "unavailable",
        // Deser-only forward-compat fallback (ADR-085): never produced locally,
        // surfaced fail-safe (never "clean") if a newer daemon sends it.
        AssuranceState::Unknown => "unknown",
    }
}

/// DSV-049: closed-set wire string for a [`SaveTimeDriverStatusV1`] (matches
/// the proto kebab-case serialiser; used for the `--json` surface). `Unknown`
/// — a driver state from a newer daemon — surfaces honestly as `"unknown"` for
/// machine consumers, distinct from the plain surface which folds it to silent.
fn save_time_driver_str(state: SaveTimeDriverStatusV1) -> &'static str {
    match state {
        SaveTimeDriverStatusV1::Attached => "attached",
        SaveTimeDriverStatusV1::Absent => "absent",
        SaveTimeDriverStatusV1::Failed => "failed",
        SaveTimeDriverStatusV1::Unknown => "unknown",
    }
}

/// Resolve the repo root via `git rev-parse --show-toplevel`. Returns
/// `None` when git is unavailable, the directory is not a worktree,
/// or the command exits non-zero — callers fall back to the CWD.
fn resolve_repo_root() -> Option<std::path::PathBuf> {
    let out = std::process::Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let path = String::from_utf8(out.stdout).ok()?;
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return None;
    }
    Some(std::path::PathBuf::from(trimmed))
}

// ---------------------------------------------------------------------------
// ADTRUST-001: legible plain-mode render
// ---------------------------------------------------------------------------
//
// `anvil status` plain-mode rebuild for the Adoption Trust Surface
// module. The default human output names the protection state from
// the closed-set vocabulary, lists L0–L5 with a one-word status,
// reports the daemon PID + uptime (or "not running"), the last
// witness commit + age (or "none yet"), and ends with a single
// next-action line. Designed to fit a 24-row terminal so the success
// test ("a developer who hasn't read the docs reads it in one pass")
// holds. The renderer accepts a `LegibleSnapshot` so tests can
// synthesise inputs without touching disk or daemon IPC; gathering
// helpers below are best-effort and degrade cleanly when the
// underlying sources are missing.

/// All inputs the legible render needs. Built by
/// [`build_legible_snapshot`] from gathered status data + an
/// activation diagnostic + on-disk pid/witness files.
#[derive(Debug, Clone)]
struct LegibleSnapshot {
    protection: WorktreeClaimState,
    layers: LayerSummary,
    daemon: DaemonSummary,
    /// DSV-007 Task 17 / UJ-005: the save-time posture line. `Hidden` keeps
    /// the pre-DSV legible block unchanged (operator opt-out); `Off` states
    /// the posture explicitly when default routing finds no live daemon.
    save_time: SaveTimePosture,
    witness: WitnessSummary,
    next_action: String,
    /// ACTTUI-019 shared fact lines (mcp/daemon/save-time).
    posture_facts: Vec<String>,
    /// ACTTUI-019 meaning when claim ≠ start protection word.
    posture_meaning: Option<String>,
}

/// One-word layer status for the L0–L5 block.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LayerState {
    /// Layer is not configured or not active in this repo.
    Off,
    /// Layer is configured but not at full strength (e.g. watch-only
    /// fallback when MCP is not attached).
    Partial,
    /// Layer is active and at full strength.
    On,
    /// Layer state cannot be determined from local signals (e.g. L1
    /// mid-edit driver, L5 audit cron — both off-process surfaces).
    Unknown,
}

impl LayerState {
    /// One-word label used in the rendered output.
    const fn label(self) -> &'static str {
        match self {
            LayerState::Off => "off",
            LayerState::Partial => "partial",
            LayerState::On => "on",
            LayerState::Unknown => "unknown",
        }
    }
}

#[derive(Debug, Clone)]
struct LayerSummary {
    l0_mcp: LayerState,
    l1_mid_edit: LayerState,
    l2_save: LayerState,
    l3_commit: LayerState,
    l4_push: LayerState,
    l5_audit: LayerState,
}

#[derive(Debug, Clone)]
enum DaemonSummary {
    /// Live process. `pid` / `uptime` are present when the local PID file
    /// is readable; when only IPC proves liveness they may be absent
    /// (STATUS-1: do not collapse that case to "not running").
    Running {
        pid: Option<u32>,
        uptime: Option<Duration>,
    },
    NotRunning,
}

#[derive(Debug, Clone)]
enum WitnessSummary {
    /// Witness line was found. `age` is `None` when the recorded
    /// timestamp cannot be parsed — the render says "age unknown"
    /// rather than collapsing to `0s ago` and pretending the witness
    /// is fresh.
    Last {
        commit_short: String,
        age: Option<Duration>,
    },
    None,
}

/// Render the legible block. Pure function — no I/O.
fn render_plain_legible(s: &LegibleSnapshot) -> String {
    use std::fmt::Write as _;

    let mut out = String::new();
    out.push_str("ANVIL STATUS\n");
    out.push('\n');
    let _ = writeln!(out, "  Protection: {}", s.protection.as_str());
    // ACTTUI-019: one compact posture line (same fact strings as start Verdict).
    if !s.posture_facts.is_empty() {
        let _ = writeln!(out, "  posture: {}", s.posture_facts.join(" · "));
    }
    // ACTTUI-017/019: when claim vocabulary differs from start, explain with
    // the same subordinate facts both surfaces print.
    if let Some(meaning) = &s.posture_meaning {
        let _ = writeln!(out, "  meaning: {meaning}");
    }
    out.push('\n');
    out.push_str("  Layers:\n");
    let _ = writeln!(out, "    L0 mcp        {}", s.layers.l0_mcp.label());
    let _ = writeln!(out, "    L1 mid-edit   {}", s.layers.l1_mid_edit.label());
    let _ = writeln!(out, "    L2 save       {}", s.layers.l2_save.label());
    let _ = writeln!(out, "    L3 commit     {}", s.layers.l3_commit.label());
    let _ = writeln!(out, "    L4 push       {}", s.layers.l4_push.label());
    let _ = writeln!(out, "    L5 audit      {}", s.layers.l5_audit.label());
    out.push('\n');
    match &s.daemon {
        DaemonSummary::Running {
            pid: Some(pid),
            uptime: Some(uptime),
        } => {
            let _ = writeln!(
                out,
                "  Daemon: pid {pid} \u{00b7} up {}",
                format_duration(*uptime)
            );
        }
        DaemonSummary::Running {
            pid: Some(pid),
            uptime: None,
        } => {
            let _ = writeln!(out, "  Daemon: pid {pid}");
        }
        DaemonSummary::Running { pid: None, .. } => {
            // IPC proved the daemon is up but the PID file is missing or
            // unreadable — still report running so status agrees with
            // `anvil intercept status` (CIB-253 / STATUS-1).
            out.push_str("  Daemon: running\n");
        }
        DaemonSummary::NotRunning => {
            out.push_str("  Daemon: not running\n");
        }
    }
    match &s.save_time {
        SaveTimePosture::Assurance(save_time) => {
            let _ = writeln!(out, "{}", render_save_time_line(save_time));
        }
        // UJ-005: state the off posture and how to change it, instead of
        // hiding the flagship gap.
        SaveTimePosture::Off => {
            out.push_str("  Save-time: off (run `anvil start` to enable)\n");
        }
        SaveTimePosture::Hidden => {}
    }
    match &s.witness {
        WitnessSummary::Last {
            commit_short,
            age: Some(age),
        } => {
            let _ = writeln!(
                out,
                "  Witness: {commit_short} \u{00b7} {} ago",
                format_duration(*age)
            );
        }
        WitnessSummary::Last {
            commit_short,
            age: None,
        } => {
            let _ = writeln!(out, "  Witness: {commit_short} \u{00b7} age unknown");
        }
        WitnessSummary::None => {
            out.push_str("  Witness: none yet\n");
        }
    }
    out.push('\n');
    let _ = writeln!(out, "  Next: {}", s.next_action);
    out
}

/// Short human duration: `45s`, `3m`, `2h`, `5d`. Floors to the
/// largest unit at or below the value. Used for daemon uptime and
/// witness age in the legible block.
fn format_duration(d: Duration) -> String {
    let s = d.as_secs();
    if s < 60 {
        format!("{s}s")
    } else if s < 3600 {
        format!("{}m", s / 60)
    } else if s < 86400 {
        format!("{}h", s / 3600)
    } else {
        format!("{}d", s / 86400)
    }
}

/// Assemble the snapshot from the gathered status data, activation
/// diagnostic, and on-disk pid/witness files. Each helper degrades to
/// the "missing" variant on any failure — the render is single-pass
/// best-effort, never blocks, and never propagates errors past
/// `print_plain` because status is a read-only diagnostic surface.
fn build_legible_snapshot(
    data: &StatusData,
    diag: &activation::ActivationDiagnostic,
    root: &Path,
    save_time: SaveTimePosture,
) -> LegibleSnapshot {
    let layers = derive_layers(data, diag);
    let protection = derive_protection(diag, &layers);
    let daemon = read_daemon_summary(diag);
    let witness = read_witness_summary(root);
    let next_action = next_action_for_diagnostic(protection, &daemon, diag).to_string();
    let shared = activation::SharedPostureFacts::from_diagnostic(diag);
    let posture_facts = shared.fact_lines();
    let posture_meaning = shared.meaning_for_status_claim(protection.as_str());
    LegibleSnapshot {
        protection,
        layers,
        daemon,
        save_time,
        witness,
        next_action,
        posture_facts,
        posture_meaning,
    }
}

/// Map per-layer signals from the activation diagnostic and gathered
/// hook data into the L0–L5 status grid.
fn derive_layers(data: &StatusData, diag: &activation::ActivationDiagnostic) -> LayerSummary {
    let l0_mcp = if diag.mcp_pre_write_wired_or_live() {
        LayerState::On
    } else if diag.mcp.is_empty() {
        LayerState::Off
    } else {
        LayerState::Partial
    };

    // L1 mid-edit lives inside an editor driver process; local
    // signals can't prove it's running, so we report Unknown rather
    // than a false Off.
    let l1_mid_edit = LayerState::Unknown;

    let l2_save = match diag.watch {
        activation::diagnostic::WatchTier::Running => LayerState::On,
        activation::diagnostic::WatchTier::Offered => LayerState::Partial,
        activation::diagnostic::WatchTier::NotRequested => LayerState::Off,
    };

    let l3_commit = hook_layer_state(data, "pre-commit");
    let l4_push = hook_layer_state(data, "pre-push");

    // L5 audit ships as a GitHub Action cron; local CLI cannot
    // observe it. Future ADTRUST-003 can confirm the workflow file
    // exists and flip this to On when present.
    let l5_audit = LayerState::Unknown;

    LayerSummary {
        l0_mcp,
        l1_mid_edit,
        l2_save,
        l3_commit,
        l4_push,
        l5_audit,
    }
}

fn hook_layer_state(data: &StatusData, name: &str) -> LayerState {
    // CIB-251: only file-mode active hooks claim L3/L4 "on". Config-mode
    // install proves presence, not fire — report Partial so the grid cannot
    // contradict a host where hook.<event>.command is ignored.
    let mut any = false;
    let mut file_active = false;
    for hook in &data.hooks {
        if hook.name != name {
            continue;
        }
        any = true;
        // Only non-config (file-mode) active rows claim full "on".
        if hook.active && !is_config_mode_hook_path(&hook.path) {
            file_active = true;
        }
    }
    if file_active {
        LayerState::On
    } else if any {
        // Present but inactive, or config-mode-only (fire not verified).
        LayerState::Partial
    } else {
        LayerState::Off
    }
}

/// Pick a closed-set protection-claim state for the legible header.
///
/// MLP2-048 added the JSON `claim` field to `anvil status --json`
/// using this same derivation, so JSON and plain agree on the
/// worktree state. MLP2-051a hoisted the resolver into
/// [`protection_claim_section`] so `anvil doctor` (and future
/// surfaces) share the same wiring; this thin wrapper preserves
/// `status.rs`'s historical doc comment and call sites.
///
/// Local signals cannot prove the per-surface state that
/// distinguishes several closed-set variants, so the local fallback
/// in [`protection_claim_section::derive_local_worktree_state`]
/// deliberately undershoots rather than over-claim — see that helper
/// for the §14.2 mapping it can prove.
fn derive_protection(
    diag: &activation::ActivationDiagnostic,
    _layers: &LayerSummary,
) -> WorktreeClaimState {
    protection_claim_section::derive_local_worktree_state(diag)
}

/// Best-effort daemon process-liveness summary for the legible block.
///
/// **Primary signal (CIB-253 / STATUS-1):** the activation diagnostic's
/// daemon-attestation IPC probe — the same family of reachability that
/// `anvil intercept status` uses. When the probe reached a live daemon
/// (`Unenforced`, `Warming`, `Enforced`, …), this reports `Running` even
/// if the local PID-file / `kill -0` path fails (Windows has no portable
/// `kill -0`; a multi-line PID parse bug previously collapsed live
/// daemons to "not running" while posture correctly said
/// `daemon: not attesting`).
///
/// **Fallback:** when attestation was not probed (`NotProbed`, typically
/// invalid/absent config), read the PID file written by `anvil start` /
/// `intercept start` and require a live process. Uptime is estimated
/// from the pid file's mtime.
///
/// Caveats:
///
/// - mtime is touched by the writer once at daemon start, so the
///   value normally reflects daemon startup. On systems where the
///   `default_pid_file_path()` directory can be perturbed by cleanup
///   utilities, `tmpfiles.d`, or backup tools, the mtime can drift
///   forward. The `elapsed()` failure path collapses that case to
///   `Duration::ZERO` rather than reporting a negative duration.
/// - `process_is_alive` shells `kill -0` on Unix; see its doc for the
///   EPERM limitation when a daemon is owned by another user. Windows
///   always returns false — rely on IPC reachability when available.
fn read_daemon_summary(diag: &activation::ActivationDiagnostic) -> DaemonSummary {
    match diag.daemon_attestation.process_reachable() {
        // IPC answered: process is up. Decorate with PID details when the
        // local record is readable; never demote to NotRunning solely
        // because the PID probe failed.
        Some(true) => match read_daemon_summary_from_pid(/* require_alive */ false) {
            running @ DaemonSummary::Running { .. } => running,
            DaemonSummary::NotRunning => DaemonSummary::Running {
                pid: None,
                uptime: None,
            },
        },
        // IPC failed: agree with intercept status that the daemon is not
        // answering on the resolved endpoint.
        Some(false) => DaemonSummary::NotRunning,
        // Not probed — fall back to PID file + process liveness.
        None => read_daemon_summary_from_pid(/* require_alive */ true),
    }
}

/// Read process details from the intercept PID file.
///
/// When `require_alive` is true, a recorded PID that does not pass
/// [`process_is_alive`] is treated as not running (stale file). When
/// false, a parseable PID is trusted as decoration for an already-proven
/// live daemon (IPC), even if the OS liveness probe is unavailable.
fn read_daemon_summary_from_pid(require_alive: bool) -> DaemonSummary {
    let Ok(pid_file) = anvil_intercept::default_pid_file_path() else {
        return DaemonSummary::NotRunning;
    };
    let Ok(contents) = std::fs::read_to_string(&pid_file) else {
        return DaemonSummary::NotRunning;
    };
    // The intercept PID record is `<pid>\n[start_time=<epoch>\n]` — only the
    // first line is the PID. Parsing the whole file fails once start_time lands
    // (#3216); `anvil intercept status` already reads line 1 via
    // `daemon_pid_for_display`.
    let Some(pid) = parse_daemon_pid_from_record(&contents) else {
        return DaemonSummary::NotRunning;
    };
    if require_alive && !process_is_alive(pid) {
        return DaemonSummary::NotRunning;
    }
    // `mtime.elapsed()` errors when mtime is in the future relative
    // to the system clock — collapsing to `Duration::ZERO` is the
    // honest answer for that case (we cannot date a daemon whose
    // recorded start is impossible).
    let uptime = std::fs::metadata(&pid_file)
        .ok()
        .and_then(|m| m.modified().ok())
        .and_then(|mtime| mtime.elapsed().ok())
        .unwrap_or_default();
    DaemonSummary::Running {
        pid: Some(pid),
        uptime: Some(uptime),
    }
}

/// Whether `anvil status` should launch the Ratatui surface. Stricter than
/// stdout-only TTY detection: the TUI blocks on keyboard input, so a session
/// with redirected stdin (piped output capture, `script` harnesses) must fall
/// back to plain output (#3222).
fn status_prefers_tui(global: &GlobalArgs) -> bool {
    !global.no_tui
        && !crate::is_non_interactive_env()
        && std::io::stdin().is_terminal()
        && std::io::stdout().is_terminal()
}

fn warn_if_status_tui_unavailable(global: &GlobalArgs) {
    if global.no_tui || !std::io::stdout().is_terminal() {
        return;
    }
    if !std::io::stdin().is_terminal() {
        eprintln!(
            "anvil: stdin is not a terminal — showing plain status (pass --no-tui to silence this hint)"
        );
    } else if crate::is_non_interactive_env() {
        eprintln!(
            "anvil: non-interactive environment — showing plain status (pass --no-tui to silence this hint)"
        );
    }
}

fn parse_daemon_pid_from_record(record: &str) -> Option<u32> {
    record
        .lines()
        .next()
        .and_then(|line| line.trim().parse::<u32>().ok())
}

#[cfg(unix)]
fn process_is_alive(pid: u32) -> bool {
    // Use `kill -0 <pid>` to probe — POSIX signal 0 does not deliver
    // anything; exit 0 means the caller has permission to signal,
    // which proves the process exists. Avoids pulling `libc` into
    // anvil-cli's direct dependency surface for a single probe.
    //
    // EPERM caveat: a non-zero exit may mean either "no such
    // process" (ESRCH) or "exists but you cannot signal it" (EPERM —
    // e.g. daemon owned by a different user). This shell-out cannot
    // distinguish those without parsing `kill`'s stderr, which is
    // not portable. The single-user flow (anvil daemon launched by
    // the same user running `anvil status`) sees zero EPERM cases,
    // so the legible surface reports `not running` on EPERM. The
    // richer cross-platform liveness check that distinguishes the
    // two lands with ADTRUST-004's hook + kernel wire-up.
    std::process::Command::new("kill")
        .arg("-0")
        .arg(pid.to_string())
        .stderr(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
}

#[cfg(not(unix))]
fn process_is_alive(_pid: u32) -> bool {
    // No portable POSIX `kill` on Windows. Fail closed — a stale pid
    // file with a valid integer would otherwise produce a phantom
    // daemon line. `ADTRUST-004` (daemon-down auto-recovery) is the
    // owner of the cross-platform liveness check that will return
    // a real answer here.
    false
}

/// Best-effort witness summary. Tails the active witness chain and
/// pulls `commit` + `ts` from the last NDJSON line. Returns `None`
/// when the chain file is absent, empty, or unreadable. Designed not
/// to hold the chain open or compete with the witness writer.
fn read_witness_summary(root: &Path) -> WitnessSummary {
    let active = root.join("anvil/witness/active.ndjson");
    let Ok(contents) = std::fs::read_to_string(&active) else {
        return WitnessSummary::None;
    };
    let Some(last) = contents.lines().rev().find(|l| !l.is_empty()) else {
        return WitnessSummary::None;
    };
    let Ok(value) = serde_json::from_str::<serde_json::Value>(last) else {
        return WitnessSummary::None;
    };
    let commit = value
        .get("commit")
        .and_then(serde_json::Value::as_str)
        .or_else(|| value.get("commit_sha").and_then(serde_json::Value::as_str));
    let ts = value.get("ts").and_then(serde_json::Value::as_str);
    let Some(commit) = commit else {
        return WitnessSummary::None;
    };
    let commit_short: String = commit.chars().take(7).collect();
    let age = ts.and_then(parse_iso_timestamp_seconds).and_then(|t| {
        std::time::SystemTime::UNIX_EPOCH
            .checked_add(Duration::from_secs(t))
            .and_then(|t| t.elapsed().ok())
    });
    WitnessSummary::Last { commit_short, age }
}

/// Parse `2026-05-07T12:34:56Z` (witness `ts` field) to seconds
/// since the epoch. Accepts only `Z`-terminated RFC 3339 strings —
/// matches what the witness writer emits today and refuses numeric
/// offsets (`+00:00`, `-08:00`) up front so the time-field byte
/// offsets are unambiguous. A non-`Z` suffix (or a missing one)
/// returns `None`; the caller renders "age unknown" rather than
/// silently misinterpreting the local time as UTC.
fn parse_iso_timestamp_seconds(s: &str) -> Option<u64> {
    let bytes = s.as_bytes();
    if bytes.len() < 20 || bytes[4] != b'-' || bytes[7] != b'-' || bytes[10] != b'T' {
        return None;
    }
    // Enforce `Z`-terminator at byte 19. The witness writer emits
    // UTC only, so anything else (a `+`/`-` offset, fractional
    // seconds without a `Z`, a truncated string) is rejected. This
    // closes the silent-misinterpretation hole that would otherwise
    // happen for `2026-05-07T12:34:56+00:00`: the byte offsets up
    // to 19 are identical to the `Z` form, so without this guard
    // the parser would happily read the local-time value and treat
    // a `-08:00` offset as UTC.
    if bytes[19] != b'Z' {
        return None;
    }
    let y: i64 = s.get(0..4)?.parse().ok()?;
    let mo: i64 = s.get(5..7)?.parse().ok()?;
    let d: i64 = s.get(8..10)?.parse().ok()?;
    let hh: i64 = s.get(11..13)?.parse().ok()?;
    let mm: i64 = s.get(14..16)?.parse().ok()?;
    let ss: i64 = s.get(17..19)?.parse().ok()?;
    // Days-from-civil (Howard Hinnant, public domain).
    let y_adj = if mo <= 2 { y - 1 } else { y };
    let era = y_adj.div_euclid(400);
    let yoe = y_adj - era * 400;
    let doy = (153 * (if mo > 2 { mo - 3 } else { mo + 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days_since_epoch = era * 146_097 + doe - 719_468;
    let secs = days_since_epoch * 86400 + hh * 3600 + mm * 60 + ss;
    u64::try_from(secs).ok()
}

/// ACTTUI-017: one-line meaning when status.protection alone would hide a live
/// L0 (or other layer) story that `anvil start` may report differently.
fn next_action_for_diagnostic(
    state: WorktreeClaimState,
    daemon: &DaemonSummary,
    diag: &activation::ActivationDiagnostic,
) -> &'static str {
    use activation::daemon_evidence::DaemonAttestation;

    if state != WorktreeClaimState::Warming {
        return next_action_for(state, daemon);
    }

    match diag.daemon_attestation {
        DaemonAttestation::NotProbed if diag.mcp_pre_write_live() => {
            "MCP is live. Run `anvil start --verify` to refresh the protection state."
        }
        DaemonAttestation::NotProbed => {
            "Restart your editor or agent, then run `anvil start --verify`."
        }
        DaemonAttestation::Unreachable => {
            "No intercept daemon is answering. Run `anvil start` in a terminal, or `anvil intercept start --foreground` headlessly, then run `anvil start --verify`."
        }
        DaemonAttestation::Unenforced | DaemonAttestation::NoParticipatingSurface => {
            "Run `anvil intercept status`, then run `anvil start --verify` after the editor or agent makes an MCP request."
        }
        DaemonAttestation::StaleHeartbeat => {
            "Restart the intercept daemon with `anvil intercept start --foreground`, then run `anvil start --verify`."
        }
        DaemonAttestation::AllSurfacesQuarantined => {
            "Restart the fenced intercept daemon with `anvil intercept start --foreground`, then run `anvil start --verify`."
        }
        DaemonAttestation::Warming => {
            "Wait a few seconds for the intercept daemon to settle, then run `anvil start --verify`."
        }
        DaemonAttestation::Enforced | DaemonAttestation::Promoted => {
            "Protection evidence is live. Run `anvil start --verify` to refresh this status."
        }
    }
}

/// Single next-action line keyed off the protection state. Kept
/// terse so it stays one row.
const fn next_action_for(state: WorktreeClaimState, daemon: &DaemonSummary) -> &'static str {
    match state {
        WorktreeClaimState::Full | WorktreeClaimState::PreWriteDaemon => {
            "Protection is live. Run `anvil doctor` if anything looks off."
        }
        WorktreeClaimState::PreWriteEmbedded => {
            "Run `anvil start` so MCP pre-write attaches to a daemon-backed surface."
        }
        WorktreeClaimState::SaveTimeOnly => {
            "Save-time only. Attach a driver or MCP shim for pre-write coverage."
        }
        WorktreeClaimState::Warming => "Restart your editor or agent so the MCP server attaches.",
        WorktreeClaimState::DegradedProtection => {
            "Run `anvil doctor` — at least one surface is degraded."
        }
        WorktreeClaimState::MultiDaemonDetected => {
            "Multiple daemons detected. Run `anvil doctor` to identify the duplicate."
        }
        WorktreeClaimState::PathUncertain => {
            "Path canonicalisation drift. Run `anvil doctor --fix` to reconcile."
        }
        WorktreeClaimState::CrossBoundaryMixed => {
            "Surfaces span an OS-locality boundary. Re-run `anvil start` inside the intended host."
        }
        WorktreeClaimState::Unprotected => match daemon {
            DaemonSummary::Running { .. } => {
                "Run `anvil init` to attach this repo to the running daemon."
            }
            DaemonSummary::NotRunning => "Run `anvil start` to turn on protection.",
        },
    }
}

// ---------------------------------------------------------------------------
// ADTRUST-002: degraded-state banner primitive
// ---------------------------------------------------------------------------

// DegradedBanner: rate-limited protection-state banner for watch TUI / hooks.
// Status itself is single-shot and already names protection state; allow-dead-code
// until those surfaces wire it. Uses WorktreeClaimState::as_str() vocabulary.

/// Banner rate limit window in seconds. Spec §ADTRUST-002 calls for
/// ≤1 banner per 60s per terminal session. Held as a `u64` so the
/// `Duration::from_secs` literal goes through a named constant and
/// stays clippy-clean on stable Rust.
#[allow(dead_code, reason = "wired by hook + watch in ADTRUST-002 follow-ups")]
const DEGRADED_BANNER_WINDOW_SECS: u64 = 60;

/// Tracks whether a degraded-state banner is due for emission given
/// the current claim state and time. `Default` is the empty state
/// (no banner has been emitted yet).
///
/// Public crate-level so the in-tree hook bridge and watch tui can
/// hold one per terminal session in their respective entry points.
/// The status surface itself does not consume `DegradedBanner` — it
/// is single-shot and already names the protection state in the
/// legible block — so the type intentionally has no other call sites
/// in this PR. The follow-up wiring lands in:
/// - `crates/anvil-cli/src/commands/hook.rs` (pre-commit / pre-push
///   exit paths).
/// - `crates/anvil-tui/src/surfaces/watch/render.rs` (TUI watch
///   surface; once-per-tick poll).
#[allow(dead_code, reason = "wired by hook + watch in ADTRUST-002 follow-ups")]
#[derive(Debug, Default)]
pub(crate) struct DegradedBanner {
    /// Last emit instant. `None` means "never emitted in this
    /// session"; the next degraded sample always fires.
    last_emit: Option<std::time::Instant>,
}

impl DegradedBanner {
    /// Decide whether to emit a banner. Returns `Some(line)` when the
    /// claim is degraded AND the rate-limit window has elapsed since
    /// the last emit; otherwise `None`. Callers print the line as-is.
    #[allow(dead_code, reason = "wired by hook + watch in ADTRUST-002 follow-ups")]
    pub(crate) fn poll(
        &mut self,
        claim: WorktreeClaimState,
        now: std::time::Instant,
    ) -> Option<String> {
        if !is_degraded_claim(claim) {
            return None;
        }
        if let Some(last) = self.last_emit
            && now.saturating_duration_since(last)
                < Duration::from_secs(DEGRADED_BANNER_WINDOW_SECS)
        {
            return None;
        }
        self.last_emit = Some(now);
        Some(format_degraded_banner(claim))
    }
}

/// Spec §14.2 names ten worktree states. Six of them are "degraded"
/// in the ADTRUST-002 sense — the protection claim is below `full`
/// AND naming it actionable in the surrounding surface.
#[allow(dead_code, reason = "wired by hook + watch in ADTRUST-002 follow-ups")]
pub(crate) const fn is_degraded_claim(claim: WorktreeClaimState) -> bool {
    matches!(
        claim,
        WorktreeClaimState::DegradedProtection
            | WorktreeClaimState::CrossBoundaryMixed
            | WorktreeClaimState::MultiDaemonDetected
            | WorktreeClaimState::PathUncertain
            | WorktreeClaimState::Unprotected
            | WorktreeClaimState::Warming
    )
}

#[allow(dead_code, reason = "wired by hook + watch in ADTRUST-002 follow-ups")]
pub(crate) fn format_degraded_banner(claim: WorktreeClaimState) -> String {
    format!(
        "anvil: {} — run `anvil doctor` to investigate",
        claim.as_str()
    )
}

// ---------------------------------------------------------------------------
// Output: JSON
// ---------------------------------------------------------------------------

/// Schema version pinned for `anvil status --json` (ADTRUST-005).
/// Editor extensions and CI consumers parse against this constant
/// and refuse any other value. Patch releases keep the version
/// stable; minor releases bump it explicitly with a JSON Schema
/// migration. The companion schema lives at
/// `schemas/anvil-status.v1.json`.
pub const STATUS_SCHEMA_VERSION: &str = "anvil.status.v1";

#[derive(Serialize)]
struct StatusOutput {
    schema_version: &'static str,
    activation: serde_json::Value,
    hooks: Vec<HookOutput>,
    profile: ProfileOutput,
    recent_runs: Vec<RunOutput>,
    /// MLP2-048: nested `ProtectionClaim` wire shape per spec §14.
    /// Carries its own `schema_version` (`anvil.protection-claim.v1`)
    /// so consumers parse the claim against
    /// `anvil_kernel_types::protection_claim::ProtectionClaim`
    /// independently of `anvil.status.v1`. Surfaces are empty in v1
    /// because local CLI signals cannot enumerate per-surface state;
    /// IPC `query_status` integration is a separate follow-up that
    /// will populate `surfaces` via
    /// `anvil_intercept::status::build_protection_claim`.
    claim: ProtectionClaim,

    /// DISTRIB-006 (ADR-060): the resolved install root, present only when
    /// `ANVIL_HOME` re-roots install state, so an operator can see which install
    /// they are talking to. Omitted entirely under the platform default, keeping
    /// the v1 output byte-for-byte unchanged for the 99% who never set it
    /// (`additionalProperties: true` makes this additive without a schema bump).
    #[serde(skip_serializing_if = "Option::is_none")]
    install_root: Option<String>,

    /// DISTRIB-006 (ADR-060): whether durable per-project writes (baseline /
    /// witness / cutoff) are gated for this session. Present only under a
    /// non-default `ANVIL_HOME`; `true` = read-only / dry-run posture (no
    /// `--touch-project-state`), `false` = the operator opted in.
    #[serde(skip_serializing_if = "Option::is_none")]
    project_writes_gated: Option<bool>,

    /// DSV-007/021: the save-time assurance + confinement snapshot. Present when
    /// routing is forced on, or when default-on routing finds a live daemon;
    /// omitted for non-daemon users so v1 output stays unchanged
    /// (`additionalProperties: true`).
    #[serde(skip_serializing_if = "Option::is_none")]
    save_time: Option<SaveTimeOutput>,

    /// DSV-049: the current worktree's save-time driver state from the daemon
    /// snapshot (`attached` / `absent` / `failed` / `unknown`). Present only
    /// when a daemon answered AND the worktree is registered (so consumers can
    /// tell "no daemon evidence" apart from an explicit `absent`); omitted
    /// otherwise so v1 output stays unchanged (`additionalProperties: true`).
    #[serde(skip_serializing_if = "Option::is_none")]
    save_time_driver: Option<&'static str>,

    /// MCPLH-005: MCP inventory + split readiness claims. Flattened so each
    /// field is omitted independently when absent (same posture as
    /// `save_time`).
    #[serde(flatten)]
    mcp: status_mcp::StatusMcpJson,
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
    daemon_snapshot: Option<&DaemonStatusV1>,
    worktree: &Path,
    save_time: Option<&SaveTimeRender>,
    mcp: Option<status_mcp::StatusMcpJson>,
) -> anyhow::Result<()> {
    let claim = protection_claim_section::resolve_protection_claim(
        activation_diag,
        daemon_snapshot,
        worktree,
    );

    // DISTRIB-006: one environment snapshot drives both install-root fields, so
    // they can never disagree (e.g. install_root present but gated-flag absent).
    // The gated flag derives from the SAME `root` value via `_from`, not a second
    // `install_root()` env read.
    let install_root_section = {
        let root = crate::install_root::install_root();
        let gated = root.is_overridden().then(|| {
            crate::install_root::project_writes_gated_from(
                &root,
                std::env::var_os(crate::install_root::TOUCH_PROJECT_STATE_ENV).as_deref(),
            )
        });
        (root.prefix().map(|p| p.display().to_string()), gated)
    };

    let output = StatusOutput {
        schema_version: STATUS_SCHEMA_VERSION,
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
        claim,
        // DISTRIB-006 (ADR-060): surface the install-root override only when set,
        // so default output is byte-for-byte unchanged. Resolve the root once so
        // both fields derive from a single environment snapshot.
        install_root: install_root_section.0,
        project_writes_gated: install_root_section.1,
        save_time: save_time.map(|st| SaveTimeOutput {
            state: assurance_state_str(st.assurance.state),
            reason: st.assurance.reason.map(watch_save_time::stale_reason_str),
            confined: st.confined,
            last_full_scan: st.assurance.last_full_scan.clone(),
        }),
        // DSV-049: the cwd worktree's driver state, mirroring the cwd-centric
        // `claim` / `save_time` fields. `None` (omitted) when no daemon
        // answered or the worktree is not registered — no over-claim.
        save_time_driver: daemon_snapshot.and_then(|snap| {
            snap.worktrees
                .iter()
                .find(|w| w.worktree == worktree)
                .map(|w| save_time_driver_str(w.save_time_driver))
        }),
        mcp: mcp.unwrap_or_default(),
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
    fn parse_daemon_pid_from_record_reads_first_line_only() {
        let pid = std::process::id();
        let record = format!("{pid}\nstart_time=9023295\n");
        assert_eq!(parse_daemon_pid_from_record(&record), Some(pid));
        assert_eq!(parse_daemon_pid_from_record(&format!("{pid}\n")), Some(pid));
        assert_eq!(
            parse_daemon_pid_from_record("not-a-pid\nstart_time=1\n"),
            None
        );
    }

    /// CIB-253 / STATUS-1: when the activation IPC probe reached a live
    /// daemon that is not attesting this worktree, the legible `Daemon:`
    /// line must not say "not running". Posture already names
    /// `daemon: not attesting`; process liveness must agree with
    /// `anvil intercept status`.
    #[test]
    fn daemon_line_running_when_ipc_reachable_but_not_attesting() {
        use activation::daemon_evidence::DaemonAttestation;
        use activation::diagnostic::{ConfigStatus, WatchTier};

        let diag = activation::ActivationDiagnostic {
            config: ConfigStatus::Valid,
            mcp: std::collections::BTreeMap::new(),
            watch: WatchTier::NotRequested,
            baseline_present: false,
            baseline_summary: None,
            last_error: None,
            all_languages_unsupported: false,
            language_profile: activation::language_profile::RepoLanguageProfile::default(),
            // Live daemon, worktree not registered — the STATUS-1 case.
            daemon_attestation: DaemonAttestation::Unenforced,
            save_time_driver_attached: false,
        };

        assert_eq!(
            diag.daemon_attestation.process_reachable(),
            Some(true),
            "Unenforced must mean process-reachable over IPC"
        );
        // Exercise the production path (not only a synthesised summary). When
        // the PID file is absent, Running carries no pid; when present, pid is
        // decoration only — either way it must not collapse to NotRunning.
        let summary = read_daemon_summary(&diag);
        assert!(
            matches!(summary, DaemonSummary::Running { .. }),
            "STATUS-1: Unenforced must report Running, got {summary:?}"
        );
        let mut snap = legible_test_snapshot(WorktreeClaimState::Unprotected);
        // Pin the IPC-up / PID-empty surface so we still assert
        // "Daemon: running" (not only "Daemon: pid …").
        snap.daemon = DaemonSummary::Running {
            pid: None,
            uptime: None,
        };
        snap.posture_facts = activation::SharedPostureFacts::from_diagnostic(&diag).fact_lines();
        let rendered = render_plain_legible(&snap);
        assert!(
            !rendered.contains("Daemon: not running"),
            "STATUS-1: must not contradict live daemon:\n{rendered}"
        );
        assert!(
            rendered.contains("Daemon: running"),
            "STATUS-1: expect process-up wording:\n{rendered}"
        );
        assert!(
            rendered.contains("daemon: not attesting"),
            "posture must still name not-attesting:\n{rendered}"
        );
    }

    #[test]
    fn daemon_line_not_running_when_ipc_unreachable() {
        use activation::daemon_evidence::DaemonAttestation;
        use activation::diagnostic::{ConfigStatus, WatchTier};

        let diag = activation::ActivationDiagnostic {
            config: ConfigStatus::Valid,
            mcp: std::collections::BTreeMap::new(),
            watch: WatchTier::NotRequested,
            baseline_present: false,
            baseline_summary: None,
            last_error: None,
            all_languages_unsupported: false,
            language_profile: activation::language_profile::RepoLanguageProfile::default(),
            daemon_attestation: DaemonAttestation::Unreachable,
            save_time_driver_attached: false,
        };
        assert_eq!(diag.daemon_attestation.process_reachable(), Some(false));
        let summary = read_daemon_summary(&diag);
        // Unreachable must not flip to Running solely because a stale PID
        // file happens to exist for a different process — IPC is the
        // agreement surface with intercept status.
        assert!(
            matches!(summary, DaemonSummary::NotRunning),
            "Unreachable must report NotRunning, got {summary:?}"
        );
    }

    #[test]
    fn render_running_with_pid_keeps_uptime_line() {
        let mut snap = legible_test_snapshot(WorktreeClaimState::Unprotected);
        snap.daemon = DaemonSummary::Running {
            pid: Some(4242),
            uptime: Some(Duration::from_secs(90)),
        };
        let rendered = render_plain_legible(&snap);
        assert!(
            rendered.contains("Daemon: pid 4242"),
            "expected pid decoration:\n{rendered}"
        );
        assert!(
            rendered.contains("up 1m"),
            "expected uptime decoration:\n{rendered}"
        );
        assert!(
            !rendered.contains("Daemon: not running"),
            "must not dual-print not-running:\n{rendered}"
        );
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
        // legacy-fallback coverage (.anvilrc deliberately) — keeps
        // `gather_profile`'s legacy fallback branch exercised.
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
        // legacy-fallback coverage (.anvilrc deliberately) — invalid
        // legacy content must surface "(invalid config)".
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

    // --- gather_profile format sniffing ---

    /// UCFG-003: init now writes canonical `snake_case` YAML; the sniff must
    /// recognise it (and the legacy `camelCase` TOML that `start --format`
    /// used to write) instead of reporting "(invalid config)".
    #[test]
    fn gather_profile_recognises_snake_case_yaml() {
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::write(
            tmp.path().join(".anvil.yaml"),
            "schema_version: \"1.0.0\"\nplanning_dir: \"plans\"\nformat: \"yaml\"\nchecks:\n  - \"secret-detection\"\n",
        )
        .unwrap();
        let profile = gather_profile(tmp.path());
        assert_eq!(profile.name, "default");
        assert_eq!(profile.checks, vec!["secret-detection"]);
    }

    #[test]
    fn gather_profile_recognises_legacy_camel_toml() {
        // legacy-fallback coverage (.anvilrc deliberately) — camelCase TOML
        // is the shape old `start --format` wrote into the legacy file.
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::write(
            tmp.path().join(".anvilrc"),
            "schemaVersion = \"1.0.0\"\nchecks = [\"secret-detection\"]\n",
        )
        .unwrap();
        let profile = gather_profile(tmp.path());
        assert_eq!(profile.name, "default");
        assert_eq!(profile.checks, vec!["secret-detection"]);
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
        assert!(
            pre_commit_config[0].path.contains("(fire not verified)"),
            "config-mode rows must not claim verified fire, got: {}",
            pre_commit_config[0].path,
        );

        cleanup(&dir);
    }

    fn bare_status_data(hooks: Vec<HookStatus>) -> StatusData {
        StatusData {
            hooks,
            profile: ProfileInfo {
                name: "(no config)".to_string(),
                checks: Vec::new(),
                path: ".anvilrc".to_string(),
            },
            recent_runs: Vec::new(),
            update_hint: None,
            insights_hint: None,
            whats_new_hint: None,
        }
    }

    /// CIB-251: config-mode-only must not flip L3/L4 to On.
    #[test]
    fn hook_layer_state_config_mode_only_is_partial() {
        let data = bare_status_data(vec![HookStatus {
            name: "pre-commit".to_string(),
            active: true,
            path: "git config hook.pre-commit.command (anvil-managed) (fire not verified)"
                .to_string(),
        }]);
        assert_eq!(hook_layer_state(&data, "pre-commit"), LayerState::Partial);
        assert_eq!(hook_layer_state(&data, "pre-push"), LayerState::Off);
    }

    /// CIB-251: file-mode active still claims On even when config-mode also present.
    #[test]
    fn hook_layer_state_file_mode_active_is_on() {
        let data = bare_status_data(vec![
            HookStatus {
                name: "pre-commit".to_string(),
                active: true,
                path: ".husky/pre-commit".to_string(),
            },
            HookStatus {
                name: "pre-commit".to_string(),
                active: true,
                path: "git config hook.pre-commit.command (anvil-managed) (fire not verified)"
                    .to_string(),
            },
        ]);
        assert_eq!(hook_layer_state(&data, "pre-commit"), LayerState::On);
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

    // -----------------------------------------------------------------
    // ADTRUST-001 legible plain-mode renderer
    // -----------------------------------------------------------------

    // Stable-Rust workaround for the `clippy::duration_suboptimal_units`
    // lint that would prefer `Duration::from_hours` / `from_days`; both
    // are still unstable as of Rust 1.95. Routing through named
    // constants keeps the literals away from the lint pattern.
    const HOUR_SECS: u64 = 3_600;
    const DAY_SECS: u64 = 86_400;
    const YEAR_SECS: u64 = 365 * DAY_SECS;

    // --- DSV-007 Task 17: save-time assurance + confinement surface ---

    #[test]
    fn status_renders_unavailable_when_daemon_absent() {
        let render = SaveTimeRender {
            assurance: watch_save_time::daemon_absent_assurance(),
            confined: None,
        };
        let line = render_save_time_line(&render);
        assert!(
            line.contains("unavailable"),
            "an absent daemon must render unavailable, got: {line}",
        );
        assert!(
            line.contains("daemon not running"),
            "absence must say the daemon is not running, got: {line}",
        );
        assert!(
            !line.contains("clean"),
            "absence must never render a stale cached clean, got: {line}",
        );
    }

    #[test]
    fn status_shows_confined_count() {
        let render = SaveTimeRender {
            assurance: WorkspaceAssurance {
                state: AssuranceState::Clean,
                reason: None,
                generation: 1,
                last_full_scan: None,
                scan_coverage: None,
            },
            confined: Some(3),
        };
        let line = render_save_time_line(&render);
        assert!(
            line.contains("confined: 3"),
            "allowlist mode must render the confined count, got: {line}",
        );
    }

    #[test]
    fn status_shows_stale_reason() {
        let render = SaveTimeRender {
            assurance: WorkspaceAssurance {
                state: AssuranceState::Stale,
                reason: Some(
                    anvil_intercept_proto::protocol::StaleReason::CrossFileResolutionNeeded,
                ),
                generation: 2,
                last_full_scan: None,
                scan_coverage: None,
            },
            confined: None,
        };
        let line = render_save_time_line(&render);
        assert!(
            line.contains("stale"),
            "must render the stale state, got: {line}"
        );
        assert!(
            line.contains("cross-file-resolution-needed"),
            "stale must name its reason, got: {line}",
        );
    }

    // --- UJ-005: status always states the save-time posture ---

    #[test]
    fn default_routing_with_no_daemon_classifies_off() {
        let posture = classify_save_time(
            watch_save_time::DaemonRoutingMode::DefaultOnWhenLive,
            None,
            None,
        );
        assert!(
            matches!(posture, SaveTimePosture::Off),
            "default routing with no live daemon must state the off posture, got: {posture:?}",
        );
    }

    #[test]
    fn opt_out_classifies_hidden() {
        // Opt-out keeps the pre-DSV surface regardless of daemon presence.
        for queried in [
            None,
            Some(WorkspaceAssurance {
                state: AssuranceState::Clean,
                reason: None,
                generation: 1,
                last_full_scan: None,
                scan_coverage: None,
            }),
        ] {
            let posture =
                classify_save_time(watch_save_time::DaemonRoutingMode::Disabled, queried, None);
            assert!(
                matches!(posture, SaveTimePosture::Hidden),
                "operator opt-out must keep the save-time line hidden, got: {posture:?}",
            );
        }
    }

    #[test]
    fn forced_on_with_no_daemon_classifies_unavailable_assurance() {
        let posture = classify_save_time(watch_save_time::DaemonRoutingMode::ForcedOn, None, None);
        match posture {
            SaveTimePosture::Assurance(render) => {
                assert_eq!(
                    render.assurance.state,
                    AssuranceState::Unavailable,
                    "forced-on absence must keep its unavailable rendering",
                );
            }
            other => panic!("forced-on absence must render an assurance, got: {other:?}"),
        }
    }

    #[test]
    fn live_daemon_classifies_assurance() {
        // A live daemon renders its assurance under both default-on and
        // forced-on routing — the full live column of the posture matrix.
        for mode in [
            watch_save_time::DaemonRoutingMode::DefaultOnWhenLive,
            watch_save_time::DaemonRoutingMode::ForcedOn,
        ] {
            let assurance = WorkspaceAssurance {
                state: AssuranceState::Clean,
                reason: None,
                generation: 1,
                last_full_scan: None,
                scan_coverage: None,
            };
            let posture = classify_save_time(mode, Some(assurance), Some(3));
            match posture {
                SaveTimePosture::Assurance(render) => {
                    assert_eq!(render.assurance.state, AssuranceState::Clean);
                    assert_eq!(
                        render.confined,
                        Some(3),
                        "a live daemon keeps its confinement rendering",
                    );
                }
                other => panic!("a live daemon must render its assurance, got: {other:?}"),
            }
        }
    }

    #[test]
    fn off_posture_renders_line_naming_anvil_start() {
        let mut snap = legible_test_snapshot(WorktreeClaimState::Unprotected);
        snap.save_time = SaveTimePosture::Off;
        let rendered = render_plain_legible(&snap);
        let line = rendered
            .lines()
            .find(|l| l.contains("Save-time:"))
            .unwrap_or_else(|| panic!("off posture must render a save-time line:\n{rendered}"));
        assert!(
            line.contains("off"),
            "off posture must say save-time is off, got: {line}",
        );
        assert!(
            line.contains("anvil start"),
            "off-state line must name `anvil start`, got: {line}",
        );
    }

    #[test]
    fn hidden_posture_renders_no_save_time_line() {
        let mut snap = legible_test_snapshot(WorktreeClaimState::Unprotected);
        snap.save_time = SaveTimePosture::Hidden;
        let rendered = render_plain_legible(&snap);
        assert!(
            !rendered.contains("Save-time:"),
            "opt-out must keep the pre-DSV surface (no save-time line):\n{rendered}",
        );
    }

    fn legible_test_snapshot(state: WorktreeClaimState) -> LegibleSnapshot {
        LegibleSnapshot {
            protection: state,
            layers: LayerSummary {
                l0_mcp: LayerState::Off,
                l1_mid_edit: LayerState::Unknown,
                l2_save: LayerState::Off,
                l3_commit: LayerState::Off,
                l4_push: LayerState::Off,
                l5_audit: LayerState::Unknown,
            },
            daemon: DaemonSummary::NotRunning,
            save_time: SaveTimePosture::Hidden,
            witness: WitnessSummary::None,
            next_action: "run `anvil start`".to_string(),
            posture_facts: vec![
                "mcp: not live".into(),
                "daemon: not attesting".into(),
                "save-time: not attached".into(),
            ],
            posture_meaning: None,
        }
    }

    fn test_activation_diagnostic() -> activation::ActivationDiagnostic {
        use activation::diagnostic::{ConfigStatus, WatchTier};

        activation::ActivationDiagnostic {
            config: ConfigStatus::Valid,
            mcp: std::collections::BTreeMap::new(),
            watch: WatchTier::NotRequested,
            baseline_present: false,
            baseline_summary: None,
            last_error: None,
            all_languages_unsupported: false,
            language_profile: activation::language_profile::RepoLanguageProfile::default(),
            daemon_attestation: activation::daemon_evidence::DaemonAttestation::NotProbed,
            save_time_driver_attached: false,
        }
    }

    /// ADTRUST-001 validation: the legible plain-mode render must
    /// fit a 24-row terminal in every closed-set state so a developer
    /// reading `anvil status` does not scroll past the protection
    /// claim. Every state is rendered twice: once with the empty
    /// snapshot (cheap fallback path) and once with a worst-case
    /// snapshot (`DaemonSummary::Running` + `WitnessSummary::Last` +
    /// the longest `next_action` string this surface emits). The
    /// budget is checked against `render_plain_legible` **plus** the
    /// worst-case rule-mode summary that `print_plain` appends (the
    /// invalid-config branch produces three additional lines), so a
    /// future regression that pushes the legible block past 21 lines
    /// fails this test before reaching the 24-row terminal budget.
    #[test]
    fn plain_mode_fits_24_rows() {
        // Worst-case rule-mode summary the surface can emit (the
        // invalid-config branch). Use a tempdir whose `.anvil.yaml` is
        // intentionally malformed so the appended block reaches the
        // three-line invalid-config render.
        let dir = make_temp_dir();
        std::fs::write(dir.join(".anvil.yaml"), "enforcement: [").unwrap();
        let rule_mode_lines = crate::config_summary::render_rule_mode_summary(&dir)
            .lines()
            .count();
        cleanup(&dir);

        let worst_action = longest_next_action();
        for &state in WorktreeClaimState::all() {
            // UJ-005: the off posture adds a line over the hidden baseline,
            // so the budget loop renders it explicitly alongside the
            // worst-case assurance line.
            let mut off_snapshot = legible_test_snapshot(state);
            off_snapshot.save_time = SaveTimePosture::Off;
            for snap in [
                legible_test_snapshot(state),
                off_snapshot,
                worst_case_snapshot(state, &worst_action),
            ] {
                let rendered = render_plain_legible(&snap);
                let legible_lines = rendered.lines().count();
                let total = legible_lines + rule_mode_lines;
                assert!(
                    total <= 24,
                    "render for {state:?} produced {legible_lines} legible + {rule_mode_lines} rule-mode lines = {total} (budget 24):\n{rendered}",
                );
            }
        }
    }

    fn worst_case_snapshot(state: WorktreeClaimState, next_action: &str) -> LegibleSnapshot {
        LegibleSnapshot {
            protection: state,
            layers: LayerSummary {
                l0_mcp: LayerState::Partial,
                l1_mid_edit: LayerState::Partial,
                l2_save: LayerState::Partial,
                l3_commit: LayerState::Partial,
                l4_push: LayerState::Partial,
                l5_audit: LayerState::Partial,
            },
            daemon: DaemonSummary::Running {
                pid: Some(4_194_303),
                uptime: Some(Duration::from_secs(YEAR_SECS)),
            },
            // Worst-case save-time line (longest reason + a wide confined count)
            // so the 24-row budget covers the DSV-007 Task 17 surface too.
            save_time: SaveTimePosture::Assurance(SaveTimeRender {
                assurance: WorkspaceAssurance {
                    state: AssuranceState::Stale,
                    reason: Some(
                        anvil_intercept_proto::protocol::StaleReason::ConfigBoundaryPolicyEdit,
                    ),
                    generation: u64::MAX,
                    last_full_scan: None,
                    scan_coverage: None,
                },
                confined: Some(9999),
            }),
            witness: WitnessSummary::Last {
                commit_short: "deadbee".to_string(),
                age: Some(Duration::from_secs(YEAR_SECS)),
            },
            next_action: next_action.to_string(),
            posture_facts: vec![
                "mcp: live".into(),
                "daemon: attesting worktree".into(),
                "save-time: attached".into(),
            ],
            posture_meaning: Some(
                "status claim may differ from start; subordinate facts listed above".into(),
            ),
        }
    }

    fn longest_next_action() -> String {
        WorktreeClaimState::all()
            .iter()
            .map(|s| next_action_for(*s, &DaemonSummary::NotRunning).to_string())
            .max_by_key(String::len)
            .expect("closed set is non-empty")
    }

    /// ADTRUST-001 validation: every closed-set protection state
    /// renders its canonical wire string verbatim. The success test
    /// hinges on the user being able to read the state and look it
    /// up in the spec without translation.
    #[test]
    fn names_protection_state() {
        for &state in WorktreeClaimState::all() {
            let rendered = render_plain_legible(&legible_test_snapshot(state));
            assert!(
                rendered.contains(state.as_str()),
                "render for {state:?} missing canonical string {:?}:\n{rendered}",
                state.as_str(),
            );
        }
    }

    /// `Protecting` maps to the closed-set `PreWriteDaemon` state in
    /// the v1 local-derivation path. `Full` requires a Participating
    /// editor driver per spec §14.2; that signal lives in the
    /// daemon registry and is unreachable from the activation
    /// diagnostic alone. Pins the honest undershoot so a future
    /// refactor cannot silently promote the claim to `Full` without
    /// wiring the missing surface check.
    #[test]
    fn status_protection_meaning_explains_warming_with_live_l0() {
        // ACTTUI-019: meaning comes from SharedPostureFacts (same as start).
        use activation::diagnostic::{ConfigStatus, McpClientId, McpTier, WatchTier};
        use std::collections::BTreeMap;

        let mut mcp = BTreeMap::new();
        mcp.insert(McpClientId::ClaudeCode, McpTier::LiveValidation.into());
        let diag = activation::ActivationDiagnostic {
            config: ConfigStatus::Valid,
            mcp,
            watch: WatchTier::NotRequested,
            baseline_present: false,
            baseline_summary: None,
            last_error: None,
            all_languages_unsupported: false,
            language_profile: activation::language_profile::RepoLanguageProfile::default(),
            daemon_attestation: activation::daemon_evidence::DaemonAttestation::NotProbed,
            save_time_driver_attached: false,
        };
        let facts = activation::SharedPostureFacts::from_diagnostic(&diag);
        let meaning = facts.meaning_for_status_claim("warming").unwrap();
        assert!(meaning.contains("warming"));
        assert!(meaning.contains("mcp: live"));
        assert!(meaning.contains("MCP is live"));
        assert!(!meaning.contains("subordinate:"));
        for line in facts.fact_lines() {
            assert!(meaning.contains(&line), "missing {line} in {meaning}");
        }
    }

    #[test]
    fn warming_with_restart_pending_names_action_without_internal_labels() {
        use activation::diagnostic::{ConfigStatus, McpClientId, McpTier, WatchTier};
        use std::collections::BTreeMap;

        let mut mcp = BTreeMap::new();
        mcp.insert(McpClientId::Cursor, McpTier::RestartRequired.into());
        let diag = activation::ActivationDiagnostic {
            config: ConfigStatus::Valid,
            mcp,
            watch: WatchTier::NotRequested,
            baseline_present: false,
            baseline_summary: None,
            last_error: None,
            all_languages_unsupported: false,
            language_profile: activation::language_profile::RepoLanguageProfile::default(),
            daemon_attestation: activation::daemon_evidence::DaemonAttestation::NotProbed,
            save_time_driver_attached: false,
        };
        let facts = activation::SharedPostureFacts::from_diagnostic(&diag);
        let mut snapshot = legible_test_snapshot(WorktreeClaimState::Warming);
        snapshot.next_action = next_action_for_diagnostic(
            WorktreeClaimState::Warming,
            &DaemonSummary::NotRunning,
            &diag,
        )
        .to_string();
        snapshot.posture_facts = facts.fact_lines();
        snapshot.posture_meaning = facts.meaning_for_status_claim("warming");

        let rendered = render_plain_legible(&snapshot);

        assert!(
            rendered
                .contains("Next: Restart your editor or agent, then run `anvil start --verify`."),
            "warming must name the action that can advance it: {rendered}"
        );
        assert!(
            !rendered.contains("subordinate:"),
            "default human output must not expose an internal fact label: {rendered}"
        );
        assert!(
            !rendered.contains("ready_restart_required"),
            "default human output must not expose a start-state token: {rendered}"
        );
    }

    #[test]
    fn warming_with_unreachable_daemon_routes_to_daemon_recovery() {
        use activation::daemon_evidence::DaemonAttestation;

        let mut diag = test_activation_diagnostic();
        diag.daemon_attestation = DaemonAttestation::Unreachable;

        let next = next_action_for_diagnostic(
            WorktreeClaimState::Warming,
            &DaemonSummary::NotRunning,
            &diag,
        );

        assert!(
            next.contains("`anvil start`"),
            "missing terminal recovery: {next}"
        );
        assert!(
            next.contains("`anvil intercept start --foreground`"),
            "missing headless recovery: {next}"
        );
        assert!(
            next.contains("`anvil start --verify`"),
            "missing verification: {next}"
        );
        assert!(
            !next.contains("Restart your editor"),
            "editor restart cannot recover an unreachable daemon: {next}"
        );
    }

    #[test]
    fn warming_with_live_mcp_and_attested_daemon_refreshes_instead_of_restarting() {
        use activation::daemon_evidence::DaemonAttestation;
        use activation::diagnostic::{McpClientId, McpTier};

        let mut diag = test_activation_diagnostic();
        diag.mcp
            .insert(McpClientId::ClaudeCode, McpTier::LiveValidation.into());
        diag.daemon_attestation = DaemonAttestation::Enforced;

        let next = next_action_for_diagnostic(
            WorktreeClaimState::Warming,
            &DaemonSummary::NotRunning,
            &diag,
        );

        assert!(
            next.contains("`anvil start --verify`"),
            "missing refresh action: {next}"
        );
        assert!(
            !next.contains("Restart your editor"),
            "live MCP must not be told to restart: {next}"
        );
    }

    struct WarmingRenderCase {
        name: &'static str,
        attestation: activation::daemon_evidence::DaemonAttestation,
        mcp_live: bool,
        expected_next: &'static str,
    }

    fn warming_render_cases() -> [WarmingRenderCase; 11] {
        use activation::daemon_evidence::DaemonAttestation;

        [
            WarmingRenderCase {
                name: "restart pending",
                attestation: DaemonAttestation::NotProbed,
                mcp_live: false,
                expected_next: "Next: Restart your editor or agent, then run `anvil start --verify`.",
            },
            WarmingRenderCase {
                name: "daemon unreachable",
                attestation: DaemonAttestation::Unreachable,
                mcp_live: false,
                expected_next: "Next: No intercept daemon is answering. Run `anvil start`",
            },
            WarmingRenderCase {
                name: "worktree unenforced",
                attestation: DaemonAttestation::Unenforced,
                mcp_live: false,
                expected_next: "Next: Run `anvil intercept status`",
            },
            WarmingRenderCase {
                name: "no participating surface",
                attestation: DaemonAttestation::NoParticipatingSurface,
                mcp_live: false,
                expected_next: "Next: Run `anvil intercept status`",
            },
            WarmingRenderCase {
                name: "stale heartbeat",
                attestation: DaemonAttestation::StaleHeartbeat,
                mcp_live: false,
                expected_next: "Next: Restart the intercept daemon",
            },
            WarmingRenderCase {
                name: "all surfaces quarantined",
                attestation: DaemonAttestation::AllSurfacesQuarantined,
                mcp_live: false,
                expected_next: "Next: Restart the fenced intercept daemon",
            },
            WarmingRenderCase {
                name: "daemon warming",
                attestation: DaemonAttestation::Warming,
                mcp_live: false,
                expected_next: "Next: Wait a few seconds",
            },
            WarmingRenderCase {
                name: "daemon enforced",
                attestation: DaemonAttestation::Enforced,
                mcp_live: false,
                expected_next: "Next: Protection evidence is live.",
            },
            WarmingRenderCase {
                name: "daemon promoted",
                attestation: DaemonAttestation::Promoted,
                mcp_live: false,
                expected_next: "Next: Protection evidence is live.",
            },
            WarmingRenderCase {
                name: "live MCP not probed",
                attestation: DaemonAttestation::NotProbed,
                mcp_live: true,
                expected_next: "Next: MCP is live. Run `anvil start --verify`",
            },
            WarmingRenderCase {
                name: "live MCP attested",
                attestation: DaemonAttestation::Enforced,
                mcp_live: true,
                expected_next: "Next: Protection evidence is live.",
            },
        ]
    }

    fn render_warming_case(case: &WarmingRenderCase) -> String {
        use activation::diagnostic::{McpClientId, McpTier};

        let mut diag = test_activation_diagnostic();
        diag.daemon_attestation = case.attestation;
        diag.mcp.insert(
            McpClientId::ClaudeCode,
            if case.mcp_live {
                McpTier::LiveValidation
            } else {
                McpTier::RestartRequired
            }
            .into(),
        );
        let facts = activation::SharedPostureFacts::from_diagnostic(&diag);
        let mut snapshot = legible_test_snapshot(WorktreeClaimState::Warming);
        snapshot.next_action = next_action_for_diagnostic(
            WorktreeClaimState::Warming,
            &DaemonSummary::NotRunning,
            &diag,
        )
        .to_string();
        snapshot.posture_facts = facts.fact_lines();
        snapshot.posture_meaning = facts.meaning_for_status_claim("warming");
        render_plain_legible(&snapshot)
    }

    fn assert_warming_render_case(case: &WarmingRenderCase) {
        let rendered = render_warming_case(case);
        assert!(
            rendered.contains(case.expected_next),
            "{} rendered the wrong recovery action: {rendered}",
            case.name
        );
        let meaning = rendered
            .lines()
            .find(|line| line.trim_start().starts_with("meaning:"))
            .unwrap_or_else(|| panic!("{} missing meaning line: {rendered}", case.name));
        for competing in ["Restart ", "Run `", "Wait ", "setup", "verify"] {
            assert!(
                !meaning.contains(competing),
                "{} meaning competes with Next via {competing:?}: {rendered}",
                case.name
            );
        }
        assert!(
            !rendered.contains("subordinate:"),
            "{}: {rendered}",
            case.name
        );
        assert!(
            !rendered.contains("ready_restart_required"),
            "{}: {rendered}",
            case.name
        );
        if case.name == "restart pending" {
            assert_eq!(
                rendered.matches("Restart your editor or agent").count(),
                1,
                "restart action must appear once, in Next: {rendered}"
            );
        } else {
            assert!(
                !rendered.contains("Restart your editor or agent"),
                "{} must not prescribe a generic editor restart: {rendered}",
                case.name
            );
        }
    }

    #[test]
    fn warming_full_render_keeps_recovery_actions_only_in_next() {
        for case in warming_render_cases() {
            assert_warming_render_case(&case);
        }
    }

    #[test]
    fn protecting_maps_to_pre_write_daemon_until_driver_signal_lands() {
        use activation::diagnostic::{
            ActivationDiagnostic, ConfigStatus, McpClientId, McpTier, WatchTier,
        };
        use std::collections::BTreeMap;

        let mut mcp = BTreeMap::new();
        mcp.insert(McpClientId::ClaudeCode, McpTier::LiveValidation.into());
        let diag = ActivationDiagnostic {
            config: ConfigStatus::Valid,
            mcp,
            watch: WatchTier::Running,
            baseline_present: false,
            baseline_summary: None,
            last_error: None,
            all_languages_unsupported: false,
            language_profile: activation::language_profile::RepoLanguageProfile::default(),
            daemon_attestation: activation::daemon_evidence::DaemonAttestation::NotProbed,
            save_time_driver_attached: false,
        };
        let data = StatusData {
            hooks: Vec::new(),
            profile: ProfileInfo {
                name: "(no config)".to_string(),
                checks: Vec::new(),
                path: ".anvilrc".to_string(),
            },
            recent_runs: Vec::new(),
            update_hint: None,
            insights_hint: None,
            whats_new_hint: None,
        };
        let layers = derive_layers(&data, &diag);
        let claim = derive_protection(&diag, &layers);
        assert_eq!(claim, WorktreeClaimState::PreWriteDaemon);
    }

    /// Witness summary parsing pulls the short SHA and falls back to
    /// `None` cleanly on malformed input. Pins the legible
    /// surface's "Witness: …" line so an unreadable chain never
    /// stalls the render.
    #[test]
    fn witness_summary_handles_missing_and_malformed() {
        let dir = make_temp_dir();
        assert!(matches!(read_witness_summary(&dir), WitnessSummary::None));

        let witness_dir = dir.join("anvil/witness");
        std::fs::create_dir_all(&witness_dir).unwrap();
        std::fs::write(witness_dir.join("active.ndjson"), "not json\n").unwrap();
        assert!(matches!(read_witness_summary(&dir), WitnessSummary::None));

        std::fs::write(
            witness_dir.join("active.ndjson"),
            "{\"commit\":\"abcdef1234567890\",\"ts\":\"2026-05-07T12:34:56Z\"}\n",
        )
        .unwrap();
        match read_witness_summary(&dir) {
            WitnessSummary::Last { commit_short, .. } => {
                assert_eq!(commit_short, "abcdef1");
            }
            WitnessSummary::None => panic!("expected Last variant"),
        }

        // Malformed `ts` (numeric offset variant the parser refuses)
        // must produce `age: None` rather than collapsing to zero —
        // the surface renders "age unknown" instead of misleading
        // "0s ago".
        std::fs::write(
            witness_dir.join("active.ndjson"),
            "{\"commit\":\"abcdef1234567890\",\"ts\":\"2026-05-07T12:34:56+00:00\"}\n",
        )
        .unwrap();
        match read_witness_summary(&dir) {
            WitnessSummary::Last {
                commit_short,
                age: None,
            } => {
                assert_eq!(commit_short, "abcdef1");
            }
            other => panic!("expected Last with age: None, got {other:?}"),
        }

        cleanup(&dir);
    }

    /// `format_duration` produces a single short token per bucket so
    /// the daemon/witness lines stay one row even at large values.
    #[test]
    fn format_duration_picks_largest_unit() {
        assert_eq!(format_duration(Duration::from_secs(5)), "5s");
        assert_eq!(format_duration(Duration::from_secs(90)), "1m");
        assert_eq!(format_duration(Duration::from_secs(HOUR_SECS)), "1h");
        assert_eq!(format_duration(Duration::from_secs(DAY_SECS)), "1d");
        assert_eq!(format_duration(Duration::from_secs(7 * DAY_SECS)), "7d");
    }

    // -----------------------------------------------------------------
    // ADTRUST-002 degraded-state banner
    // -----------------------------------------------------------------

    /// ADTRUST-002 validation: a degraded sample inside the 60-second
    /// rate-limit window must NOT re-emit the banner. The first sample
    /// returns `Some`; the second (well under 60s later) returns
    /// `None`.
    #[test]
    fn degraded_banner_rate_limited() {
        let mut banner = DegradedBanner::default();
        let t0 = std::time::Instant::now();
        let first = banner.poll(WorktreeClaimState::DegradedProtection, t0);
        assert!(first.is_some(), "first degraded sample must emit");
        let inside_window = banner.poll(
            WorktreeClaimState::DegradedProtection,
            t0 + Duration::from_secs(10),
        );
        assert!(
            inside_window.is_none(),
            "second sample inside the 60s window must suppress: {inside_window:?}"
        );
    }

    /// ADTRUST-002 validation: once the rate-limit window has elapsed
    /// the next degraded sample re-emits. Pins the "no silent middle"
    /// contract — the user must see the banner again on the next
    /// save-time interaction past 60s.
    #[test]
    fn degraded_emits_within_60s() {
        let mut banner = DegradedBanner::default();
        let t0 = std::time::Instant::now();
        let _ = banner.poll(WorktreeClaimState::MultiDaemonDetected, t0);
        let past_window = banner.poll(
            WorktreeClaimState::MultiDaemonDetected,
            t0 + Duration::from_secs(61),
        );
        assert!(
            past_window.is_some(),
            "sample past the 60s window must re-emit; got {past_window:?}"
        );
    }

    /// `Full` and `PreWriteDaemon` are NOT degraded — the banner must
    /// stay silent so it cannot drown out the surrounding output when
    /// protection is live. `is_degraded_claim` is the closed-set gate
    /// the surface depends on.
    #[test]
    fn full_and_pre_write_daemon_are_not_degraded() {
        let mut banner = DegradedBanner::default();
        let t0 = std::time::Instant::now();
        assert!(banner.poll(WorktreeClaimState::Full, t0).is_none());
        assert!(
            banner
                .poll(WorktreeClaimState::PreWriteDaemon, t0)
                .is_none()
        );
        assert!(banner.poll(WorktreeClaimState::SaveTimeOnly, t0).is_none());
    }

    /// Banner content names the closed-set string and points at
    /// `anvil doctor`. Pinning both keeps the surfaces honest — they
    /// cannot invent wording or omit the recovery pointer.
    #[test]
    fn degraded_banner_names_state_and_doctor() {
        let line = format_degraded_banner(WorktreeClaimState::PathUncertain);
        assert!(
            line.contains("path-uncertain"),
            "missing closed-set string: {line}"
        );
        assert!(
            line.contains("anvil doctor"),
            "missing recovery pointer: {line}"
        );
    }

    /// User-authored config-mode entries surface without the `(anvil-managed)`
    /// tag — the surface must distinguish the two so users can tell their
    /// own commands from anvil's.
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

    // -----------------------------------------------------------------
    // MLP2-048: resolve_protection_claim — daemon-snapshot wiring.
    // -----------------------------------------------------------------

    use anvil_intercept_proto::status::{
        DaemonStatusV1, FenceStateV1, HealthStateV1, IpcStateV1, LatencyMidEditMapV1,
        WorktreeStatusV1,
    };
    use anvil_intercept_proto::{SessionId, SessionRecord, SessionStatus};
    use anvil_kernel_types::protection_claim::SurfaceClaimState;

    fn snapshot_with_session_at(worktree: &Path, fenced: bool, draining: bool) -> DaemonStatusV1 {
        let session = SessionRecord {
            id: SessionId::new("sess-test"),
            worktree: worktree.to_path_buf(),
            pid: Some(4242),
            pgid: Some(4242),
            started_at_unix: 1_700_000_000,
            last_heartbeat_unix: 1_700_000_010,
            status: SessionStatus::Active,
            agent_tag: None,
            daemon_issued_tag: None,
        };
        DaemonStatusV1 {
            sessions: vec![session.clone()],
            worktrees: vec![WorktreeStatusV1 {
                worktree: worktree.to_path_buf(),
                session_id: session.id.clone(),
                fenced,
                cascaded: false,
                cascade_since: None,
                save_time_driver: SaveTimeDriverStatusV1::Absent,
            }],
            fences: if fenced {
                vec![FenceStateV1 {
                    worktree: worktree.to_path_buf(),
                    reason: "test fence".to_owned(),
                    fenced_at_unix: 1_700_000_000,
                }]
            } else {
                vec![]
            },
            health: HealthStateV1 {
                uptime_seconds: 5,
                version: "0.7.0-beta".to_owned(),
                ipc_state: if draining {
                    IpcStateV1::Draining
                } else {
                    IpcStateV1::Serving
                },
            },
            latency: LatencyMidEditMapV1::default(),
            cache_entries: None,
            cache_invalidations_total: None,
            in_flight_evaluations: None,
            cache_invalidations_rate_limited: None,
            telemetry_subscriber_count: None,
            telemetry_dropped_envelopes: None,
            generated_at_unix: 0,
        }
    }

    /// ACTMO-017: the registered-worktrees section lists durable members with
    /// their membership label and flags the current directory; live
    /// (non-durable) sessions are excluded.
    #[test]
    fn render_registered_worktrees_lists_membership_and_flags_cwd() {
        use anvil_intercept_proto::session::{ACTIVATION_SPINE_CLAIMED_AGENT_ID, AgentTag};

        let wt_a = Path::new("/tmp/anvil-status-reg-a");
        let wt_b = Path::new("/tmp/anvil-status-reg-b");
        let spine = AgentTag::new("anvil-start", ACTIVATION_SPINE_CLAIMED_AGENT_ID, 0);
        let durable = |wt: &Path, id: &str| SessionRecord {
            id: SessionId::new(id),
            worktree: wt.to_path_buf(),
            pid: None,
            pgid: None,
            started_at_unix: 0,
            last_heartbeat_unix: 0,
            status: SessionStatus::Active,
            agent_tag: Some(spine.clone()),
            daemon_issued_tag: None,
        };

        // Reuse the fixture boilerplate, then override the registry view: a
        // clean durable member, a fenced durable member, and a live (untagged)
        // session that must NOT appear in the registered section.
        let mut snapshot = snapshot_with_session_at(wt_a, false, false);
        let mut live = durable(Path::new("/tmp/anvil-status-live"), "live");
        live.agent_tag = None;
        snapshot.sessions = vec![durable(wt_a, "sa"), durable(wt_b, "sb"), live];
        snapshot.worktrees = vec![
            WorktreeStatusV1 {
                worktree: wt_a.to_path_buf(),
                session_id: SessionId::new("sa"),
                fenced: false,
                cascaded: false,
                cascade_since: None,
                save_time_driver: SaveTimeDriverStatusV1::Absent,
            },
            WorktreeStatusV1 {
                worktree: wt_b.to_path_buf(),
                session_id: SessionId::new("sb"),
                fenced: true,
                cascaded: false,
                cascade_since: None,
                save_time_driver: SaveTimeDriverStatusV1::Absent,
            },
        ];

        let out = render_registered_worktrees(Some(&snapshot), Some(wt_a));
        assert!(
            out.contains("Registered worktrees:"),
            "section header: {out}"
        );
        assert!(
            out.contains("anvil-status-reg-a [registered] (current)"),
            "{out}"
        );
        assert!(out.contains("anvil-status-reg-b [fenced]"), "{out}");
        assert!(
            !out.contains("anvil-status-live"),
            "live sessions excluded: {out}"
        );
    }

    #[test]
    fn render_registered_worktrees_degrades_when_daemon_unavailable() {
        let out = render_registered_worktrees(None, None);
        assert!(out.contains("(daemon unavailable)"), "{out}");
    }

    /// DSV-049: the plain registered-worktrees section surfaces the
    /// save-time driver state per worktree — `attached` and `failed`
    /// are shown as an explicit `driver: …` segment; `absent` stays
    /// silent so the pre-DSV-049 surface is byte-identical for the
    /// common (supervision-off) case.
    #[test]
    fn status_save_time_driver_segment_renders_attached_failed_and_silent_absent() {
        let attached = Path::new("/tmp/anvil-status-drv-attached");
        let failed = Path::new("/tmp/anvil-status-drv-failed");
        let absent = Path::new("/tmp/anvil-status-drv-absent");

        let mut snapshot = snapshot_with_session_at(attached, false, false);
        let durable = |wt: &Path, id: &str| {
            use anvil_intercept_proto::session::{ACTIVATION_SPINE_CLAIMED_AGENT_ID, AgentTag};
            SessionRecord {
                id: SessionId::new(id),
                worktree: wt.to_path_buf(),
                pid: None,
                pgid: None,
                started_at_unix: 0,
                last_heartbeat_unix: 0,
                status: SessionStatus::Active,
                agent_tag: Some(AgentTag::new(
                    "anvil-start",
                    ACTIVATION_SPINE_CLAIMED_AGENT_ID,
                    0,
                )),
                daemon_issued_tag: None,
            }
        };
        snapshot.sessions = vec![
            durable(attached, "da"),
            durable(failed, "df"),
            durable(absent, "dn"),
        ];
        let entry = |wt: &Path, id: &str, drv: SaveTimeDriverStatusV1| WorktreeStatusV1 {
            worktree: wt.to_path_buf(),
            session_id: SessionId::new(id),
            fenced: false,
            cascaded: false,
            cascade_since: None,
            save_time_driver: drv,
        };
        snapshot.worktrees = vec![
            entry(attached, "da", SaveTimeDriverStatusV1::Attached),
            entry(failed, "df", SaveTimeDriverStatusV1::Failed),
            entry(absent, "dn", SaveTimeDriverStatusV1::Absent),
        ];

        let out = render_registered_worktrees(Some(&snapshot), None);
        assert!(
            out.contains("anvil-status-drv-attached [registered] driver: attached"),
            "{out}"
        );
        assert!(
            out.contains("anvil-status-drv-failed [registered] driver: failed"),
            "{out}"
        );
        // Absent: no driver segment — byte-identical to the pre-DSV-049 line.
        assert!(
            out.contains("anvil-status-drv-absent [registered]\n"),
            "absent driver must stay silent: {out}"
        );
        assert!(!out.contains("driver: absent"), "{out}");
    }

    /// DSV-049: the `--json` wire-string mapper covers every proto arm,
    /// including the forward-compat `Unknown` (surfaced honestly to
    /// machine consumers as `"unknown"`, unlike the plain surface which
    /// folds it to silent).
    #[test]
    fn status_save_time_driver_str_maps_every_arm() {
        assert_eq!(
            save_time_driver_str(SaveTimeDriverStatusV1::Attached),
            "attached"
        );
        assert_eq!(
            save_time_driver_str(SaveTimeDriverStatusV1::Absent),
            "absent"
        );
        assert_eq!(
            save_time_driver_str(SaveTimeDriverStatusV1::Failed),
            "failed"
        );
        assert_eq!(
            save_time_driver_str(SaveTimeDriverStatusV1::Unknown),
            "unknown"
        );
    }

    /// DSV-049: an `Unknown` driver state (from a newer daemon) folds to
    /// silent on the plain surface — the wire contract's "treat unknown
    /// fail-safe as absent" rule, so an older CLI never renders an
    /// unrecognised state as coverage.
    #[test]
    fn status_save_time_driver_segment_treats_unknown_as_silent() {
        let wt = Path::new("/tmp/anvil-status-drv-unknown");
        let mut snapshot = snapshot_with_session_at(wt, false, false);
        snapshot.worktrees[0].save_time_driver = SaveTimeDriverStatusV1::Unknown;
        assert_eq!(driver_segment(&snapshot, wt), "");
    }

    #[test]
    fn render_registered_worktrees_reports_none_and_unregistered_cwd() {
        // A snapshot whose only session is non-durable yields an empty set.
        let snapshot = snapshot_with_session_at(Path::new("/tmp/anvil-status-x"), false, false);
        let out = render_registered_worktrees(Some(&snapshot), None);
        assert!(out.contains("Registered worktrees: (none)"), "{out}");
    }

    /// Helper: an `ActivationDiagnostic` that maps to `Unprotected`
    /// from the local-only fallback path, plus an empty `LayerSummary`
    /// for the local-derivation path. Both are unused in the
    /// daemon-snapshot branch but must be supplied for the call sig.
    fn unprotected_diag_and_layers() -> (activation::ActivationDiagnostic, LayerSummary) {
        // Synthetic: live `activation::verify` on a missing path still
        // reads the operator HOME MCP configs and can yield
        // `ReadyRestartRequired` (Warming) on a machine with anvil
        // installed. These tests only need a local-only fallback diag.
        let diag = test_activation_diagnostic();
        let layers = LayerSummary {
            l0_mcp: LayerState::Off,
            l1_mid_edit: LayerState::Unknown,
            l2_save: LayerState::Off,
            l3_commit: LayerState::Off,
            l4_push: LayerState::Off,
            l5_audit: LayerState::Unknown,
        };
        (diag, layers)
    }

    /// When a live daemon snapshot is supplied,
    /// `protection_claim_section::resolve_protection_claim` MUST
    /// consult it: surfaces enumerate real sessions on the queried
    /// worktree and the state collapses to `PreWriteDaemon` for a
    /// clean session. Closes the MLP2-048 audit gap — local-only path
    /// emitted an empty `surfaces` array regardless of daemon state.
    #[test]
    fn resolve_protection_claim_uses_daemon_snapshot_when_available() {
        let (diag, _layers) = unprotected_diag_and_layers();
        let worktree = Path::new("/tmp/wt-resolve-pre");
        let snapshot = snapshot_with_session_at(worktree, false, false);
        let claim =
            protection_claim_section::resolve_protection_claim(&diag, Some(&snapshot), worktree);
        assert_eq!(claim.worktree_state, WorktreeClaimState::PreWriteDaemon);
        assert_eq!(claim.surfaces.len(), 1);
        assert_eq!(claim.surfaces[0].identifier, "sess-test");
        assert_eq!(claim.surfaces[0].state, SurfaceClaimState::Participating);
    }

    /// A fenced session reaches `DegradedProtection` + `Quarantined`
    /// surface through the snapshot path.
    #[test]
    fn resolve_protection_claim_reflects_fenced_session_from_snapshot() {
        let (diag, _layers) = unprotected_diag_and_layers();
        let worktree = Path::new("/tmp/wt-resolve-fenced");
        let snapshot = snapshot_with_session_at(worktree, true, false);
        let claim =
            protection_claim_section::resolve_protection_claim(&diag, Some(&snapshot), worktree);
        assert_eq!(claim.worktree_state, WorktreeClaimState::DegradedProtection);
        assert_eq!(claim.surfaces[0].state, SurfaceClaimState::Quarantined);
    }

    /// IPC `Draining` reaches `Warming` + `Detached` through
    /// `protection_claim_section::resolve_protection_claim`. Closes
    /// the CLI-side gap that the adversarial review flagged:
    /// previously this transition was only exercised in the
    /// intercept-side parity tests.
    #[test]
    fn resolve_protection_claim_reflects_draining_ipc_from_snapshot() {
        let (diag, _layers) = unprotected_diag_and_layers();
        let worktree = Path::new("/tmp/wt-resolve-drain");
        let snapshot = snapshot_with_session_at(worktree, false, true);
        let claim =
            protection_claim_section::resolve_protection_claim(&diag, Some(&snapshot), worktree);
        assert_eq!(claim.worktree_state, WorktreeClaimState::Warming);
        assert_eq!(claim.surfaces[0].state, SurfaceClaimState::Detached);
    }

    /// When the daemon snapshot is absent (daemon down / connect
    /// failed) the shared resolver falls back to the locally-
    /// derivable worktree state with an explicitly empty `surfaces`
    /// array. The spec permits this — it does not over-claim per-
    /// surface coverage when there is no daemon evidence.
    #[test]
    fn resolve_protection_claim_falls_back_when_snapshot_absent() {
        let (diag, _layers) = unprotected_diag_and_layers();
        let worktree = Path::new("/tmp/wt-resolve-fallback");
        let claim = protection_claim_section::resolve_protection_claim(&diag, None, worktree);
        assert!(
            claim.surfaces.is_empty(),
            "fallback path must not invent surfaces: {claim:?}",
        );
        // The local-only derivation collapses to `Unprotected` for a
        // path that does not exist; pin the worktree state too so a
        // future refactor of `derive_local_worktree_state` cannot
        // silently upgrade the fallback claim.
        assert_eq!(claim.worktree_state, WorktreeClaimState::Unprotected);
    }

    /// Snapshot present but the queried worktree is unknown to the
    /// daemon → `Unprotected` with empty surfaces, even though the
    /// daemon is up. Pins that the worktree key matters, not just
    /// snapshot presence.
    #[test]
    fn resolve_protection_claim_unknown_worktree_in_snapshot_is_unprotected() {
        let (diag, _layers) = unprotected_diag_and_layers();
        let known_worktree = Path::new("/tmp/wt-resolve-known");
        let queried = Path::new("/tmp/wt-resolve-not-in-snapshot");
        let snapshot = snapshot_with_session_at(known_worktree, false, false);
        let claim =
            protection_claim_section::resolve_protection_claim(&diag, Some(&snapshot), queried);
        assert_eq!(claim.worktree_state, WorktreeClaimState::Unprotected);
        assert!(claim.surfaces.is_empty());
    }

    fn status_output_for_mcp_test(mcp: status_mcp::StatusMcpJson) -> StatusOutput {
        StatusOutput {
            schema_version: STATUS_SCHEMA_VERSION,
            activation: serde_json::json!({
                "state": "protecting",
                "headline": "Protecting",
                "config": "valid",
                "mcp": [],
                "watch": "not_requested"
            }),
            hooks: vec![],
            profile: ProfileOutput {
                name: "test".into(),
                checks: vec![],
                path: String::new(),
            },
            recent_runs: vec![],
            claim: ProtectionClaim::new(WorktreeClaimState::PreWriteDaemon, vec![]),
            install_root: None,
            project_writes_gated: None,
            save_time: None,
            save_time_driver: None,
            mcp,
        }
    }

    #[test]
    fn status_json_omits_mcp_inventory_when_empty() {
        let value = serde_json::to_value(status_output_for_mcp_test(
            status_mcp::StatusMcpJson::default(),
        ))
        .expect("serialize");
        for key in [
            "cli_version",
            "mcp_skew",
            "mcp_processes",
            "graph",
            "agent_ready",
            "graph_ready",
            "protecting",
        ] {
            assert!(
                value.get(key).is_none(),
                "empty inventory must omit {key}: {value}"
            );
        }
    }

    #[test]
    fn status_json_includes_split_claims_when_inventory_present() {
        let inventory = status_mcp::classify_inventory(
            "0.9.5-beta",
            &[status_mcp::McpProcessRecord {
                pid: 200,
                parent_pid: Some(100),
                parent_command: "grok".into(),
                version: Some("0.9.2-beta".into()),
                current: false,
                orphan: false,
            }],
        );
        let graph = status_mcp::GraphReadiness {
            state: status_mcp::GraphState::Stale,
            reason: Some("scan-timeout".into()),
        };
        let mcp = status_mcp::status_mcp_json("0.9.5-beta", true, Some(&inventory), Some(&graph))
            .expect("mcp json");
        let value = serde_json::to_value(status_output_for_mcp_test(mcp)).expect("serialize");
        assert_eq!(value["cli_version"], "0.9.5-beta");
        assert_eq!(value["mcp_skew"], true);
        assert_eq!(value["mcp_processes"]["total"], 1);
        assert_eq!(value["mcp_processes"]["skewed"], 1);
        assert_eq!(value["agent_ready"], false);
        assert_eq!(value["graph_ready"], false);
        assert_eq!(value["protecting"], true);
        assert_eq!(value["graph"]["state"], "stale");
        assert_eq!(value["graph"]["reason"], "scan-timeout");
    }

    // ── INSIGHTS-004 first-week hint tests (drive the nudge in status) ──

    fn seed_project_id_with_created_at(dir: &Path, days_ago: i64) {
        let anvil_dir = dir.join("anvil");
        std::fs::create_dir_all(&anvil_dir).unwrap();
        let created = (chrono::Utc::now() - chrono::Duration::days(days_ago))
            .to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
        let contents = format!(
            "# test project-id for INSIGHTS-004\nproject_uuid: 01999999-aaaa-bbbb-cccc-000000000004\ncreated_at: {created}\n"
        );
        std::fs::write(anvil_dir.join("project-id"), contents).unwrap();
        // Ensure no stale hint state from other tests.
        let _ = std::fs::remove_file(dir.join(".anvil/insights-hint.json"));
    }

    #[test]
    fn first_week_hint_shown_once() {
        use chrono::Utc;
        let dir = make_temp_dir();
        seed_project_id_with_created_at(&dir, 2); // well inside 14d

        // Simulate status plain path with recent install: the nudge
        // should be computed and present for human output.
        let mut data = gather_status_data(dir.to_str().unwrap());
        // Force the caller wiring path (status run does this for !json).
        data.insights_hint =
            crate::insights::first_week_hint::first_week_insights_hint(&dir, Utc::now(), false);

        // In plain render the hint appears as a trailing line.
        // We assert on the data (the render just prints it); the
        // presence proves the once-in-window gate opened.
        assert!(
            data.insights_hint.is_some(),
            "first-week user must see the insights nudge"
        );
        let line = data.insights_hint.as_ref().unwrap();
        assert!(
            line.contains("watched"),
            "nudge must mention watched activity"
        );
        assert!(line.contains("run `anvil insights`"));

        // Second computation in same week must be suppressed by the
        // internal state written on first emission.
        let second =
            crate::insights::first_week_hint::first_week_insights_hint(&dir, Utc::now(), false);
        assert!(
            second.is_none(),
            "nudge must be emitted at most once per week"
        );

        cleanup(&dir);
    }

    #[test]
    fn first_week_hint_suppressed_when_project_writes_gated() {
        use chrono::Utc;
        let dir = make_temp_dir();
        seed_project_id_with_created_at(&dir, 2); // well inside 14d

        // Under a gated project root the status surface must neither emit the
        // nudge nor write the real project's hint state (DISTRIB-006 / ADR-060).
        let hint =
            crate::insights::first_week_hint::first_week_insights_hint(&dir, Utc::now(), true);
        assert!(
            hint.is_none(),
            "gated status must not emit the first-week nudge"
        );
        assert!(
            !dir.join(".anvil/insights-hint.json").exists(),
            "gated status must not write the project hint state"
        );

        cleanup(&dir);
    }

    #[test]
    fn hint_suppressed_after_use() {
        use chrono::Utc;
        let dir = make_temp_dir();
        seed_project_id_with_created_at(&dir, 1);

        // Simulate running `anvil insights` (the default summary).
        crate::insights::first_week_hint::record_insights_viewed(&dir, chrono::Utc::now());

        // Now status/walk should see no nudge.
        let hint =
            crate::insights::first_week_hint::first_week_insights_hint(&dir, Utc::now(), false);
        assert!(
            hint.is_none(),
            "hint must be suppressed for the week after running insights"
        );

        cleanup(&dir);
    }
}
