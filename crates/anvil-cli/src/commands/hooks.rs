use std::path::{Path, PathBuf};

use anvil_kernel_types::hooks::{ANVIL_CONFIG_HOOK_PATTERN, is_anvil_managed_command};
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
        #[arg(long, conflicts_with = "config")]
        husky: bool,
        /// Install hooks via Git 2.54 native `hook.<event>.command` config
        /// instead of writing files to `.husky/` or `.git/hooks/`.
        #[arg(long)]
        config: bool,
    },
    /// Remove Anvil git hooks
    Uninstall {
        /// Only remove pre-commit hook
        #[arg(long, conflicts_with = "pre_push_only")]
        pre_commit_only: bool,
        /// Only remove pre-push hook
        #[arg(long, conflicts_with = "pre_commit_only")]
        pre_push_only: bool,
        /// Remove Git 2.54 native `hook.<event>.command` config entries
        /// instead of files under `.husky/` or `.git/hooks/`.
        #[arg(long)]
        config: bool,
    },
    /// Show status of Anvil git hooks
    Status,
}

/// Stable marker embedded in managed hooks to identify Anvil ownership.
const ANVIL_HOOK_MARKER: &str = "# @anvil-managed";

/// Minimum Git version required for native `hook.<event>.command` config
/// support (added in Git 2.54). Used by the `--config` install path.
const MIN_CONFIG_HOOK_GIT_MAJOR: u32 = 2;
const MIN_CONFIG_HOOK_GIT_MINOR: u32 = 54;

/// Path to the rollout policy doc surfaced when `--config` is refused.
const HOOK_COMPAT_DOC: &str = "docs/guides/git-hook-compatibility.md";

/// Pre-commit command body installed via `git config --add hook.pre-commit.command`.
/// The leading `ANVIL_HOOK=1 anvil gate` segment doubles as the ownership marker
/// for uninstall — matched by `ANVIL_CONFIG_HOOK_PATTERN` (re-exported from
/// `anvil_kernel_types::hooks`).
const PRE_COMMIT_CONFIG_COMMAND: &str = "ANVIL_HOOK=1 anvil gate --progress";
const PRE_PUSH_CONFIG_COMMAND: &str = "ANVIL_HOOK=1 anvil gate";

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

/// Parse a `git --version` line into a `(major, minor, patch)` triple.
///
/// Accepts the canonical `git version 2.54.0` form along with the common
/// vendor variants like `git version 2.54.0.1.gabcdef` (Apple/Homebrew) and
/// `git version 2.54.0.windows.1`. Anything that fails to yield at least a
/// major and minor component returns `None` so the caller can refuse cleanly.
fn parse_git_version(raw: &str) -> Option<(u32, u32, u32)> {
    let stripped = raw.trim().strip_prefix("git version ")?;
    let core = stripped.split_whitespace().next().unwrap_or(stripped);
    let mut parts = core.split('.');
    let major = parts.next()?.parse::<u32>().ok()?;
    let minor = parts.next()?.parse::<u32>().ok()?;
    // Patch is optional — Git has historically shipped two-component
    // versions for early release candidates.
    let patch = parts.next().and_then(|p| {
        // Strip vendor suffixes such as `.windows.1` by parsing only the
        // leading digit run.
        let digits: String = p.chars().take_while(char::is_ascii_digit).collect();
        digits.parse::<u32>().ok()
    });
    Some((major, minor, patch.unwrap_or(0)))
}

/// True when the parsed version meets the 2.54 floor required for native
/// config-mode hook execution.
fn supports_config_hooks(version: (u32, u32, u32)) -> bool {
    let (major, minor, _patch) = version;
    (major, minor) >= (MIN_CONFIG_HOOK_GIT_MAJOR, MIN_CONFIG_HOOK_GIT_MINOR)
}

/// Probe the local `git` binary for its version. Returns the parsed triple
/// when both the invocation and the parse succeed.
fn detect_git_version() -> Result<(u32, u32, u32)> {
    let output = std::process::Command::new("git")
        .arg("--version")
        .output()
        .context("running git --version")?;
    if !output.status.success() {
        bail!("git --version exited non-zero");
    }
    let raw = String::from_utf8_lossy(&output.stdout).into_owned();
    parse_git_version(&raw).with_context(|| format!("parsing git version output: {}", raw.trim()))
}

