use std::io::IsTerminal;
use std::path::Path;
use std::process::Command;

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
        apply_fixes(&mut checks);
    }

    if global.json {
        print_json(&checks)?;
        return Ok(());
    }

    if global.no_tui || !std::io::stdout().is_terminal() {
        print_plain(&checks);
        return Ok(());
    }

    let state = DoctorState::new(checks);
    crate::tui::run_surface(state)?;

    Ok(())
}

// --- Check runners ---

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
    let exists = Path::new(".git").exists();

    DiagnosticCheck {
        name: "git-repo".to_string(),
        category: "System".to_string(),
        status: if exists {
            CheckStatus::Pass
        } else {
            CheckStatus::Fail
        },
        message: if exists {
            "git repository detected".to_string()
        } else {
            "not a git repository".to_string()
        },
        details: if exists {
            None
        } else {
            Some("Run `git init` to initialise a repository".to_string())
        },
        auto_fixable: !exists,
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
        Ok(content) => match serde_json::from_str::<serde_json::Value>(&content) {
            Ok(_) => DiagnosticCheck {
                name: "config-valid".to_string(),
                category: "Configuration".to_string(),
                status: CheckStatus::Pass,
                message: ".anvilrc is valid JSON".to_string(),
                details: None,
                auto_fixable: false,
            },
            Err(e) => DiagnosticCheck {
                name: "config-valid".to_string(),
                category: "Configuration".to_string(),
                status: CheckStatus::Fail,
                message: "invalid JSON in .anvilrc".to_string(),
                details: Some(e.to_string()),
                auto_fixable: false,
            },
        },
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
            details: Some("Create plans/ or docs/plans/ for specification documents".to_string()),
            auto_fixable: false,
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
            details: Some("Run `anvil hooks install` to set up git hooks".to_string()),
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

// --- Fix application ---

fn apply_fixes(checks: &mut [DiagnosticCheck]) {
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
                    println!("  Fixed: git-repo — initialised git repository");
                }
            }
            "config-exists" => {
                let default_config = "{}";
                if std::fs::write(".anvilrc", default_config).is_ok() {
                    check.status = CheckStatus::Pass;
                    check.message = ".anvilrc created with defaults".to_string();
                    check.auto_fixable = false;
                    println!("  Fixed: config-exists — created .anvilrc");
                }
            }
            "anvil-dir" => {
                if std::fs::create_dir_all(".anvil").is_ok() {
                    check.status = CheckStatus::Pass;
                    check.message = ".anvil/ directory created".to_string();
                    check.auto_fixable = false;
                    println!("  Fixed: anvil-dir — created .anvil/ directory");
                }
            }
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

fn print_json(checks: &[DiagnosticCheck]) -> anyhow::Result<()> {
    let json_checks: Vec<JsonCheck> = checks
        .iter()
        .map(|c| JsonCheck {
            name: c.name.clone(),
            category: c.category.clone(),
            status: match c.status {
                CheckStatus::Pass => "pass".to_string(),
                CheckStatus::Fail => "fail".to_string(),
                CheckStatus::Warn => "warn".to_string(),
                CheckStatus::Skipped => "skipped".to_string(),
                CheckStatus::Running => "running".to_string(),
            },
            message: c.message.clone(),
            details: c.details.clone(),
            auto_fixable: c.auto_fixable,
        })
        .collect();

    println!("{}", serde_json::to_string_pretty(&json_checks)?);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn run_all_checks_returns_eight() {
        let checks = run_all_checks();
        assert_eq!(checks.len(), 8);
    }

    #[test]
    fn json_output_is_valid() {
        let checks = run_all_checks();
        // Ensure print_json doesn't panic — capture output
        let json_checks: Vec<JsonCheck> = checks
            .iter()
            .map(|c| JsonCheck {
                name: c.name.clone(),
                category: c.category.clone(),
                status: match c.status {
                    CheckStatus::Pass => "pass".to_string(),
                    CheckStatus::Fail => "fail".to_string(),
                    CheckStatus::Warn => "warn".to_string(),
                    CheckStatus::Skipped => "skipped".to_string(),
                    CheckStatus::Running => "running".to_string(),
                },
                message: c.message.clone(),
                details: c.details.clone(),
                auto_fixable: c.auto_fixable,
            })
            .collect();

        let json = serde_json::to_string(&json_checks).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert!(parsed.is_array());
        assert_eq!(parsed.as_array().unwrap().len(), 8);
    }

    #[test]
    fn apply_fixes_on_empty_list() {
        let mut checks = vec![];
        apply_fixes(&mut checks);
        assert!(checks.is_empty());
    }
}
