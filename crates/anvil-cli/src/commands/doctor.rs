use std::io::IsTerminal;
use std::path::Path;
use std::process::Command;

use anvil_kernel_types::{
    Notification, NotificationClass, NotificationContext, NotificationPriority,
};
use anvil_tui::surfaces::doctor::{CheckStatus, DiagnosticCheck, DoctorState};
use serde::Serialize;

use crate::GlobalArgs;

#[derive(Debug, clap::Args)]
pub struct DoctorArgs {
    /// Auto-fix issues where possible
    #[arg(long)]
    fix: bool,
}

pub fn run(args: &DoctorArgs, global: &GlobalArgs) -> anyhow::Result<()> {
    let mut checks = run_all_checks();

    if args.fix {
        apply_fixes(&mut checks, global.json);
    }

    if global.json {
        print_json(&checks)?;
    } else if global.no_tui || !std::io::stdout().is_terminal() {
        print_plain(&checks);
    } else {
        let mut state = DoctorState::new(checks.clone());
        loop {
            state = crate::tui::run_surface(state)?;
            if state.wants_fix {
                if let Some(idx) = state.fix_index {
                    apply_fix_at(&mut state.checks, idx);
                    let fresh = collect_checks();
                    state.checks = fresh;
                    state.selected = idx.min(state.checks.len().saturating_sub(1));
                }
                state.wants_fix = false;
                state.fix_index = None;
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
            }
        }
        _ => DiagnosticCheck {
            name: "git-available".to_string(),
            category: "System".to_string(),
            status: CheckStatus::Fail,
            message: "git not found on PATH".to_string(),
            details: Some("Install git from https://git-scm.com".to_string()),
            auto_fixable: false,
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

    DiagnosticCheck {
        name: "git-repo".to_string(),
        category: "System".to_string(),
        status: if is_repo {
            CheckStatus::Pass
        } else {
            CheckStatus::Fail
        },
        message: if is_repo {
            "git repository detected".to_string()
        } else {
            "not a git repository".to_string()
        },
        details: if is_repo {
            None
        } else {
            Some("Run `git init` to initialise a repository".to_string())
        },
        auto_fixable: !is_repo,
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
        details: if exists {
            None
        } else {
            Some("Create .anvilrc with default configuration".to_string())
        },
        auto_fixable: !exists,
    }
}

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
        details: if exists {
            None
        } else {
            Some("Create .anvil/ directory for Anvil state files".to_string())
        },
        auto_fixable: !exists,
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
        details: if writable {
            None
        } else {
            Some("Check directory permissions on .anvil/".to_string())
        },
        auto_fixable: false,
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
        }
    } else {
        DiagnosticCheck {
            name: "plans-dir".to_string(),
            category: "Configuration".to_string(),
            status: CheckStatus::Warn,
            message: "no plans directory found".to_string(),
            details: Some("Create plans/ directory for specification documents".to_string()),
            auto_fixable: true,
        }
    }
}

fn check_hooks_installed() -> DiagnosticCheck {
    let hook_path = Path::new(".husky/pre-commit");

    if !hook_path.exists() {
        return DiagnosticCheck {
            name: "hooks-installed".to_string(),
            category: "Hooks".to_string(),
            status: CheckStatus::Warn,
            message: "git hooks not installed".to_string(),
            details: Some(
                "Create .husky/pre-commit (chmod +x) with your Anvil checks (e.g. anvil gate once shipped)"
                    .to_string(),
            ),
            auto_fixable: false,
        };
    }

    #[cfg(unix)]
    let executable = {
        use std::os::unix::fs::PermissionsExt;
        std::fs::metadata(hook_path)
            .map(|m| m.permissions().mode() & 0o111 != 0)
            .unwrap_or(false)
    };

    #[cfg(not(unix))]
    let executable = true;

    DiagnosticCheck {
        name: "hooks-installed".to_string(),
        category: "Hooks".to_string(),
        status: if executable {
            CheckStatus::Pass
        } else {
            CheckStatus::Warn
        },
        message: if executable {
            "pre-commit hook installed and executable".to_string()
        } else {
            "pre-commit hook found but not executable".to_string()
        },
        details: if executable {
            None
        } else {
            Some("Run `chmod +x .husky/pre-commit`".to_string())
        },
        auto_fixable: false,
    }
}

