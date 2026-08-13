//! `anvil uninstall` — remove Anvil from the project and optional user state.
//!
//! Conservative by design: confirms destructive steps and leaves unrelated
//! editor config alone unless it is a recognised anvil entry.

use std::fs;
use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use clap::Args;
use serde::Serialize;
use serde_json::Value;

use crate::GlobalArgs;
use crate::commands::hooks;
use crate::util::{atomic_write, refuse_if_parent_is_symlink, workspace_root};

/// Probe whether a path is "present" without following symlinks.
///
/// Unlike `Path::exists()`, this returns `true` for dangling symlinks
/// — so the uninstall planner sees them and the per-action refusal
/// (`remove_directory` / `remove_file`) gets a chance to surface them
/// instead of silently leaving stale symlinks behind. Used everywhere
/// the planner asks "should I include this in the plan?".
fn path_present(path: &Path) -> bool {
    fs::symlink_metadata(path).is_ok()
}

/// Stable MCP server-name key used by Anvil entries. Mirrors the
/// `SERVER_NAME` constant in `activation/mcp_client/cursor.rs` and
/// `activation/mcp_client/claude_code.rs`.
const MCP_SERVER_KEY: &str = "anvil";

#[derive(Debug, Args)]
#[command(
    about = "Remove anvil state from this project, or from the machine with --global.",
    after_help = "Scope:\n  - Default: current project only (.anvil/, the project config file, and anvil-managed hooks).\n  - --global: also removes user-level anvil state, credentials, anvil MCP entries, and the running daemon.\n  - The anvil binary is never removed; uninstall it with Homebrew, WinGet, Scoop, Cargo, or your installer path after cleaning state."
)]
#[allow(clippy::struct_excessive_bools)] // CLI flags, intentional shape.
pub struct UninstallArgs {
    /// Skip the interactive confirmation prompt.
    #[arg(long, short = 'y')]
    pub yes: bool,

    /// Show what would be removed without removing anything.
    #[arg(long, short = 'n')]
    pub dry_run: bool,

    /// Also remove user-level state: `~/.anvil/`, anvil MCP entries from
    /// `~/.claude.json` and `~/.cursor/mcp.json`, stored credentials, and daemon.
    #[arg(long)]
    pub global: bool,

    /// Do not edit MCP config files even when `--global` is set.
    #[arg(long)]
    pub keep_mcp: bool,

    /// Do not attempt to stop the running daemon.
    #[arg(long)]
    pub keep_daemon: bool,

    /// Continue past per-step errors instead of stopping.
    #[arg(long)]
    pub force: bool,
}

/// One uninstall operation. Each variant is independent — failure of
/// one does not invalidate the plan, only its own outcome.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum Action {
    StopDaemon {
        pid_file: PathBuf,
    },
    RemoveGitHooks,
    RemoveProjectAnvil {
        path: PathBuf,
    },
    RemoveAnvilrc {
        path: PathBuf,
    },
    RemoveMcpEntry {
        path: PathBuf,
        label: &'static str,
    },
    RemoveUserAnvil {
        path: PathBuf,
        install_root_scoped: bool,
    },
    RemoveCredentials {
        path: PathBuf,
    },
}

impl Action {
    fn describe(&self) -> String {
        match self {
            Self::StopDaemon { pid_file } => {
                format!("Stop daemon (pid file: {})", pid_file.display())
            }
            Self::RemoveGitHooks => "Remove anvil-managed git hooks".to_string(),
            Self::RemoveProjectAnvil { path } => {
                format!("Remove project state: {}", path.display())
            }
            Self::RemoveAnvilrc { path } => format!("Remove config file: {}", path.display()),
            Self::RemoveMcpEntry { path, label } => {
                format!(
                    "Remove anvil entry from {label} MCP config: {}",
                    path.display()
                )
            }
            Self::RemoveUserAnvil { path, .. } => {
                format!("Remove user state: {}", path.display())
            }
            Self::RemoveCredentials { path } => {
                format!("Remove credentials: {}", path.display())
            }
        }
    }
}

#[derive(Debug, Default, Serialize)]
struct Plan {
    actions: Vec<Action>,
}

