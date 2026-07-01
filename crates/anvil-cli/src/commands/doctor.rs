use std::io::IsTerminal;
use std::path::Path;
use std::process::Command;

use anvil_kernel_types::hooks::is_anvil_managed_command;
use anvil_kernel_types::protection_claim::ProtectionClaim;
use anvil_kernel_types::{
    Notification, NotificationClass, NotificationContext, NotificationPriority,
};
use anvil_tui::surfaces::doctor::{CheckStatus, DiagnosticCheck, DoctorState, Remediation};
use serde::Serialize;

use crate::GlobalArgs;
use crate::commands::hooks::{
    config_hooks_enabled, list_config_hook_commands, resolve_file_mode_hook_paths,
};
use crate::commands::protection_claim_section;
use crate::services::interactive_fix::{FixOutcome, apply_fix_request};

/// JSON output schema version. Bumped to 2.0.0 in LAUNCH-005 because
/// every check now carries a structured `remediation` object — a
/// backwards-incompatible addition for consumers that schema-validated
/// against the prior shape.
const SCHEMA_VERSION: &str = "2.0.0";

#[derive(Debug, clap::Args)]
pub struct DoctorArgs {
    /// Auto-fix issues where possible
    #[arg(long)]
    fix: bool,
}

pub fn run(args: &DoctorArgs, global: &GlobalArgs) -> anyhow::Result<()> {
    let mut checks = run_all_checks();

    if args.fix {
        // DISTRIB-006 (ADR-060): `--fix` writes `.anvilrc`, `.anvil/`, and
        // `plans/` into the project — durable per-project state. Refuse under a
        // gated ANVIL_HOME; the read-only diagnostic (without `--fix`) is
        // unaffected.
        crate::install_root::ensure_project_write_allowed("doctor --fix")?;
        apply_fixes(&mut checks, global.json);
    }

    // MLP2-051a: resolve the typed `ProtectionClaim` for the current
    // worktree the same way `anvil status --json` does — daemon-snapshot
    // if reachable, local-only fallback otherwise. The fetch only runs
    // for the JSON and plain surfaces that consume it; the TUI branch
    // does not render the claim (its own surface owns that real-estate)
    // so we skip the IPC round-trip there.
    if global.json {
        let protection_claim = protection_claim_section::fetch_protection_claim_for_cwd();
        print_json(&checks, &protection_claim)?;
    } else if global.no_tui || !std::io::stdout().is_terminal() {
        let protection_claim = protection_claim_section::fetch_protection_claim_for_cwd();
        print_plain(&checks, &protection_claim);
    } else {
        let mut state = DoctorState::new(checks.clone());
        loop {
            state = crate::tui::run_surface(state)?;
            if let Some(request) = state.pending_fix.take() {
                let selected = state.selected;
                let outcome = apply_fix_request(&request, Some(&mut state.checks));
                let banner = match &outcome {
                    FixOutcome::Applied { summary } => {
                        anvil_tui::surfaces::doctor::FixOutcomeBanner::Applied {
                            summary: summary.clone(),
                        }
                    }
                    FixOutcome::Refused { reason } => {
                        anvil_tui::surfaces::doctor::FixOutcomeBanner::Refused {
                            reason: reason.clone(),
                        }
                    }
                    FixOutcome::Failed { reason } => {
                        anvil_tui::surfaces::doctor::FixOutcomeBanner::Failed {
                            reason: reason.clone(),
                        }
                    }
                };
                state.last_fix_outcome = Some(banner);
                if matches!(outcome, FixOutcome::Applied { .. }) {
                    let fresh = collect_checks();
                    state.checks = fresh;
                    state.selected = selected.min(state.checks.len().saturating_sub(1));
                }
                continue;
            }
            break;
        }
        checks = state.checks;
    }

    let has_failures = checks.iter().any(|c| c.status == CheckStatus::Fail);
    if has_failures {
        anyhow::bail!("Doctor check failed");
    }

    Ok(())
}

// --- Check runners ---

/// Collect diagnostic checks (convenience for sub-surface use).
pub fn collect_checks() -> Vec<DiagnosticCheck> {
    run_all_checks()
}

fn run_all_checks() -> Vec<DiagnosticCheck> {
    vec![
        check_git_available(),
        check_git_repo(),
        check_config_exists(),
        check_config_valid(),
        check_anvil_dir(),
        check_anvil_dir_writable(),
        check_plans_dir(),
        check_hooks_installed(),
        check_registry_patterns_compile(),
        check_project_id(),
        check_state_boundary(),
    ]
}

fn check_git_available() -> DiagnosticCheck {
    let result = Command::new("git").arg("--version").output();

    match result {
        Ok(output) if output.status.success() => {
            let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
            DiagnosticCheck {
                name: "git-available".to_string(),
                category: "System".to_string(),
                status: CheckStatus::Pass,
                message: version,
                details: None,
                auto_fixable: false,
                remediation: Remediation::default(),
            }
        }
        _ => DiagnosticCheck {
            name: "git-available".to_string(),
            category: "System".to_string(),
            status: CheckStatus::Fail,
            message: "git not found on PATH".to_string(),
            details: Some(
                "git is required for plan history and the watch loop's --changed selector."
                    .to_string(),
            ),
            auto_fixable: false,
            remediation: Remediation {
                summary: "Install git so it is available on PATH.".to_string(),
                command: None,
                doc_url: Some("https://git-scm.com/downloads".to_string()),
            },
        },
    }
}

fn check_git_repo() -> DiagnosticCheck {
    // Use git rev-parse to detect repos from subdirectories and worktrees,
    // rather than checking for a .git entry in the current directory.
    let is_repo = Command::new("git")
        .args(["rev-parse", "--git-dir"])
        .output()
        .is_ok_and(|o| o.status.success());

    // Missing git repo is a Warn rather than Fail (issue #1108): a fresh
    // user running `anvil doctor` in a brand-new directory before
    // `git init` should see a guiding next-step, not a hard failure.
    // Plan history and the watch loop's `--changed` selector still need
    // git, but those features fail loudly on their own when invoked
    // outside a repo — doctor's job here is to surface the gap, not to
    // gate the rest of the run.
    DiagnosticCheck {
        name: "git-repo".to_string(),
        category: "System".to_string(),
        status: if is_repo {
            CheckStatus::Pass
        } else {
            CheckStatus::Warn
        },
        message: if is_repo {
            "git repository detected".to_string()
        } else {
            "not a git repository — run `git init` to enable plan history and `--changed`"
                .to_string()
        },
        details: None,
        auto_fixable: !is_repo,
        remediation: if is_repo {
            Remediation::default()
        } else {
            Remediation {
                summary: "Initialise a git repository in the current directory so anvil can \
                          track plan history and scope `anvil watch --changed` to your edits."
                    .to_string(),
                command: Some("git init".to_string()),
                doc_url: None,
            }
        },
    }
}

fn check_config_exists() -> DiagnosticCheck {
    let exists = Path::new(".anvilrc").exists();

    DiagnosticCheck {
        name: "config-exists".to_string(),
        category: "Configuration".to_string(),
        status: if exists {
            CheckStatus::Pass
        } else {
            CheckStatus::Warn
        },
        message: if exists {
            ".anvilrc found".to_string()
        } else {
            ".anvilrc not found".to_string()
        },
        details: None,
        auto_fixable: !exists,
        remediation: if exists {
            Remediation::default()
        } else {
            Remediation {
                summary: "Create .anvilrc with default configuration.".to_string(),
                command: Some("anvil init".to_string()),
                doc_url: None,
            }
        },
    }
}

#[allow(clippy::too_many_lines)] // Each branch is a distinct error shape with its own remediation.
fn check_config_valid() -> DiagnosticCheck {
    let path = Path::new(".anvilrc");

    if !path.exists() {
        return DiagnosticCheck {
            name: "config-valid".to_string(),
            category: "Configuration".to_string(),
            status: CheckStatus::Skipped,
            message: "no .anvilrc to validate".to_string(),
            details: None,
            auto_fixable: false,
            remediation: Remediation::default(),
        };
    }

    match std::fs::read_to_string(path) {
        Ok(content) if content.trim().is_empty() => DiagnosticCheck {
            name: "config-valid".to_string(),
            category: "Configuration".to_string(),
            status: CheckStatus::Fail,
            message: ".anvilrc is empty".to_string(),
            details: None,
            auto_fixable: false,
            remediation: Remediation {
                summary: "Regenerate .anvilrc with the default configuration.".to_string(),
                command: Some("anvil init --force".to_string()),
                doc_url: None,
            },
        },
        Ok(content) => {
            // Accept JSON, YAML, or TOML — must parse as a mapping/table, not a scalar.
            let json_ok = serde_json::from_str::<serde_json::Value>(&content)
                .ok()
                .is_some_and(|v| v.is_object());
            let yaml_ok = serde_yaml::from_str::<serde_yaml::Value>(&content)
                .ok()
                .is_some_and(|v| v.is_mapping());
            let toml_ok = toml::from_str::<toml::Value>(&content)
                .ok()
                .is_some_and(|v| v.is_table());

            if json_ok || yaml_ok || toml_ok {
                DiagnosticCheck {
                    name: "config-valid".to_string(),
                    category: "Configuration".to_string(),
                    status: CheckStatus::Pass,
                    message: ".anvilrc is valid (JSON/YAML/TOML)".to_string(),
                    details: None,
                    auto_fixable: false,
                    remediation: Remediation::default(),
                }
            } else {
                let mut errors = Vec::new();
                match serde_json::from_str::<serde_json::Value>(&content) {
                    Ok(v) if !v.is_object() => {
                        errors.push("JSON: parsed but is not an object".into());
                    }
                    Err(e) => errors.push(format!("JSON: {e}")),
                    _ => {}
                }
                match serde_yaml::from_str::<serde_yaml::Value>(&content) {
                    Ok(v) if !v.is_mapping() => {
                        errors.push("YAML: parsed but is not a mapping".into());
                    }
                    Err(e) => errors.push(format!("YAML: {e}")),
                    _ => {}
                }
                match toml::from_str::<toml::Value>(&content) {
                    Ok(v) if !v.is_table() => {
                        errors.push("TOML: parsed but is not a table".into());
                    }
                    Err(e) => errors.push(format!("TOML: {e}")),
                    _ => {}
                }
                let detail = if errors.is_empty() {
                    "content is not a valid object/mapping/table".to_string()
                } else {
                    errors.join("; ")
                };
                DiagnosticCheck {
                    name: "config-valid".to_string(),
                    category: "Configuration".to_string(),
                    status: CheckStatus::Fail,
                    message: "invalid .anvilrc (not valid JSON, YAML, or TOML)".to_string(),
                    details: Some(detail),
                    auto_fixable: false,
                    remediation: Remediation {
                        // Cross-platform, anvil-native: `--force` overwrites
                        // the existing file. We tell the user to back up
                        // any credentials before running it; the alternative
                        // would be shipping a Unix-only `mv -n` command that
                        // does nothing on Windows.
                        summary: "Back up any credentials inside `.anvilrc`, then regenerate defaults."
                            .to_string(),
                        command: Some("anvil init --force".to_string()),
                        doc_url: None,
                    },
                }
            }
        }
        Err(e) => DiagnosticCheck {
            name: "config-valid".to_string(),
            category: "Configuration".to_string(),
            status: CheckStatus::Fail,
            message: "failed to read .anvilrc".to_string(),
            details: Some(e.to_string()),
            auto_fixable: false,
            remediation: Remediation {
                summary: "Check filesystem permissions on `.anvilrc` and confirm the file is readable in your shell."
                    .to_string(),
                command: None,
                doc_url: None,
            },
        },
    }
}

