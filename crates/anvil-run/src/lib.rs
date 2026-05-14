//! `anvil-run`: wrapped-launch ingress for the Anvil Intercept Loop
//! (INTL module — see `plans/modules/intercept-launcher.aps.md`).
//!
//! The launcher wraps an arbitrary agent command in a controlled
//! environment: it resolves the launch context, queries the daemon for
//! reachability and worktree-fence status, registers a session, spawns
//! the child in a dedicated process group (Unix) or named Job Object
//! (Windows), heartbeats while the child runs, and unregisters on
//! exit. The binary is single-shot per child process.
//!
//! ## Module map
//!
//! - [`cli`] — `clap` parser (INTL-001).
//! - [`context`] — cwd / repo / worktree / tmux pane resolution
//!   (INTL-001).
//! - [`ipc`] — sync JSON-RPC client over the per-user UDS / named
//!   pipe; reuses [`anvil_intercept::ipc`] for socket discovery.
//! - [`preflight`] — daemon connectivity + fence decision (INTL-002).
//! - [`session`] — session id generation + AgentTag plumbing
//!   (INTL-003).
//! - [`spawn`] — process-group launch + env injection (INTL-004).
//! - [`cleanup`] — drop guard that unregisters on every exit path
//!   (INTL-005).
//! - [`heartbeat`] — periodic liveness ticker (INTL-009).
//! - [`hook`] — `anvil-run hook register` side-channel for sessions
//!   not started via the launcher (INTL-007).
//! - [`blocked`] — UX for refused launches (INTL-008).
//! - [`exit_codes`] — stable exit codes the shell wrappers can switch
//!   on (INTL-008).
//! - [`run`] — top-level orchestration glue.
//!
//! ## Trust model
//!
//! Per [`plans/modules/intercept-launcher.aps.md`] the env propagation
//! contract (`ANVIL_TASK_ID`, `ANVIL_AGENT_TAG`) is **advisory only**.
//! The daemon authenticates against the witness chain and its own
//! registration record; nothing here should ever be treated as proof
//! of identity by downstream code.

#![forbid(unsafe_code)]

pub mod blocked;
pub mod cleanup;
pub mod cli;
pub mod context;
pub mod exit_codes;
pub mod heartbeat;
pub mod hook;
pub mod ipc;
pub mod preflight;
pub mod run;
pub mod session;
pub mod spawn;
