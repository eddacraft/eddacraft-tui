//! Top-level orchestration glue.
//!
//! `main` parses argv and calls into this module. The orchestrator
//! is thin on purpose — each step lives in its own focused module
//! ([`context`], [`preflight`], [`session`], [`spawn`], [`cleanup`],
//! [`heartbeat`], [`hook`], [`blocked`]). Keeping `run` thin means
//! the heavy logic is testable without spawning a real binary.

use std::io::Write;

use anyhow::Context;

use crate::blocked::{RefusalReason, exit_code_for, render};
use crate::cleanup::SessionGuard;
use crate::cli::{Cli, Command, HookCommand, WrapArgs};
use crate::context::LaunchContext;
use crate::exit_codes::{EXIT_BAD_CONFIG, EXIT_SPAWN_FAILED, EXIT_USAGE, forward_child_status};
use crate::heartbeat::HeartbeatHandle;
use crate::hook::HookError;
use crate::preflight;
use crate::session::{self, RegistrationRequest, pid_starttime_or_fallback};
use crate::{hook, spawn};

/// Run the launcher. Returns the process exit code the caller
/// should propagate to the OS.
pub fn run(cli: Cli) -> i32 {
    match cli.command {
        Some(Command::Hook(args)) => run_hook(args),
        None => run_wrap(cli.wrap),
    }
}

fn run_hook(args: crate::cli::HookArgs) -> i32 {
    match args.command {
        HookCommand::Register(reg) => match hook::run_register(&reg) {
            Ok(reg) => {
                let _ = writeln!(
                    std::io::stdout().lock(),
                    "registered hook session {} for pid {} in worktree {}",
                    reg.session_id.as_str(),
                    reg.pid,
                    reg.worktree.display(),
                );
                0
            }
            // Map by failure class so operators get the right
            // recovery suggestion: a bad `--cwd` is not a daemon
            // outage, and `--pid 0` is not a registration problem.
            Err(HookError::InvalidPid) => {
                let _ = writeln!(
                    std::io::stderr().lock(),
                    "anvil-run: hook register: --pid must be a positive integer"
                );
                EXIT_USAGE
            }
            Err(HookError::ParentPidUnavailable) => {
                let _ = writeln!(
                    std::io::stderr().lock(),
                    "anvil-run: hook register: parent PID unavailable; pass --pid explicitly"
                );
                EXIT_BAD_CONFIG
            }
            Err(HookError::BadContext(err)) => {
                let _ = writeln!(
                    std::io::stderr().lock(),
                    "anvil-run: hook register: bad launch context: {err}"
                );
                EXIT_BAD_CONFIG
            }
            Err(HookError::Daemon(err)) => emit_refusal(&RefusalReason::DaemonUnavailable {
                message: preflight::refusal_message_for(&err),
            }),
        },
    }
}

fn run_wrap(args: WrapArgs) -> i32 {
    let WrapArgs {
        tool,
        claimed_agent_id,
        worktree,
        cwd,
        dry_run,
        no_heartbeat,
        command,
    } = args;

    let (tool, program, program_args) = match validate_wrap_args(tool, &command) {
        Ok(parts) => parts,
        Err(code) => return code,
    };

    let ctx = match LaunchContext::resolve(cwd, worktree) {
        Ok(ctx) => ctx,
        Err(err) => {
            let _ = writeln!(
                std::io::stderr().lock(),
                "anvil-run: bad launch context: {err}"
            );
            return EXIT_BAD_CONFIG;
        }
    };

    let preflight = match preflight::run(&ctx.worktree) {
        Ok(d) => d,
        Err(err) => {
            return emit_refusal(&RefusalReason::DaemonUnavailable {
                message: preflight::refusal_message_for(&err),
            });
        }
    };
    if let Some(reason) = RefusalReason::from_preflight(preflight.clone()) {
        return emit_refusal(&reason);
    }

    if dry_run {
        let _ = writeln!(
            std::io::stdout().lock(),
            "anvil-run dry-run: would launch {program} {args:?} in {cwd} (worktree {wt}); daemon preflight={preflight:?}",
            args = program_args,
            cwd = ctx.cwd.display(),
            wt = ctx.worktree.display()
        );
        return 0;
    }

    run_wrap_spawn(WrapSpawn {
        tool,
        claimed_agent_id,
        program,
        program_args,
        ctx,
        no_heartbeat,
    })
}

fn validate_wrap_args(
    tool: Option<String>,
    command: &[String],
) -> Result<(String, String, Vec<String>), i32> {
    let Some(tool) = tool else {
        let _ = writeln!(
            std::io::stderr().lock(),
            "anvil-run: --tool is required in wrap mode."
        );
        return Err(EXIT_USAGE);
    };
    let Some((program, rest)) = command.split_first() else {
        let _ = writeln!(
            std::io::stderr().lock(),
            "anvil-run: no wrapped command supplied. Use `anvil-run --tool <name> -- <cmd> [args...]`."
        );
        return Err(EXIT_USAGE);
    };
    Ok((tool, program.clone(), rest.to_vec()))
}

