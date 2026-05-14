//! `anvil uninstall` — remove Anvil from the current project and
//! optionally from user-level state.
//!
//! Built for beta users who need to reinstall cleanly. The command is
//! deliberately conservative:
//!
//! - Defaults to the **current project only** — `.anvil/`, `.anvilrc`,
//!   and Anvil-managed git hooks. User state under `~/.anvil/`,
//!   credentials, and editor MCP entries are touched only with
//!   `--global`.
//! - Confirms before deleting unless `--yes` is passed.
//! - `--dry-run` shows the plan and exits without modifying anything.
//! - On error, stops and reports unless `--force` is set.
//! - Does **not** remove the `anvil` binary itself. The command prints
//!   a per-install-method hint (Homebrew / curl-installer / cargo) at
//!   the end.
//!
//! The implementation is a thin orchestrator over existing primitives:
//! [`commands::hooks::uninstall_all_managed_hooks`] for git hooks, the
//! `anvil-intercept` PID file convention for the daemon, and surgical
//! JSON edits for MCP config files (so other server entries are
//! preserved).

use std::fs;
use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use clap::Args;
use serde::Serialize;
use serde_json::Value;

use crate::GlobalArgs;
use crate::commands::hooks;
use crate::util::{atomic_write, workspace_root};

/// Stable MCP server-name key used by Anvil entries. Mirrors the
/// `SERVER_NAME` constant in `activation/mcp_client/cursor.rs` and
/// `activation/mcp_client/claude_code.rs`.
const MCP_SERVER_KEY: &str = "anvil";

#[derive(Debug, Args)]
#[allow(clippy::struct_excessive_bools)] // CLI flags, intentional shape.
pub struct UninstallArgs {
    /// Skip the interactive confirmation prompt.
    #[arg(long, short = 'y')]
    pub yes: bool,

    /// Show what would be removed without removing anything.
    #[arg(long, short = 'n')]
    pub dry_run: bool,