/// Build the refusal message used when `anvil hooks install --config` is
/// invoked on a Git older than the 2.54 floor. Extracted so the test
/// suite can pin the wording without depending on the local `git
/// --version` (and so [`ensure_config_hook_support`] has a single
/// formatter, not an inlined string).
fn config_hook_support_error(detected: (u32, u32, u32)) -> String {
    let (major, minor, patch) = detected;
    format!(
        "anvil hooks --config requires Git >= {MIN_CONFIG_HOOK_GIT_MAJOR}.{MIN_CONFIG_HOOK_GIT_MINOR} \
         (detected {major}.{minor}.{patch}). See {HOOK_COMPAT_DOC} for the rollout policy."
    )
}

/// Refuse `--config` mode when the local Git is older than 2.54, surfacing
/// the policy doc so the user knows where to read more. Only `install
/// --config` enforces this — `uninstall --config` only manipulates
/// `git config` keys, which works on any modern Git, so users who
/// downgrade Git after install can still clean up.
fn ensure_config_hook_support() -> Result<()> {
    let version = detect_git_version()?;
    if supports_config_hooks(version) {
        return Ok(());
    }
    bail!("{}", config_hook_support_error(version));
}

/// Run `git config <args>` inside `workspace_root` and return the captured
/// stdout. Non-zero exits surface the recorded stderr.
fn git_config(workspace_root: &Path, args: &[&str]) -> Result<String> {
    let output = std::process::Command::new("git")
        .arg("-C")
        .arg(workspace_root)
        .arg("config")
        .args(args)
        .output()
        .with_context(|| format!("running git config {}", args.join(" ")))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        bail!("git config {} failed: {}", args.join(" "), stderr);
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

/// `git config --get-all hook.<event>.command`. Treats exit code 1 (no key
/// set) as an empty result instead of an error.
///
/// Exposed at `pub(crate)` so other commands (`status`, `doctor`) can detect
/// config-mode hooks without re-implementing the `git config` invocation or
/// duplicating the "exit 1 means empty" handling.
pub(crate) fn list_config_hook_commands(workspace_root: &Path, event: &str) -> Result<Vec<String>> {
    let key = format!("hook.{event}.command");
    let output = std::process::Command::new("git")
        .arg("-C")
        .arg(workspace_root)
        .args(["config", "--get-all", &key])
        .output()
        .with_context(|| format!("running git config --get-all {key}"))?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout)
            .lines()
            .map(ToString::to_string)
            .collect())
    } else if output.status.code() == Some(1) {
        // git config exits 1 when the key does not exist — treat as empty.
        Ok(Vec::new())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        bail!("git config --get-all {key} failed: {stderr}")
    }
}

/// Install one config-mode hook. Skips when an Anvil-managed entry already
/// exists for `event`, so re-running `install --config` is a no-op.
fn install_config_hook(
    workspace_root: &Path,
    event: &str,
    command: &str,
    force: bool,
) -> Result<HookResult> {
    let existing = list_config_hook_commands(workspace_root, event)?;
    let already_managed = existing.iter().any(|c| is_anvil_managed_command(c));

    if already_managed && !force {
        return Ok(HookResult {
            hook: event.to_string(),
            action: "skipped".to_string(),
            message: format!("{event} already installed (Anvil-managed config hook)"),
        });
    }

    if force && already_managed {
        // Replace the existing Anvil-managed entry so we do not stack
        // duplicates after repeated `--force` runs.
        git_config(
            workspace_root,
            &[
                "--unset-all",
                &format!("hook.{event}.command"),
                ANVIL_CONFIG_HOOK_PATTERN,
            ],
        )?;
    }

    git_config(
        workspace_root,
        &["--add", &format!("hook.{event}.command"), command],
    )?;

    let action = if already_managed {
        "updated"
    } else {
        "created"
    };
    Ok(HookResult {
        hook: event.to_string(),
        action: action.to_string(),
        message: format!("{event} {action} (config-mode)"),
    })
}