#[derive(Debug, Clone, Serialize)]
struct ActionOutcome {
    action: Action,
    status: OutcomeStatus,
    detail: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
enum OutcomeStatus {
    Removed,
    NotPresent,
    Failed,
}

/// Top-level entry point invoked by `main.rs`.
pub fn run(args: &UninstallArgs, global: &GlobalArgs) -> Result<()> {
    // `--json` non-dry-run must be `--yes`. Interactive prompts in JSON
    // mode would emit human text to stderr while a script is parsing
    // stdout, and could hang on stdin reads in automation. Surface a
    // clear error instead.
    if global.json && !args.dry_run && !args.yes {
        anyhow::bail!(
            "`--json` requires `--yes` for non-dry-run invocations; \
             interactive confirmation is not supported in JSON mode"
        );
    }

    let cwd = std::env::current_dir().context("could not resolve current directory")?;
    let project_root = workspace_root().unwrap_or(cwd);
    // Match install's home resolution (honours USERPROFILE on Windows) so
    // uninstall removes MCP entries from the same home install wrote to.
    let home = crate::util::user_home_dir();
    let install_root = crate::install_root::install_root();
    let install_user_dir = install_root.user_dir();

    let plan = build_plan_with_install_user_dir(
        &project_root,
        home.as_deref(),
        install_user_dir.as_deref(),
        args,
    )?;

    // Human mode: print plan immediately. JSON mode defers all output
    // to the final envelope so stdout contains exactly one JSON
    // document.
    if !global.json {
        print_plan_human(&plan, &project_root, args);
    }

    if args.dry_run {
        if global.json {
            emit_json_envelope(&plan, &project_root, args, None);
        } else {
            eprintln!("\n(dry run — nothing was removed)");
        }
        return Ok(());
    }

    if plan_requires_project_write_gate(&project_root, &plan) {
        crate::install_root::ensure_project_write_allowed("uninstall")?;
    }

    if plan.actions.is_empty() {
        if global.json {
            emit_json_envelope(&plan, &project_root, args, Some(&[]));
        } else {
            eprintln!("Nothing to remove.");
            print_binary_hint();
        }
        return Ok(());
    }

    if !args.yes && !confirm()? {
        if !global.json {
            eprintln!("Aborted. Nothing was removed.");
        }
        return Ok(());
    }

    let outcomes = execute_plan(&plan, args.force);

    if global.json {
        emit_json_envelope(&plan, &project_root, args, Some(&outcomes));
    } else {
        print_outcomes_human(&outcomes);
        print_binary_hint();
    }

    let any_failed = outcomes
        .iter()
        .any(|o| matches!(o.status, OutcomeStatus::Failed));
    if any_failed && !args.force {
        anyhow::bail!(
            "one or more uninstall steps failed; re-run with --force to continue past errors"
        );
    }
    Ok(())
}

/// Build the destructive plan from the current filesystem state.
/// `path_present` rather than `exists()` is used throughout so dangling
/// symlinks are included in the plan and refused at execution rather
/// than silently skipped. Read-only — never modifies anything.
#[cfg(test)]
fn build_plan(project_root: &Path, home: Option<&Path>, args: &UninstallArgs) -> Result<Plan> {
    build_plan_with_install_user_dir(project_root, home, None, args)
}

/// True when the plan would mutate durable per-project state (ADR-060 gate).
fn plan_requires_project_write_gate(project_root: &Path, plan: &Plan) -> bool {
    plan.actions.iter().any(|action| match action {
        Action::RemoveProjectAnvil { .. } | Action::RemoveAnvilrc { .. } => true,
        Action::RemoveGitHooks => project_root.join(".git").exists(),
        _ => false,
    })
}

fn build_plan_with_install_user_dir(
    project_root: &Path,
    home: Option<&Path>,
    install_user_dir: Option<&Path>,
    args: &UninstallArgs,
) -> Result<Plan> {
    let mut actions = Vec::new();

    // 1) Git hooks — always attempted; the silent helper is a no-op
    //    when no managed hook (and/or no git repo) is present.
    actions.push(Action::RemoveGitHooks);

    // 2) Project-local state.
    let dot_anvil = project_root.join(".anvil");
    if path_present(&dot_anvil) {
        actions.push(Action::RemoveProjectAnvil { path: dot_anvil });
    }
    for name in [
        ".anvilrc",
        ".anvil.yaml",
        ".anvil.yml",
        ".anvil.json",
        ".anvil.toml",
    ] {
        let config = project_root.join(name);
        if path_present(&config) {
            actions.push(Action::RemoveAnvilrc { path: config });
        }
    }

    // 3) User-level state — only with `--global`. The daemon stop
    //    action lives in this branch because the user-level intercept
    //    daemon is shared across every Anvil-enabled project on the
    //    machine. Stopping it from a project-local uninstall would
    //    disrupt unrelated work; making it `--global`-only matches
    //    the documented project-scope default.
    if args.global {
        let home = home.context(
            "--global was requested but the user's home directory could not be \
             resolved; refusing to proceed so user-level state is not silently \
             skipped. Set $HOME (or %USERPROFILE% on Windows) and retry.",
        )?;

        // Daemon stop — user-level. Always-includes-if-pid-file-present
        // because every other Anvil session on the machine would block
        // a daemon stop otherwise. `--keep-daemon` is the escape hatch.
        if !args.keep_daemon
            && let Ok(pid_file) = anvil_intercept::default_pid_file_path()
            && path_present(&pid_file)
        {
            actions.push(Action::StopDaemon { pid_file });
        }

        // Editor MCP entries: surgical JSON edits only.
        if !args.keep_mcp {
            let claude = home.join(".claude.json");
            if path_present(&claude) {
                actions.push(Action::RemoveMcpEntry {
                    path: claude,
                    label: "Claude Code",
                });
            }
            let cursor = home.join(".cursor").join("mcp.json");
            if path_present(&cursor) {
                actions.push(Action::RemoveMcpEntry {
                    path: cursor,
                    label: "Cursor",
                });
            }
        }

        // User state directory. Default installs clean the historical
        // `~/.anvil/`; an ANVIL_HOME override cleans the active install-owned
        // user root (`<ANVIL_HOME>/user/`) and must not touch production's
        // default user state.
        let user_anvil = install_user_dir.map_or_else(|| home.join(".anvil"), Path::to_path_buf);
        if path_present(&user_anvil) {
            actions.push(Action::RemoveUserAnvil {
                path: user_anvil,
                install_root_scoped: install_user_dir.is_some(),
            });
        }

        // Auth credentials: see `auth/credentials.rs` for the
        // canonical XDG path with a macOS fallback. Windows uses
        // `dirs::config_dir()` so we add that candidate explicitly.
        if install_user_dir.is_none() {
            for candidate in credentials_candidates(home) {
                if path_present(&candidate) {
                    actions.push(Action::RemoveCredentials { path: candidate });
                }
            }
        }
    }

    Ok(Plan { actions })
}

/// Candidate credential paths to probe for `--global` removal. Mirrors
/// `auth::credentials::credentials_path` precedence (XDG → ~/.config →
/// macOS Application Support → Windows `dirs::config_dir()`).
fn credentials_candidates(home: &Path) -> Vec<PathBuf> {
    let mut v = Vec::new();
    if let Some(xdg) = std::env::var_os("XDG_CONFIG_HOME") {
        let p = PathBuf::from(xdg);
        if !p.as_os_str().is_empty() {
            v.push(p.join("anvil").join("credentials.json"));
        }
    }
    v.push(home.join(".config").join("anvil").join("credentials.json"));
    if cfg!(target_os = "macos") {
        v.push(
            home.join("Library")
                .join("Application Support")
                .join("anvil")
                .join("credentials.json"),
        );
    }
    // Windows: `dirs::config_dir()` returns `%APPDATA%` (Roaming).
    // `auth::credentials::credentials_dir()` uses this path, so we
    // mirror it here. Probing on every platform is cheap and avoids
    // an `else` branch that drifts from auth code.
    if let Some(cfg) = dirs::config_dir() {
        v.push(cfg.join("anvil").join("credentials.json"));
    }
    v
}

fn execute_plan(plan: &Plan, force: bool) -> Vec<ActionOutcome> {
    let mut outcomes = Vec::with_capacity(plan.actions.len());
    for action in &plan.actions {
        let outcome = execute_action(action);
        let failed = matches!(outcome.status, OutcomeStatus::Failed);
        outcomes.push(outcome);
        if failed && !force {
            break;
        }
    }
    outcomes
}

fn execute_action(action: &Action) -> ActionOutcome {
    let result: Result<(OutcomeStatus, String)> = match action {
        Action::StopDaemon { pid_file } => stop_daemon(pid_file),
        Action::RemoveGitHooks => remove_git_hooks(),
        Action::RemoveProjectAnvil { path } => remove_directory(path),
        Action::RemoveUserAnvil {
            path,
            install_root_scoped,
        } => {
            if *install_root_scoped {
                remove_install_root_user_directory(path)
            } else {
                remove_directory(path)
            }
        }
        Action::RemoveAnvilrc { path } | Action::RemoveCredentials { path } => remove_file(path),
        Action::RemoveMcpEntry { path, .. } => remove_mcp_entry(path),
    };

    match result {
        Ok((status, detail)) => ActionOutcome {
            action: action.clone(),
            status,
            detail,
        },
        Err(err) => ActionOutcome {
            action: action.clone(),
            status: OutcomeStatus::Failed,
            detail: format!("{err:#}"),
        },
    }
}

#[cfg(any(unix, windows))]
fn stop_daemon(_pid_file: &Path) -> Result<(OutcomeStatus, String)> {
    match anvil_intercept::request_daemon_stop()? {
        anvil_intercept::StopOutcome::Signalled { pid } => Ok((
            OutcomeStatus::Removed,
            format!("daemon stop requested (pid {pid})"),
        )),
        anvil_intercept::StopOutcome::NotRunning => {
            Ok((OutcomeStatus::NotPresent, "daemon not running".into()))
        }
        anvil_intercept::StopOutcome::StaleCleared { pid } => Ok((
            OutcomeStatus::Removed,
            format!("stale daemon pid file removed (pid {pid})"),
        )),
    }
}

#[cfg(not(any(unix, windows)))]
fn stop_daemon(pid_file: &Path) -> Result<(OutcomeStatus, String)> {
    let (status, detail) = remove_file(pid_file)?;
    let detail = match status {
        OutcomeStatus::Removed => {
            "removed pid file (daemon stop unsupported on this platform)".into()
        }
        OutcomeStatus::NotPresent => "pid file already gone".into(),
        OutcomeStatus::Failed => detail,
    };
    Ok((status, detail))
}

fn remove_git_hooks() -> Result<(OutcomeStatus, String)> {
    // The silent helper handles "not a git repository" by returning
    // Ok(()) (nothing to remove), so no error-string sniffing is
    // needed here. Real I/O failures still propagate.
    hooks::uninstall_all_managed_hooks_silent()?;
    Ok((OutcomeStatus::Removed, "managed hooks cleared".into()))
}

fn remove_directory(path: &Path) -> Result<(OutcomeStatus, String)> {
    // Probe with `symlink_metadata` so dangling symlinks register as
    // present rather than being silently treated as `NotPresent`. The
    // refusal below then names the symlink explicitly.
    let meta = match fs::symlink_metadata(path) {
        Ok(m) => m,
        Err(err) if err.kind() == io::ErrorKind::NotFound => {
            return Ok((OutcomeStatus::NotPresent, "already gone".into()));
        }
        Err(err) => return Err(err).context("stat path"),
    };
    if meta.file_type().is_symlink() {
        anyhow::bail!(
            "{} is a symlink (possibly dangling); refusing to follow. \
             Remove the symlink manually if you intended to clean it up.",
            path.display()
        );
    }
    fs::remove_dir_all(path).with_context(|| format!("removing {}", path.display()))?;
    Ok((OutcomeStatus::Removed, "directory removed".into()))
}

fn remove_install_root_user_directory(path: &Path) -> Result<(OutcomeStatus, String)> {
    refuse_if_parent_is_symlink(path).with_context(|| {
        format!(
            "refused to remove {}: ANVIL_HOME prefix is a symlink",
            path.display()
        )
    })?;
    remove_directory(path)
}

fn remove_file(path: &Path) -> Result<(OutcomeStatus, String)> {
    let meta = match fs::symlink_metadata(path) {
        Ok(m) => m,
        Err(err) if err.kind() == io::ErrorKind::NotFound => {
            return Ok((OutcomeStatus::NotPresent, "already gone".into()));
        }
        Err(err) => return Err(err).context("stat path"),
    };
    if meta.file_type().is_symlink() {
        anyhow::bail!(
            "{} is a symlink (possibly dangling); refusing to remove. \
             Remove the symlink manually if you intended to clean it up.",
            path.display()
        );
    }
    fs::remove_file(path).with_context(|| format!("removing {}", path.display()))?;
    Ok((OutcomeStatus::Removed, "file removed".into()))
}

/// Strip the `mcpServers.anvil` entry (if any) from a JSON config file,
/// preserving all other server entries and unrelated keys. Safe on
/// files that don't contain our entry or aren't JSON-parseable: those
/// return `NotPresent` or an error, never a corrupted write.
///
/// Symlink-hardened on two axes:
///
/// 1. The target file itself: if `~/.claude.json` or `~/.cursor/mcp.json`
///    is a symlink, we refuse rather than transparently follow it (an
///    `atomic_write` would replace the link with a regular file,
///    breaking whatever the user had set up).
/// 2. The parent directory: `atomic_write` writes its tempfile inside
///    the parent, so a symlinked `~/.cursor` could redirect the
///    write outside the user's home. The same parent-symlink guard
///    used by the activation install path (`refuse_if_parent_is_symlink`)
///    is applied here before any write.
fn remove_mcp_entry(path: &Path) -> Result<(OutcomeStatus, String)> {
    let meta = match fs::symlink_metadata(path) {
        Ok(m) => m,
        Err(err) if err.kind() == io::ErrorKind::NotFound => {
            return Ok((OutcomeStatus::NotPresent, "file does not exist".into()));
        }
        Err(err) => return Err(err).context("stat MCP config path"),
    };
    if meta.file_type().is_symlink() {
        anyhow::bail!(
            "{} is a symlink; refusing to follow. \
             Edit the link target manually to remove the `{MCP_SERVER_KEY}` entry.",
            path.display()
        );
    }

    let raw = fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;

    let mut value: Value = match serde_json::from_str(&raw) {
        Ok(v) => v,
        Err(err) => {
            anyhow::bail!(
                "could not parse JSON in {}: {err}. Edit the file manually to remove the `{MCP_SERVER_KEY}` entry.",
                path.display()
            );
        }
    };

    let Some(servers) = value.get_mut("mcpServers").and_then(Value::as_object_mut) else {
        return Ok((
            OutcomeStatus::NotPresent,
            "no `mcpServers` key in file".into(),
        ));
    };

    if servers.remove(MCP_SERVER_KEY).is_none() {
        return Ok((
            OutcomeStatus::NotPresent,
            format!("no `{MCP_SERVER_KEY}` entry in `mcpServers`"),
        ));
    }

    // Parent-symlink guard before atomic_write: stops a symlinked
    // `~/.cursor/` (or similar) parent directory from redirecting the
    // tempfile-rename outside the user's intended location. Mirrors
    // the guard used by the activation install path.
    refuse_if_parent_is_symlink(path).with_context(|| {
        format!(
            "refused to write {}: parent directory is a symlink",
            path.display()
        )
    })?;

    let rendered = serde_json::to_vec_pretty(&value).context("re-serialising MCP config")?;
    atomic_write(path, &rendered).with_context(|| format!("writing {}", path.display()))?;
    Ok((
        OutcomeStatus::Removed,
        format!("`{MCP_SERVER_KEY}` entry removed"),
    ))
}

fn print_plan_human(plan: &Plan, project_root: &Path, args: &UninstallArgs) {
    if plan.actions.is_empty() {
        eprintln!("Nothing to remove for {}.", project_root.display());
        return;
    }
    eprintln!("Will remove the following from {}:", project_root.display());
    for action in &plan.actions {
        eprintln!("  - {}", action.describe());
    }
    if !args.global {
        eprintln!(
            "\nUser-level state (~/.anvil/, credentials, MCP entries, user-level \
             daemon) preserved. Pass --global to remove those as well."
        );
    }
}

fn print_outcomes_human(outcomes: &[ActionOutcome]) {
    eprintln!();
    for outcome in outcomes {
        let prefix = match outcome.status {
            OutcomeStatus::Removed => "✓",
            OutcomeStatus::NotPresent => "·",
            OutcomeStatus::Failed => "✗",
        };
        eprintln!(
            "  {prefix} {} — {}",
            outcome.action.describe(),
            outcome.detail
        );
    }
}

/// Emit the entire result as a single JSON document on stdout. Used by
/// `--json` mode so callers parse exactly one envelope. `outcomes` is
/// `None` for `--dry-run` (plan only), `Some(&[])` when the plan is
/// empty, and `Some(...)` otherwise.
fn emit_json_envelope(
    plan: &Plan,
    project_root: &Path,
    args: &UninstallArgs,
    outcomes: Option<&[ActionOutcome]>,
) {
    let payload = serde_json::json!({
        "project_root": project_root.display().to_string(),
        "global": args.global,
        "dry_run": args.dry_run,
        "actions": plan.actions,
        "outcomes": outcomes,
    });
    println!("{payload}");
}

fn confirm() -> Result<bool> {
    eprint!("\nProceed? [y/N] ");
    io::stderr().flush().ok();
    let mut line = String::new();
    io::stdin().lock().read_line(&mut line)?;
    let answer = line.trim().to_ascii_lowercase();
    Ok(answer == "y" || answer == "yes")
}

fn print_binary_hint() {
    let exe = std::env::current_exe().ok();
    let path_str = exe.as_ref().map(|p| p.display().to_string());

    if let Some(p) = &path_str {
        if is_homebrew_path(p) {
            eprintln!(
                "\nThe `anvil` binary itself is managed by Homebrew at {p}.\n\
                 Remove it with: brew uninstall eddacraft/tap/anvil"
            );
            return;
        }
        if p.contains("/.cargo/bin/") {
            eprintln!(
                "\nThe `anvil` binary itself was installed via cargo at {p}.\n\
                 Remove it with: cargo uninstall eddacraft-anvil"
            );
            return;
        }
        if is_curl_installer_path(p) {
            eprintln!(
                "\nThe `anvil` binary itself was installed via the curl installer at {p}.\n\
                 Remove it with: rm {p}\n\
                 (or re-run `anvil update` after reinstalling to refresh the binary in place)."
            );
            return;
        }
        if is_scoop_path(p) {
            eprintln!(
                "\nThe `anvil` binary itself is managed by Scoop at {p}.\n\
                 Remove it with: scoop uninstall anvil"
            );
            return;
        }
        if is_winget_path(p) {
            eprintln!(
                "\nThe `anvil` binary itself was installed via WinGet at {p}.\n\
                 Remove it with: winget uninstall eddacraft.anvil"
            );
            return;
        }
        eprintln!(
            "\nThe `anvil` binary itself is at {p}. Remove it manually if you \
             do not plan to reinstall."
        );
    } else {
        eprintln!(
            "\nThe `anvil` binary itself was not detected; remove it manually if you \
             do not plan to reinstall."
        );
    }
}

/// Detect the curl-installer (axoupdater sidecar) install layout.
/// Covers the documented default `~/.eddacraft/bin/anvil` and the
/// `$ANVIL_INSTALL_DIR` / `$CARGO_DIST_FORCE_INSTALL_DIR` overrides
/// that cargo-dist emits, plus the `~/.local/bin/anvil` fallback the
/// installer falls back to when neither is set.
fn is_curl_installer_path(p: &str) -> bool {
    p.contains("/.eddacraft/bin/anvil")
        || p.contains("/.local/bin/anvil")
        || p.contains("/eddacraft-anvil/bin/anvil")
}

fn is_homebrew_path(p: &str) -> bool {
    p.contains("/Cellar/anvil/")
        || p.contains("/Cellar/eddacraft-anvil/")
        || p.contains("/opt/homebrew/bin/anvil")
        || p.contains("/usr/local/bin/anvil") && p.contains("brew")
        || p.contains("/home/linuxbrew/.linuxbrew/bin/anvil")
}

/// Detect the Scoop install layout. Scoop drops binaries under
/// `<scoop_root>/apps/anvil/<version>/` with a shim at
/// `<scoop_root>/shims/anvil.exe`; `<scoop_root>` defaults to `~/scoop`
/// but can be relocated via `$SCOOP`, so we match on the trailing
/// layout rather than the root. Path comes back from `current_exe()`
/// with backslashes on Windows, so we normalise to forward-slash +
/// lowercase first (Windows paths are case-insensitive). The `apps`
/// arm is anchored with a trailing slash and the `shims` arm with the
/// exact filename so neighbour shims like `anvil-helper.exe` don't
/// false-match.
fn is_scoop_path(p: &str) -> bool {
    let n = p.replace('\\', "/").to_ascii_lowercase();
    n.contains("/scoop/apps/anvil/") || n.contains("/scoop/shims/anvil.exe")
}

/// Detect the `WinGet` portable install layout. `WinGet` portable
/// packages land in `%LOCALAPPDATA%\Microsoft\WinGet\Packages\<id>_<source>\`
/// with a shim under `%LOCALAPPDATA%\Microsoft\WinGet\Links\`. We match
/// either, since `current_exe()` may resolve to the real binary or to
/// the alias depending on how the user launched it. Both arms are
/// anchored to filename/separator boundaries so a hypothetical
/// `eddacraft.anvil-extra` or `Links\anvil-foo.exe` doesn't false-match.
fn is_winget_path(p: &str) -> bool {
    let n = p.replace('\\', "/").to_ascii_lowercase();
    n.contains("/winget/packages/eddacraft.anvil_") || n.contains("/winget/links/anvil.exe")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn write(path: &Path, content: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, content).unwrap();
    }

