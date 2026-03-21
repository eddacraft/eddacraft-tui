#![allow(dead_code)]
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
        #[arg(long)]
        pre_commit_only: bool,
        /// Only install pre-push hook
        #[arg(long)]
        pre_push_only: bool,
        /// Install hooks in .husky directory
        #[arg(long)]
        husky: bool,
    },
    /// Remove Anvil git hooks
    Uninstall {
        /// Only remove pre-commit hook
        #[arg(long)]
        pre_commit_only: bool,
        /// Only remove pre-push hook
        #[arg(long)]
        pre_push_only: bool,
    },
    /// Show status of Anvil git hooks
    Status,
}

const PRE_COMMIT_HOOK: &str = r#"#!/bin/sh
# Anvil pre-commit hook
ANVIL_HOOK=1 anvil gate --progress || {
  echo "Anvil gate checks failed. Fix issues or bypass with: ANVIL_SKIP_HOOKS=1 git commit"
  exit 1
}
"#;

const PRE_PUSH_HOOK: &str = r#"#!/bin/sh
# Anvil pre-push hook
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
        .trim()
        .strip_prefix("gitdir: ")
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
    let has_pkg_dep = workspace_root.join("package.json").exists();
    (
        has_husky_dir || has_pkg_dep,
        if has_husky_dir { Some(husky_dir) } else { None },
    )
}

fn install_hook(hooks_dir: &Path, name: &str, content: &str, force: bool) -> Result<HookResult> {
    let path = hooks_dir.join(name);
    if path.exists() && !force {
        let is_anvil = std::fs::read_to_string(&path)
            .unwrap_or_default()
            .contains("Anvil");
        if is_anvil {
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

    if path.exists() && force {
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

    let action = if path.with_extension("bak").exists() {
        "updated"
    } else {
        "created"
    };
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
    if !content.contains("Anvil") {
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

fn resolve_hooks_dir(workspace_root: &Path, git_dir: &Path, husky: bool) -> Result<PathBuf> {
    if husky {
        let dir = workspace_root.join(".husky");
        std::fs::create_dir_all(&dir).context("creating .husky directory")?;
        return Ok(dir);
    }
    let (detected, husky_dir_opt) = detect_husky(workspace_root);
    if let Some(dir) = husky_dir_opt
        && detected
    {
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
    if !pre_push_only {
        results.push(install_hook(
            &hooks_dir,
            "pre-commit",
            PRE_COMMIT_HOOK,
            force,
        )?);
    }
    if !pre_commit_only {
        results.push(install_hook(&hooks_dir, "pre-push", PRE_PUSH_HOOK, force)?);
    }

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
        println!("  pre-commit: Validates planning documents");
        println!("  pre-push:   Runs quality gates");
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
            &[("removed", "\u{2713}"), ("skipped", "\u{26a0}")],
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
            let anvil_managed = installed
                && std::fs::read_to_string(&path)
                    .unwrap_or_default()
                    .contains("Anvil");
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
    let workspace_root = std::env::current_dir().context("getting current directory")?;
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
}