/// Bundle of arguments threaded into [`run_wrap_spawn`]. Pulled into
/// a struct so the helper does not trip
/// `clippy::needless_pass_by_value` for the by-value `String` /
/// `Vec<String>` fields that the wrap path needs to keep referencing
/// across the spawn / register / wait stages.
struct WrapSpawn {
    tool: String,
    claimed_agent_id: Option<String>,
    program: String,
    program_args: Vec<String>,
    ctx: LaunchContext,
    no_heartbeat: bool,
}

fn run_wrap_spawn(spawn_args: WrapSpawn) -> i32 {
    let WrapSpawn {
        tool,
        claimed_agent_id,
        program,
        program_args,
        ctx,
        no_heartbeat,
    } = spawn_args;
    let session_id = session::new_session_id();
    let claimed = claimed_agent_id.unwrap_or_else(|| format!("{tool}-{}", session_id.as_str()));
    // Launcher's own pid_starttime; the child's is captured after
    // spawn and reported separately via `session.report_process`.
    let launcher_pid_starttime = pid_starttime_or_fallback(std::process::id());
    let registration = match session::register(&RegistrationRequest {
        session_id: &session_id,
        worktree: &ctx.worktree,
        cwd: &ctx.cwd,
        driver_id: &tool,
        claimed_agent_id: &claimed,
        pid_starttime: launcher_pid_starttime,
        tmux_pane: ctx.tmux_pane.as_deref(),
    }) {
        Ok(r) => r,
        Err(err) => {
            return classify_register_failure(&err);
        }
    };
    let guard = SessionGuard::arm(registration.session_id.clone());
    let heartbeat = if no_heartbeat {
        None
    } else {
        Some(HeartbeatHandle::spawn_default(
            registration.session_id.clone(),
        ))
    };

    let cmd = spawn::build_command(
        &program,
        &program_args,
        &ctx.cwd,
        &registration.session_id,
        &registration.agent_tag,
    );
    let spawned = match spawn::spawn(cmd, &registration.session_id) {
        Ok(s) => s,
        Err(err) => {
            let _ = writeln!(
                std::io::stderr().lock(),
                "anvil-run: failed to start {program}: {err:#}"
            );
            return EXIT_SPAWN_FAILED;
        }
    };
    if let Err(err) = spawn::report_to_daemon(&registration.session_id, &spawned)
        .context("reporting process metadata to daemon")
    {
        // Non-fatal: the daemon may not yet implement
        // `session.report_process` (the helper itself absorbs the
        // "Method not found" case). Everything else still gets a
        // warning rather than terminating the child — once the
        // INTD half lands, `report_to_daemon` will return Ok in the
        // happy path and operators can tighten this if/when a
        // failure here turns out to be load-bearing.
        let _ = writeln!(std::io::stderr().lock(), "anvil-run: warning: {err:#}");
    }

    let status = match spawn::wait_for_child(spawned.child) {
        Ok(status) => status,
        Err(err) => {
            let _ = writeln!(std::io::stderr().lock(), "anvil-run: wait failed: {err:#}");
            return EXIT_SPAWN_FAILED;
        }
    };

    if let Some(h) = heartbeat {
        h.stop();
    }
    // Unregister explicitly so the daemon sees the close before
    // the guard's drop runs (drop will become a no-op).
    if let Err(err) = session::unregister(&registration.session_id) {
        let _ = writeln!(std::io::stderr().lock(), "anvil-run: warning: {err:#}");
    }
    guard.disarm();
    forward_child_status(status)
}

/// Map a `session.register` failure to the right launcher exit code.
/// Transport-level errors render as "daemon unavailable"; daemon-side
/// rejections (fenced after preflight, params invalid, worktree
/// owned) get the more generic spawn-failed path so the operator is
/// not told to restart the daemon for a content-level reject.
fn classify_register_failure(err: &anyhow::Error) -> i32 {
    if let Some(
        crate::ipc::ClientError::DaemonNotRunning { .. }
        | crate::ipc::ClientError::DaemonRefused { .. }
        | crate::ipc::ClientError::Io(_),
    ) = err.downcast_ref::<crate::ipc::ClientError>()
    {
        return emit_refusal(&RefusalReason::DaemonUnavailable {
            message: preflight::refusal_message_for(err),
        });
    }
    let _ = writeln!(
        std::io::stderr().lock(),
        "anvil-run: session.register rejected: {err:#}"
    );
    EXIT_SPAWN_FAILED
}

fn emit_refusal(reason: &RefusalReason) -> i32 {
    let mut stderr = std::io::stderr().lock();
    let _ = stderr.write_all(render(reason).as_bytes());
    exit_code_for(reason)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::WrapArgs;

    #[test]
    fn missing_tool_exits_with_usage_code() {
        let args = WrapArgs {
            tool: None,
            claimed_agent_id: None,
            worktree: None,
            cwd: None,
            dry_run: false,
            no_heartbeat: true,
            command: vec!["true".into()],
        };
        assert_eq!(run_wrap(args), EXIT_USAGE);
    }

    #[test]
    fn missing_command_exits_with_usage_code() {
        let args = WrapArgs {
            tool: Some("claude-code".into()),
            claimed_agent_id: None,
            worktree: None,
            cwd: None,
            dry_run: false,
            no_heartbeat: true,
            command: vec![],
        };
        assert_eq!(run_wrap(args), EXIT_USAGE);
    }
}
