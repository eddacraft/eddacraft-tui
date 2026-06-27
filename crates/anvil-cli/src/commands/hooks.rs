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
    /// Install anvil git hooks (pre-commit and pre-push)
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
    /// Remove anvil git hooks
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
    /// Show status of anvil git hooks
    Status,
}

/// Stable marker embedded in managed hooks to identify anvil ownership.
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
# anvil pre-commit hook
[ "$ANVIL_SKIP_HOOKS" = "1" ] && exit 0
command -v anvil >/dev/null 2>&1 || { echo "anvil not found on PATH, skipping hook"; exit 0; }
ANVIL_HOOK=1 anvil gate --progress || {
  echo "anvil gate checks failed. Fix issues or bypass with: ANVIL_SKIP_HOOKS=1 git commit"
  exit 1
}
"#;

const PRE_PUSH_HOOK: &str = r#"#!/bin/sh
# @anvil-managed
# anvil pre-push hook
[ "$ANVIL_SKIP_HOOKS" = "1" ] && exit 0
command -v anvil >/dev/null 2>&1 || { echo "anvil not found on PATH, skipping hook"; exit 0; }
ANVIL_HOOK=1 anvil gate || {
  echo "anvil gate checks failed. Fix issues or bypass with: ANVIL_SKIP_HOOKS=1 git push"
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
    // DISTRIB-006 (ADR-060): installing a git hook into `.git/hooks` / `.husky`
    // is a durable per-project mutation. Refuse under a gated ANVIL_HOME without
    // `--touch-project-state`. The read-only `--config` advice mode does not reach
    // here.
    crate::install_root::ensure_project_write_allowed("hooks install")?;

    let path = hooks_dir.join(name);
    let existed_before = path.exists();

    if existed_before && !force {
        if is_anvil_managed(&path) {
            return Ok(HookResult {
                hook: name.to_string(),
                action: "skipped".to_string(),
                message: format!("{name} already installed (anvil-managed)"),
            });
        }
        return Ok(HookResult {
            hook: name.to_string(),
            action: "skipped".to_string(),
            message: format!("{name} exists but is not anvil-managed (use --force)"),
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
    // DISTRIB-006 (ADR-060): removing a git hook is a durable per-project
    // mutation — refuse under a gated ANVIL_HOME without `--touch-project-state`.
    crate::install_root::ensure_project_write_allowed("hooks uninstall")?;

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
            message: format!("{name} exists but is not anvil-managed"),
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
    /// Per-event coexistence report. GHOOK-004 makes this first-class so
    /// `anvil hooks status --json` can be consumed by automation that
    /// needs to know about duplicate-execution risk, third-party hook
    /// managers, foreign config-mode entries, and `core.hooksPath`.
    coexistence: Vec<CoexistenceReport>,
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

/// `git config --get hook.<event>.enabled` — returns whether config-mode
/// hooks for `event` are enabled. Git's default when the key is absent is
/// **true**, so this matches that semantic: only an explicit
/// case-insensitive `false` flips the result. Best-effort: any IO or
/// parse error is treated as "default = enabled" so a transient `git`
/// failure cannot misreport an installed hook as disabled.
///
/// Exposed at `pub(crate)` so `status.rs` and `doctor.rs` can mark
/// disabled config entries inactive without depending on the regex /
/// `git config` plumbing themselves.
pub(crate) fn config_hooks_enabled(workspace_root: &Path, event: &str) -> bool {
    let key = format!("hook.{event}.enabled");
    let output = std::process::Command::new("git")
        .arg("-C")
        .arg(workspace_root)
        .args(["config", "--get", &key])
        .output();
    let Ok(output) = output else {
        return true; // git unavailable — assume Git's default
    };
    if !output.status.success() {
        // Exit 1 = key absent (Git default → enabled). Other non-zero
        // exits are treated the same — better to over-report enabled
        // than to silently mark a working hook disabled on an
        // unrelated `git config` failure.
        return true;
    }
    let raw = String::from_utf8_lossy(&output.stdout)
        .trim()
        .to_ascii_lowercase();
    // Honour Git's accepted boolean spellings: `false`, `0`, `off`, `no`
    // disable; anything else (including the explicit `true`) enables.
    !matches!(raw.as_str(), "false" | "0" | "off" | "no")
}

/// Install one config-mode hook. Skips when an anvil-managed entry already
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
            message: format!("{event} already installed (anvil-managed config hook)"),
        });
    }

    if force && already_managed {
        // Replace the existing anvil-managed entry so we do not stack
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

/// Remove anvil-managed config-mode entries for `event`. User-authored
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
            message: format!("{event} has no anvil-managed config hook"),
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

/// Third-party hook manager detected alongside anvil's own install. We never
/// edit, refuse, or remove these — only surface them so the user knows
/// another tool may be wiring the same events.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum ThirdPartyManager {
    /// `.husky/<event>` files (file-mode), Husky-owned.
    Husky,
    /// `.lefthook.yml` or `lefthook.yml`.
    Lefthook,
    /// `.pre-commit-config.yaml` (the python pre-commit framework).
    PreCommit,
}

impl ThirdPartyManager {
    fn label(self) -> &'static str {
        match self {
            Self::Husky => "Husky",
            Self::Lefthook => "Lefthook",
            Self::PreCommit => "pre-commit framework",
        }
    }

    fn config_path(self) -> &'static str {
        match self {
            Self::Husky => ".husky/",
            Self::Lefthook => ".lefthook.yml | lefthook.yml",
            Self::PreCommit => ".pre-commit-config.yaml",
        }
    }
}