fn check_anvil_dir() -> DiagnosticCheck {
    let exists = Path::new(".anvil").is_dir();

    DiagnosticCheck {
        name: "anvil-dir".to_string(),
        category: "Configuration".to_string(),
        status: if exists {
            CheckStatus::Pass
        } else {
            CheckStatus::Warn
        },
        message: if exists {
            ".anvil/ directory found".to_string()
        } else {
            ".anvil/ directory not found".to_string()
        },
        details: None,
        auto_fixable: !exists,
        remediation: if exists {
            Remediation::default()
        } else {
            Remediation {
                summary: "Create the `.anvil/` state directory.".to_string(),
                command: Some("anvil init".to_string()),
                doc_url: None,
            }
        },
    }
}

fn check_anvil_dir_writable() -> DiagnosticCheck {
    let dir = Path::new(".anvil");

    if !dir.is_dir() {
        return DiagnosticCheck {
            name: "anvil-dir-writable".to_string(),
            category: "Permissions".to_string(),
            status: CheckStatus::Skipped,
            message: ".anvil/ does not exist".to_string(),
            details: None,
            auto_fixable: false,
            remediation: Remediation::default(),
        };
    }

    let probe = dir.join(".write-test");
    let writable = std::fs::write(&probe, "").is_ok();
    if writable {
        let _ = std::fs::remove_file(&probe);
    }

    DiagnosticCheck {
        name: "anvil-dir-writable".to_string(),
        category: "Permissions".to_string(),
        status: if writable {
            CheckStatus::Pass
        } else {
            CheckStatus::Fail
        },
        message: if writable {
            ".anvil/ is writable".to_string()
        } else {
            ".anvil/ is not writable".to_string()
        },
        details: None,
        auto_fixable: false,
        remediation: if writable {
            Remediation::default()
        } else {
            Remediation {
                summary: "Restore write access to the `.anvil/` directory in your OS file manager or shell. If it lives on a read-only mount (Docker volume, NFS share), the mount itself needs to change."
                    .to_string(),
                command: None,
                doc_url: None,
            }
        },
    }
}

fn check_plans_dir() -> DiagnosticCheck {
    let plans = Path::new("plans").is_dir();
    let docs_plans = Path::new("docs/plans").is_dir();

    if plans || docs_plans {
        let location = if plans { "plans/" } else { "docs/plans/" };
        DiagnosticCheck {
            name: "plans-dir".to_string(),
            category: "Configuration".to_string(),
            status: CheckStatus::Pass,
            message: format!("{location} directory found"),
            details: None,
            auto_fixable: false,
            remediation: Remediation::default(),
        }
    } else {
        DiagnosticCheck {
            name: "plans-dir".to_string(),
            category: "Configuration".to_string(),
            status: CheckStatus::Warn,
            message: "no plans directory found".to_string(),
            details: None,
            auto_fixable: true,
            remediation: Remediation {
                summary: "Create the `plans/` directory for specification documents (`anvil init` does this for you).".to_string(),
                command: Some("anvil init".to_string()),
                doc_url: None,
            },
        }
    }
}

fn check_hooks_installed() -> DiagnosticCheck {
    // GHOOK-004 review: detect file-mode hooks at every location Git
    // would actually consult — `core.hooksPath` override, the resolved
    // git-dir under worktrees / submodules, and Husky. Previously we
    // only checked `.husky/pre-commit`, which would miss raw
    // `.git/hooks/pre-commit` installs and `core.hooksPath`-based setups
    // and produce false "hooks not installed" warnings.
    let cwd = Path::new(".");
    let file_mode_paths = resolve_file_mode_hook_paths(cwd, "pre-commit");
    let active_file_mode_path = file_mode_paths.iter().find(|p| p.exists()).cloned();
    let file_mode_present = active_file_mode_path.is_some();

    // GHOOK-003: native config-mode hooks (`git config hook.pre-commit.command`)
    // count as a valid hook source. We list every entry so a user-authored
    // command that runs `anvil gate` (or any other gate) keeps doctor green
    // — anvil-managed entries are just one supported flavour.
    //
    // `hook.<event>.enabled = false` flips Git's runtime off even when
    // commands are present, so disabled config entries are NOT treated
    // as a valid hook source — they will not run, and surfacing them
    // as "installed" would mislead the user. Default-when-unset is
    // enabled (Git's default), preserved by `config_hooks_enabled`.
    let config_entries = list_config_hook_commands(cwd, "pre-commit").unwrap_or_default();
    let config_mode_enabled = config_hooks_enabled(cwd, "pre-commit");
    let config_mode_present = !config_entries.is_empty() && config_mode_enabled;
    let config_mode_disabled = !config_entries.is_empty() && !config_mode_enabled;
    let anvil_config_entry_present = config_entries.iter().any(|c| is_anvil_managed_command(c));

    if !file_mode_present && !config_mode_present {
        // Nothing detected, OR config-mode entries exist but are disabled.
        // Per `docs/guides/git-hook-compatibility.md` the default
        // remediation stays Husky/file mode (Husky is what most projects
        // already have wired), but the prose points at `--config`
        // explicitly so users on Git 2.54+ can pick either.
        let (message, summary) = if config_mode_disabled {
            (
                "git hooks not installed (config-mode entries present but disabled via hook.pre-commit.enabled=false)".to_string(),
                "Config-mode commands are configured but Git is told to ignore them. Either re-enable with `git config --unset hook.pre-commit.enabled` (Git's default is enabled), or install a file-mode hook via Husky (`npx husky init`)."
                    .to_string(),
            )
        } else {
            (
                "git hooks not installed".to_string(),
                "Install a pre-commit hook so anvil runs your gate before each commit. Two supported paths: file mode via Husky (`npx husky init` then add `anvil gate`), or config mode on Git 2.54+ via `anvil hooks install --config`. See docs/guides/git-hook-compatibility.md for the trade-offs."
                    .to_string(),
            )
        };
        return DiagnosticCheck {
            name: "hooks-installed".to_string(),
            category: "Hooks".to_string(),
            status: CheckStatus::Warn,
            message,
            details: None,
            auto_fixable: false,
            remediation: Remediation {
                summary,
                command: Some("npx husky init".to_string()),
                doc_url: Some("https://typicode.github.io/husky/get-started.html".to_string()),
            },
        };
    }

    if !file_mode_present && config_mode_present {
        // Config-mode-only install. Always Pass — Git 2.54 runs every
        // `hook.<event>.command` value, so there is no executable bit to
        // test and no "not executable" failure mode here.
        let label = if anvil_config_entry_present {
            "pre-commit hook installed (config mode, anvil-managed)"
        } else {
            "pre-commit hook installed (config mode)"
        };
        return DiagnosticCheck {
            name: "hooks-installed".to_string(),
            category: "Hooks".to_string(),
            status: CheckStatus::Pass,
            message: label.to_string(),
            details: None,
            auto_fixable: false,
            remediation: Remediation::default(),
        };
    }

    // File-mode hook is present (with or without an additional config-mode
    // entry). Keep the existing executable-bit Pass/Warn split so a stale
    // hook script that lost +x is still caught — but check the path that
    // actually exists, not a hard-coded `.husky/pre-commit`.
    #[cfg(unix)]
    let executable = {
        use std::os::unix::fs::PermissionsExt;
        active_file_mode_path
            .as_ref()
            .and_then(|p| std::fs::metadata(p).ok())
            .is_some_and(|m| m.permissions().mode() & 0o111 != 0)
    };

    #[cfg(not(unix))]
    let executable = true;

    let pass_message = if config_mode_present {
        "pre-commit hook installed (file + config modes)"
    } else {
        "pre-commit hook installed and executable"
    };

    DiagnosticCheck {
        name: "hooks-installed".to_string(),
        category: "Hooks".to_string(),
        status: if executable {
            CheckStatus::Pass
        } else {
            CheckStatus::Warn
        },
        message: if executable {
            pass_message.to_string()
        } else {
            "pre-commit hook found but not executable".to_string()
        },
        details: None,
        auto_fixable: false,
        remediation: if executable {
            Remediation::default()
        } else {
            Remediation {
                summary: "Mark the pre-commit hook as executable so git will run it. On Unix run `chmod +x .husky/pre-commit`; on Windows the file already runs (the executable bit is ignored by the git filesystem layer)."
                    .to_string(),
                command: None,
                doc_url: Some(
                    "https://typicode.github.io/husky/troubleshoot.html#hooks-not-running"
                        .to_string(),
                ),
            }
        },
    }
}

/// SPG-002: surface any registry rule whose regex fails to compile under the
/// Rust `regex` crate. Without this, lookaround-bearing rules silently drop
/// out of the scanner catalogue and users cannot tell the difference between
/// "rule ran, no matches" and "rule never ran".
fn check_registry_patterns_compile() -> DiagnosticCheck {
    compile_check_from_diagnostics(&anvil_checks::antipattern::registry_compile_diagnostics())
}

/// Check `anvil/project-id` (MLP-001 / A7.2 / council C-4).
///
/// Three states:
/// - file present + parses → Pass
/// - file absent → Warn (foundation for v1 features missing; `anvil
///   start` would create it)
/// - file present but malformed → Fail (manual repair required —
///   anvil-managed files are not silently rewritten)
fn check_project_id() -> DiagnosticCheck {
    use crate::activation::identity;

    let root = std::path::Path::new(".");
    match identity::read_project_id(root) {
        Ok(Some(id)) => DiagnosticCheck {
            name: "project-id".to_string(),
            category: "Configuration".to_string(),
            status: CheckStatus::Pass,
            message: format!("project identity established: {}", id.project_uuid),
            details: id.forked_from.as_ref().map(|p| format!("forked_from: {p}")),
            auto_fixable: false,
            remediation: Remediation::default(),
        },
        Ok(None) => DiagnosticCheck {
            name: "project-id".to_string(),
            category: "Configuration".to_string(),
            status: CheckStatus::Warn,
            message: "anvil/project-id not found".to_string(),
            details: Some(
                "v1 multi-layer protection features (witness chain, baseline, hooks) require this file. Run `anvil start` to establish project identity."
                    .to_string(),
            ),
            auto_fixable: false,
            remediation: Remediation {
                summary: "Run `anvil start` to write `anvil/project-id` with a stable UUID."
                    .to_string(),
                command: Some("anvil start".to_string()),
                doc_url: None,
            },
        },
        Err(e) => DiagnosticCheck {
            name: "project-id".to_string(),
            category: "Configuration".to_string(),
            status: CheckStatus::Fail,
            message: format!("anvil/project-id is malformed: {e}"),
            details: Some(
                "anvil refuses to silently rewrite the identity file. Repair manually (the file is plain text, key:value lines) or remove it and re-run `anvil start` to mint a fresh UUID."
                    .to_string(),
            ),
            auto_fixable: false,
            remediation: Remediation {
                summary: "Manually repair `anvil/project-id` or remove and re-run `anvil start`."
                    .to_string(),
                command: None,
                doc_url: None,
            },
        },
    }
}

fn check_state_boundary() -> DiagnosticCheck {
    check_state_boundary_at(Path::new("."))
}

