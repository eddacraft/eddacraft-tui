use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use clap::Args;
use serde::Serialize;

use crate::GlobalArgs;

/// Marker comment embedded in generated hooks so we can reliably detect
/// Anvil-managed hooks without false-positives from user comments.
const ANVIL_MARKER: &str = "# @anvil-managed";

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

const PRE_COMMIT_HOOK: &str = r#"#!/bin/sh
# @anvil-managed
# Anvil pre-commit hook — runs diagnostic checks.

if [ "${ANVIL_SKIP_HOOKS:-0}" = "1" ]; then
  exit 0
fi

if ! command -v anvil >/dev/null 2>&1; then
  echo "anvil: command not found — skipping doctor checks"
  exit 0
fi

ANVIL_HOOK=1 anvil doctor --no-tui || {
  echo "Anvil doctor checks failed. Fix issues or bypass with: ANVIL_SKIP_HOOKS=1 git commit"
  exit 1
}
"#;

const PRE_PUSH_HOOK: &str = r#"#!/bin/sh
# @anvil-managed
# Anvil pre-push hook — runs diagnostic checks before push.

if [ "${ANVIL_SKIP_HOOKS:-0}" = "1" ]; then
  exit 0
fi

if ! command -v anvil >/dev/null 2>&1; then
  echo "anvil: command not found — skipping doctor checks"
  exit 0
fi

ANVIL_HOOK=1 anvil doctor --no-tui || {
  echo "Anvil doctor checks failed. Fix issues or bypass with: ANVIL_SKIP_HOOKS=1 git push"
  exit 1
}
"#;

fn is_anvil_managed(content: &str) -> bool {
    content.contains(ANVIL_MARKER)
}

fn resolve_git_dir(workspace_root: &Path) -> Result<PathBuf> {
    if let Ok(output) = std::process::Command::new("git")
        .args(["rev-parse", "--git-dir"])
        .current_dir(workspace_root)
        .output()
        && output.status.success()
    {
        let gitdir = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let resolved = workspace_root.join(gitdir);
        if resolved.exists() {
            return Ok(resolved);
        }
    }

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
            let trimmed = line.trim();
            trimmed
                .strip_prefix("gitdir:")
                .map(|rest| rest.trim().to_string())
        })
        .context(".git file does not contain gitdir reference")?;
    let resolved = workspace_root.join(&gitdir);
    if !resolved.exists() {
        bail!("Git directory not found: {}", resolved.display());
    }
    Ok(resolved)
}

fn detect_husky(workspace_root: &Path) -> (bool, Option<PathBuf>) {
    let husky_dir = workspace_root.join(".husky");
    if husky_dir.is_dir() {
        (true, Some(husky_dir))
    } else {
        let package_json = workspace_root.join("package.json");
        if let Ok(content) = std::fs::read_to_string(package_json)
            && content.contains("\"husky\"")
        {
            (true, Some(workspace_root.join(".husky")))
        } else {
            (false, None)
        }
    }
}