/// Remove Anvil-managed config-mode entries for `event`. User-authored
/// `hook.<event>.command` entries are left intact via the regex match.
fn uninstall_config_hook(workspace_root: &Path, event: &str) -> Result<HookResult> {
    let existing = list_config_hook_commands(workspace_root, event)?;
    let managed_count = existing
        .iter()
        .filter(|c| is_anvil_managed_command(c))
        .count();

    if managed_count == 0 {
        return Ok(HookResult {
            hook: event.to_string(),
            action: "none".to_string(),
            message: format!("{event} has no Anvil-managed config hook"),
        });
    }

    git_config(
        workspace_root,
        &[
            "--unset-all",
            &format!("hook.{event}.command"),
            ANVIL_CONFIG_HOOK_PATTERN,
        ],
    )?;

    Ok(HookResult {
        hook: event.to_string(),
        action: "removed".to_string(),
        message: format!("{event} removed (config-mode)"),
    })
}

/// Warn (not block) when a file-mode hook for `event` already exists in
/// either `.git/hooks/` or `.husky/`. Matches the `warnings over blocks`
/// rule from `docs/vision/anvil-scope-guard.md` so we do not silently
/// double-install hook execution. Formal coexistence handling lands in
/// GHOOK-004.
fn warn_on_file_mode_collision(workspace_root: &Path, git_dir: &Path, event: &str) -> Vec<PathBuf> {
    let mut found = Vec::new();
    for dir in [git_dir.join("hooks"), workspace_root.join(".husky")] {
        let path = dir.join(event);
        if path.exists() {
            found.push(path);
        }
    }
    found
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
            config,
        } => {
            if *config {
                ensure_config_hook_support()?;

                let mut results = Vec::new();
                let mut collisions: Vec<PathBuf> = Vec::new();
                if !*pre_push_only {
                    collisions.extend(warn_on_file_mode_collision(
                        &workspace_root,
                        &git_dir,
                        "pre-commit",
                    ));
                    results.push(install_config_hook(
                        &workspace_root,
                        "pre-commit",
                        PRE_COMMIT_CONFIG_COMMAND,
                        *force,
                    )?);
                }
                if !*pre_commit_only {
                    collisions.extend(warn_on_file_mode_collision(
                        &workspace_root,
                        &git_dir,
                        "pre-push",
                    ));
                    results.push(install_config_hook(
                        &workspace_root,
                        "pre-push",
                        PRE_PUSH_CONFIG_COMMAND,
                        *force,
                    )?);
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
                    if !collisions.is_empty() {
                        crate::output::plain::blank();
                        crate::output::plain::warn(
                            "File-mode hook(s) detected alongside config-mode install — \
                             both may run on the same event:",
                        );
                        for path in &collisions {
                            println!("    - {}", path.display());
                        }
                        println!("    See {HOOK_COMPAT_DOC} for coexistence guidance.");
                    }
                    crate::output::plain::blank();
                    println!("  pre-commit: Runs quality gates ({PRE_COMMIT_CONFIG_COMMAND})");
                    println!("  pre-push:   Runs quality gates ({PRE_PUSH_CONFIG_COMMAND})");
                    crate::output::plain::blank();
                    println!("  Bypass: ANVIL_SKIP_HOOKS=1 git commit");
                }
                return Ok(());
            }

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
            config,
        } => {
            if *config {
                // Intentionally do NOT call `ensure_config_hook_support()`
                // here. Removing entries is just `git config --unset-all`,
                // which works on any modern Git regardless of whether the
                // 2.54 hook-execution machinery is present. A user who
                // downgraded Git after install (or moved a repo to an
                // older machine) must still be able to clean up — that
                // is a strictly safer state than refusing.
                let mut results = Vec::new();
                if !*pre_push_only {
                    results.push(uninstall_config_hook(&workspace_root, "pre-commit")?);
                }
                if !*pre_commit_only {
                    results.push(uninstall_config_hook(&workspace_root, "pre-push")?);
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
                return Ok(());
            }

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

    #[test]
    fn install_hook_creates_new_hook() {
        let dir = tempfile::tempdir().unwrap();
        let result = install_hook(dir.path(), "pre-commit", PRE_COMMIT_HOOK, false).unwrap();
        assert_eq!(result.action, "created");

        let content = std::fs::read_to_string(dir.path().join("pre-commit")).unwrap();
        assert!(content.contains(ANVIL_HOOK_MARKER));
    }

    #[test]
    fn install_hook_skips_existing_unmanaged() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("pre-commit"), "#!/bin/sh\necho custom").unwrap();

        let result = install_hook(dir.path(), "pre-commit", PRE_COMMIT_HOOK, false).unwrap();
        assert_eq!(result.action, "skipped");
        assert!(result.message.contains("--force"));
    }

    #[test]
    fn install_hook_skips_existing_managed() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("pre-commit"), PRE_COMMIT_HOOK).unwrap();

        let result = install_hook(dir.path(), "pre-commit", PRE_COMMIT_HOOK, false).unwrap();
        assert_eq!(result.action, "skipped");
        assert!(result.message.contains("already installed"));
    }

    #[test]
    fn uninstall_hook_removes_managed() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("pre-commit"), PRE_COMMIT_HOOK).unwrap();

        let result = uninstall_hook(dir.path(), "pre-commit").unwrap();
        assert_eq!(result.action, "removed");
        assert!(!dir.path().join("pre-commit").exists());
    }

    #[test]
    fn uninstall_hook_skips_unmanaged() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("pre-commit"), "#!/bin/sh\necho custom").unwrap();

        let result = uninstall_hook(dir.path(), "pre-commit").unwrap();
        assert_eq!(result.action, "skipped");
        assert!(
            dir.path().join("pre-commit").exists(),
            "should not delete unmanaged hook"
        );
    }

    #[test]
    fn uninstall_hook_missing_returns_none() {
        let dir = tempfile::tempdir().unwrap();
        let result = uninstall_hook(dir.path(), "pre-commit").unwrap();
        assert_eq!(result.action, "none");
    }

    #[test]
    fn resolve_git_dir_standard() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join(".git")).unwrap();
        let result = resolve_git_dir(dir.path()).unwrap();
        assert_eq!(result, dir.path().join(".git"));
    }

    #[test]
    fn resolve_git_dir_worktree() {
        let dir = tempfile::tempdir().unwrap();
        let actual_git = dir.path().join("actual-git-dir");
        std::fs::create_dir_all(&actual_git).unwrap();
        std::fs::write(
            dir.path().join(".git"),
            format!("gitdir: {}", actual_git.display()),
        )
        .unwrap();

        let result = resolve_git_dir(dir.path()).unwrap();
        assert_eq!(result, actual_git);
    }

    #[test]
    fn resolve_git_dir_not_a_repo() {
        let dir = tempfile::tempdir().unwrap();
        let err = resolve_git_dir(dir.path()).unwrap_err();
        assert!(err.to_string().contains("Not a Git repository"));
    }

    #[test]
    fn detect_husky_with_directory() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join(".husky")).unwrap();

        let (detected, husky_dir) = detect_husky(dir.path());
        assert!(detected);
        assert!(husky_dir.is_some());
    }

    #[test]
    fn detect_husky_with_package_json_dep() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("package.json"),
            r#"{"devDependencies": {"husky": "^9.0.0"}}"#,
        )
        .unwrap();

        let (detected, _) = detect_husky(dir.path());
        assert!(detected);
    }

    #[test]
    fn no_husky_in_plain_node_project() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("package.json"),
            r#"{"devDependencies": {"vitest": "^1.0.0"}}"#,
        )
        .unwrap();

        let (detected, husky_dir) = detect_husky(dir.path());
        assert!(!detected);
        assert!(husky_dir.is_none());
    }

    #[test]
    fn no_husky_when_empty_dir() {
        let dir = tempfile::tempdir().unwrap();
        let (detected, _) = detect_husky(dir.path());
        assert!(!detected);
    }

    #[test]
    fn managed_detection_on_missing_file() {
        let dir = tempfile::tempdir().unwrap();
        let missing_path = dir.path().join("definitely-does-not-exist");
        assert!(!missing_path.exists());
        assert!(!is_anvil_managed(&missing_path));
    }

    // ----------------- GHOOK-002 (config-mode) -----------------

    #[test]
    fn args_parses_install_config_flag() {
        let w = Wrapper::try_parse_from(["test", "install", "--config"]).unwrap();
        let _ = format!("{:?}", w.inner);
    }

    #[test]
    fn args_parses_uninstall_config_flag() {
        let w = Wrapper::try_parse_from(["test", "uninstall", "--config"]).unwrap();
        let _ = format!("{:?}", w.inner);
    }

    #[test]
    fn args_rejects_install_config_with_husky() {
        // `--config` and `--husky` are mutually exclusive: config-mode does
        // not write files, so combining them is incoherent.
        let result = Wrapper::try_parse_from(["test", "install", "--config", "--husky"]);
        assert!(result.is_err());
    }

    #[test]
    fn parse_git_version_canonical() {
        assert_eq!(parse_git_version("git version 2.54.0"), Some((2, 54, 0)));
        assert_eq!(parse_git_version("git version 2.54.1"), Some((2, 54, 1)));
        assert_eq!(parse_git_version("git version 3.0.0"), Some((3, 0, 0)));
    }

    #[test]
    fn parse_git_version_two_component() {
        // Old release-candidate builds occasionally shipped a 2-part version.
        assert_eq!(parse_git_version("git version 2.54"), Some((2, 54, 0)));
    }

    #[test]
    fn parse_git_version_apple_variant() {
        assert_eq!(
            parse_git_version("git version 2.54.0.1.gabcdef"),
            Some((2, 54, 0)),
        );
    }

    #[test]
    fn parse_git_version_windows_variant() {
        assert_eq!(
            parse_git_version("git version 2.54.0.windows.1"),
            Some((2, 54, 0)),
        );
    }

    #[test]
    fn parse_git_version_rejects_garbage() {
        assert_eq!(parse_git_version(""), None);
        assert_eq!(parse_git_version("not a version"), None);
        assert_eq!(parse_git_version("git version oops"), None);
    }

    #[test]
    fn supports_config_hooks_floor() {
        assert!(supports_config_hooks((2, 54, 0)));
        assert!(supports_config_hooks((2, 54, 5)));
        assert!(supports_config_hooks((2, 60, 0)));
        assert!(supports_config_hooks((3, 0, 0)));
    }

    #[test]
    fn supports_config_hooks_below_floor() {
        assert!(!supports_config_hooks((2, 53, 99)));
        assert!(!supports_config_hooks((2, 51, 0)));
        assert!(!supports_config_hooks((1, 99, 99)));
    }

    /// The version-refusal error wording is part of the public CLI contract:
    /// users land on the policy doc and see the 2.54 floor explicitly. The
    /// test exercises the canonical formatter `config_hook_support_error`
    /// directly so a wording change in `ensure_config_hook_support` cannot
    /// silently pass — `ensure_config_hook_support` itself just calls the
    /// formatter and `bail!`s with its result.
    #[test]
    fn config_hook_support_error_mentions_2_54_doc_and_flag() {
        let err = config_hook_support_error((2, 51, 0));
        assert!(
            err.contains("2.54"),
            "error should mention the 2.54 floor: {err}"
        );
        assert!(
            err.contains(HOOK_COMPAT_DOC),
            "error should point at the policy doc: {err}",
        );
        assert!(
            err.contains("--config"),
            "error should reference the offending flag: {err}",
        );
        assert!(
            err.contains("2.51.0"),
            "error should report the detected version: {err}",
        );
    }

    #[test]
    fn is_anvil_managed_command_recognises_only_anvil_lines() {
        // Re-using the same predicate for install + uninstall keeps the
        // two paths in sync. This guards the predicate's behaviour.
        assert!(is_anvil_managed_command(PRE_COMMIT_CONFIG_COMMAND));
        assert!(is_anvil_managed_command(PRE_PUSH_CONFIG_COMMAND));
        // A user-authored command that happens to set ANVIL_HOOK=1 but
        // does not invoke `anvil gate` must not be claimed as ours.
        assert!(!is_anvil_managed_command("ANVIL_HOOK=1 npm run my-gate"));
        assert!(!is_anvil_managed_command("npm run lint-staged"));
    }

    #[test]
    fn config_hook_pattern_matches_install_commands() {
        // Sanity check: the regex used at uninstall time must match the
        // exact strings we install. If anyone edits one of the commands or
        // the marker prefix without updating the other, this trips first.
        let pattern = regex::Regex::new(ANVIL_CONFIG_HOOK_PATTERN).unwrap();
        assert!(pattern.is_match(PRE_COMMIT_CONFIG_COMMAND));
        assert!(pattern.is_match(PRE_PUSH_CONFIG_COMMAND));
        assert!(!pattern.is_match("npm run lint-staged"));
    }

    /// End-to-end round-trip exercised against a real `git init`'d repo.
    /// Skipped when the host's Git is older than 2.54 — config hooks are
    /// the feature under test, so there is nothing useful to assert when
    /// the floor is not met. Run with `cargo test -p eddacraft-anvil`.
    #[test]
    fn config_hook_install_uninstall_round_trip() {
        let Ok(version) = detect_git_version() else {
            eprintln!("skipping: git --version unavailable");
            return;
        };
        if !supports_config_hooks(version) {
            eprintln!(
                "skipping: host git is {}.{}.{} (< 2.54)",
                version.0, version.1, version.2,
            );
            return;
        }

        let dir = tempfile::tempdir().unwrap();
        let status = std::process::Command::new("git")
            .args(["init", "-q"])
            .current_dir(dir.path())
            .status()
            .expect("git init");
        assert!(status.success(), "git init must succeed");

        // Install pre-commit + pre-push.
        let pre_commit =
            install_config_hook(dir.path(), "pre-commit", PRE_COMMIT_CONFIG_COMMAND, false)
                .unwrap();
        assert_eq!(pre_commit.action, "created");
        let pre_push =
            install_config_hook(dir.path(), "pre-push", PRE_PUSH_CONFIG_COMMAND, false).unwrap();
        assert_eq!(pre_push.action, "created");

        // git config --get-all must return exactly our line for each event.
        let pre_commit_values = list_config_hook_commands(dir.path(), "pre-commit").unwrap();
        assert_eq!(
            pre_commit_values,
            vec![PRE_COMMIT_CONFIG_COMMAND.to_string()]
        );
        let pre_push_values = list_config_hook_commands(dir.path(), "pre-push").unwrap();
        assert_eq!(pre_push_values, vec![PRE_PUSH_CONFIG_COMMAND.to_string()]);

        // Re-running install is a no-op (skipped, not stacked).
        let pre_commit_again =
            install_config_hook(dir.path(), "pre-commit", PRE_COMMIT_CONFIG_COMMAND, false)
                .unwrap();
        assert_eq!(pre_commit_again.action, "skipped");
        let after_repeat = list_config_hook_commands(dir.path(), "pre-commit").unwrap();
        assert_eq!(after_repeat.len(), 1, "must not stack duplicate entries");

        // Uninstall both events.
        let removed_commit = uninstall_config_hook(dir.path(), "pre-commit").unwrap();
        assert_eq!(removed_commit.action, "removed");
        let removed_push = uninstall_config_hook(dir.path(), "pre-push").unwrap();
        assert_eq!(removed_push.action, "removed");

        // git config --get-all must now be empty for both events.
        assert!(
            list_config_hook_commands(dir.path(), "pre-commit")
                .unwrap()
                .is_empty()
        );
        assert!(
            list_config_hook_commands(dir.path(), "pre-push")
                .unwrap()
                .is_empty()
        );

        // Idempotent uninstall: repeating returns "none".
        let none = uninstall_config_hook(dir.path(), "pre-commit").unwrap();
        assert_eq!(none.action, "none");
    }

    /// User-authored `hook.<event>.command` entries must NOT be removed by
    /// `uninstall --config`. The regex marker is the only safe gate.
    #[test]
    fn config_hook_uninstall_preserves_user_entries() {
        let Ok(version) = detect_git_version() else {
            eprintln!("skipping: git --version unavailable");
            return;
        };
        if !supports_config_hooks(version) {
            eprintln!(
                "skipping: host git is {}.{}.{} (< 2.54)",
                version.0, version.1, version.2,
            );
            return;
        }

        let dir = tempfile::tempdir().unwrap();
        let status = std::process::Command::new("git")
            .args(["init", "-q"])
            .current_dir(dir.path())
            .status()
            .expect("git init");
        assert!(status.success());

        // Add a user-authored entry first.
        git_config(
            dir.path(),
            &["--add", "hook.pre-commit.command", "npm run my-thing"],
        )
        .unwrap();
        // Install the Anvil-managed entry alongside it.
        install_config_hook(dir.path(), "pre-commit", PRE_COMMIT_CONFIG_COMMAND, false).unwrap();

        let mixed = list_config_hook_commands(dir.path(), "pre-commit").unwrap();
        assert_eq!(mixed.len(), 2);

        // Uninstall must remove only the Anvil-managed line.
        let removed = uninstall_config_hook(dir.path(), "pre-commit").unwrap();
        assert_eq!(removed.action, "removed");

        let remaining = list_config_hook_commands(dir.path(), "pre-commit").unwrap();
        assert_eq!(remaining, vec!["npm run my-thing".to_string()]);
    }
}