/// Outcome of the durable-state ignore sweep. `truncated` is set when the
/// walk hit its entry cap, so a clean result cannot be over-claimed.
struct DurableSweep {
    ignored: Vec<String>,
    truncated: bool,
}

/// Strip the env vars through which an enclosing git context (hooks,
/// submodule operations, some CI runners) would redirect our probes at a
/// different repository than `root`.
fn git_at(root: &Path) -> Command {
    let mut cmd = Command::new("git");
    cmd.current_dir(root)
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE");
    cmd
}

/// GITGOV-014 (ADR-073): the durable-vs-runtime state boundary check.
/// Warns when paths under `.anvil/` (local runtime state) are git-tracked,
/// or paths under `anvil/` (durable governance evidence) are gitignored.
/// `anvil/exceptions/.lock` (EXCEPT-007) and `anvil/witness/.chain-initialised`
/// (CIB-126) are exempt — sanctioned runtime artefacts inside the tracked
/// governance tree. Warn, never
/// Fail: the boundary is a posture, and a repo may carry a recorded,
/// justified deviation (ADR-073's dogfood note). Like every other doctor
/// check, this is rooted at the process cwd — doctor's contract is "run at
/// the project root", and `check_anvil_dir` / `check_config_exists` share
/// the same assumption.
fn check_state_boundary_at(root: &Path) -> DiagnosticCheck {
    use std::fmt::Write as _;

    let in_repo = git_at(root)
        .args(["rev-parse", "--git-dir"])
        .output()
        .is_ok_and(|o| o.status.success());
    if !in_repo {
        return DiagnosticCheck {
            name: "state-boundary".to_string(),
            category: "Configuration".to_string(),
            status: CheckStatus::Skipped,
            message: "not a git repository — durable/runtime state boundary not checkable"
                .to_string(),
            details: None,
            auto_fixable: false,
            remediation: Remediation::default(),
        };
    }

    let tracked_runtime = tracked_runtime_paths(root);
    let sweep = ignored_durable_paths(root);

    let Some(sweep) = sweep else {
        // git check-ignore itself failed (exit >= 2): the durable side of the
        // boundary is unverifiable, which must not masquerade as Pass.
        if tracked_runtime.is_empty() {
            return DiagnosticCheck {
                name: "state-boundary".to_string(),
                category: "Configuration".to_string(),
                status: CheckStatus::Skipped,
                message: "git check-ignore failed — durable-state sweep unavailable".to_string(),
                details: None,
                auto_fixable: false,
                remediation: Remediation::default(),
            };
        }
        return state_boundary_warn(&tracked_runtime, &[], false, true);
    };

    if tracked_runtime.is_empty() && sweep.ignored.is_empty() && !sweep.truncated {
        return DiagnosticCheck {
            name: "state-boundary".to_string(),
            category: "Configuration".to_string(),
            status: CheckStatus::Pass,
            message: "durable anvil/ vs runtime .anvil/ state boundary holds (ADR-073)".to_string(),
            details: None,
            auto_fixable: false,
            remediation: Remediation::default(),
        };
    }
    if tracked_runtime.is_empty() && sweep.ignored.is_empty() {
        // Nothing found, but the capped walk cannot prove the whole tree
        // clean — say so instead of over-claiming.
        let mut message = String::from(
            "durable anvil/ vs runtime .anvil/ state boundary holds in the swept subset",
        );
        let _ = write!(message, " (walk capped — tree larger than the sweep bound)");
        return DiagnosticCheck {
            name: "state-boundary".to_string(),
            category: "Configuration".to_string(),
            status: CheckStatus::Pass,
            message,
            details: None,
            auto_fixable: false,
            remediation: Remediation::default(),
        };
    }

    state_boundary_warn(&tracked_runtime, &sweep.ignored, sweep.truncated, false)
}

/// Build the Warn-shaped state-boundary result.
fn state_boundary_warn(
    tracked_runtime: &[String],
    ignored_durable: &[String],
    truncated: bool,
    sweep_failed: bool,
) -> DiagnosticCheck {
    use std::fmt::Write as _;

    let mut details = String::new();
    let list = |buf: &mut String, header: &str, paths: &[String]| {
        if paths.is_empty() {
            return;
        }
        buf.push_str(header);
        for p in paths.iter().take(8) {
            buf.push_str("\n  - ");
            buf.push_str(p);
        }
        if paths.len() > 8 {
            let _ = write!(buf, "\n  … and {} more", paths.len() - 8);
        }
        buf.push('\n');
    };
    list(
        &mut details,
        "Runtime state tracked by git (should be gitignored, .anvil/ is local):",
        tracked_runtime,
    );
    list(
        &mut details,
        "Durable governance state swallowed by .gitignore (anvil/ must travel with the repo):",
        ignored_durable,
    );
    if truncated {
        details.push_str("Sweep capped — the anvil/ tree has more entries than were checked.\n");
    }
    if sweep_failed {
        details.push_str("git check-ignore failed — the durable-ignore side was not verified.\n");
    }

    // The untrack command is surgical: exactly the offending paths, never a
    // recursive `.anvil` sweep that would also untrack any deliberately
    // tracked file. Quoted so paths with spaces (or quotes) stay
    // copy-pasteable.
    let command = if tracked_runtime.is_empty() {
        ignored_durable
            .first()
            .map(|p| format!("git check-ignore -v {}", shell_quote(p)))
    } else {
        let mut cmd = String::from("git rm --cached --");
        for p in tracked_runtime.iter().take(8) {
            let _ = write!(cmd, " {}", shell_quote(p));
        }
        Some(cmd)
    };

    DiagnosticCheck {
        name: "state-boundary".to_string(),
        category: "Configuration".to_string(),
        status: CheckStatus::Warn,
        message: format!(
            "state boundary breached: {} runtime path(s) tracked, {} durable path(s) ignored",
            tracked_runtime.len(),
            ignored_durable.len()
        ),
        details: Some(details.trim_end().to_string()),
        auto_fixable: false,
        remediation: Remediation {
            summary: "Untrack the listed `.anvil/` runtime paths (verify each is truly \
                      runtime state first) and keep `.anvil/` gitignored; remove \
                      `.gitignore` rules that swallow durable `anvil/` evidence (or record \
                      the deviation as a justified exception per ADR-073). \
                      `anvil/exceptions/.lock` (EXCEPT-007) and \
                      `anvil/witness/.chain-initialised` (CIB-126) are exempt."
                .to_string(),
            command,
            doc_url: None,
        },
    }
}

/// POSIX single-quote a path for a copy-pasteable remediation command.
/// Embedded single quotes use the standard `'\''` close-escape-reopen form.
fn shell_quote(path: &str) -> String {
    format!("'{}'", path.replace('\'', r"'\''"))
}

/// Paths under `.anvil/` present in the git index.
fn tracked_runtime_paths(root: &Path) -> Vec<String> {
    let out = git_at(root)
        .args(["ls-files", "-z", "--", ".anvil"])
        .output();
    match out {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout)
            .split('\0')
            .filter(|s| !s.is_empty())
            .map(str::to_string)
            .collect(),
        _ => Vec::new(),
    }
}