/// Structured coexistence report for a single hook event.
///
/// This is the formal GHOOK-004 generalisation of the earlier
/// `warn_on_file_mode_collision` stub. It collects every signal anvil knows
/// about so callers can decide whether to surface a warning, populate a
/// status panel, or drop the data straight into JSON. Detection is strictly
/// non-destructive: nothing in this module ever writes, edits, or removes
/// the entries it discovers — that is the anvil scope-guard "warnings over
/// blocks" rule applied to hook coexistence.
#[derive(Debug, Clone, Default, Serialize)]
pub(crate) struct CoexistenceReport {
    /// Hook event this report describes (e.g. `pre-commit`).
    pub event: String,
    /// File-mode hook scripts found at `.git/hooks/<event>` or
    /// `.husky/<event>`. When this list is non-empty AND any config-mode
    /// entry is present, Git 2.54 will run **both** sources.
    pub file_mode_paths: Vec<PathBuf>,
    /// Third-party hook managers detected in the workspace (Husky / Lefthook
    /// / pre-commit framework). Repo-wide, not event-specific — these
    /// markers imply non-anvil ownership across all events.
    pub third_party_managers: Vec<ThirdPartyManager>,
    /// Number of `hook.<event>.command` entries that do NOT match the
    /// anvil-managed prefix. anvil never touches these. `None` when the
    /// `git config --get-all` probe could not be run (no `git`, repo
    /// transient state, etc) so callers can distinguish "zero foreign
    /// entries" from "could not determine".
    pub foreign_config_entries: Option<usize>,
    /// Value of `git config core.hooksPath` if set — when present, Git
    /// bypasses `.git/hooks/` entirely and resolves file-mode hooks
    /// against this directory instead. anvil does not override this.
    pub core_hooks_path: Option<String>,
}

impl CoexistenceReport {
    /// True when at least one signal was found that warrants surfacing.
    pub(crate) fn has_findings(&self) -> bool {
        !self.file_mode_paths.is_empty()
            || !self.third_party_managers.is_empty()
            || self.foreign_config_entries.unwrap_or(0) > 0
            || self.core_hooks_path.is_some()
    }

    /// True when both a file-mode hook AND any config-mode entry exist for
    /// this event — Git 2.54 will run both, so the user should know.
    /// Only the caller that owns the config-mode-entry-count knows the
    /// full picture, so this just reports the file-mode side; the
    /// config-mode entry count is a separate parameter to the printer.
    pub(crate) fn has_file_mode_collision(&self) -> bool {
        !self.file_mode_paths.is_empty()
    }
}

/// Probe the workspace for every coexistence signal anvil knows about. The
/// returned report is purely informational — callers decide whether to
/// warn, render, or drop it. Used by `install --config`, `uninstall
/// --config`, and `status` so all three surfaces speak the same language.
///
/// Generalises and replaces the GHOOK-002 `warn_on_file_mode_collision`
/// stub. The four signals it captures map directly to the GHOOK-004
/// behaviour rules:
///
/// 1. **Duplicate-execution risk** — `file_mode_paths` listing both
///    `.git/hooks/<event>` and `.husky/<event>` so the caller can pair
///    that against the live `hook.<event>.command` count.
/// 2. **Third-party manager presence** — `.husky/`, lefthook YAML, or
///    `.pre-commit-config.yaml` mark non-anvil hook ownership.
/// 3. **Foreign config-mode entries** — count of `hook.<event>.command`
///    values that do NOT match the anvil-managed prefix; anvil never
///    edits these.
/// 4. **`core.hooksPath` override** — when set, file-mode hooks resolve
///    against this directory and `.git/hooks/` is bypassed. Documented
///    in the public guide; we never override it.
pub(crate) fn detect_coexistence(
    workspace_root: &Path,
    _git_dir: &Path,
    event: &str,
) -> CoexistenceReport {
    let mut report = CoexistenceReport {
        event: event.to_string(),
        ..CoexistenceReport::default()
    };

    // (1) File-mode hook scripts on disk for this event. Use the shared
    // resolver so detection mirrors Git's actual lookup rules:
    // - `core.hooksPath`, when set, replaces `.git/hooks/`
    // - `.git`-as-file (worktrees / submodules) resolves through
    //   `resolve_git_dir`, not naive `<workspace>/.git/hooks/`
    // The previous shape (hard-coded `git_dir.join("hooks")` plus
    // `.husky`) missed both cases and produced false-negatives in
    // worktrees and false-positives when `core.hooksPath` was set.
    for path in resolve_file_mode_hook_paths(workspace_root, event) {
        if path.exists() {
            report.file_mode_paths.push(path);
        }
    }

    // (2) Third-party hook managers — repo-wide, event-agnostic.
    if workspace_root.join(".husky").is_dir() {
        report.third_party_managers.push(ThirdPartyManager::Husky);
    }
    if workspace_root.join(".lefthook.yml").exists() || workspace_root.join("lefthook.yml").exists()
    {
        report
            .third_party_managers
            .push(ThirdPartyManager::Lefthook);
    }
    if workspace_root.join(".pre-commit-config.yaml").exists() {
        report
            .third_party_managers
            .push(ThirdPartyManager::PreCommit);
    }

    // (3) Foreign `hook.<event>.command` entries. Best-effort: when the
    // probe fails (no `git`, transient config error, …) we leave the field
    // as `None` so callers can render "(unknown)" rather than "0".
    report.foreign_config_entries = match list_config_hook_commands(workspace_root, event) {
        Ok(entries) => Some(
            entries
                .iter()
                .filter(|c| !is_anvil_managed_command(c))
                .count(),
        ),
        Err(_) => None,
    };

    // (4) `core.hooksPath` override.
    report.core_hooks_path = read_core_hooks_path(workspace_root);

    report
}

