use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use clap::Args;
use serde::Serialize;

use crate::GlobalArgs;

#[derive(Debug, Args)]
pub struct HooksArgs {
    #[command(subcommand)]
    command: HooksCommand,
}

#[derive(Debug, clap::Subcommand)]
enum HooksCommand {
    /// Install Anvil git hooks (pre-commit and pre-push)
    Install {
        /// Overwrite existing hooks
        #[arg(long, short)]
        force: bool,
        /// Only install pre-commit hook
        #[arg(long, conflicts_with = "pre_push_only")]
        pre_commit_only: bool,
        /// Only install pre-push hook
        #[arg(long, conflicts_with = "pre_commit_only")]
        pre_push_only: bool,
        /// Install hooks in .husky directory
        #[arg(long)]
        husky: bool,
    },
    /// Remove Anvil git hooks
    Uninstall {
        /// Only remove pre-commit hook
        #[arg(long, conflicts_with = "pre_push_only")]
        pre_commit_only: bool,
        /// Only remove pre-push hook
        #[arg(long, conflicts_with = "pre_commit_only")]
        pre_push_only: bool,
    },
    /// Show status of Anvil git hooks
    Status,
}

/// Stable marker embedded in managed hooks to identify Anvil ownership.
const ANVIL_HOOK_MARKER: &str = "# @anvil-managed";

const PRE_COMMIT_HOOK: &str = r#"#!/bin/sh
# @anvil-managed
# Anvil pre-commit hook
[ "$ANVIL_SKIP_HOOKS" = "1" ] && exit 0
command -v anvil >/dev/null 2>&1 || { echo "anvil not found on PATH, skipping hook"; exit 0; }
ANVIL_HOOK=1 anvil gate --progress || {
  echo "Anvil gate checks failed. Fix issues or bypass with: ANVIL_SKIP_HOOKS=1 git commit"
  exit 1
}
"#;

const PRE_PUSH_HOOK: &str = r#"#!/bin/sh
# @anvil-managed
# Anvil pre-push hook
[ "$ANVIL_SKIP_HOOKS" = "1" ] && exit 0
command -v anvil >/dev/null 2>&1 || { echo "anvil not found on PATH, skipping hook"; exit 0; }
ANVIL_HOOK=1 anvil gate || {
  echo "Anvil gate checks failed. Fix issues or bypass with: ANVIL_SKIP_HOOKS=1 git push"
  exit 1
}
"#;

fn resolve_git_dir(workspace_root: &Path) -> Result<PathBuf> {
    let git_path = workspace_root.join(".git");
    if !git_path.exists() {
        bail!("Not a Git repository");
    }
    if git_path.is_dir() {
        return Ok(git_path);
    }
    let content = std::fs::read_to_string(&git_path).context("reading .git file")?;
    let gitdir = content
        .lines()
        .find_map(|line| {
            let line = line.trim();
            line.strip_prefix("gitdir:").map(|p| p.trim().to_string())
        })
        .context(".git file does not contain gitdir reference")?;
    let resolved = workspace_root.join(gitdir);
    if !resolved.exists() {
        bail!("Git directory not found: {}", resolved.display());
    }
    Ok(resolved)
}

fn detect_husky(workspace_root: &Path) -> (bool, Option<PathBuf>) {
    let husky_dir = workspace_root.join(".husky");
    let has_husky_dir = husky_dir.is_dir();

    // Check for husky in package.json devDependencies rather than just
    // testing for package.json existence, which would false-positive on
    // any Node project.
    let has_husky_dep = if workspace_root.join("package.json").exists() {
        std::fs::read_to_string(workspace_root.join("package.json"))
            .ok()
            .and_then(|content| serde_json::from_str::<serde_json::Value>(&content).ok())
            .is_some_and(|pkg| {
                let in_dev = pkg
                    .get("devDependencies")
                    .and_then(|d| d.get("husky"))
                    .is_some();
                let in_deps = pkg
                    .get("dependencies")
                    .and_then(|d| d.get("husky"))
                    .is_some();
                in_dev || in_deps
            })
    } else {
        false
    };

    let detected = has_husky_dir || has_husky_dep;

    (detected, if detected { Some(husky_dir) } else { None })
}

fn is_anvil_managed(path: &Path) -> bool {
    std::fs::read_to_string(path)
        .unwrap_or_default()
        .contains(ANVIL_HOOK_MARKER)
}

