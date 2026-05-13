//! MLP-014 attribution primitives: env-var propagation + Linux
//! process-tree ancestor walk.
//!
//! Multi-agent worktrees need a way to scope fences and witness lines
//! to a specific sub-agent rather than to the whole worktree. This
//! crate ships the smallest primitives the higher layers (registry,
//! fence layer, hook) compose:
//!
//! - **Env propagation** ([`env`]) — encode an [`AgentTag`] into the
//!   `ANVIL_AGENT_TAG` env var, decode it back out, and set both
//!   `ANVIL_AGENT_TAG` + `ANVIL_TASK_ID` on a [`std::process::Command`]
//!   so child processes inherit attribution.
//! - **Process introspection** ([`process`]) — read `pid_starttime`
//!   and parent pid via `/proc/<pid>/stat`. Linux-only in v1; other
//!   platforms return [`io::ErrorKind::Unsupported`].
//! - **Process-tree walk** ([`walk`]) — climb from a starting PID
//!   toward init, invoking a caller-supplied visitor until it finds a
//!   match or the walk terminates (parent unknown, init reached,
//!   max-depth exceeded).
//!
//! ## Trust model
//!
//! `ANVIL_AGENT_TAG` and `ANVIL_TASK_ID` are advisory hints, not
//! authenticated identity. Any same-UID process can spoof or unset
//! them. The daemon MUST cross-check an env-supplied tag against the
//! `AgentTag` it issued for this pid lineage at INTL-003 registration;
//! mismatches are treated as missing, not honoured. The witness chain
//! (ADR-037 D-2) and `validate_at_l4` (ADR-037 D-5) are the
//! authentication backstops.
//!
//! This crate exposes the read / parse / walk primitives. The
//! registry-side issued-tag check lives in `anvil-intercept`.
//!
//! ## Deferred follow-ups (not v1)
//!
//! - Registry key change from `WorktreeKey` to `(WorktreeKey,
//!   AgentTag)` — extends `anvil-intercept/src/registry.rs` and is
//!   tracked under MLP-014 in the module plan.
//! - Per-worktree session cap config
//!   (`enforcement.session.per_worktree_max`).
//! - `degraded:fence-cascade` mode at 5 fences / 60s.
//! - macOS / Windows `pid_starttime` + `parent_pid` — Linux-only in
//!   v1.
//! - TS driver-client mirror at
//!   `packages/anvil-driver-client/src/session.ts`.
//!
//! See `plans/modules/multilayer-protection.aps.md` task MLP-014.

pub mod env;
pub mod process;
pub mod walk;

pub use anvil_intercept_proto::session::{ANVIL_AGENT_TAG_ENV, ANVIL_TASK_ID_ENV, AgentTag};
pub use env::{ParseAgentTagError, agent_tag_from_env_value, agent_tag_to_env_value};
pub use process::{ProcessInfoError, parent_pid, pid_starttime};
pub use walk::{WalkError, WalkOutcome, walk_ancestors};