    #[test]
    fn build_plan_includes_existing_project_state() {
        // legacy-fallback coverage (.anvilrc deliberately) — uninstall must
        // plan removal of the legacy config file, not only `.anvil.<ext>`.
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        write(&root.join(".anvilrc"), "[anvil]\n");
        write(&root.join(".anvil").join("baseline.json"), "{}");

        let args = UninstallArgs {
            yes: false,
            dry_run: true,
            global: false,
            keep_mcp: false,
            keep_daemon: true,
            force: false,
        };
        let plan = build_plan(root, None, &args).unwrap();
        assert!(
            plan.actions
                .iter()
                .any(|a| matches!(a, Action::RemoveAnvilrc { .. }))
        );
        assert!(
            plan.actions
                .iter()
                .any(|a| matches!(a, Action::RemoveProjectAnvil { .. }))
        );
        // Hooks step is always queued; daemon step is suppressed.
        assert!(
            plan.actions
                .iter()
                .any(|a| matches!(a, Action::RemoveGitHooks))
        );
        assert!(
            !plan
                .actions
                .iter()
                .any(|a| matches!(a, Action::StopDaemon { .. }))
        );
    }

    #[test]
    fn build_plan_skips_missing_project_state() {
        let tmp = TempDir::new().unwrap();
        let args = UninstallArgs {
            yes: true,
            dry_run: true,
            global: false,
            keep_mcp: false,
            keep_daemon: true,
            force: false,
        };
        let plan = build_plan(tmp.path(), None, &args).unwrap();
        // Only the hooks action is queued; nothing else exists.
        assert_eq!(plan.actions.len(), 1);
        assert!(matches!(plan.actions[0], Action::RemoveGitHooks));
    }