fn install_hook(hooks_dir: &Path, name: &str, content: &str, force: bool) -> Result<HookResult> {
    let path = hooks_dir.join(name);
    let existed_before = path.exists();

    if existed_before && !force {
        if is_anvil_managed(&path) {
            return Ok(HookResult {
                hook: name.to_string(),
                action: "skipped".to_string(),
                message: format!("{name} already installed (Anvil-managed)"),
            });
        }
        return Ok(HookResult {
            hook: name.to_string(),
            action: "skipped".to_string(),
            message: format!("{name} exists but is not Anvil-managed (use --force)"),
        });
    }

    if existed_before && force {
        let backup = path.with_extension("bak");
        std::fs::copy(&path, &backup).with_context(|| format!("backing up {}", path.display()))?;
    }

    std::fs::write(&path, content).with_context(|| format!("writing {}", path.display()))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755))
            .with_context(|| format!("setting permissions on {}", path.display()))?;
    }

    let action = if existed_before { "updated" } else { "created" };
    Ok(HookResult {
        hook: name.to_string(),
        action: action.to_string(),
        message: format!("{name} {action}"),
    })
}

fn uninstall_hook(hooks_dir: &Path, name: &str) -> Result<HookResult> {
    let path = hooks_dir.join(name);
    if !path.exists() {
        return Ok(HookResult {
            hook: name.to_string(),
            action: "none".to_string(),
            message: format!("{name} not found"),
        });
    }

    if !is_anvil_managed(&path) {
        return Ok(HookResult {
            hook: name.to_string(),
            action: "skipped".to_string(),
            message: format!("{name} exists but is not Anvil-managed"),
        });
    }

    std::fs::remove_file(&path).with_context(|| format!("removing {}", path.display()))?;

    Ok(HookResult {
        hook: name.to_string(),
        action: "removed".to_string(),
        message: format!("{name} removed"),
    })
}

#[derive(Debug, Serialize)]
struct HookResult {
    hook: String,
    action: String,
    message: String,
}

#[derive(Debug, Serialize, Clone)]
struct HookStatusInfo {
    location: String,
    hook: String,
    installed: bool,
    anvil_managed: bool,
}

#[derive(Debug, Serialize)]
struct HooksStatusData {
    hooks: Vec<HookStatusInfo>,
    husky_detected: bool,
}

fn find_repo_root() -> Result<PathBuf> {
    let output = std::process::Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .context("running git rev-parse")?;
    if !output.status.success() {
        bail!("Not inside a Git repository");
    }
    Ok(PathBuf::from(
        String::from_utf8_lossy(&output.stdout).trim().to_string(),
    ))
}