    /// Also remove user-level state: `~/.anvil/`, Anvil MCP entries from
    /// `~/.claude.json` and `~/.cursor/mcp.json`, and stored credentials.
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
    StopDaemon { pid_file: PathBuf },
    RemoveGitHooks,
    RemoveProjectAnvil { path: PathBuf },
    RemoveAnvilrc { path: PathBuf },
    RemoveMcpEntry { path: PathBuf, label: &'static str },
    RemoveUserAnvil { path: PathBuf },
    RemoveCredentials { path: PathBuf },
}

impl Action {
    fn describe(&self) -> String {
        match self {
            Self::StopDaemon { pid_file } => {
                format!("Stop daemon (pid file: {})", pid_file.display())
            }
            Self::RemoveGitHooks => "Remove Anvil-managed git hooks".to_string(),
            Self::RemoveProjectAnvil { path } => {
                format!("Remove project state: {}", path.display())
            }
            Self::RemoveAnvilrc { path } => format!("Remove config file: {}", path.display()),
            Self::RemoveMcpEntry { path, label } => {
                format!(
                    "Remove Anvil entry from {label} MCP config: {}",
                    path.display()
                )
            }
            Self::RemoveUserAnvil { path } => {
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

#[derive(Debug, Serialize)]
struct ActionOutcome {
    action: Action,
    status: OutcomeStatus,
    detail: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
enum OutcomeStatus {
    Removed,
    NotPresent,
    Failed,
}

/// Top-level entry point invoked by `main.rs`.
pub fn run(args: &UninstallArgs, global: &GlobalArgs) -> Result<()> {
    let cwd = std::env::current_dir().context("could not resolve current directory")?;
    let project_root = workspace_root().unwrap_or(cwd);
    let home = dirs::home_dir();

    let plan = build_plan(&project_root, home.as_deref(), args)?;

    print_plan(&plan, &project_root, args, global.json);

    if args.dry_run {
        if !global.json {
            eprintln!("\n(dry run — nothing was removed)");
        }
        return Ok(());
    }

    if plan.actions.is_empty() {
        if !global.json {
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

    let outcomes = execute_plan(&plan, args.force, global);
    print_outcomes(&outcomes, global.json);

    if !global.json {
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

/// Build the destructive plan from the current filesystem state. Only
/// includes actions for things that exist (or, for the daemon, for
/// which a PID file is present). Read-only — never modifies anything.
#[allow(clippy::unnecessary_wraps)] // Result reserved for future fallible discovery steps.
fn build_plan(project_root: &Path, home: Option<&Path>, args: &UninstallArgs) -> Result<Plan> {
    let mut actions = Vec::new();

    // 1) Daemon — only included when a PID file exists.
    if !args.keep_daemon
        && let Ok(pid_file) = anvil_intercept::default_pid_file_path()
        && pid_file.exists()
    {
        actions.push(Action::StopDaemon { pid_file });
    }

    // 2) Git hooks — always attempted; the underlying call is a no-op
    //    when no managed hook is present.
    actions.push(Action::RemoveGitHooks);

    // 3) Project-local state.
    let dot_anvil = project_root.join(".anvil");
    if dot_anvil.exists() {
        actions.push(Action::RemoveProjectAnvil { path: dot_anvil });
    }
    let anvilrc = project_root.join(".anvilrc");
    if anvilrc.exists() {
        actions.push(Action::RemoveAnvilrc { path: anvilrc });
    }

    // 4) User-level state — only with --global.
    if args.global
        && let Some(home) = home
    {
        // Editor MCP entries: surgical JSON edits only.
        if !args.keep_mcp {
            let claude = home.join(".claude.json");
            if claude.exists() {
                actions.push(Action::RemoveMcpEntry {
                    path: claude,
                    label: "Claude Code",
                });
            }
            let cursor = home.join(".cursor").join("mcp.json");
            if cursor.exists() {
                actions.push(Action::RemoveMcpEntry {
                    path: cursor,
                    label: "Cursor",
                });
            }
        }

        // User state directory: `~/.anvil/` (project caches,
        // per-user activation marker, etc.).
        let user_anvil = home.join(".anvil");
        if user_anvil.exists() {
            actions.push(Action::RemoveUserAnvil { path: user_anvil });
        }

        // Auth credentials: see `auth/credentials.rs` for the
        // canonical XDG path with a macOS fallback.
        for candidate in credentials_candidates(home) {
            if candidate.exists() {
                actions.push(Action::RemoveCredentials { path: candidate });
            }
        }
    }

    Ok(Plan { actions })
}

/// Candidate credential paths to probe for `--global` removal. Mirrors
/// `auth::credentials::credentials_path` precedence.
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
    v
}

fn execute_plan(plan: &Plan, force: bool, global: &GlobalArgs) -> Vec<ActionOutcome> {
    let mut outcomes = Vec::with_capacity(plan.actions.len());
    for action in &plan.actions {
        let outcome = execute_action(action, global);
        let failed = matches!(outcome.status, OutcomeStatus::Failed);
        outcomes.push(outcome);
        if failed && !force {
            break;
        }
    }
    outcomes
}

fn execute_action(action: &Action, global: &GlobalArgs) -> ActionOutcome {
    let result: Result<(OutcomeStatus, String)> = match action {
        Action::StopDaemon { pid_file } => stop_daemon(pid_file),
        Action::RemoveGitHooks => remove_git_hooks(global),
        Action::RemoveProjectAnvil { path } | Action::RemoveUserAnvil { path } => {
            remove_directory(path)
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

#[cfg(unix)]
fn stop_daemon(pid_file: &Path) -> Result<(OutcomeStatus, String)> {
    let raw = match fs::read_to_string(pid_file) {
        Ok(s) => s,
        Err(err) if err.kind() == io::ErrorKind::NotFound => {
            return Ok((OutcomeStatus::NotPresent, "pid file already gone".into()));
        }
        Err(err) => return Err(err).context("reading pid file"),
    };

    // Pid files written by `anvil intercept` start with the PID on the
    // first line. Parse leniently to tolerate older formats.
    let pid: i32 = raw
        .lines()
        .next()
        .unwrap_or("")
        .trim()
        .parse()
        .context("pid file did not start with a numeric pid")?;

    // SIGTERM first; if the process is already gone, kill(1) exits
    // non-zero and we treat that as success.
    let term_sent = send_signal(pid, "TERM");
    if !term_sent {
        let _ = fs::remove_file(pid_file);
        return Ok((
            OutcomeStatus::Removed,
            format!("daemon (pid {pid}) was not running; cleaned pid file"),
        ));
    }

    // Best-effort wait for the daemon to exit. 5 attempts × 200ms.
    for _ in 0..5 {
        std::thread::sleep(std::time::Duration::from_millis(200));
        if !process_exists(pid) {
            let _ = fs::remove_file(pid_file);
            return Ok((
                OutcomeStatus::Removed,
                format!("daemon stopped (pid {pid})"),
            ));
        }
    }

    // Fall back to SIGKILL — beta users want a clean state above all.
    let _ = send_signal(pid, "KILL");
    let _ = fs::remove_file(pid_file);
    Ok((
        OutcomeStatus::Removed,
        format!("daemon force-killed (pid {pid})"),
    ))
}

#[cfg(not(unix))]
fn stop_daemon(pid_file: &Path) -> Result<(OutcomeStatus, String)> {
    // On Windows the daemon shutdown surface lands with INTD-002; for
    // now we just remove the stale pid file so reinstall is unblocked.
    if pid_file.exists() {
        fs::remove_file(pid_file).context("removing pid file")?;
        return Ok((
            OutcomeStatus::Removed,
            "removed pid file (kill not supported on this platform)".into(),
        ));
    }
    Ok((OutcomeStatus::NotPresent, "pid file already gone".into()))
}

/// Send a POSIX signal to `pid` by shelling out to `kill(1)`. Returns
/// `true` when the signal was delivered, `false` when the process was
/// already gone (or `kill(1)` is unavailable). Avoids any `unsafe`
/// extern "C" binding to satisfy the crate-level `forbid(unsafe_code)`.
#[cfg(unix)]
fn send_signal(pid: i32, sig: &str) -> bool {
    std::process::Command::new("kill")
        .arg(format!("-{sig}"))
        .arg(pid.to_string())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
}

/// Probe whether a process is still alive using `kill -0 <pid>`.
#[cfg(unix)]
fn process_exists(pid: i32) -> bool {
    send_signal(pid, "0")
}

fn remove_git_hooks(global: &GlobalArgs) -> Result<(OutcomeStatus, String)> {
    match hooks::uninstall_all_managed_hooks(global) {
        Ok(()) => Ok((OutcomeStatus::Removed, "managed hooks cleared".into())),
        // "not a git repository" is a normal state when uninstalling
        // from a stuck install outside a repo — treat as no-op.
        Err(err) => {
            let msg = format!("{err:#}").to_lowercase();
            if msg.contains("not a git repository") || msg.contains("no .git") {
                Ok((OutcomeStatus::NotPresent, "not in a git repository".into()))
            } else {
                Err(err)
            }
        }
    }
}

fn remove_directory(path: &Path) -> Result<(OutcomeStatus, String)> {
    if !path.exists() {
        return Ok((OutcomeStatus::NotPresent, "already gone".into()));
    }
    // Refuse symlinks — never follow a hostile or stale symlink out of
    // the project root or home directory.
    let meta = fs::symlink_metadata(path).context("stat path")?;
    if meta.file_type().is_symlink() {
        anyhow::bail!("{} is a symlink; refusing to follow", path.display());
    }
    fs::remove_dir_all(path).with_context(|| format!("removing {}", path.display()))?;
    Ok((OutcomeStatus::Removed, "directory removed".into()))
}

fn remove_file(path: &Path) -> Result<(OutcomeStatus, String)> {
    if !path.exists() {
        return Ok((OutcomeStatus::NotPresent, "already gone".into()));
    }
    let meta = fs::symlink_metadata(path).context("stat path")?;
    if meta.file_type().is_symlink() {
        anyhow::bail!("{} is a symlink; refusing to remove", path.display());
    }
    fs::remove_file(path).with_context(|| format!("removing {}", path.display()))?;
    Ok((OutcomeStatus::Removed, "file removed".into()))
}

/// Strip the `mcpServers.anvil` entry (if any) from a JSON config file,
/// preserving all other server entries and unrelated keys. Safe on
/// files that don't contain our entry or aren't JSON-parseable: those
/// return `NotPresent` or an error, never a corrupted write.
fn remove_mcp_entry(path: &Path) -> Result<(OutcomeStatus, String)> {
    let raw = match fs::read_to_string(path) {
        Ok(s) => s,
        Err(err) if err.kind() == io::ErrorKind::NotFound => {
            return Ok((OutcomeStatus::NotPresent, "file does not exist".into()));
        }
        Err(err) => return Err(err).with_context(|| format!("reading {}", path.display())),
    };

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

    let rendered = serde_json::to_vec_pretty(&value).context("re-serialising MCP config")?;
    atomic_write(path, &rendered).with_context(|| format!("writing {}", path.display()))?;
    Ok((
        OutcomeStatus::Removed,
        format!("`{MCP_SERVER_KEY}` entry removed"),
    ))
}

fn print_plan(plan: &Plan, project_root: &Path, args: &UninstallArgs, json: bool) {
    if json {
        // Emit the plan as JSON to stdout so callers can pre-flight.
        let payload = serde_json::json!({
            "project_root": project_root.display().to_string(),
            "global": args.global,
            "dry_run": args.dry_run,
            "actions": plan.actions,
        });
        println!("{payload}");
        return;
    }
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
            "\nUser-level state (~/.anvil/, credentials, MCP entries) preserved. \
             Pass --global to remove those as well."
        );
    }
}

fn print_outcomes(outcomes: &[ActionOutcome], json: bool) {
    if json {
        let payload = serde_json::json!({ "outcomes": outcomes });
        println!("{payload}");
        return;
    }
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

fn is_homebrew_path(p: &str) -> bool {
    p.contains("/Cellar/anvil/")
        || p.contains("/Cellar/eddacraft-anvil/")
        || p.contains("/opt/homebrew/bin/anvil")
        || p.contains("/usr/local/bin/anvil") && p.contains("brew")
        || p.contains("/home/linuxbrew/.linuxbrew/bin/anvil")
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
        let kinds: Vec<&Action> = plan.actions.iter().collect();
        assert!(matches!(
            kinds
                .iter()
                .find(|a| matches!(a, Action::RemoveAnvilrc { .. })),
            Some(_)
        ));
        assert!(matches!(
            kinds
                .iter()
                .find(|a| matches!(a, Action::RemoveProjectAnvil { .. })),
            Some(_)
        ));
        // Hooks step is always queued; daemon step is suppressed.
        assert!(kinds.iter().any(|a| matches!(a, Action::RemoveGitHooks)));
        assert!(!kinds.iter().any(|a| matches!(a, Action::StopDaemon { .. })));
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
}