fn install_hook(
    hooks_dir: &Path,
    name: &str,
    content: &str,
    force: bool,
    had_existing: &mut bool,
) -> Result<HookResult> {
    let path = hooks_dir.join(name);
    if path.exists() && !force {
        let existing = std::fs::read_to_string(&path).unwrap_or_default();
        if is_anvil_managed(&existing) {
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

    let existed = path.exists();
    if existed && force {
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

    let action = if existed { "updated" } else { "created" };
    *had_existing = existed;
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

    let content = std::fs::read_to_string(&path).unwrap_or_default();
    if !is_anvil_managed(&content) {
        return Ok(HookResult {
            hook: name.to_string(),
            action: "skipped".to_string(),
            message: format!("{name} exists but is not Anvil-managed"),
        });
    }

    std::fs::remove_file(&path).with_context(|| format!("removing {}", path.display()))?;

    let backup = path.with_extension("bak");
    if backup.exists() {
        std::fs::rename(&backup, &path)
            .with_context(|| format!("restoring backup {}", backup.display()))?;
        return Ok(HookResult {
            hook: name.to_string(),
            action: "restored".to_string(),
            message: format!("{name} removed — original hook restored from backup"),
        });
    }

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

fn resolve_hooks_dir(workspace_root: &Path, git_dir: &Path, husky: bool) -> Result<PathBuf> {
    if husky {
        let dir = workspace_root.join(".husky");
        std::fs::create_dir_all(&dir).context("creating .husky directory")?;
        return Ok(dir);
    }
    let (_detected, husky_dir_opt) = detect_husky(workspace_root);
    if let Some(dir) = husky_dir_opt {
        std::fs::create_dir_all(&dir).context("creating detected .husky directory")?;
        eprintln!("Husky detected — installing hooks in .husky directory");
        return Ok(dir);
    }
    let dir = git_dir.join("hooks");
    std::fs::create_dir_all(&dir).context("creating hooks directory")?;
    Ok(dir)
}

fn print_hook_results(results: &[HookResult], icons: &[(&str, &str)]) {
    println!();
    for r in results {
        let icon = icons
            .iter()
            .find(|(action, _)| *action == r.action)
            .map_or("", |(_, icon)| *icon);
        if icon.is_empty() {
            println!("  {}", r.message);
        } else {
            println!("  {icon} {}", r.message);
        }
    }
}

#[allow(clippy::fn_params_excessive_bools)]
fn run_install(
    workspace_root: &Path,
    git_dir: &Path,
    global: &GlobalArgs,
    force: bool,
    pre_commit_only: bool,
    pre_push_only: bool,
    husky: bool,
) -> Result<()> {
    let hooks_dir = resolve_hooks_dir(workspace_root, git_dir, husky)?;

    let mut results = Vec::new();
    let mut had_existing = false;
    if !pre_push_only {
        results.push(install_hook(
            &hooks_dir,
            "pre-commit",
            PRE_COMMIT_HOOK,
            force,
            &mut had_existing,
        )?);
    }
    if !pre_commit_only {
        results.push(install_hook(
            &hooks_dir,
            "pre-push",
            PRE_PUSH_HOOK,
            force,
            &mut had_existing,
        )?);
    }
    let _ = had_existing;

    if global.json {
        crate::output::json::print(&results)?;
    } else {
        print_hook_results(
            &results,
            &[
                ("created", "\u{2713}"),
                ("updated", "\u{21bb}"),
                ("skipped", "\u{26a0}"),
            ],
        );
        println!();
        println!("  pre-commit: Runs Anvil doctor checks");
        println!("  pre-push:   Runs Anvil doctor checks");
        println!();
        println!("  Bypass: ANVIL_SKIP_HOOKS=1 git commit");
    }
    Ok(())
}

fn run_uninstall(
    workspace_root: &Path,
    git_dir: &Path,
    global: &GlobalArgs,
    pre_commit_only: bool,
    pre_push_only: bool,
) -> Result<()> {
    let mut results = Vec::new();
    for dir in [git_dir.join("hooks"), workspace_root.join(".husky")] {
        if !dir.exists() {
            continue;
        }
        if !pre_push_only {
            results.push(uninstall_hook(&dir, "pre-commit")?);
        }
        if !pre_commit_only {
            results.push(uninstall_hook(&dir, "pre-push")?);
        }
    }

    if global.json {
        crate::output::json::print(&results)?;
    } else {
        print_hook_results(
            &results,
            &[
                ("removed", "\u{2713}"),
                ("restored", "\u{21bb}"),
                ("skipped", "\u{26a0}"),
            ],
        );
    }
    Ok(())
}

fn run_status(workspace_root: &Path, git_dir: &Path, global: &GlobalArgs) -> Result<()> {
    let (husky_detected, _) = detect_husky(workspace_root);
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
            let anvil_managed =
                installed && is_anvil_managed(&std::fs::read_to_string(&path).unwrap_or_default());
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
        println!();
        println!("Anvil Git Hooks Status");
        println!();
        for status in &hooks {
            let indicator = if status.anvil_managed {
                "\u{2713} installed (Anvil-managed)"
            } else if status.installed {
                "\u{26a0} exists (not Anvil-managed)"
            } else {
                "not installed"
            };
            println!("  {}/{}: {}", status.location, status.hook, indicator);
        }
        if husky_detected {
            println!();
            println!("  \u{2139} Husky detected in this project");
        }
    }
    Ok(())
}

pub fn run(args: &HooksArgs, global: &GlobalArgs) -> Result<()> {
    let workspace_root = if let Ok(output) = std::process::Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        && output.status.success()
    {
        PathBuf::from(String::from_utf8_lossy(&output.stdout).trim())
    } else {
        std::env::current_dir().context("getting current directory")?
    };
    let git_dir = resolve_git_dir(&workspace_root)?;

    match &args.command {
        HooksCommand::Install {
            force,
            pre_commit_only,
            pre_push_only,
            husky,
        } => run_install(
            &workspace_root,
            &git_dir,
            global,
            *force,
            *pre_commit_only,
            *pre_push_only,
            *husky,
        ),
        HooksCommand::Uninstall {
            pre_commit_only,
            pre_push_only,
        } => run_uninstall(
            &workspace_root,
            &git_dir,
            global,
            *pre_commit_only,
            *pre_push_only,
        ),
        HooksCommand::Status => run_status(&workspace_root, &git_dir, global),
    }
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
    fn conflicting_flags_rejected() {
        let result =
            Wrapper::try_parse_from(["test", "install", "--pre-commit-only", "--pre-push-only"]);
        assert!(result.is_err());
    }

    #[test]
    fn marker_detection() {
        assert!(is_anvil_managed("#!/bin/sh\n# @anvil-managed\n"));
        assert!(!is_anvil_managed("#!/bin/sh\n# Anvil hook\n"));
        assert!(!is_anvil_managed(""));
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
    fn hook_scripts_contain_marker() {
        assert!(is_anvil_managed(PRE_COMMIT_HOOK));
        assert!(is_anvil_managed(PRE_PUSH_HOOK));
    }
}