    #[test]
    fn build_plan_with_global_includes_user_state_when_present() {
        let tmp = TempDir::new().unwrap();
        let home = tmp.path();
        write(&home.join(".claude.json"), r#"{"mcpServers":{"anvil":{}}}"#);
        write(
            &home.join(".cursor").join("mcp.json"),
            r#"{"mcpServers":{"anvil":{}}}"#,
        );
        fs::create_dir_all(home.join(".anvil")).unwrap();

        let args = UninstallArgs {
            yes: true,
            dry_run: true,
            global: true,
            keep_mcp: false,
            keep_daemon: true,
            force: false,
        };
        let project_root = TempDir::new().unwrap();
        let plan = build_plan(project_root.path(), Some(home), &args).unwrap();
        assert!(plan.actions.iter().any(|a| matches!(
            a,
            Action::RemoveMcpEntry {
                label: "Claude Code",
                ..
            }
        )));
        assert!(plan.actions.iter().any(|a| matches!(
            a,
            Action::RemoveMcpEntry {
                label: "Cursor",
                ..
            }
        )));
        assert!(
            plan.actions
                .iter()
                .any(|a| matches!(a, Action::RemoveUserAnvil { .. }))
        );
    }

    #[test]
    fn global_under_anvil_home_removes_prefix_user_dir_only() {
        let tmp = TempDir::new().unwrap();
        let home = tmp.path().join("prod-home");
        let prefix = tmp.path().join("candidate");
        fs::create_dir_all(home.join(".anvil")).unwrap();
        fs::create_dir_all(prefix.join("user")).unwrap();
        write(
            &home.join(".config").join("anvil").join("credentials.json"),
            "{}",
        );
        write(&prefix.join("user").join("credentials.json"), "{}");

        let args = UninstallArgs {
            yes: true,
            dry_run: true,
            global: true,
            keep_mcp: true,
            keep_daemon: true,
            force: false,
        };
        let project_root = TempDir::new().unwrap();
        let plan = build_plan_with_install_user_dir(
            project_root.path(),
            Some(&home),
            Some(&prefix.join("user")),
            &args,
        )
        .unwrap();

        assert!(plan.actions.iter().any(|a| matches!(
            a,
            Action::RemoveUserAnvil {
                path,
                install_root_scoped: true,
            } if path == &prefix.join("user")
        )));
        assert!(
            !plan
                .actions
                .iter()
                .any(|a| matches!(a, Action::RemoveUserAnvil { path, .. } if path == &home.join(".anvil"))),
            "ANVIL_HOME uninstall must not plan production ~/.anvil removal"
        );
        assert!(
            !plan.actions.iter().any(|a| matches!(
                a,
                Action::RemoveCredentials { path } if path.starts_with(&home)
            )),
            "ANVIL_HOME uninstall must not plan production credential removal"
        );
    }

    #[test]
    fn global_with_keep_mcp_omits_mcp_actions() {
        let tmp = TempDir::new().unwrap();
        let home = tmp.path();
        write(&home.join(".claude.json"), r#"{"mcpServers":{"anvil":{}}}"#);
        let project_root = TempDir::new().unwrap();
        let args = UninstallArgs {
            yes: true,
            dry_run: true,
            global: true,
            keep_mcp: true,
            keep_daemon: true,
            force: false,
        };
        let plan = build_plan(project_root.path(), Some(home), &args).unwrap();
        assert!(
            !plan
                .actions
                .iter()
                .any(|a| matches!(a, Action::RemoveMcpEntry { .. }))
        );
    }

    #[test]
    fn remove_mcp_entry_preserves_other_servers() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("mcp.json");
        write(
            &path,
            r#"{"mcpServers":{"anvil":{"command":"/x/anvil"},"other":{"command":"/y/other"}},"unrelated":42}"#,
        );

        let (status, _) = remove_mcp_entry(&path).unwrap();
        assert!(matches!(status, OutcomeStatus::Removed));

        let after: Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        let servers = after.get("mcpServers").unwrap().as_object().unwrap();
        assert!(!servers.contains_key("anvil"));
        assert!(servers.contains_key("other"));
        assert_eq!(after.get("unrelated").and_then(Value::as_i64), Some(42));
    }

    #[test]
    fn remove_mcp_entry_is_no_op_when_entry_absent() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("mcp.json");
        write(&path, r#"{"mcpServers":{"other":{}}}"#);

        let (status, _) = remove_mcp_entry(&path).unwrap();
        assert!(matches!(status, OutcomeStatus::NotPresent));

        let after = fs::read_to_string(&path).unwrap();
        assert!(after.contains("other"));
    }

    #[test]
    fn remove_mcp_entry_handles_missing_file() {
        let tmp = TempDir::new().unwrap();
        let (status, _) = remove_mcp_entry(&tmp.path().join("nope.json")).unwrap();
        assert!(matches!(status, OutcomeStatus::NotPresent));
    }

    #[test]
    fn remove_mcp_entry_refuses_malformed_json() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("mcp.json");
        write(&path, "not json");
        let err = remove_mcp_entry(&path).unwrap_err();
        assert!(format!("{err:#}").contains("could not parse JSON"));
    }

