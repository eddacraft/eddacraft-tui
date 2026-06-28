//! INTL-001 / INTL-007 / INTL-008: `clap` definitions for `anvil-run`.
//!
//! The CLI has two modes:
//!
//! - **Wrap mode** (default): `anvil-run --tool <name> -- <cmd...>`
//!   wraps a child command in a controlled session.
//! - **Hook mode** (INTL-007): `anvil-run hook register` registers a
//!   side-channel session for a calling agent (e.g. Claude Code
//!   `PreToolUse`).
//!
//! The shape is deliberately small. Every flag has a single
//! responsibility; anything that needs a config file belongs to
//! `.anvil.yaml` and is read by the daemon, not by the launcher.

use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

/// Top-level `anvil-run` command.
#[derive(Debug, Parser)]
#[command(
    name = "anvil-run",
    version,
    about = "Wrap an agent process launch in an anvil-managed session.",
    long_about = "anvil-run resolves the launch context, checks the daemon's \
fence state, registers a session, places the child in a dedicated \
process group (Unix) or Job Object (Windows), heartbeats while the \
child runs, and unregisters on exit.\n\nSee `plans/modules/intercept-launcher.aps.md` for the contract."
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,

    #[command(flatten)]
    pub wrap: WrapArgs,
}

/// `anvil-run` subcommands. When omitted, the launcher operates in
/// wrap-mode and reads [`WrapArgs`].
#[derive(Debug, Subcommand)]
pub enum Command {
    /// Hook side-channel surface (INTL-007). Lets tools that did not
    /// start through `anvil-run` register a session with the daemon.
    Hook(HookArgs),
}

#[derive(Debug, Args)]
pub struct WrapArgs {
    /// Driver / tool identifier — e.g. `claude-code`, `codex`,
    /// `aider`. Passed to the daemon at registration so per-driver
    /// policy applies.
    #[arg(long, value_name = "NAME")]
    pub tool: Option<String>,

    /// Optional claimed agent id (driver-supplied). Opaque to the
    /// launcher; the daemon's session registry decides what to do
    /// with it. Defaults to a stable per-invocation token.
    #[arg(long = "agent-id", value_name = "ID")]
    pub claimed_agent_id: Option<String>,

    /// Override the worktree root the daemon should fence-check
    /// against. Defaults to walking up from `--cwd` to the nearest
    /// git worktree.
    #[arg(long, value_name = "PATH")]
    pub worktree: Option<PathBuf>,

    /// Override the working directory the child is spawned in.
    /// Defaults to the launcher's own cwd.
    #[arg(long, value_name = "PATH")]
    pub cwd: Option<PathBuf>,

    /// Print the resolved plan + the daemon decision and exit
    /// without spawning. Useful for smoke tests and shell-wrapper
    /// debugging.
    #[arg(long)]
    pub dry_run: bool,

    /// **Internal test-only field — never parsed from the command
    /// line.** `clap::Arg::skip` means the field is excluded from
    /// argv parsing and always defaults to `false`; tests that need
    /// to disable the heartbeat thread set this directly when they
    /// construct `WrapArgs`. Shipping a real `--no-heartbeat` flag
    /// would let a long-running session age out of the daemon
    /// registry, weakening the controlled-session guarantee.
    #[arg(skip)]
    pub no_heartbeat: bool,

    /// The wrapped command and its arguments. Everything after `--`
    /// is forwarded verbatim to the child.
    #[arg(last = true, allow_hyphen_values = true)]
    pub command: Vec<String>,
}

#[derive(Debug, Args)]
pub struct HookArgs {
    #[command(subcommand)]
    pub command: HookCommand,
}

#[derive(Debug, Subcommand)]
pub enum HookCommand {
    /// Register the calling process as a side-channel session
    /// (INTL-007). Enforcement for hook-registered sessions is
    /// capped at fence-only by the daemon.
    Register(HookRegisterArgs),
}

#[derive(Debug, Args)]
pub struct HookRegisterArgs {
    /// Driver id the calling tool wants to be recorded under.
    #[arg(long, value_name = "NAME")]
    pub tool: String,

    /// Working directory of the calling tool. Defaults to the
    /// launcher's cwd.
    #[arg(long, value_name = "PATH")]
    pub cwd: Option<PathBuf>,

    /// PID of the calling tool. Defaults to the parent PID (the
    /// process that invoked the hook). Required to be > 0.
    #[arg(long, value_name = "PID")]
    pub pid: Option<u32>,
}

/// Parse the process command line. Thin wrapper around `clap::parse`
/// so tests can call it with a constructed `argv`.
pub fn parse_from<I, T>(argv: I) -> Result<Cli, clap::Error>
where
    I: IntoIterator<Item = T>,
    T: Into<std::ffi::OsString> + Clone,
{
    Cli::try_parse_from(argv)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrap_mode_parses_tool_and_trailing_command() {
        let cli = parse_from([
            "anvil-run",
            "--tool",
            "claude-code",
            "--",
            "claude",
            "code",
            "--help",
        ])
        .expect("parse");
        assert!(cli.command.is_none(), "expected wrap mode");
        assert_eq!(cli.wrap.tool.as_deref(), Some("claude-code"));
        assert_eq!(
            cli.wrap.command,
            vec!["claude".to_owned(), "code".to_owned(), "--help".to_owned(),],
        );
    }

    #[test]
    fn wrap_mode_requires_a_command_after_double_dash() {
        // No `--`/trailing argv → still parses (we validate the
        // command separately in `run`), but the trailing list is
        // empty. Pin the shape so the caller can reject it with a
        // clear UX rather than a clap parse error.
        let cli = parse_from(["anvil-run", "--tool", "claude-code"]).expect("parse");
        assert!(cli.wrap.command.is_empty());
    }

    #[test]
    fn hook_register_subcommand_is_routed_separately() {
        let cli =
            parse_from(["anvil-run", "hook", "register", "--tool", "claude-code"]).expect("parse");
        match cli.command.expect("hook subcommand routed") {
            Command::Hook(args) => match args.command {
                HookCommand::Register(reg) => {
                    assert_eq!(reg.tool, "claude-code");
                    assert!(reg.pid.is_none(), "pid defaults to parent");
                }
            },
        }
    }

    #[test]
    fn dry_run_flag_round_trips() {
        let cli = parse_from([
            "anvil-run",
            "--tool",
            "claude-code",
            "--dry-run",
            "--",
            "true",
        ])
        .expect("parse");
        assert!(cli.wrap.dry_run);
    }

    #[test]
    fn worktree_override_is_parsed_as_a_path() {
        let cli = parse_from([
            "anvil-run",
            "--tool",
            "claude-code",
            "--worktree",
            "/tmp/wt",
            "--",
            "true",
        ])
        .expect("parse");
        assert_eq!(cli.wrap.worktree, Some(PathBuf::from("/tmp/wt")));
    }
}