#[allow(clippy::too_many_lines)]
pub fn run(args: &HooksArgs, global: &GlobalArgs) -> Result<()> {
    let workspace_root = find_repo_root()?;
    let git_dir = resolve_git_dir(&workspace_root)?;

    match &args.command {
        HooksCommand::Install {
            force,
            pre_commit_only,
            pre_push_only,
            husky,
        } => {
            let hooks_dir = if *husky {
                let dir = workspace_root.join(".husky");
                std::fs::create_dir_all(&dir).context("creating .husky directory")?;
                dir
            } else {
                let (_detected, husky_dir_opt) = detect_husky(&workspace_root);
                if let Some(dir) = husky_dir_opt {
                    std::fs::create_dir_all(&dir).context("creating detected .husky directory")?;
                    eprintln!("Husky detected -- installing hooks in .husky directory");
                    dir
                } else {
                    let dir = git_dir.join("hooks");
                    std::fs::create_dir_all(&dir).context("creating hooks directory")?;
                    dir
                }
            };

            let mut results = Vec::new();
            if !*pre_push_only {
                results.push(install_hook(
                    &hooks_dir,
                    "pre-commit",
                    PRE_COMMIT_HOOK,
                    *force,
                )?);
            }
            if !*pre_commit_only {
                results.push(install_hook(&hooks_dir, "pre-push", PRE_PUSH_HOOK, *force)?);
            }

            if global.json {
                crate::output::json::print(&results)?;
            } else {
                crate::output::plain::blank();
                for r in &results {
                    match r.action.as_str() {
                        "created" => crate::output::plain::success(&r.message),
                        "updated" => println!("  \u{21bb} {}", r.message),
                        "skipped" => crate::output::plain::warn(&r.message),
                        _ => println!("  {}", r.message),
                    }
                }
                crate::output::plain::blank();
                println!("  pre-commit: Runs quality gates (anvil gate --progress)");
                println!("  pre-push:   Runs quality gates (anvil gate)");
                crate::output::plain::blank();
                println!("  Bypass: ANVIL_SKIP_HOOKS=1 git commit");
            }
        }
        HooksCommand::Uninstall {
            pre_commit_only,
            pre_push_only,
        } => {
            let mut results = Vec::new();
            for dir in [git_dir.join("hooks"), workspace_root.join(".husky")] {
                if !dir.exists() {
                    continue;
                }
                if !*pre_push_only {
                    results.push(uninstall_hook(&dir, "pre-commit")?);
                }
                if !*pre_commit_only {
                    results.push(uninstall_hook(&dir, "pre-push")?);
                }
            }

            if global.json {
                crate::output::json::print(&results)?;
            } else {
                crate::output::plain::blank();
                for r in &results {
                    match r.action.as_str() {
                        "removed" => crate::output::plain::success(&r.message),
                        "skipped" => crate::output::plain::warn(&r.message),
                        _ => println!("  {}", r.message),
                    }
                }
            }
        }
        HooksCommand::Status => {
            let (husky_detected, _) = detect_husky(&workspace_root);
            let mut hooks = Vec::new();

            for (dir, name) in [
                (git_dir.join("hooks"), ".git/hooks"),
                (workspace_root.join(".husky"), ".husky"),
            ] {
                if !dir.exists() {
                    continue;
                }
                for hook_name in ["pre-commit", "pre-push"] {
                    let path = dir.join(hook_name);
                    let installed = path.exists();
                    let anvil_managed = installed && is_anvil_managed(&path);
                    hooks.push(HookStatusInfo {
                        location: name.to_string(),
                        hook: hook_name.to_string(),
                        installed,
                        anvil_managed,
                    });
                }
            }

            let data = HooksStatusData {
                hooks: hooks.clone(),
                husky_detected,
            };

            if global.json {
                crate::output::json::print(&data)?;
            } else {
                crate::output::plain::blank();
                crate::output::plain::section("Anvil Git Hooks Status");
                for status in &hooks {
                    let indicator = if status.anvil_managed {
                        "installed (Anvil-managed)"
                    } else if status.installed {
                        "exists (not Anvil-managed)"
                    } else {
                        "not installed"
                    };
                    println!("  {}/{}: {indicator}", status.location, status.hook);
                }
                if husky_detected {
                    crate::output::plain::blank();
                    println!("  Husky detected in this project");
                }
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[derive(Parser)]
    struct Wrapper {
        #[command(flatten)]
        inner: HooksArgs,
    }

    #[test]
    fn args_parses_install() {
        let w = Wrapper::try_parse_from(["test", "install"]).unwrap();
        let _ = format!("{:?}", w.inner);
    }

    #[test]
    fn args_parses_status() {
        let w = Wrapper::try_parse_from(["test", "status"]).unwrap();
        let _ = format!("{:?}", w.inner);
    }

    #[test]
    fn args_parses_uninstall() {
        let w = Wrapper::try_parse_from(["test", "uninstall"]).unwrap();
        let _ = format!("{:?}", w.inner);
    }

    #[test]
    fn args_parses_install_force() {
        let w = Wrapper::try_parse_from(["test", "install", "--force"]).unwrap();
        let _ = format!("{:?}", w.inner);
    }

    #[test]
    fn args_rejects_conflicting_install_flags() {
        let result =
            Wrapper::try_parse_from(["test", "install", "--pre-commit-only", "--pre-push-only"]);
        assert!(result.is_err());
    }

    #[test]
    fn args_rejects_conflicting_uninstall_flags() {
        let result =
            Wrapper::try_parse_from(["test", "uninstall", "--pre-commit-only", "--pre-push-only"]);
        assert!(result.is_err());
    }

    #[test]
    fn marker_detection() {
        let dir = tempfile::tempdir().unwrap();
        let managed = dir.path().join("managed");
        std::fs::write(&managed, "#!/bin/sh\n# @anvil-managed\necho hi").unwrap();
        assert!(is_anvil_managed(&managed));

        let unmanaged = dir.path().join("unmanaged");
        std::fs::write(&unmanaged, "#!/bin/sh\necho hi").unwrap();
        assert!(!is_anvil_managed(&unmanaged));
    }

    #[test]
    fn hook_scripts_contain_marker() {
        assert!(PRE_COMMIT_HOOK.contains(ANVIL_HOOK_MARKER));
        assert!(PRE_PUSH_HOOK.contains(ANVIL_HOOK_MARKER));
    }

    #[test]
    fn hook_scripts_contain_skip_check() {
        assert!(PRE_COMMIT_HOOK.contains("ANVIL_SKIP_HOOKS"));
        assert!(PRE_PUSH_HOOK.contains("ANVIL_SKIP_HOOKS"));
    }

    #[test]
    fn hook_scripts_check_anvil_exists() {
        assert!(PRE_COMMIT_HOOK.contains("command -v anvil"));
        assert!(PRE_PUSH_HOOK.contains("command -v anvil"));
    }

    #[test]
    fn force_install_creates_backup_of_existing_hook() {
        let dir = tempfile::tempdir().unwrap();
        let original = "#!/bin/sh\necho original";
        std::fs::write(dir.path().join("pre-commit"), original).unwrap();

        let result = install_hook(dir.path(), "pre-commit", PRE_COMMIT_HOOK, true).unwrap();
        assert_eq!(result.action, "updated");

        let backup = dir.path().join("pre-commit.bak");
        assert!(backup.exists(), ".bak file should exist after --force");
        assert_eq!(std::fs::read_to_string(&backup).unwrap(), original);

        let installed = std::fs::read_to_string(dir.path().join("pre-commit")).unwrap();
        assert!(installed.contains(ANVIL_HOOK_MARKER));
    }
}