    #[test]
    fn remove_directory_refuses_symlinks() {
        #[cfg(unix)]
        {
            let tmp = TempDir::new().unwrap();
            let real_dir = tmp.path().join("real");
            fs::create_dir(&real_dir).unwrap();
            let link = tmp.path().join("link");
            std::os::unix::fs::symlink(&real_dir, &link).unwrap();
            let err = remove_directory(&link).unwrap_err();
            assert!(format!("{err:#}").contains("symlink"));
            // Real dir must still exist.
            assert!(real_dir.exists());
        }
    }

    #[test]
    fn remove_install_root_user_directory_refuses_symlinked_prefix() {
        #[cfg(unix)]
        {
            let tmp = TempDir::new().unwrap();
            let outside = TempDir::new().unwrap();
            fs::create_dir_all(outside.path().join("user")).unwrap();
            write(&outside.path().join("user").join("keep.txt"), "keep");
            let link_prefix = tmp.path().join("candidate-link");
            std::os::unix::fs::symlink(outside.path(), &link_prefix).unwrap();

            let err = remove_install_root_user_directory(&link_prefix.join("user")).unwrap_err();
            let msg = format!("{err:#}");
            assert!(
                msg.contains("symlink"),
                "expected symlinked ANVIL_HOME prefix refusal; got {msg}"
            );
            assert!(
                outside.path().join("user").join("keep.txt").exists(),
                "outside target must not be deleted"
            );
        }
    }