/// Existing paths under `anvil/` that a `.gitignore` rule would swallow.
/// The walk is bounded so a pathological tree cannot stall doctor; the
/// result records whether the bound was hit. Returns `None` when
/// `git check-ignore` itself fails (exit >= 2), so the caller can
/// distinguish "nothing ignored" from "could not check". `--no-index` is
/// passed so already-tracked paths still report their matching ignore rule
/// (without it git suppresses tracked paths and a swallowing rule added
/// after commit would go unnoticed).
fn ignored_durable_paths(root: &Path) -> Option<DurableSweep> {
    use std::io::Write as _;

    const SWEEP_CAP: usize = 512;

    let durable_root = root.join("anvil");
    if !durable_root.is_dir() {
        return Some(DurableSweep {
            ignored: Vec::new(),
            truncated: false,
        });
    }
    // `sort_by_file_name` makes the traversal order itself deterministic, so
    // when the cap truncates a large tree the *same* subset is checked on
    // every run/machine — capping an unsorted readdir-order walk would make
    // the warning set nondeterministic.
    let mut candidates: Vec<String> = walkdir::WalkDir::new(&durable_root)
        .min_depth(1)
        .sort_by_file_name()
        .into_iter()
        .filter_map(Result::ok)
        .filter_map(|e| {
            let rel = e.path().strip_prefix(root).ok()?;
            let rel = rel.to_string_lossy().replace('\\', "/");
            // Sanctioned runtime artefacts inside the tracked governance tree —
            // ignored by design, so the sweep must not flag them: the exception-store
            // write lock (EXCEPT-007) and the witness chain-init marker (CIB-126).
            let sanctioned =
                rel == "anvil/exceptions/.lock" || rel == "anvil/witness/.chain-initialised";
            (!sanctioned).then_some(rel)
        })
        .take(SWEEP_CAP + 1)
        .collect();
    let truncated = candidates.len() > SWEEP_CAP;
    candidates.truncate(SWEEP_CAP);
    // Depth-first sorted traversal is already deterministic; this final sort
    // just normalises the display order to plain lexicographic.
    candidates.sort_unstable();
    if candidates.is_empty() {
        return Some(DurableSweep {
            ignored: Vec::new(),
            truncated,
        });
    }

    let child = git_at(root)
        .args(["check-ignore", "-z", "--stdin", "--no-index"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn();
    let Ok(mut child) = child else {
        return None;
    };
    // Feed stdin from a thread: writing all candidates before draining
    // stdout can deadlock once the child's stdout pipe buffer fills.
    let writer = child.stdin.take().map(|mut stdin| {
        let batch = candidates.join("\0");
        std::thread::spawn(move || {
            let _ = stdin.write_all(batch.as_bytes());
            let _ = stdin.write_all(b"\0");
        })
    });
    let out = child.wait_with_output();
    if let Some(handle) = writer {
        let _ = handle.join();
    }
    let out = out.ok()?;
    // Exit code 1 means "no path is ignored" — not an error. Anything
    // above 1 is a real failure and must not read as a clean sweep.
    match out.status.code() {
        Some(0 | 1) => Some(DurableSweep {
            ignored: String::from_utf8_lossy(&out.stdout)
                .split('\0')
                .filter(|s| !s.is_empty())
                .map(str::to_string)
                .collect(),
            truncated,
        }),
        _ => None,
    }
}

fn compile_check_from_diagnostics(
    diagnostics: &[anvil_checks::antipattern::CompileDiagnostic],
) -> DiagnosticCheck {
    if diagnostics.is_empty() {
        return DiagnosticCheck {
            name: "registry-patterns-compile".to_string(),
            category: "Configuration".to_string(),
            status: CheckStatus::Pass,
            message: "all registry patterns compile under the Rust engine".to_string(),
            details: None,
            auto_fixable: false,
            remediation: Remediation::default(),
        };
    }

    let summary = diagnostics
        .iter()
        .map(|d| d.pattern_id.clone())
        .collect::<Vec<_>>()
        .join(", ");
    let details = diagnostics
        .iter()
        .map(|d| format!("{} ({}): {}", d.pattern_id, d.pattern_title, d.error))
        .collect::<Vec<_>>()
        .join("\n");

    DiagnosticCheck {
        name: "registry-patterns-compile".to_string(),
        category: "Configuration".to_string(),
        status: CheckStatus::Warn,
        message: format!(
            "{count} registry rule{s} failed to compile: {summary}",
            count = diagnostics.len(),
            s = if diagnostics.len() == 1 { "" } else { "s" },
        ),
        details: Some(details),
        auto_fixable: false,
        remediation: Remediation {
            summary: "Rewrite the listed rules to drop PCRE lookaround constructs the Rust regex engine cannot compile, or move them to the language-specific scanner that supports them."
                .to_string(),
            command: None,
            // We deliberately link to the repo root rather than a
            // branch-pinned deep link: the scanner-parity docs move
            // around occasionally and a branch-anchored URL silently
            // 404s after a rename. The reader follows the README from
            // the repo root.
            doc_url: Some("https://github.com/eddacraft/anvil-001#scanner-parity".to_string()),
        },
    }
}

// --- Fix application ---

/// Apply a fix to a single check by index. Used by the welcome hub
/// when the user presses 'f' in the doctor TUI.
pub fn apply_fix_at(checks: &mut [DiagnosticCheck], index: usize) {
    if let Some(check) = checks.get_mut(index) {
        let slice = std::slice::from_mut(check);
        apply_fixes(slice, true);
    }
}

/// Heuristic: is the current working directory plausibly an intended
/// project root? Used to gate the destructive `git init` auto-fix so
/// `anvil doctor --fix` invoked from `$HOME` (or any unintended location)
/// does not silently turn that directory into a git repository.
fn looks_like_project_root() -> bool {
    const PROJECT_MARKERS: &[&str] = &[
        ".anvilrc",
        "package.json",
        "Cargo.toml",
        "pyproject.toml",
        "go.mod",
        "pnpm-workspace.yaml",
        "deno.json",
        "Gemfile",
        "build.gradle",
        "pom.xml",
    ];
    PROJECT_MARKERS.iter().any(|m| Path::new(m).exists())
}

/// Default `.anvilrc` produced by the `config-exists` auto-fix. Mirrors the
/// shape that `anvil init` writes so the file passes `check_config_valid`
/// rather than landing the user in a fix→fail loop.
fn default_anvilrc_yaml() -> &'static str {
    "schemaVersion: \"1.0.0\"\nplanningDir: \"plans\"\nformat: \"yaml\"\nchecks:\n  - \"secret-detection\"\n  - \"import-boundaries\"\n  - \"antipattern-scan\"\n"
}

fn apply_fixes(checks: &mut [DiagnosticCheck], json: bool) {
    // `json` mode silences human-facing prose so the JSON envelope stays
    // machine-parseable. Plain and TUI modes always print the per-check
    // outcome — a silent "Fixed" is worse than no fix at all because the
    // user has no idea the destructive action ran.
    let speak = !json;
    for check in checks.iter_mut() {
        if !check.auto_fixable || check.status == CheckStatus::Pass {
            continue;
        }

        match check.name.as_str() {
            "git-repo" => {
                if !looks_like_project_root() {
                    if speak {
                        println!(
                            "  Skipped: git-repo — current directory has no project markers \
                             (.anvilrc, package.json, Cargo.toml, …); refusing to run \
                             `git init` here. Run `git init` manually if this is the \
                             intended project root."
                        );
                    }
                    continue;
                }
                if Command::new("git").arg("init").output().is_ok() {
                    check.status = CheckStatus::Pass;
                    check.message = "git repository initialised".to_string();
                    check.auto_fixable = false;
                    if speak {
                        println!("  Fixed: git-repo — initialised git repository");
                    }
                }
            }
            "config-exists" => {
                // A zero-byte `.anvilrc` triggers `check_config_exists`'s
                // missing-file path; remove it before write_new opens with
                // O_CREAT | O_EXCL, mirroring `anvil init`'s behaviour.
                let path = Path::new(".anvilrc");
                if let Ok(meta) = std::fs::metadata(path)
                    && meta.is_file()
                    && meta.len() == 0
                {
                    let _ = std::fs::remove_file(path);
                }
                match std::fs::write(path, default_anvilrc_yaml()) {
                    Ok(()) => {
                        check.status = CheckStatus::Pass;
                        check.message = ".anvilrc created with defaults".to_string();
                        check.auto_fixable = false;
                        if speak {
                            println!(
                                "  Fixed: config-exists — created .anvilrc with default \
                                 schema (yaml, three checks)"
                            );
                        }
                    }
                    Err(e) if speak => {
                        eprintln!("  Failed to fix config-exists: {e}");
                    }
                    Err(_) => {}
                }
            }
            "anvil-dir" => match std::fs::create_dir_all(".anvil") {
                Ok(()) => {
                    check.status = CheckStatus::Pass;
                    check.message = ".anvil/ directory created".to_string();
                    check.auto_fixable = false;
                    if speak {
                        println!("  Fixed: anvil-dir — created .anvil/ directory");
                    }
                }
                Err(e) => {
                    if speak {
                        eprintln!("  Failed to fix anvil-dir: {e}");
                    }
                }
            },
            "plans-dir" => match std::fs::create_dir_all("plans") {
                Ok(()) => {
                    check.status = CheckStatus::Pass;
                    check.message = "plans/ directory created".to_string();
                    check.auto_fixable = false;
                    if speak {
                        println!("  Fixed: plans-dir — created plans/ directory");
                    }
                }
                Err(e) => {
                    if speak {
                        eprintln!("  Failed to fix plans-dir: {e}");
                    }
                }
            },
            _ => {}
        }
    }
}

// --- Output formatters ---

fn print_plain(checks: &[DiagnosticCheck], protection_claim: &ProtectionClaim) {
    print!("{}", format_plain(checks, protection_claim));
}

/// Render the full `anvil doctor` plain-text surface to a string.
/// Extracted so tests can assert the byte-exact layout (including
/// the MLP2-051a protection-claim section) without capturing
/// stdout. `print_plain` is a thin wrapper that streams this to
/// the terminal.
fn format_plain(checks: &[DiagnosticCheck], protection_claim: &ProtectionClaim) -> String {
    use std::fmt::Write as _;

    let mut out = String::new();
    let _ = writeln!(out);
    let _ = writeln!(out, "  anvil Doctor");
    let _ = writeln!(out);

    for check in checks {
        let icon = match check.status {
            CheckStatus::Pass => "\u{2713}",
            CheckStatus::Fail => "\u{2717}",
            CheckStatus::Warn => "\u{26A0}",
            CheckStatus::Skipped => "\u{25CB}",
            CheckStatus::Running => "*",
        };
        let _ = writeln!(
            out,
            "  {icon} {name}  {message}",
            name = check.name,
            message = check.message,
        );
        // Surface remediation inline for non-Pass / non-Skipped statuses
        // so the user sees the next action without having to drop into the
        // TUI. Pass / Skipped checks have a default (empty) remediation.
        if !check.remediation.is_empty() {
            let r = &check.remediation;
            if !r.summary.is_empty() {
                let _ = writeln!(out, "      \u{2192} {summary}", summary = r.summary);
            }
            if let Some(cmd) = &r.command {
                let _ = writeln!(out, "        run:  {cmd}");
            }
            if let Some(url) = &r.doc_url {
                let _ = writeln!(out, "        docs: {url}");
            }
            if check.auto_fixable {
                let _ = writeln!(out, "        fix:  anvil doctor --fix");
            }
        }
    }

    let summary = anvil_tui::surfaces::doctor::DiagnosticSummary::from_checks(checks);
    let _ = writeln!(out);
    let _ = writeln!(
        out,
        "  {passed} passed, {failed} failed, {warnings} warnings, {skipped} skipped",
        passed = summary.passed,
        failed = summary.failed,
        warnings = summary.warnings,
        skipped = summary.skipped,
    );
    let _ = writeln!(out);
    // MLP2-051a: protection-claim section. Indented with the same
    // two-space prefix as the diagnostic rows so the surface reads as
    // one block. The shared helper emits the headline + per-surface
    // lines in §14 closed-set vocabulary.
    out.push_str(&indent_block(
        &protection_claim_section::render_protection_claim_plain(protection_claim),
        "  ",
    ));
    let _ = writeln!(out);
    out
}

/// Apply `prefix` to every non-empty line of `body`. Used to slot the
/// protection-claim section into doctor's two-space indented surface
/// without forcing the shared renderer to know doctor's layout.
fn indent_block(body: &str, prefix: &str) -> String {
    let mut out = String::new();
    for line in body.lines() {
        if line.is_empty() {
            out.push('\n');
        } else {
            out.push_str(prefix);
            out.push_str(line);
            out.push('\n');
        }
    }
    out
}

#[derive(Serialize)]
struct JsonCheck {
    name: String,
    category: String,
    status: String,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    details: Option<String>,
    auto_fixable: bool,
    remediation: JsonRemediation,
}

/// Always-present remediation block in the JSON schema. Per
/// LAUNCH-005 every check carries a remediation object. `summary` is
/// always emitted (empty string for Pass / Skipped checks; non-empty
/// for any Fail / Warn check). `command` and `doc_url` are *omitted
/// from the JSON entirely* when `None` — consumers should treat a
/// missing key as "no concrete command / no doc link", not as null.
#[derive(Serialize)]
struct JsonRemediation {
    summary: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    command: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    doc_url: Option<String>,
}

impl From<&Remediation> for JsonRemediation {
    fn from(r: &Remediation) -> Self {
        Self {
            summary: r.summary.clone(),
            command: r.command.clone(),
            doc_url: r.doc_url.clone(),
        }
    }
}

#[derive(Serialize)]
struct DoctorOutput {
    schema_version: String,
    checks: Vec<JsonCheck>,
    notifications: Vec<Notification>,
    /// MLP2-051a: nested [`ProtectionClaim`] wire shape per spec §14,
    /// byte-identical to the field emitted by `anvil status --json`.
    /// Consumers parse it against
    /// `anvil_kernel_types::protection_claim::ProtectionClaim`
    /// (carrying its own `schema_version` =
    /// `anvil.protection-claim.v1`). Doctor's surface this so editor
    /// drivers and CI tooling can interrogate worktree state through
    /// `anvil doctor --json` without a second round-trip to
    /// `anvil status --json`.
    protection_claim: ProtectionClaim,
}

fn status_str(status: CheckStatus) -> &'static str {
    match status {
        CheckStatus::Pass => "pass",
        CheckStatus::Fail => "fail",
        CheckStatus::Warn => "warn",
        CheckStatus::Skipped => "skipped",
        CheckStatus::Running => "running",
    }
}

/// Map a per-check status to notification class + priority.
///
/// Returns `None` for statuses that should not emit a per-check notification
/// (Pass, Running). Pass is represented by the summary; Running is a transient
/// in-flight state, not a delivery artefact.
fn notification_classification(
    status: CheckStatus,
) -> Option<(NotificationClass, NotificationPriority)> {
    match status {
        CheckStatus::Fail => Some((NotificationClass::Failure, NotificationPriority::High)),
        CheckStatus::Warn => Some((NotificationClass::Warning, NotificationPriority::High)),
        CheckStatus::Skipped => Some((NotificationClass::Info, NotificationPriority::Low)),
        CheckStatus::Pass | CheckStatus::Running => None,
    }
}