/// Read `git config core.hooksPath`. Returns `None` when the key is not set
/// (exit 1) or when `git` is unreachable. Trims trailing newline so the
/// raw value is suitable for direct rendering.
pub(crate) fn read_core_hooks_path(workspace_root: &Path) -> Option<String> {
    let output = std::process::Command::new("git")
        .arg("-C")
        .arg(workspace_root)
        .args(["config", "--get", "core.hooksPath"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let value = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if value.is_empty() { None } else { Some(value) }
}

/// Resolve the file-mode hook paths Git would actually consult for `event`
/// at this repo. Mirrors Git's resolution rules so detection in `status`,
/// `doctor`, `install --config`, and onboarding all agree:
///
/// - When `core.hooksPath` is set, that directory is the only file-mode
///   location Git looks at (`<hooksPath>/<event>`); `.git/hooks/` is
///   bypassed entirely.
/// - Otherwise: `<resolved git-dir>/hooks/<event>` (handles the
///   `.git`-as-file case for worktrees / submodules) plus the
///   `.husky/<event>` Husky entry point if present.
///
/// Returns paths regardless of whether they exist on disk; callers test
/// `.exists()` on each. Best-effort: a missing `git` falls back to the
/// `<workspace_root>/.git/hooks/<event>` plus `.husky/<event>` pair so
/// detection still works in environments without git on PATH.
pub(crate) fn resolve_file_mode_hook_paths(workspace_root: &Path, event: &str) -> Vec<PathBuf> {
    if let Some(custom) = read_core_hooks_path(workspace_root) {
        let custom_path = Path::new(&custom);
        let resolved = if custom_path.is_absolute() {
            custom_path.to_path_buf()
        } else {
            workspace_root.join(custom_path)
        };
        return vec![resolved.join(event)];
    }
    let git_dir = resolve_git_dir(workspace_root).unwrap_or_else(|_| workspace_root.join(".git"));
    let mut paths = vec![git_dir.join("hooks").join(event)];
    let husky = workspace_root.join(".husky").join(event);
    if husky != paths[0] {
        paths.push(husky);
    }
    paths
}

/// Render a human-readable coexistence block to stdout. Used by `install
/// --config`, `uninstall --config`, and `status`. Caller passes the live
/// count of anvil-managed config entries for the event so the duplicate-
/// execution warning can include both sides of the picture.
fn print_coexistence_report(report: &CoexistenceReport, anvil_managed_config_entries: usize) {
    if !report.has_findings() {
        return;
    }
    crate::output::plain::blank();
    let any_config_present =
        anvil_managed_config_entries > 0 || report.foreign_config_entries.unwrap_or(0) > 0;
    if report.has_file_mode_collision() && any_config_present {
        crate::output::plain::warn(&format!(
            "Duplicate-execution risk for `{}`: both a file-mode hook and a \
             config-mode entry are present. Git 2.54 runs both.",
            report.event,
        ));
        for path in &report.file_mode_paths {
            println!("    file mode  - {}", path.display());
        }
        if anvil_managed_config_entries > 0 {
            println!(
                "    config mode - hook.{}.command (anvil-managed) x{}",
                report.event, anvil_managed_config_entries,
            );
        }
        if let Some(n) = report.foreign_config_entries
            && n > 0
        {
            println!(
                "    config mode - hook.{}.command (foreign) x{}",
                report.event, n,
            );
        }
    } else if report.has_file_mode_collision() {
        crate::output::plain::warn(&format!(
            "File-mode hook(s) detected for `{}`:",
            report.event,
        ));
        for path in &report.file_mode_paths {
            println!("    - {}", path.display());
        }
    }

    if !report.third_party_managers.is_empty() {
        crate::output::plain::warn("Other hook managers detected:");
        for mgr in &report.third_party_managers {
            println!("    - {} ({})", mgr.label(), mgr.config_path());
        }
    }

    if let Some(n) = report.foreign_config_entries
        && n > 0
    {
        crate::output::plain::warn(&format!(
            "{n} foreign hook.{}.command entr{} present (left untouched).",
            report.event,
            if n == 1 { "y" } else { "ies" },
        ));
    }

    if let Some(path) = report.core_hooks_path.as_ref() {
        crate::output::plain::info(&format!(
            "core.hooksPath is set to `{path}` — file-mode hooks resolve there, not .git/hooks/.",
        ));
    }

    println!("    See {HOOK_COMPAT_DOC} for the coexistence policy.");
}

/// Remove every anvil-managed git hook from the current workspace
/// without writing anything to stdout/stderr. Used by `anvil uninstall`
/// so its own output (human or JSON envelope) is not interleaved with
/// the hooks command's renderer.
///
/// Calls the inner primitives (`uninstall_hook`, `uninstall_config_hook`)
/// directly across both file-mode locations (`.git/hooks/`, `.husky/`)
/// and config-mode (Git 2.54 `hook.<event>.command`) for the
/// pre-commit and pre-push events. Each primitive is a no-op when its
/// target is not present, so the whole call is idempotent.
///
/// Returns `Ok(())` when invoked outside a git repository — there is
/// nothing to remove and that is not an error condition for uninstall.
pub fn uninstall_all_managed_hooks_silent() -> Result<()> {
    // Any failure to locate a git repo is treated as "nothing to
    // remove" — uninstall must still proceed for the rest of the
    // project-local state. The specific error string varies across
    // git versions (`Not inside a Git repository`, `not a git
    // repository`, etc.), so swallow them all rather than string-match.
    let Ok(workspace_root) = find_repo_root() else {
        return Ok(());
    };
    let Ok(git_dir) = resolve_git_dir(&workspace_root) else {
        return Ok(());
    };

    // File-mode hooks: `.git/hooks/` and `.husky/`.
    for dir in [git_dir.join("hooks"), workspace_root.join(".husky")] {
        if !dir.exists() {
            continue;
        }
        let _ = uninstall_hook(&dir, "pre-commit")?;
        let _ = uninstall_hook(&dir, "pre-push")?;
    }

    // Config-mode hooks (Git 2.54 `hook.<event>.command`).
    let _ = uninstall_config_hook(&workspace_root, "pre-commit")?;
    let _ = uninstall_config_hook(&workspace_root, "pre-push")?;

    Ok(())
}

/// Install activation-time hook coverage without rendering the `anvil hooks`
/// command surface.
///
/// `anvil start` uses this for ACTMO-005 so the MCP-optional activation spine
/// includes commit and push hooks. The install policy mirrors the default
/// `anvil hooks install` path: prefer a detected Husky directory, otherwise
/// write Anvil-managed file-mode hooks under `.git/hooks/`. Existing unmanaged
/// hooks are preserved by [`install_hook`]'s non-force skip semantics.
pub(crate) fn install_activation_hooks_silent(workspace_root: &Path) -> Result<()> {
    // A non-Git directory has nowhere to install commit/push hooks, and that is
    // an expected, benign state — not an error. Returning `Err` here makes the
    // activation orchestrator print "could not install git hooks (Not a Git
    // repository)", which is misleading noise outside a repo (Copilot review).
    // Treat it as a silent no-op instead.
    if !workspace_root.join(".git").exists() {
        tracing::debug!(
            workspace = %workspace_root.display(),
            "activation: skipping git hook install — not a Git repository",
        );
        return Ok(());
    }
    let git_dir = resolve_git_dir(workspace_root)?;
    let hooks_dir = {
        let (_detected, husky_dir_opt) = detect_husky(workspace_root);
        if let Some(dir) = husky_dir_opt {
            std::fs::create_dir_all(&dir).context("creating detected .husky directory")?;
            dir
        } else {
            let dir = git_dir.join("hooks");
            std::fs::create_dir_all(&dir).context("creating hooks directory")?;
            dir
        }
    };

    let _ = install_hook(&hooks_dir, "pre-commit", PRE_COMMIT_HOOK, false)?;
    let _ = install_hook(&hooks_dir, "pre-push", PRE_PUSH_HOOK, false)?;
    Ok(())
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
                let mut reports: Vec<CoexistenceReport> = Vec::new();
                if !*pre_push_only {
                    reports.push(detect_coexistence(&workspace_root, &git_dir, "pre-commit"));
                    results.push(install_config_hook(
                        &workspace_root,
                        "pre-commit",
                        PRE_COMMIT_CONFIG_COMMAND,
                        *force,
                    )?);
                }
                if !*pre_commit_only {
                    reports.push(detect_coexistence(&workspace_root, &git_dir, "pre-push"));
                    results.push(install_config_hook(
                        &workspace_root,
                        "pre-push",
                        PRE_PUSH_CONFIG_COMMAND,
                        *force,
                    )?);
                }

                if global.json {
                    #[derive(Serialize)]
                    struct InstallConfigOutput<'a> {
                        results: &'a [HookResult],
                        coexistence: &'a [CoexistenceReport],
                    }
                    crate::output::json::print(&InstallConfigOutput {
                        results: &results,
                        coexistence: &reports,
                    })?;
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
                    for report in &reports {
                        // Read the live anvil-managed count from the repo
                        // rather than assuming 1. After a `skipped` action
                        // the count is whatever existed before; a repo
                        // could also legitimately have multiple managed
                        // entries from historical force-installs that
                        // GHOOK-002 has since collapsed but residual rows
                        // are still possible. Read once per event so the
                        // duplicate-risk maths in `print_coexistence_report`
                        // matches reality.
                        let managed_count = list_config_hook_commands(
                            &workspace_root,
                            &report.event,
                        )
                        .map_or(0, |entries| {
                            entries
                                .iter()
                                .filter(|c| is_anvil_managed_command(c))
                                .count()
                        });
                        print_coexistence_report(report, managed_count);
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
                let mut reports: Vec<CoexistenceReport> = Vec::new();
                if !*pre_push_only {
                    results.push(uninstall_config_hook(&workspace_root, "pre-commit")?);
                    reports.push(detect_coexistence(&workspace_root, &git_dir, "pre-commit"));
                }
                if !*pre_commit_only {
                    results.push(uninstall_config_hook(&workspace_root, "pre-push")?);
                    reports.push(detect_coexistence(&workspace_root, &git_dir, "pre-push"));
                }

                if global.json {
                    #[derive(Serialize)]
                    struct UninstallConfigOutput<'a> {
                        results: &'a [HookResult],
                        coexistence: &'a [CoexistenceReport],
                    }
                    crate::output::json::print(&UninstallConfigOutput {
                        results: &results,
                        coexistence: &reports,
                    })?;
                } else {
                    crate::output::plain::blank();
                    for r in &results {
                        match r.action.as_str() {
                            "removed" => crate::output::plain::success(&r.message),
                            "skipped" => crate::output::plain::warn(&r.message),
                            _ => println!("  {}", r.message),
                        }
                    }
                    // Post-uninstall: anvil-managed entries are gone, so
                    // any duplicate-execution risk that remains is between
                    // file-mode hooks and foreign config-mode entries the
                    // user owns. Pass anvil-managed count = 0.
                    for report in &reports {
                        print_coexistence_report(report, 0);
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

            // Build a coexistence report per event the status surface
            // covers. Live anvil-managed config entries are also counted
            // here so the structured output shows the duplicate-execution
            // risk without the caller needing to reach back into
            // `git config`.
            let mut coexistence: Vec<CoexistenceReport> = Vec::new();
            let mut anvil_managed_per_event: Vec<(String, usize)> = Vec::new();
            for event in ["pre-commit", "pre-push"] {
                coexistence.push(detect_coexistence(&workspace_root, &git_dir, event));
                let managed = list_config_hook_commands(&workspace_root, event)
                    .unwrap_or_default()
                    .iter()
                    .filter(|c| is_anvil_managed_command(c))
                    .count();
                anvil_managed_per_event.push((event.to_string(), managed));
            }

            let data = HooksStatusData {
                hooks: hooks.clone(),
                husky_detected,
                coexistence: coexistence.clone(),
            };

            if global.json {
                crate::output::json::print(&data)?;
            } else {
                crate::output::plain::blank();
                crate::output::plain::section("anvil Git Hooks Status");
                for status in &hooks {
                    let indicator = if status.anvil_managed {
                        "installed (anvil-managed)"
                    } else if status.installed {
                        "exists (not anvil-managed)"
                    } else {
                        "not installed"
                    };
                    println!("  {}/{}: {indicator}", status.location, status.hook);
                }
                for (report, (_event, managed)) in
                    coexistence.iter().zip(anvil_managed_per_event.iter())
                {
                    print_coexistence_report(report, *managed);
                }
                if husky_detected
                    && !coexistence
                        .iter()
                        .any(|r| r.third_party_managers.contains(&ThirdPartyManager::Husky))
                {
                    // Husky detected via package.json devDependency only —
                    // no `.husky/` dir on disk yet. Surface as a hint so the
                    // user knows the dependency is present but no hooks
                    // have been wired through it.
                    crate::output::plain::blank();
                    println!("  Husky listed in package.json but no .husky/ directory yet");
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
        // Install the anvil-managed entry alongside it.
        install_config_hook(dir.path(), "pre-commit", PRE_COMMIT_CONFIG_COMMAND, false).unwrap();

        let mixed = list_config_hook_commands(dir.path(), "pre-commit").unwrap();
        assert_eq!(mixed.len(), 2);

        // Uninstall must remove only the anvil-managed line.
        let removed = uninstall_config_hook(dir.path(), "pre-commit").unwrap();
        assert_eq!(removed.action, "removed");

        let remaining = list_config_hook_commands(dir.path(), "pre-commit").unwrap();
        assert_eq!(remaining, vec!["npm run my-thing".to_string()]);
    }

    // ----------------- GHOOK-004 (coexistence) -----------------

    /// Helper: skip when host Git is older than 2.54. GHOOK-004 detection
    /// reads `git config` like the install path, so a host without a
    /// usable Git binary cannot exercise the report.
    fn require_config_hook_support() -> bool {
        match detect_git_version() {
            Ok(v) if supports_config_hooks(v) => true,
            Ok(v) => {
                eprintln!("skipping: host git is {}.{}.{} (< 2.54)", v.0, v.1, v.2);
                false
            }
            Err(_) => {
                eprintln!("skipping: git --version unavailable");
                false
            }
        }
    }

    fn init_repo(dir: &Path) {
        let status = std::process::Command::new("git")
            .args(["init", "-q"])
            .current_dir(dir)
            .status()
            .expect("git init");
        assert!(status.success(), "git init must succeed");
    }

    /// Sanity check: an empty repo with no markers produces an empty
    /// report. `has_findings` returns `false` so callers know not to
    /// bother rendering anything.
    #[test]
    fn coexistence_empty_repo_has_no_findings() {
        if !require_config_hook_support() {
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        init_repo(dir.path());
        let git_dir = dir.path().join(".git");

        let report = detect_coexistence(dir.path(), &git_dir, "pre-commit");
        assert!(report.file_mode_paths.is_empty());
        assert!(report.third_party_managers.is_empty());
        assert_eq!(report.foreign_config_entries, Some(0));
        assert!(!report.has_findings());
        assert!(!report.has_file_mode_collision());
    }

    /// (a) `install --config` in a repo with `.husky/pre-commit` warns
    /// naming both: the file-mode path is captured AND the third-party
    /// manager is named.
    #[test]
    fn coexistence_detects_husky_file_mode_collision() {
        if !require_config_hook_support() {
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        init_repo(dir.path());
        let husky_dir = dir.path().join(".husky");
        std::fs::create_dir_all(&husky_dir).unwrap();
        std::fs::write(
            husky_dir.join("pre-commit"),
            "#!/bin/sh\nnpm run lint-staged\n",
        )
        .unwrap();

        let git_dir = dir.path().join(".git");
        let report = detect_coexistence(dir.path(), &git_dir, "pre-commit");

        assert!(report.has_file_mode_collision());
        assert!(
            report
                .file_mode_paths
                .iter()
                .any(|p| p == &husky_dir.join("pre-commit"))
        );
        assert!(
            report
                .third_party_managers
                .contains(&ThirdPartyManager::Husky),
            "report should name Husky alongside the file-mode path: {report:?}",
        );
    }

    /// (c) `install --config` in a repo with `.lefthook.yml` flags the
    /// lefthook manager. No file-mode `pre-commit` script is created;
    /// the YAML alone is the signal.
    #[test]
    fn coexistence_detects_lefthook_manager() {
        if !require_config_hook_support() {
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        init_repo(dir.path());
        std::fs::write(
            dir.path().join(".lefthook.yml"),
            "pre-commit:\n  commands: {}\n",
        )
        .unwrap();

        let git_dir = dir.path().join(".git");
        let report = detect_coexistence(dir.path(), &git_dir, "pre-commit");

        assert!(
            report
                .third_party_managers
                .contains(&ThirdPartyManager::Lefthook),
            "lefthook YAML must surface as a third-party manager: {report:?}",
        );
        assert!(report.has_findings());
    }

    /// `pre-commit-config.yaml` flags the pre-commit framework manager.
    #[test]
    fn coexistence_detects_pre_commit_framework() {
        if !require_config_hook_support() {
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        init_repo(dir.path());
        std::fs::write(dir.path().join(".pre-commit-config.yaml"), "repos: []\n").unwrap();

        let git_dir = dir.path().join(".git");
        let report = detect_coexistence(dir.path(), &git_dir, "pre-commit");

        assert!(
            report
                .third_party_managers
                .contains(&ThirdPartyManager::PreCommit),
            "pre-commit framework must surface as a third-party manager: {report:?}",
        );
    }

    /// (d) `install --config` preserves a foreign `hook.pre-commit.command`
    /// entry. After install, `git config --get-all` returns BOTH lines:
    /// the user's foreign entry AND anvil's managed entry.
    #[test]
    fn install_config_preserves_foreign_entry() {
        if !require_config_hook_support() {
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        init_repo(dir.path());

        // Seed a foreign entry first.
        git_config(
            dir.path(),
            &[
                "--add",
                "hook.pre-commit.command",
                "npm run my-foreign-gate",
            ],
        )
        .unwrap();

        // Install the anvil-managed entry alongside it.
        let result =
            install_config_hook(dir.path(), "pre-commit", PRE_COMMIT_CONFIG_COMMAND, false)
                .unwrap();
        assert_eq!(result.action, "created");

        // git config --get-all must now return BOTH lines, in install order.
        let entries = list_config_hook_commands(dir.path(), "pre-commit").unwrap();
        assert_eq!(
            entries,
            vec![
                "npm run my-foreign-gate".to_string(),
                PRE_COMMIT_CONFIG_COMMAND.to_string(),
            ],
            "foreign entry must survive install --config alongside the anvil entry",
        );

        // Coexistence report should count exactly 1 foreign entry.
        let report = detect_coexistence(dir.path(), &dir.path().join(".git"), "pre-commit");
        assert_eq!(report.foreign_config_entries, Some(1));
    }

    /// (e) `uninstall --config` with a foreign entry leaves it alone.
    /// Same expectation as the GHOOK-002 round-trip test, but driven
    /// through the coexistence report so the GHOOK-004 contract is
    /// pinned: foreign entries are NEVER touched.
    #[test]
    fn uninstall_config_leaves_foreign_entry_alone() {
        if !require_config_hook_support() {
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        init_repo(dir.path());

        // Seed both an anvil-managed and a foreign entry.
        install_config_hook(dir.path(), "pre-commit", PRE_COMMIT_CONFIG_COMMAND, false).unwrap();
        git_config(
            dir.path(),
            &[
                "--add",
                "hook.pre-commit.command",
                "npm run my-foreign-gate",
            ],
        )
        .unwrap();

        // Uninstall removes only the anvil entry.
        let removed = uninstall_config_hook(dir.path(), "pre-commit").unwrap();
        assert_eq!(removed.action, "removed");

        let remaining = list_config_hook_commands(dir.path(), "pre-commit").unwrap();
        assert_eq!(remaining, vec!["npm run my-foreign-gate".to_string()]);

        // Post-uninstall, the report should list 1 foreign entry left
        // behind — the user still owns it, anvil does not touch it.
        let report = detect_coexistence(dir.path(), &dir.path().join(".git"), "pre-commit");
        assert_eq!(report.foreign_config_entries, Some(1));
    }

    /// (b) Status in a repo with both file and config entries reports the
    /// duplicate-execution risk in the structured output. The status
    /// surface owns the printer; here we verify the underlying report
    /// carries both signals so the printer can pair them.
    #[test]
    fn status_reports_duplicate_execution_risk_when_both_modes_present() {
        if !require_config_hook_support() {
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        init_repo(dir.path());

        // File-mode hook in `.git/hooks/pre-commit`.
        let git_hooks = dir.path().join(".git").join("hooks");
        std::fs::create_dir_all(&git_hooks).unwrap();
        std::fs::write(git_hooks.join("pre-commit"), "#!/bin/sh\necho legacy\n").unwrap();

        // Config-mode entry installed via anvil.
        install_config_hook(dir.path(), "pre-commit", PRE_COMMIT_CONFIG_COMMAND, false).unwrap();

        let report = detect_coexistence(dir.path(), &dir.path().join(".git"), "pre-commit");
        assert!(
            report.has_file_mode_collision(),
            "file-mode path must be captured for the duplicate-execution check",
        );
        assert_eq!(
            report.foreign_config_entries,
            Some(0),
            "anvil's own entry must not count as foreign",
        );

        // Confirm the live anvil-managed count is 1 — the printer pairs
        // this with the file-mode path to render the duplicate-execution
        // warning. The CLI status arm does the same join.
        let managed = list_config_hook_commands(dir.path(), "pre-commit")
            .unwrap()
            .iter()
            .filter(|c| is_anvil_managed_command(c))
            .count();
        assert_eq!(managed, 1);
    }

    /// `core.hooksPath` is captured when set so the public docs can point
    /// users at the precedence behaviour Git itself enforces.
    #[test]
    fn coexistence_reports_core_hooks_path_when_set() {
        if !require_config_hook_support() {
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        init_repo(dir.path());

        let custom_dir = dir.path().join("custom-hooks");
        std::fs::create_dir_all(&custom_dir).unwrap();
        git_config(
            dir.path(),
            &["core.hooksPath", custom_dir.to_str().unwrap()],
        )
        .unwrap();

        let report = detect_coexistence(dir.path(), &dir.path().join(".git"), "pre-commit");
        assert_eq!(
            report.core_hooks_path.as_deref(),
            Some(custom_dir.to_str().unwrap()),
            "core.hooksPath should be reported verbatim when set: {report:?}",
        );
    }

    /// Regression: a hook script under `core.hooksPath` must register as a
    /// file-mode hook in the coexistence report. Previously the resolver
    /// only looked at `<git_dir>/hooks/<event>` plus `.husky/<event>` and
    /// missed `core.hooksPath`-based installs entirely.
    #[test]
    fn coexistence_detects_file_mode_hook_under_core_hooks_path() {
        if !require_config_hook_support() {
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        init_repo(dir.path());

        let custom_dir = dir.path().join("custom-hooks");
        std::fs::create_dir_all(&custom_dir).unwrap();
        git_config(
            dir.path(),
            &["core.hooksPath", custom_dir.to_str().unwrap()],
        )
        .unwrap();
        let hook_path = custom_dir.join("pre-commit");
        std::fs::write(&hook_path, "#!/bin/sh\nexit 0\n").unwrap();

        let report = detect_coexistence(dir.path(), &dir.path().join(".git"), "pre-commit");
        assert!(
            report.file_mode_paths.iter().any(|p| p == &hook_path),
            "expected core.hooksPath/pre-commit to surface as a file-mode hook: {report:?}"
        );
    }

    /// Regression: when `core.hooksPath` is set, a stale `.git/hooks/<event>`
    /// must NOT register as a file-mode hook — Git will not run it.
    /// Previously the resolver always checked `<git_dir>/hooks/<event>`
    /// regardless of `core.hooksPath` and produced a false-positive
    /// duplicate-execution warning.
    #[test]
    fn coexistence_ignores_stale_dot_git_hook_when_core_hooks_path_overrides() {
        if !require_config_hook_support() {
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        init_repo(dir.path());

        // Stale hook under `.git/hooks/` — Git would not run this once
        // core.hooksPath is set.
        let stale = dir.path().join(".git").join("hooks").join("pre-commit");
        std::fs::create_dir_all(stale.parent().unwrap()).unwrap();
        std::fs::write(&stale, "#!/bin/sh\nexit 0\n").unwrap();

        let custom_dir = dir.path().join("custom-hooks");
        std::fs::create_dir_all(&custom_dir).unwrap();
        git_config(
            dir.path(),
            &["core.hooksPath", custom_dir.to_str().unwrap()],
        )
        .unwrap();

        let report = detect_coexistence(dir.path(), &dir.path().join(".git"), "pre-commit");
        assert!(
            !report.file_mode_paths.iter().any(|p| p == &stale),
            "stale .git/hooks/pre-commit must not register when core.hooksPath overrides: {report:?}"
        );
    }

    /// The serialised `coexistence` field has stable shape — the GHOOK-003
    /// status JSON contract extends to GHOOK-004 without breaking existing
    /// consumers. Deliberately encoded as a snapshot so wording changes
    /// trip a review.
    #[test]
    fn coexistence_report_serialises_to_expected_shape() {
        let report = CoexistenceReport {
            event: "pre-commit".to_string(),
            file_mode_paths: vec![PathBuf::from(".husky/pre-commit")],
            third_party_managers: vec![ThirdPartyManager::Husky, ThirdPartyManager::Lefthook],
            foreign_config_entries: Some(2),
            core_hooks_path: Some(".my-hooks".to_string()),
        };
        let json = serde_json::to_value(&report).unwrap();
        assert_eq!(json["event"], "pre-commit");
        assert_eq!(json["file_mode_paths"][0], ".husky/pre-commit");
        assert_eq!(json["third_party_managers"][0], "husky");
        assert_eq!(json["third_party_managers"][1], "lefthook");
        assert_eq!(json["foreign_config_entries"], 2);
        assert_eq!(json["core_hooks_path"], ".my-hooks");
    }

    /// `hook.<event>.enabled` semantics: Git's default when the key is
    /// absent is `true`; only explicit boolean-falsey values flip it.
    /// `config_hooks_enabled` must mirror Git's parser, otherwise
    /// status / doctor / onboarding will misreport disabled hooks as
    /// active.
    #[test]
    fn config_hooks_enabled_honours_git_boolean_semantics() {
        let dir = tempfile::tempdir().unwrap();
        let status = std::process::Command::new("git")
            .args(["init", "-q"])
            .current_dir(dir.path())
            .status()
            .expect("git init");
        assert!(status.success());

        // Default-when-unset is enabled (matches Git's behaviour).
        assert!(
            config_hooks_enabled(dir.path(), "pre-commit"),
            "missing hook.pre-commit.enabled key must default to enabled"
        );

        // Each falsey form Git accepts must flip to disabled.
        for value in ["false", "0", "off", "no", "False", "OFF"] {
            git_config(
                dir.path(),
                &["--replace-all", "hook.pre-commit.enabled", value],
            )
            .unwrap();
            assert!(
                !config_hooks_enabled(dir.path(), "pre-commit"),
                "hook.pre-commit.enabled={value} must report disabled"
            );
        }

        // Explicit truthy values stay enabled.
        for value in ["true", "1", "on", "yes"] {
            git_config(
                dir.path(),
                &["--replace-all", "hook.pre-commit.enabled", value],
            )
            .unwrap();
            assert!(
                config_hooks_enabled(dir.path(), "pre-commit"),
                "hook.pre-commit.enabled={value} must report enabled"
            );
        }
    }
}