    #[test]
    fn remove_file_returns_not_present_when_missing() {
        let tmp = TempDir::new().unwrap();
        let (status, _) = remove_file(&tmp.path().join("nope")).unwrap();
        assert!(matches!(status, OutcomeStatus::NotPresent));
    }

    #[test]
    fn credentials_candidates_includes_xdg_when_set() {
        let home = PathBuf::from("/home/u");
        let v = temp_env::with_var("XDG_CONFIG_HOME", Some("/xdg"), || {
            credentials_candidates(&home)
        });
        assert!(v.iter().any(|p| p.starts_with("/xdg")));
        assert!(v.iter().any(|p| p.starts_with("/home/u/.config")));
    }

    #[test]
    fn daemon_stop_is_not_planned_without_global() {
        // Even if a pid file exists, the project-scoped default must
        // not include a StopDaemon action — the daemon is user-level
        // and stopping it would disrupt every other Anvil-enabled
        // project on the machine.
        let tmp = TempDir::new().unwrap();
        let args = UninstallArgs {
            yes: true,
            dry_run: true,
            global: false,
            keep_mcp: false,
            keep_daemon: false,
            force: false,
        };
        let plan = build_plan(tmp.path(), None, &args).unwrap();
        assert!(
            !plan
                .actions
                .iter()
                .any(|a| matches!(a, Action::StopDaemon { .. }))
        );
    }