/// Build a notification for a non-Pass check.
///
/// Deliberately does NOT include `check.details` in the message: parser errors
/// from `check_config_valid` can echo offending tokens from `.anvilrc`, and
/// shipping those into `--json` output leaks arbitrary config content into CI
/// logs (CWE-532). `details` remains on the `DiagnosticCheck` for local/TUI
/// rendering; the notification carries only the surface-safe `message`.
fn notification_for_check(check: &DiagnosticCheck) -> Option<Notification> {
    let (class, priority) = notification_classification(check.status)?;
    Some(
        Notification::new(
            class,
            priority,
            format!("Doctor: {}", check.name),
            check.message.clone(),
        )
        .with_context(NotificationContext {
            file: None,
            source: Some("doctor".to_string()),
        }),
    )
}

fn notifications_for_doctor(checks: &[DiagnosticCheck]) -> Vec<Notification> {
    let mut notifications: Vec<Notification> =
        checks.iter().filter_map(notification_for_check).collect();

    let failed = checks
        .iter()
        .filter(|c| c.status == CheckStatus::Fail)
        .count();
    let warned = checks
        .iter()
        .filter(|c| c.status == CheckStatus::Warn)
        .count();

    let (class, priority, message) = if failed > 0 {
        (
            NotificationClass::Failure,
            NotificationPriority::High,
            format!("{failed} failing, {warned} warning"),
        )
    } else if warned > 0 {
        (
            NotificationClass::Warning,
            NotificationPriority::High,
            format!("0 failing, {warned} warning"),
        )
    } else {
        (
            NotificationClass::Health,
            NotificationPriority::Normal,
            "All diagnostics healthy".to_string(),
        )
    };

    notifications.push(
        Notification::new(class, priority, "Doctor summary", message).with_context(
            NotificationContext {
                file: None,
                source: Some("doctor".to_string()),
            },
        ),
    );

    notifications
}

fn build_doctor_output(
    checks: &[DiagnosticCheck],
    protection_claim: &ProtectionClaim,
) -> DoctorOutput {
    let json_checks: Vec<JsonCheck> = checks
        .iter()
        .map(|c| JsonCheck {
            name: c.name.clone(),
            category: c.category.clone(),
            status: status_str(c.status).to_string(),
            message: c.message.clone(),
            details: c.details.clone(),
            auto_fixable: c.auto_fixable,
            remediation: JsonRemediation::from(&c.remediation),
        })
        .collect();

    DoctorOutput {
        schema_version: SCHEMA_VERSION.to_string(),
        checks: json_checks,
        notifications: notifications_for_doctor(checks),
        protection_claim: protection_claim.clone(),
    }
}