/// SPG-002: surface any registry rule whose regex fails to compile under the
/// Rust `regex` crate. Without this, lookaround-bearing rules silently drop
/// out of the scanner catalogue and users cannot tell the difference between
/// "rule ran, no matches" and "rule never ran".
fn check_registry_patterns_compile() -> DiagnosticCheck {
    compile_check_from_diagnostics(&anvil_checks::antipattern::registry_compile_diagnostics())
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
        details: Some(format!(
            "{details}\n\nSee tests/scanner-parity/README.md — 'Rust-side handling \
             of PCRE lookaround rules' — for the pattern-rewrite contract and \
             fix guidance."
        )),
        auto_fixable: false,
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

fn apply_fixes(checks: &mut [DiagnosticCheck], quiet: bool) {
    for check in checks.iter_mut() {
        if !check.auto_fixable || check.status == CheckStatus::Pass {
            continue;
        }

        match check.name.as_str() {
            "git-repo" => {
                if Command::new("git").arg("init").output().is_ok() {
                    check.status = CheckStatus::Pass;
                    check.message = "git repository initialised".to_string();
                    check.auto_fixable = false;
                    if !quiet {
                        println!("  Fixed: git-repo — initialised git repository");
                    }
                }
            }
            "config-exists" => {
                let default_config = "{}";
                if std::fs::write(".anvilrc", default_config).is_ok() {
                    check.status = CheckStatus::Pass;
                    check.message = ".anvilrc created with defaults".to_string();
                    check.auto_fixable = false;
                    if !quiet {
                        println!("  Fixed: config-exists — created .anvilrc");
                    }
                }
            }
            "anvil-dir" => match std::fs::create_dir_all(".anvil") {
                Ok(()) => {
                    check.status = CheckStatus::Pass;
                    check.message = ".anvil/ directory created".to_string();
                    check.auto_fixable = false;
                    if !quiet {
                        println!("  Fixed: anvil-dir — created .anvil/ directory");
                    }
                }
                Err(e) => {
                    if !quiet {
                        eprintln!("  Failed to fix anvil-dir: {e}");
                    }
                }
            },
            "plans-dir" => match std::fs::create_dir_all("plans") {
                Ok(()) => {
                    check.status = CheckStatus::Pass;
                    check.message = "plans/ directory created".to_string();
                    check.auto_fixable = false;
                    if !quiet {
                        println!("  Fixed: plans-dir — created plans/ directory");
                    }
                }
                Err(e) => {
                    if !quiet {
                        eprintln!("  Failed to fix plans-dir: {e}");
                    }
                }
            },
            _ => {}
        }
    }
}

// --- Output formatters ---

fn print_plain(checks: &[DiagnosticCheck]) {
    println!();
    println!("  Anvil Doctor");
    println!();

    for check in checks {
        let icon = match check.status {
            CheckStatus::Pass => "\u{2713}",
            CheckStatus::Fail => "\u{2717}",
            CheckStatus::Warn => "\u{26A0}",
            CheckStatus::Skipped => "\u{25CB}",
            CheckStatus::Running => "*",
        };
        println!(
            "  {icon} {name}  {message}",
            name = check.name,
            message = check.message,
        );
    }

    let summary = anvil_tui::surfaces::doctor::DiagnosticSummary::from_checks(checks);
    println!();
    println!(
        "  {passed} passed, {failed} failed, {warnings} warnings, {skipped} skipped",
        passed = summary.passed,
        failed = summary.failed,
        warnings = summary.warnings,
        skipped = summary.skipped,
    );
    println!();
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
}

#[derive(Serialize)]
struct DoctorOutput {
    checks: Vec<JsonCheck>,
    notifications: Vec<Notification>,
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

fn build_doctor_output(checks: &[DiagnosticCheck]) -> DoctorOutput {
    let json_checks: Vec<JsonCheck> = checks
        .iter()
        .map(|c| JsonCheck {
            name: c.name.clone(),
            category: c.category.clone(),
            status: status_str(c.status).to_string(),
            message: c.message.clone(),
            details: c.details.clone(),
            auto_fixable: c.auto_fixable,
        })
        .collect();

    DoctorOutput {
        checks: json_checks,
        notifications: notifications_for_doctor(checks),
    }
}

fn print_json(checks: &[DiagnosticCheck]) -> anyhow::Result<()> {
    let output = build_doctor_output(checks);
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
        assert!(details.contains("tests/scanner-parity/README.md"));
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
        // Status depends on whether .git exists in cwd
        assert!(matches!(
            check.status,
            CheckStatus::Pass | CheckStatus::Fail
        ));
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

    #[test]
    fn json_output_is_valid() {
        let checks = run_all_checks();
        let output = build_doctor_output(&checks);
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
        let tmp = tempfile::tempdir().unwrap();
        let original = std::env::current_dir().unwrap();
        std::env::set_current_dir(tmp.path()).unwrap();

        let mut checks = vec![DiagnosticCheck {
            name: "anvil-dir".to_string(),
            category: "Configuration".to_string(),
            status: CheckStatus::Warn,
            message: ".anvil/ directory not found".to_string(),
            details: Some("Create .anvil/ directory for Anvil state files".to_string()),
            auto_fixable: true,
        }];

        apply_fixes(&mut checks, true);

        assert_eq!(checks[0].status, CheckStatus::Pass);
        assert!(!checks[0].auto_fixable);
        assert!(tmp.path().join(".anvil").is_dir());

        std::env::set_current_dir(original).unwrap();
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
        };
        let json: serde_json::Value = serde_json::to_value(&check).unwrap();
        assert_eq!(json["name"], "test-check");
        assert_eq!(json["category"], "Test");
        assert_eq!(json["status"], "pass");
        assert_eq!(json["message"], "all good");
        assert_eq!(json["details"], "extra info");
        assert_eq!(json["auto_fixable"], true);
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
}