    #[test]
    fn build_plan_errors_when_global_set_without_home() {
        // Silently skipping global cleanup when `dirs::home_dir()`
        // returns `None` would let an uninstall succeed while leaving
        // user-level state behind. Refuse instead.
        let tmp = TempDir::new().unwrap();
        let args = UninstallArgs {
            yes: true,
            dry_run: true,
            global: true,
            keep_mcp: false,
            keep_daemon: true,
            force: false,
        };
        let err = build_plan(tmp.path(), None, &args).unwrap_err();
        let msg = format!("{err:#}");
        assert!(
            msg.contains("home directory could not be resolved"),
            "expected message about home directory; got: {msg}"
        );
    }

    #[test]
    fn build_plan_picks_up_dangling_symlinks() {
        // exists() returns false for dangling symlinks; switching to
        // symlink_metadata() lets the planner see them so the per-
        // action refusal can surface them at execution time.
        #[cfg(unix)]
        {
            let tmp = TempDir::new().unwrap();
            let root = tmp.path();
            let dangling = root.join(".anvil.yaml");
            std::os::unix::fs::symlink("/nonexistent/path/to/anywhere", &dangling).unwrap();
            assert!(!dangling.exists(), "fixture: dangling symlink expected");
            assert!(
                path_present(&dangling),
                "path_present should detect a dangling symlink"
            );

            let args = UninstallArgs {
                yes: true,
                dry_run: true,
                global: false,
                keep_mcp: false,
                keep_daemon: true,
                force: false,
            };
            let plan = build_plan(root, None, &args).unwrap();
            assert!(
                plan.actions
                    .iter()
                    .any(|a| matches!(a, Action::RemoveAnvilrc { .. })),
                "dangling .anvil.yaml symlink should appear in plan"
            );
        }
    }