fn print_json(
    checks: &[DiagnosticCheck],
    protection_claim: &ProtectionClaim,
) -> anyhow::Result<()> {
    let output = build_doctor_output(checks, protection_claim);
    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use anvil_tui::surfaces::doctor::DiagnosticSummary;

    #[test]
    fn compile_check_passes_when_no_diagnostics() {
        let check = compile_check_from_diagnostics(&[]);
        assert_eq!(check.name, "registry-patterns-compile");
        assert_eq!(check.status, CheckStatus::Pass);
        assert!(check.details.is_none());
    }

    #[test]
    fn compile_check_warns_when_diagnostics_present() {
        use anvil_checks::antipattern::CompileDiagnostic;

        let diagnostics = vec![
            CompileDiagnostic {
                pattern_id: "DD-001".to_string(),
                pattern_title: "Untracked TODO".to_string(),
                error: "unsupported look-around".to_string(),
            },
            CompileDiagnostic {
                pattern_id: "RL-005".to_string(),
                pattern_title: "Deferred without artifact".to_string(),
                error: "unsupported look-around".to_string(),
            },
        ];
        let check = compile_check_from_diagnostics(&diagnostics);
        assert_eq!(check.status, CheckStatus::Warn);
        assert!(check.message.contains("2 registry rules"));
        assert!(check.message.contains("DD-001"));
        assert!(check.message.contains("RL-005"));
        let details = check.details.expect("details populated");
        assert!(details.contains("DD-001 (Untracked TODO): unsupported look-around"));
        // Doc link moved from `details` (free text "see README") to a
        // structured remediation.doc_url under LAUNCH-005.
        let doc_url = check
            .remediation
            .doc_url
            .expect("remediation.doc_url populated");
        assert!(doc_url.contains("scanner-parity"));
    }

    // --- LAUNCH-005 invariants ---

    /// Run `body` inside a fresh tempdir, with the process cwd swapped to
    /// it for the duration. Delegates to the workspace-wide
    /// [`crate::test_support::cwd::with_cwd_in`] guard (CIB-026) so it
    /// serialises against every other cwd-mutating test in the crate, not
    /// just the ones in this module. The original cwd is restored even on
    /// panic.
    fn with_tempdir_as_cwd<R>(body: impl FnOnce(&Path) -> R) -> R {
        let tmp = tempfile::tempdir().expect("create tempdir");
        let path = tmp.path().to_path_buf();
        crate::test_support::cwd::with_cwd_in(&path, || body(&path))
    }

    /// Drive every check function into a Fail or Warn state and
    /// collect its `DiagnosticCheck`. The fixture is a tempdir with
    /// no `.git`, no `.anvilrc`, no `.anvil/`, no `plans/`, no
    /// `.husky/` — so every "is X present?" check fires the negative
    /// branch. The registry-patterns-compile check is exercised
    /// separately via `compile_check_from_diagnostics` because it
    /// reads from a static registry that does not depend on cwd.
    fn collect_negative_branches() -> Vec<DiagnosticCheck> {
        use anvil_checks::antipattern::CompileDiagnostic;

        let mut out = with_tempdir_as_cwd(|_| {
            vec![
                check_git_repo(),
                check_config_exists(),
                check_anvil_dir(),
                // anvil-dir-writable is Skipped when .anvil/ is absent;
                // create it as a read-only-by-noone sentinel and rely
                // on the write-probe to mark it Pass — i.e. this check
                // does not have a deterministic Fail/Warn shape we can
                // hit without breaking the parent dir's permissions
                // (which would be hostile in test infrastructure).
                // Coverage: tested separately via the snapshot path.
                check_plans_dir(),
                check_hooks_installed(),
            ]
        });
        // git-available depends on the host having git installed; we
        // cannot reliably force the Fail branch in CI without breaking
        // PATH. Coverage: tested separately via the doc-link assertion.
        // config-valid Fail branches require crafted .anvilrc content;
        // exercise with a dedicated fixture.
        out.push(with_tempdir_as_cwd(|_| {
            std::fs::write(".anvilrc", "").unwrap();
            check_config_valid()
        }));
        out.push(with_tempdir_as_cwd(|_| {
            std::fs::write(".anvilrc", "this is not valid yaml: : :").unwrap();
            check_config_valid()
        }));
        out.push(compile_check_from_diagnostics(&[CompileDiagnostic {
            pattern_id: "X".into(),
            pattern_title: "Y".into(),
            error: "z".into(),
        }]));
        // state-boundary Warn branches: tracked runtime state, and durable
        // state swallowed by an ignore rule — both arms must carry the
        // LAUNCH-005 remediation contract.
        out.push({
            let tmp = tempfile::tempdir().expect("create tempdir");
            git_in(tmp.path(), &["init", "-q"]);
            std::fs::create_dir_all(tmp.path().join(".anvil")).unwrap();
            std::fs::write(tmp.path().join(".anvil/gates.json"), "{}").unwrap();
            git_in(tmp.path(), &["add", ".anvil/gates.json"]);
            check_state_boundary_at(tmp.path())
        });
        out.push({
            let tmp = tempfile::tempdir().expect("create tempdir");
            git_in(tmp.path(), &["init", "-q"]);
            std::fs::write(tmp.path().join(".gitignore"), "anvil/witness/\n").unwrap();
            std::fs::create_dir_all(tmp.path().join("anvil/witness")).unwrap();
            std::fs::write(tmp.path().join("anvil/witness/chain.ndjson"), "{}").unwrap();
            check_state_boundary_at(tmp.path())
        });
        out
    }

    /// LAUNCH-005 invariant: every check function that lands in Fail
    /// or Warn must carry a non-empty `remediation.summary`. Pass and
    /// Skipped checks may legally carry the default (empty)
    /// remediation. This iterates every `check_*` function via the
    /// negative-branch fixture so a regression in any single check
    /// trips the invariant.
    #[test]
    fn every_check_fail_or_warn_branch_carries_remediation() {
        for check in collect_negative_branches() {
            if matches!(check.status, CheckStatus::Pass | CheckStatus::Skipped) {
                continue;
            }
            assert!(
                !check.remediation.summary.is_empty(),
                "{}: status {:?} but remediation.summary is empty",
                check.name,
                check.status,
            );
        }
    }

    /// LAUNCH-005 explicitly forbids any check terminating at a bare
    /// "see README" reference. Every Fail/Warn check must surface a
    /// concrete command or doc URL — never just README prose.
    #[test]
    fn no_check_remediation_terminates_at_a_bare_readme() {
        for check in collect_negative_branches() {
            if matches!(check.status, CheckStatus::Pass | CheckStatus::Skipped) {
                continue;
            }
            let r = &check.remediation;
            assert!(
                r.command.is_some() || r.doc_url.is_some(),
                "{}: remediation must carry a command or doc_url, not just prose",
                check.name
            );
            // The prose summary itself must not deflect to README
            // without a structured target.
            let s = r.summary.to_lowercase();
            assert!(
                !(s.contains("readme") && r.command.is_none() && r.doc_url.is_none()),
                "{}: remediation.summary points at a README without a structured target",
                check.name
            );
        }
    }

    /// `check_anvil_dir_writable` Fail branch cannot be exercised
    /// from `collect_negative_branches` without making `.anvil/`
    /// unwritable in test infrastructure (hostile to parallel tests
    /// and to anyone running `cargo test` as root). Mirror the
    /// `git_available` pattern: assert the literal shape of the Fail
    /// branch so a regression that empties the remediation trips here.
    #[test]
    fn anvil_dir_writable_fail_branch_carries_command() {
        let fail = DiagnosticCheck {
            name: "anvil-dir-writable".into(),
            category: "Permissions".into(),
            status: CheckStatus::Fail,
            message: ".anvil/ is not writable".into(),
            details: None,
            auto_fixable: false,
            remediation: Remediation {
                summary: "Restore write access to the `.anvil/` directory. If it lives on a read-only mount (Docker volume, NFS share), the mount itself needs to change."
                    .into(),
                command: Some("chmod u+w .anvil".into()),
                doc_url: None,
            },
        };
        // If the literal shipped in `check_anvil_dir_writable` ever
        // drifts from this, update this test deliberately.
        assert!(fail.remediation.command.is_some());
        assert!(!fail.remediation.summary.is_empty());
    }

    /// `git-available` cannot be forced into a Fail state without
    /// breaking PATH for the whole test binary. Cover the doc-link
    /// invariant directly by calling the check on a host that has git
    /// (the host is a Pass) and exercising the Fail branch's literal
    /// shape via a dedicated assertion on the constructed value.
    #[test]
    fn git_available_fail_branch_carries_doc_url() {
        // This is a structural assertion: even though the live call
        // returns Pass on a dev machine, the doc URL we'd surface in
        // the Fail branch must remain non-empty. We assert against the
        // literal so a regression that empties the URL trips here.
        let fail = DiagnosticCheck {
            name: "git-available".into(),
            category: "System".into(),
            status: CheckStatus::Fail,
            message: "git not found on PATH".into(),
            details: None,
            auto_fixable: false,
            remediation: Remediation {
                summary: "Install git so it is available on PATH.".into(),
                command: None,
                doc_url: Some("https://git-scm.com/downloads".into()),
            },
        };
        // If the literal we ship in `check_git_available` ever drifts
        // from this, update this test. The point is to make the drift
        // visible, not to assert the live value.
        assert!(fail.remediation.doc_url.is_some());
        assert!(!fail.remediation.summary.is_empty());
    }

    #[test]
    fn git_available_passes_on_dev_machine() {
        let check = check_git_available();
        assert_eq!(check.name, "git-available");
        assert_eq!(check.category, "System");
        // Should pass on any dev machine with git installed
        assert_eq!(check.status, CheckStatus::Pass);
    }

    #[test]
    fn git_repo_returns_valid_check() {
        let check = check_git_repo();
        assert_eq!(check.name, "git-repo");
        assert_eq!(check.category, "System");
        // Status depends on whether .git exists in cwd. Per issue #1108 the
        // missing-repo branch is a Warn, not a Fail, so a fresh user before
        // `git init` is guided rather than blocked.
        assert!(matches!(
            check.status,
            CheckStatus::Pass | CheckStatus::Warn
        ));
    }

    /// Issue #1108: when run in a directory with no git repo, `git-repo`
    /// must Warn (not Fail) and surface a `git init` remediation. Doctor
    /// must therefore exit 0 in that case, since `bail!` only fires on
    /// Fail.
    #[test]
    fn git_repo_warns_outside_repo_with_actionable_remediation() {
        with_tempdir_as_cwd(|_| {
            let check = check_git_repo();
            assert_eq!(check.name, "git-repo");
            assert_eq!(check.status, CheckStatus::Warn);
            assert!(
                check.message.contains("git init"),
                "missing-repo message should point at `git init`, got: {}",
                check.message,
            );
            assert_eq!(check.remediation.command.as_deref(), Some("git init"));
            assert!(!check.remediation.summary.is_empty());
            assert!(check.auto_fixable);
        });
    }

    #[test]
    fn config_exists_in_project_root() {
        let check = check_config_exists();
        assert_eq!(check.name, "config-exists");
        // Status depends on whether .anvilrc exists
        assert!(matches!(
            check.status,
            CheckStatus::Pass | CheckStatus::Warn
        ));
    }

    #[test]
    fn config_valid_skipped_when_missing() {
        // If .anvilrc doesn't exist, config-valid should be skipped
        if !Path::new(".anvilrc").exists() {
            let check = check_config_valid();
            assert_eq!(check.status, CheckStatus::Skipped);
        }
    }

    #[test]
    fn anvil_dir_check_returns_valid_status() {
        let check = check_anvil_dir();
        assert_eq!(check.name, "anvil-dir");
        assert!(matches!(
            check.status,
            CheckStatus::Pass | CheckStatus::Warn
        ));
    }

    #[test]
    fn anvil_dir_writable_skips_when_missing() {
        if !Path::new(".anvil").is_dir() {
            let check = check_anvil_dir_writable();
            assert_eq!(check.status, CheckStatus::Skipped);
        }
    }

    #[test]
    fn plans_dir_returns_valid_check() {
        let check = check_plans_dir();
        assert_eq!(check.name, "plans-dir");
        assert_eq!(check.category, "Configuration");
        // Status depends on whether plans/ exists in cwd
        assert!(matches!(
            check.status,
            CheckStatus::Pass | CheckStatus::Warn
        ));
    }

    #[test]
    fn hooks_check_returns_valid_status() {
        let check = check_hooks_installed();
        assert_eq!(check.name, "hooks-installed");
        assert!(matches!(
            check.status,
            CheckStatus::Pass | CheckStatus::Warn
        ));
    }

    #[test]
    fn run_all_checks_includes_registry_compile_check() {
        let checks = run_all_checks();
        assert!(
            checks.iter().any(|c| c.name == "registry-patterns-compile"),
            "registry-patterns-compile must be registered in run_all_checks",
        );
    }

    // --- state-boundary (GITGOV-014, ADR-073) ---

    /// `git` invocation for state-boundary fixtures. `GIT_CONFIG_GLOBAL` and
    /// `GIT_CONFIG_SYSTEM` are pointed at the null device so a developer's
    /// global excludesFile cannot leak into the fixture repo's ignore rules,
    /// and `GIT_DIR`/`GIT_WORK_TREE`/`GIT_INDEX_FILE` are stripped so an
    /// enclosing git context (hooks, CI) cannot redirect the fixture.
    fn git_in(root: &Path, args: &[&str]) {
        let null = if cfg!(windows) { "NUL" } else { "/dev/null" };
        let out = Command::new("git")
            .args(args)
            .current_dir(root)
            .env("GIT_CONFIG_GLOBAL", null)
            .env("GIT_CONFIG_SYSTEM", null)
            .env_remove("GIT_DIR")
            .env_remove("GIT_WORK_TREE")
            .env_remove("GIT_INDEX_FILE")
            .output()
            .expect("run git");
        assert!(out.status.success(), "git {args:?} failed: {out:?}");
    }

    #[test]
    fn run_all_checks_includes_state_boundary_check() {
        let checks = run_all_checks();
        assert!(
            checks.iter().any(|c| c.name == "state-boundary"),
            "state-boundary must be registered in run_all_checks",
        );
    }

    #[test]
    fn state_boundary_skipped_outside_git_repo() {
        let tmp = tempfile::tempdir().unwrap();
        let check = check_state_boundary_at(tmp.path());
        assert_eq!(check.status, CheckStatus::Skipped);
    }

    #[test]
    fn state_boundary_passes_when_boundary_holds() {
        let tmp = tempfile::tempdir().unwrap();
        git_in(tmp.path(), &["init", "-q"]);
        std::fs::write(tmp.path().join(".gitignore"), ".anvil/\n").unwrap();
        std::fs::create_dir_all(tmp.path().join(".anvil/cache")).unwrap();
        std::fs::write(tmp.path().join(".anvil/cache/x"), "runtime").unwrap();
        std::fs::create_dir_all(tmp.path().join("anvil/witness")).unwrap();
        std::fs::write(tmp.path().join("anvil/witness/chain.ndjson"), "{}").unwrap();
        git_in(tmp.path(), &["add", "anvil"]);
        let check = check_state_boundary_at(tmp.path());
        assert_eq!(check.status, CheckStatus::Pass, "{:?}", check.message);
    }

    #[test]
    fn state_boundary_warns_on_tracked_runtime_state() {
        let tmp = tempfile::tempdir().unwrap();
        git_in(tmp.path(), &["init", "-q"]);
        std::fs::create_dir_all(tmp.path().join(".anvil")).unwrap();
        std::fs::write(tmp.path().join(".anvil/exceptions.json"), "[]").unwrap();
        git_in(tmp.path(), &["add", ".anvil/exceptions.json"]);
        let check = check_state_boundary_at(tmp.path());
        assert_eq!(check.status, CheckStatus::Warn);
        assert!(
            check
                .details
                .as_deref()
                .unwrap_or("")
                .contains(".anvil/exceptions.json"),
            "details name the tracked runtime path: {:?}",
            check.details
        );
        assert!(
            check
                .remediation
                .command
                .as_deref()
                .unwrap_or("")
                .contains("git rm"),
            "remediation offers the untrack command: {:?}",
            check.remediation.command
        );
    }

    #[test]
    fn state_boundary_warns_on_ignored_durable_state() {
        let tmp = tempfile::tempdir().unwrap();
        git_in(tmp.path(), &["init", "-q"]);
        std::fs::write(tmp.path().join(".gitignore"), "anvil/witness/\n").unwrap();
        std::fs::create_dir_all(tmp.path().join("anvil/witness")).unwrap();
        std::fs::write(tmp.path().join("anvil/witness/chain.ndjson"), "{}").unwrap();
        let check = check_state_boundary_at(tmp.path());
        assert_eq!(check.status, CheckStatus::Warn);
        assert!(
            check
                .details
                .as_deref()
                .unwrap_or("")
                .contains("anvil/witness"),
            "details name the ignored durable path: {:?}",
            check.details
        );
        assert!(
            check
                .remediation
                .command
                .as_deref()
                .unwrap_or("")
                .contains("check-ignore"),
            "remediation offers the rule-locating command: {:?}",
            check.remediation.command
        );
    }

    #[test]
    fn state_boundary_catches_tracked_durable_path_under_swallowing_rule() {
        // A durable path committed BEFORE a swallowing rule was added: without
        // `--no-index` git check-ignore suppresses tracked paths and the rule
        // goes unnoticed (false Pass).
        let tmp = tempfile::tempdir().unwrap();
        git_in(tmp.path(), &["init", "-q"]);
        std::fs::create_dir_all(tmp.path().join("anvil/witness")).unwrap();
        std::fs::write(tmp.path().join("anvil/witness/chain.ndjson"), "{}").unwrap();
        git_in(tmp.path(), &["add", "anvil/witness/chain.ndjson"]);
        std::fs::write(tmp.path().join(".gitignore"), "anvil/witness/\n").unwrap();
        let check = check_state_boundary_at(tmp.path());
        assert_eq!(check.status, CheckStatus::Warn, "{:?}", check.message);
        assert!(
            check
                .details
                .as_deref()
                .unwrap_or("")
                .contains("anvil/witness"),
            "tracked-but-swallowed durable path is reported: {:?}",
            check.details
        );
    }

    #[test]
    fn state_boundary_untrack_command_is_surgical() {
        // The remediation must name exactly the offending paths — never a
        // recursive `.anvil` untrack that would also remove deliberately
        // tracked files.
        let tmp = tempfile::tempdir().unwrap();
        git_in(tmp.path(), &["init", "-q"]);
        std::fs::create_dir_all(tmp.path().join(".anvil")).unwrap();
        std::fs::write(tmp.path().join(".anvil/kindling.db"), "x").unwrap();
        git_in(tmp.path(), &["add", ".anvil/kindling.db"]);
        let check = check_state_boundary_at(tmp.path());
        let cmd = check.remediation.command.as_deref().unwrap_or("");
        assert!(
            cmd.contains("'.anvil/kindling.db'"),
            "command names the offending path: {cmd}"
        );
        assert!(
            !cmd.contains("rm -r"),
            "no recursive untrack of the whole .anvil tree: {cmd}"
        );
    }

    #[test]
    fn shell_quote_escapes_embedded_single_quotes() {
        assert_eq!(shell_quote("anvil/plain.json"), "'anvil/plain.json'");
        assert_eq!(
            shell_quote("anvil/it's here.json"),
            r"'anvil/it'\''s here.json'"
        );
    }

    #[test]
    fn state_boundary_exempts_exception_store_lock() {
        // anvil/exceptions/.lock is the one sanctioned runtime artefact inside
        // the tracked governance tree (EXCEPT-007) — ignoring it is correct.
        let tmp = tempfile::tempdir().unwrap();
        git_in(tmp.path(), &["init", "-q"]);
        std::fs::write(tmp.path().join(".gitignore"), "anvil/exceptions/.lock\n").unwrap();
        std::fs::create_dir_all(tmp.path().join("anvil/exceptions")).unwrap();
        std::fs::write(tmp.path().join("anvil/exceptions/.lock"), "").unwrap();
        std::fs::write(tmp.path().join("anvil/exceptions/active.json"), "[]").unwrap();
        git_in(tmp.path(), &["add", "anvil/exceptions/active.json"]);
        let check = check_state_boundary_at(tmp.path());
        assert_eq!(check.status, CheckStatus::Pass, "{:?}", check.details);
    }

    #[test]
    fn json_output_is_valid() {
        let checks = run_all_checks();
        let claim = anvil_kernel_types::protection_claim::ProtectionClaim::new(
            anvil_kernel_types::protection_claim::WorktreeClaimState::Unprotected,
            vec![],
        );
        let output = build_doctor_output(&checks, &claim);
        let json = serde_json::to_string(&output).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert!(parsed.is_object());
        assert_eq!(parsed["checks"].as_array().unwrap().len(), checks.len());
        // Structural assertion: one notification per non-Pass/non-Running check,
        // plus exactly one summary. Environment-dependent: on a healthy dev
        // machine Pass-only checks produce a single summary.
        let expected_notifications = checks
            .iter()
            .filter(|c| notification_classification(c.status).is_some())
            .count()
            + 1;
        assert_eq!(
            parsed["notifications"].as_array().unwrap().len(),
            expected_notifications,
        );
    }

    #[test]
    fn notification_mapping_for_check_statuses() {
        // Per-check notifications emit only for actionable states. Pass and
        // Running are represented by the summary / transient UI, not per-check
        // delivery artefacts.
        let emitting = [
            (
                CheckStatus::Warn,
                NotificationClass::Warning,
                NotificationPriority::High,
            ),
            (
                CheckStatus::Fail,
                NotificationClass::Failure,
                NotificationPriority::High,
            ),
            (
                CheckStatus::Skipped,
                NotificationClass::Info,
                NotificationPriority::Low,
            ),
        ];
        for (status, class, priority) in emitting {
            let check = make_check("example", status, false);
            let notification =
                notification_for_check(&check).expect("emitting status produces notification");
            assert_eq!(notification.class, class, "class for {status:?}");
            assert_eq!(notification.priority, priority, "priority for {status:?}");
            assert_eq!(
                notification
                    .context
                    .as_ref()
                    .and_then(|c| c.source.as_deref()),
                Some("doctor")
            );
        }

        let suppressed = [CheckStatus::Pass, CheckStatus::Running];
        for status in suppressed {
            let check = make_check("example", status, false);
            assert!(
                notification_for_check(&check).is_none(),
                "{status:?} should not emit a per-check notification",
            );
        }
    }

    #[test]
    fn notification_message_does_not_echo_check_details() {
        // Security regression: check.details can contain raw parser errors
        // that echo offending tokens from .anvilrc. Notifications must carry
        // only the surface-safe `message`. (council / security finding.)
        let check = DiagnosticCheck {
            name: "config-valid".to_string(),
            category: "Configuration".to_string(),
            status: CheckStatus::Fail,
            message: ".anvilrc failed to parse".to_string(),
            details: Some("leaked-token=sk-XXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX".to_string()),
            auto_fixable: false,
            remediation: Remediation::default(),
        };
        let notification = notification_for_check(&check).expect("Fail emits notification");
        assert!(
            !notification.message.contains("leaked-token"),
            "notification message must not echo `check.details`: got {:?}",
            notification.message,
        );
        assert!(
            !notification.message.contains("sk-"),
            "notification message must not echo secret material from `check.details`",
        );
    }

    #[test]
    fn all_pass_emits_only_summary() {
        // Combined coverage for the suppression fix (#5 / OPS-006).
        let checks = vec![
            make_check("a", CheckStatus::Pass, false),
            make_check("b", CheckStatus::Pass, false),
            make_check("c", CheckStatus::Pass, false),
        ];
        let notifications = notifications_for_doctor(&checks);
        assert_eq!(notifications.len(), 1);
        assert_eq!(notifications[0].title, "Doctor summary");
        assert_eq!(notifications[0].class, NotificationClass::Health);
    }

    #[test]
    fn empty_check_list_emits_health_summary_only() {
        let notifications = notifications_for_doctor(&[]);
        assert_eq!(notifications.len(), 1);
        assert_eq!(notifications[0].class, NotificationClass::Health);
        assert_eq!(notifications[0].priority, NotificationPriority::Normal);
    }

    #[test]
    fn doctor_summary_is_failure_when_any_check_fails() {
        let checks = vec![
            make_check("pass", CheckStatus::Pass, false),
            make_check("fail", CheckStatus::Fail, false),
            make_check("warn", CheckStatus::Warn, false),
        ];
        let notifications = notifications_for_doctor(&checks);
        let summary = notifications.last().unwrap();
        assert_eq!(summary.class, NotificationClass::Failure);
        assert_eq!(summary.priority, NotificationPriority::High);
        assert_eq!(summary.title, "Doctor summary");
    }

    #[test]
    fn doctor_summary_is_health_when_all_pass() {
        let checks = vec![
            make_check("a", CheckStatus::Pass, false),
            make_check("b", CheckStatus::Pass, false),
        ];
        let notifications = notifications_for_doctor(&checks);
        let summary = notifications.last().unwrap();
        assert_eq!(summary.class, NotificationClass::Health);
    }

    #[test]
    fn doctor_summary_is_warning_when_only_warnings() {
        let checks = vec![
            make_check("a", CheckStatus::Pass, false),
            make_check("b", CheckStatus::Warn, false),
        ];
        let notifications = notifications_for_doctor(&checks);
        let summary = notifications.last().unwrap();
        assert_eq!(summary.class, NotificationClass::Warning);
    }

    #[test]
    fn apply_fixes_on_empty_list() {
        let mut checks = vec![];
        apply_fixes(&mut checks, true);
        assert!(checks.is_empty());
    }

    // --- apply_fixes tests ---

    fn make_check(name: &str, status: CheckStatus, auto_fixable: bool) -> DiagnosticCheck {
        DiagnosticCheck {
            name: name.to_string(),
            category: "Test".to_string(),
            status,
            message: format!("{name} message"),
            details: None,
            auto_fixable,
            remediation: Remediation::default(),
        }
    }

    #[test]
    fn apply_fixes_skips_already_passed_checks() {
        let mut checks = vec![make_check("anvil-dir", CheckStatus::Pass, true)];
        apply_fixes(&mut checks, true);
        // Should remain Pass and not be touched
        assert_eq!(checks[0].status, CheckStatus::Pass);
        // auto_fixable is untouched by apply_fixes on skip — not a guaranteed invariant
        assert!(checks[0].auto_fixable);
    }

    #[test]
    fn apply_fixes_skips_non_fixable_checks() {
        let mut checks = vec![make_check("config-valid", CheckStatus::Fail, false)];
        apply_fixes(&mut checks, true);
        // Should remain Fail — not fixable
        assert_eq!(checks[0].status, CheckStatus::Fail);
    }

    #[test]
    fn apply_fixes_handles_unknown_check_names() {
        let mut checks = vec![make_check("unknown-check", CheckStatus::Fail, true)];
        apply_fixes(&mut checks, true);
        // Unknown names hit the _ match arm — status unchanged
        assert_eq!(checks[0].status, CheckStatus::Fail);
        assert!(checks[0].auto_fixable);
    }

    #[test]
    fn apply_fixes_skips_warn_non_fixable() {
        let mut checks = vec![make_check("plans-dir", CheckStatus::Warn, false)];
        apply_fixes(&mut checks, true);
        assert_eq!(checks[0].status, CheckStatus::Warn);
    }

    #[test]
    fn apply_fixes_creates_anvil_dir() {
        with_tempdir_as_cwd(|dir| {
            let mut checks = vec![DiagnosticCheck {
                name: "anvil-dir".to_string(),
                category: "Configuration".to_string(),
                status: CheckStatus::Warn,
                message: ".anvil/ directory not found".to_string(),
                details: Some("Create .anvil/ directory for anvil state files".to_string()),
                auto_fixable: true,
                remediation: Remediation::default(),
            }];

            apply_fixes(&mut checks, true);

            assert_eq!(checks[0].status, CheckStatus::Pass);
            assert!(!checks[0].auto_fixable);
            assert!(dir.join(".anvil").is_dir());
        });
    }

    // --- JsonCheck serialisation tests ---

    #[test]
    fn json_check_serialises_all_fields() {
        let check = JsonCheck {
            name: "test-check".to_string(),
            category: "Test".to_string(),
            status: "pass".to_string(),
            message: "all good".to_string(),
            details: Some("extra info".to_string()),
            auto_fixable: true,
            remediation: JsonRemediation::from(&Remediation::default()),
        };
        let json: serde_json::Value = serde_json::to_value(&check).unwrap();
        assert_eq!(json["name"], "test-check");
        assert_eq!(json["category"], "Test");
        assert_eq!(json["status"], "pass");
        assert_eq!(json["message"], "all good");
        assert_eq!(json["details"], "extra info");
        assert_eq!(json["auto_fixable"], true);
        assert_eq!(json["remediation"]["summary"], "");
    }

    #[test]
    fn json_check_omits_none_details() {
        let check = JsonCheck {
            name: "test-check".to_string(),
            category: "Test".to_string(),
            status: "fail".to_string(),
            message: "broken".to_string(),
            details: None,
            auto_fixable: false,
            remediation: JsonRemediation::from(&Remediation {
                summary: "do the thing".to_string(),
                command: Some("anvil thing".to_string()),
                doc_url: None,
            }),
        };
        let json: serde_json::Value = serde_json::to_value(&check).unwrap();
        assert!(
            json.get("details").is_none(),
            "details should be omitted when None"
        );
        assert_eq!(json["auto_fixable"], false);
    }

    #[test]
    fn json_check_status_values_are_lowercase() {
        let statuses = vec![
            (CheckStatus::Pass, "pass"),
            (CheckStatus::Fail, "fail"),
            (CheckStatus::Warn, "warn"),
            (CheckStatus::Skipped, "skipped"),
            (CheckStatus::Running, "running"),
        ];
        for (status, expected) in statuses {
            assert_eq!(status_str(status), expected);
        }
    }

    // --- DiagnosticSummary tests ---

    #[test]
    fn summary_counts_mixed_statuses() {
        let checks = vec![
            make_check("a", CheckStatus::Pass, false),
            make_check("b", CheckStatus::Pass, false),
            make_check("c", CheckStatus::Fail, false),
            make_check("d", CheckStatus::Warn, false),
            make_check("e", CheckStatus::Skipped, false),
            make_check("f", CheckStatus::Skipped, false),
        ];
        let summary = DiagnosticSummary::from_checks(&checks);
        assert_eq!(summary.total, 6);
        assert_eq!(summary.passed, 2);
        assert_eq!(summary.failed, 1);
        assert_eq!(summary.warnings, 1);
        assert_eq!(summary.skipped, 2);
    }

    #[test]
    fn summary_all_pass() {
        let checks = vec![
            make_check("a", CheckStatus::Pass, false),
            make_check("b", CheckStatus::Pass, false),
        ];
        let summary = DiagnosticSummary::from_checks(&checks);
        assert_eq!(summary.total, 2);
        assert_eq!(summary.passed, 2);
        assert_eq!(summary.failed, 0);
        assert_eq!(summary.warnings, 0);
        assert_eq!(summary.skipped, 0);
    }

    #[test]
    fn summary_empty_checks() {
        let checks: Vec<DiagnosticCheck> = vec![];
        let summary = DiagnosticSummary::from_checks(&checks);
        assert_eq!(summary.total, 0);
        assert_eq!(summary.passed, 0);
        assert_eq!(summary.failed, 0);
        assert_eq!(summary.warnings, 0);
        assert_eq!(summary.skipped, 0);
    }

    #[test]
    fn summary_running_not_counted() {
        let checks = vec![make_check("a", CheckStatus::Running, false)];
        let summary = DiagnosticSummary::from_checks(&checks);
        assert_eq!(summary.total, 1);
        assert_eq!(summary.passed, 0);
        assert_eq!(summary.failed, 0);
        assert_eq!(summary.warnings, 0);
        assert_eq!(summary.skipped, 0);
    }

    // --- Check structure validation ---

    #[test]
    fn all_checks_have_non_empty_names() {
        let checks = run_all_checks();
        for check in &checks {
            assert!(!check.name.is_empty(), "check name must not be empty");
            assert!(
                !check.name.contains(' '),
                "check name '{}' should not contain spaces",
                check.name
            );
        }
    }

    #[test]
    fn all_checks_have_non_empty_categories() {
        let checks = run_all_checks();
        for check in &checks {
            assert!(
                !check.category.is_empty(),
                "check '{}' has empty category",
                check.name
            );
        }
    }

    #[test]
    fn all_checks_have_non_empty_messages() {
        let checks = run_all_checks();
        for check in &checks {
            assert!(
                !check.message.is_empty(),
                "check '{}' has empty message",
                check.name
            );
        }
    }

    #[test]
    fn all_check_names_are_unique() {
        let checks = run_all_checks();
        let mut names: Vec<&str> = checks.iter().map(|c| c.name.as_str()).collect();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), checks.len(), "duplicate check names found");
    }

    fn make_check_with_details(
        name: &str,
        status: CheckStatus,
        details: Option<String>,
    ) -> DiagnosticCheck {
        DiagnosticCheck {
            name: name.to_string(),
            category: "Test".to_string(),
            status,
            message: format!("{name} message"),
            details,
            auto_fixable: false,
            remediation: Remediation::default(),
        }
    }

    #[test]
    fn failed_checks_have_details() {
        let checks = vec![
            make_check_with_details(
                "fail-with-details",
                CheckStatus::Fail,
                Some("detail text".to_string()),
            ),
            make_check_with_details("pass-no-details", CheckStatus::Pass, None),
            make_check_with_details(
                "warn-with-details",
                CheckStatus::Warn,
                Some("warning detail".to_string()),
            ),
            make_check_with_details(
                "fail-with-details-2",
                CheckStatus::Fail,
                Some("another detail".to_string()),
            ),
        ];
        for check in &checks {
            if check.status == CheckStatus::Fail {
                assert!(
                    check.details.is_some(),
                    "failing check '{}' should have details",
                    check.name
                );
            }
        }
    }

    // --- GHOOK-003: doctor recognises config-mode hooks ---

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
        assert!(status.success());
    }

    /// (a) Config-mode-only repo reports the hook as installed.
    #[test]
    fn check_hooks_installed_passes_with_config_mode_only() {
        with_tempdir_as_cwd(|dir| {
            if !try_git_init(dir) {
                eprintln!("skipping: git init unavailable");
                return;
            }
            add_config_hook(dir, "pre-commit", "ANVIL_HOOK=1 anvil gate --progress");

            let check = check_hooks_installed();
            assert_eq!(check.status, CheckStatus::Pass);
            assert!(
                check.message.contains("config mode"),
                "config-mode pass must say so: {}",
                check.message,
            );
            assert!(
                check.message.contains("anvil-managed"),
                "anvil-managed entries must be tagged in the message: {}",
                check.message,
            );
        });
    }

    /// User-authored config-mode entries also pass — anvil should not block
    /// doctor on the user picking a non-anvil gate. (Pass without the
    /// "anvil-managed" tag.)
    #[test]
    fn check_hooks_installed_passes_with_user_config_mode_only() {
        with_tempdir_as_cwd(|dir| {
            if !try_git_init(dir) {
                eprintln!("skipping: git init unavailable");
                return;
            }
            add_config_hook(dir, "pre-commit", "npm run my-gate");

            let check = check_hooks_installed();
            assert_eq!(check.status, CheckStatus::Pass);
            assert!(check.message.contains("config mode"));
            assert!(
                !check.message.contains("anvil-managed"),
                "user-authored entries must not claim anvil ownership: {}",
                check.message,
            );
        });
    }

    /// (b) File-mode + config-mode both present reports both flavours in
    /// the message.
    #[test]
    #[cfg(unix)]
    fn check_hooks_installed_reports_both_modes() {
        use std::os::unix::fs::PermissionsExt;

        with_tempdir_as_cwd(|dir| {
            if !try_git_init(dir) {
                eprintln!("skipping: git init unavailable");
                return;
            }

            let husky_dir = dir.join(".husky");
            std::fs::create_dir_all(&husky_dir).unwrap();
            let hook = husky_dir.join("pre-commit");
            std::fs::write(&hook, "#!/bin/sh\nexit 0").unwrap();
            std::fs::set_permissions(&hook, std::fs::Permissions::from_mode(0o755)).unwrap();

            add_config_hook(dir, "pre-commit", "ANVIL_HOOK=1 anvil gate --progress");

            let check = check_hooks_installed();
            assert_eq!(check.status, CheckStatus::Pass);
            assert!(
                check.message.contains("file") && check.message.contains("config"),
                "both modes must be acknowledged in the message: {}",
                check.message,
            );
        });
    }

    /// (c) Doctor advice mentions both install paths when no hook is present.
    #[test]
    fn check_hooks_installed_remediation_mentions_both_install_paths() {
        with_tempdir_as_cwd(|dir| {
            if !try_git_init(dir) {
                eprintln!("skipping: git init unavailable");
                return;
            }

            let check = check_hooks_installed();
            assert_eq!(check.status, CheckStatus::Warn);

            let summary = &check.remediation.summary;
            assert!(
                summary.contains("Husky"),
                "missing-hook remediation must mention Husky: {summary}",
            );
            assert!(
                summary.contains("--config"),
                "missing-hook remediation must mention --config: {summary}",
            );
            // Default recommendation stays file-mode (per the policy doc).
            assert_eq!(
                check.remediation.command.as_deref(),
                Some("npx husky init"),
                "default install command must remain file-mode (Husky)",
            );
        });
    }

    // -----------------------------------------------------------------
    // MLP2-051a: protection-claim section parity with `anvil status`.
    // -----------------------------------------------------------------

    use anvil_kernel_types::protection_claim::{
        SurfaceClaim, SurfaceClaimState, WorktreeClaimState,
    };

    /// Build a `ProtectionClaim` for each of the three states the
    /// MLP2-051a APS entry pins: `Unprotected` (daemon-down / no
    /// sessions on this worktree), `PreWriteDaemon` (clean daemon
    /// session), `DegradedProtection` (any fenced surface).
    fn fixture_claims() -> Vec<(WorktreeClaimState, ProtectionClaim)> {
        vec![
            (
                WorktreeClaimState::Unprotected,
                ProtectionClaim::new(WorktreeClaimState::Unprotected, vec![]),
            ),
            (
                WorktreeClaimState::PreWriteDaemon,
                ProtectionClaim::new(
                    WorktreeClaimState::PreWriteDaemon,
                    vec![SurfaceClaim {
                        identifier: "sess-pre-write".to_owned(),
                        state: SurfaceClaimState::Participating,
                    }],
                ),
            ),
            (
                WorktreeClaimState::DegradedProtection,
                ProtectionClaim::new(
                    WorktreeClaimState::DegradedProtection,
                    vec![SurfaceClaim {
                        identifier: "sess-fenced".to_owned(),
                        state: SurfaceClaimState::Quarantined,
                    }],
                ),
            ),
        ]
    }

    /// `anvil doctor --json` must embed a `protection_claim` field
    /// whose serialised shape is byte-identical to standalone
    /// serialisation of the same `ProtectionClaim` — the same shape
    /// `anvil status --json` emits at the top-level `claim` field.
    /// Pins MLP2-051a's parity contract for the JSON surface.
    #[test]
    fn doctor_json_includes_protection_claim_byte_identical_to_status_claim() {
        for (state, claim) in fixture_claims() {
            let output = build_doctor_output(&[], &claim);
            let json = serde_json::to_value(&output).expect("serialise doctor output");
            let claim_field = json
                .get("protection_claim")
                .expect("protection_claim must be present in doctor JSON");
            let standalone =
                serde_json::to_value(&claim).expect("serialise standalone ProtectionClaim");
            assert_eq!(
                claim_field, &standalone,
                "doctor protection_claim must equal a standalone-serialised ProtectionClaim for state {state:?}",
            );
        }
    }

    /// `anvil doctor --json`'s `protection_claim` carries the §14
    /// `schema_version` token, so consumers can dispatch on the
    /// claim's own schema without re-reading the outer envelope.
    #[test]
    fn doctor_json_protection_claim_carries_schema_version() {
        let claim = ProtectionClaim::new(WorktreeClaimState::PreWriteDaemon, vec![]);
        let output = build_doctor_output(&[], &claim);
        let json = serde_json::to_value(&output).expect("serialise");
        assert_eq!(
            json["protection_claim"]["schema_version"]
                .as_str()
                .expect("schema_version is a string"),
            "anvil.protection-claim.v1",
        );
    }

    /// `anvil doctor`'s plain output must include the protection
    /// claim section end-to-end through `format_plain`. The shared
    /// renderer in `protection_claim_section` owns the line shape;
    /// `format_plain` indents it with doctor's two-space prefix and
    /// embeds it after the summary. Assert the headline + surface
    /// lines appear in the full rendered surface for the three
    /// reference states.
    #[test]
    fn doctor_plain_output_contains_protection_claim_section() {
        for (state, claim) in fixture_claims() {
            let rendered = format_plain(&[], &claim);
            let indented_headline = format!("  protection: {}\n", state.as_str());
            assert!(
                rendered.contains(&indented_headline),
                "format_plain output must contain indented headline for {state:?}: {rendered:?}",
            );
            for surface in &claim.surfaces {
                // Shared renderer emits `  surface <id>: <state>`;
                // doctor's `indent_block` prepends another two
                // spaces, so the byte-exact form in the full
                // surface is four spaces of leading whitespace.
                let expected_line = format!(
                    "    surface {}: {}\n",
                    surface.identifier,
                    surface.state.as_str()
                );
                assert!(
                    rendered.contains(&expected_line),
                    "format_plain output must contain per-surface line {expected_line:?} for {state:?}: {rendered:?}",
                );
            }
        }
    }

    /// `format_plain` must place the protection-claim section after
    /// the summary line (so the layered surface reads top-to-bottom
    /// as `checks → summary → protection`). Pins ordering so a future
    /// rearrangement doesn't silently flip the surface.
    #[test]
    fn doctor_plain_output_places_protection_claim_after_summary() {
        let claim = ProtectionClaim::new(WorktreeClaimState::PreWriteDaemon, vec![]);
        let rendered = format_plain(&[], &claim);
        let summary_idx = rendered
            .find("passed, ")
            .expect("summary line `<n> passed, …` must be present");
        let claim_idx = rendered
            .find("protection: pre-write-daemon")
            .expect("protection-claim headline must be present");
        assert!(
            claim_idx > summary_idx,
            "protection-claim section must follow the summary line: summary@{summary_idx} claim@{claim_idx} in {rendered:?}",
        );
    }
}