    #[test]
    fn remove_file_refuses_dangling_symlinks() {
        // The per-action refusal then names the symlink rather than
        // silently leaving it behind as "already gone".
        #[cfg(unix)]
        {
            let tmp = TempDir::new().unwrap();
            let dangling = tmp.path().join("link");
            std::os::unix::fs::symlink("/no/such/target", &dangling).unwrap();
            let err = remove_file(&dangling).unwrap_err();
            let msg = format!("{err:#}");
            assert!(
                msg.contains("symlink"),
                "expected symlink refusal; got {msg}"
            );
            // Symlink must still be in place after refusal.
            assert!(fs::symlink_metadata(&dangling).is_ok());
        }
    }

    #[test]
    fn remove_mcp_entry_refuses_symlinked_target() {
        #[cfg(unix)]
        {
            let tmp = TempDir::new().unwrap();
            let real_target = tmp.path().join("real.json");
            write(&real_target, r#"{"mcpServers":{"anvil":{}}}"#);
            let link = tmp.path().join("link.json");
            std::os::unix::fs::symlink(&real_target, &link).unwrap();

            let err = remove_mcp_entry(&link).unwrap_err();
            let msg = format!("{err:#}");
            assert!(
                msg.contains("symlink"),
                "expected symlink refusal; got {msg}"
            );

            // Real target is unmodified.
            let after = fs::read_to_string(&real_target).unwrap();
            assert!(after.contains("anvil"));
        }
    }

    #[test]
    fn is_scoop_path_matches_default_and_shim_layouts() {
        // Default per-user install (forward slashes — what we'd see
        // after current_exe().display() on Windows is backslashed, so
        // both forms must match).
        assert!(is_scoop_path(
            "C:/Users/alice/scoop/apps/anvil/current/anvil.exe"
        ));
        assert!(is_scoop_path(
            r"C:\Users\alice\scoop\apps\anvil\0.7.0-beta\anvil.exe"
        ));
        // Shim layout (what callers usually invoke).
        assert!(is_scoop_path(r"C:\Users\alice\scoop\shims\anvil.exe"));
        // Relocated $SCOOP root — the layout suffix is the same.
        assert!(is_scoop_path(
            r"D:\tools\scoop\apps\anvil\current\anvil.exe"
        ));
        // Case-insensitive (Windows paths).
        assert!(is_scoop_path(
            r"C:\Users\Alice\Scoop\Apps\Anvil\current\anvil.exe"
        ));
    }

    #[test]
    fn is_scoop_path_rejects_unrelated_layouts() {
        assert!(!is_scoop_path("/opt/homebrew/bin/anvil"));
        assert!(!is_scoop_path("/home/u/.cargo/bin/anvil"));
        // Different scoop app — must not false-positive on substrings.
        assert!(!is_scoop_path(
            r"C:\Users\alice\scoop\apps\ripgrep\current\rg.exe"
        ));
        // Neighbour shims that start with "anvil" must not match —
        // shim arm is anchored to the exact filename `anvil.exe`.
        assert!(!is_scoop_path(
            r"C:\Users\alice\scoop\shims\anvil-helper.exe"
        ));
        assert!(!is_scoop_path(r"C:\Users\alice\scoop\shims\anvil2.exe"));
    }

    #[test]
    fn is_winget_path_matches_package_and_link_layouts() {
        // Real binary inside the WinGet packages tree (source suffix
        // varies, so we only anchor on the package id prefix).
        assert!(is_winget_path(
            r"C:\Users\alice\AppData\Local\Microsoft\WinGet\Packages\eddacraft.anvil_Microsoft.Winget.Source_8wekyb3d8bbwe\anvil.exe"
        ));
        // WinGet portable alias link.
        assert!(is_winget_path(
            r"C:\Users\alice\AppData\Local\Microsoft\WinGet\Links\anvil.exe"
        ));
        // Case-insensitive.
        assert!(is_winget_path(
            r"C:\Users\Alice\AppData\Local\Microsoft\WinGet\Packages\Eddacraft.Anvil_x\anvil.exe"
        ));
    }

    #[test]
    fn is_winget_path_rejects_unrelated_layouts() {
        assert!(!is_winget_path("/opt/homebrew/bin/anvil"));
        // A different WinGet package must not match.
        assert!(!is_winget_path(
            r"C:\Users\alice\AppData\Local\Microsoft\WinGet\Packages\other.tool_x\other.exe"
        ));
        // Hypothetical sibling package whose id is a prefix of ours
        // must not match — packages arm is anchored on the `_` source
        // separator.
        assert!(!is_winget_path(
            r"C:\Users\alice\AppData\Local\Microsoft\WinGet\Packages\eddacraft.anvil-extra_x\x.exe"
        ));
        // Neighbour link with a name starting with "anvil" must not
        // match — links arm is anchored to the exact filename.
        assert!(!is_winget_path(
            r"C:\Users\alice\AppData\Local\Microsoft\WinGet\Links\anvil-helper.exe"
        ));
    }
}
