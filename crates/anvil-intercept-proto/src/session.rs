//! MLP-014 / INTL-003 session-key vocabulary.
//!
//! This is the **stub** `AgentTag` definition landed during the Wave 0
//! readiness review (2026-05-13) so the INTL ↔ MLP-014 interface
//! exists in code rather than only in planning prose. Behavioural use
//! arrives with MLP-014 (registry key change) and INTL-003 / INTL-004
//! (launcher-side propagation). Both items remain in their respective
//! modules' task lists.
//!
//! Scope deliberately narrow:
//!
//! - `AgentTag` — the composite minted by the daemon from
//!   `(driver_id, claimed_agent_id, pid_starttime)`.
//! - Env-var name constants for the launcher → child propagation
//!   contract (`ANVIL_TASK_ID`, `ANVIL_AGENT_TAG`).
//!
//! Anything else MLP-014 needs (per-task fence keys, the
//! `degraded:fence-cascade` mode, attribution-chain dispatch) lives in
//! `anvil-intercept` proper and is out of scope here.
//!
//! See `plans/modules/multilayer-protection.aps.md` MLP-014 and
//! `plans/modules/intercept-launcher.aps.md` INTL-003 / INTL-004.

use serde::{Deserialize, Serialize};

/// Environment variable carrying the daemon-minted `AgentTag` from a
/// launcher to its child process. Advisory only — the daemon MUST
/// cross-check the env-supplied tag against the `AgentTag` it issued
/// for this pid lineage at INTL-003 before honouring it. See ADR-037
/// D-2 for the witness-chain authentication backstop.
pub const ANVIL_AGENT_TAG_ENV: &str = "ANVIL_AGENT_TAG";

/// Environment variable carrying the per-task identifier that scopes
/// fence isolation in multi-agent worktrees (MLP-014). Same trust
/// caveat as `ANVIL_AGENT_TAG_ENV`: env is forgeable by any same-UID
/// peer, so absence triggers a process-tree walk fallback rather than
/// being treated as authoritative.
pub const ANVIL_TASK_ID_ENV: &str = "ANVIL_TASK_ID";

/// ACTMO-014: the `claimed_agent_id` the activation spine (`anvil start`,
/// `anvil workspace register`) stamps on its `AgentTag`. The daemon keys
/// **durable worktree membership** on this value: a registration carrying
/// it is persisted under `ANVIL_HOME`, exempt from the 30 s heartbeat TTL,
/// and reloaded on startup — a *membership* registration rather than a live
/// *agent-session* lease (ADR-094 decision 1). The CLI side
/// (`anvil-cli/src/.../registration`) and the daemon registry both reference
/// this single constant so the durability predicate cannot drift between
/// producer and consumer.
pub const ACTIVATION_SPINE_CLAIMED_AGENT_ID: &str = "activation-spine";

/// Composite identity for a session within a worktree. Minted by the
/// daemon at INTL-003 registration time from the launcher-supplied
/// `(driver_id, claimed_agent_id)` plus the kernel-reported
/// `pid_starttime`. Combined with `WorktreeKey` in MLP-014 to form
/// the per-task fence scope.
///
/// **Trust model.** `AgentTag` is not authenticated identity. Any
/// same-UID process can claim any `driver_id` / `claimed_agent_id`
/// pair; `pid_starttime` makes after-the-fact PID reuse detectable
/// but does not prove the process started where the launcher said it
/// did. The daemon honours a tag only when it matches a registration
/// it issued in this session; the witness chain (ADR-037 D-2) and
/// `validate_at_l4` (ADR-037 D-5) are the authentication backstops.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AgentTag {
    /// Identifies the driver framework that launched the agent
    /// (`anvil-run`, `claude-code-pretool`, `direct-mcp`, …). Drawn
    /// from the surface driver registry; never user-supplied.
    pub driver_id: String,

    /// Free-form identifier the driver claims for this agent
    /// instance. Opaque to the proto layer; the daemon may apply
    /// per-driver well-formedness rules before honouring.
    pub claimed_agent_id: String,

    /// Process start time as Unix seconds since epoch, captured at
    /// spawn. Defends against PID reuse — a recycled PID with a
    /// different `pid_starttime` is treated as a different session.
    pub pid_starttime: u64,
}

impl AgentTag {
    /// Construct an `AgentTag`. No validation: the daemon's session
    /// registry is the single authority on which tags are honoured.
    pub fn new(
        driver_id: impl Into<String>,
        claimed_agent_id: impl Into<String>,
        pid_starttime: u64,
    ) -> Self {
        Self {
            driver_id: driver_id.into(),
            claimed_agent_id: claimed_agent_id.into(),
            pid_starttime,
        }
    }

    /// ACTMO-014: `true` when this tag marks **durable worktree membership**
    /// — i.e. its `claimed_agent_id` is
    /// [`ACTIVATION_SPINE_CLAIMED_AGENT_ID`]. The daemon registry uses this to
    /// decide whether a registration is persisted and exempt from the
    /// heartbeat TTL (ADR-094 decision 1). Live agent sessions (a different
    /// `claimed_agent_id`) keep the existing lease semantics.
    #[must_use]
    pub fn is_durable_membership(&self) -> bool {
        self.claimed_agent_id == ACTIVATION_SPINE_CLAIMED_AGENT_ID
    }
}

/// MLP2-025b: launcher PID + `pid_starttime` carried on
/// `IpcCommand::RegisterSession` to seed the daemon's lineage
/// index. The pair is wrapped in a struct (rather than two parallel
/// optional fields on `RegisterSession`) so the "one supplied, the
/// other not" mis-pairing is foreclosed by the type system.
///
/// The launcher (anvil-run / driver-client) reports its **own** PID
/// and `pid_starttime` here; the daemon trusts the launcher's
/// register-time claim about itself. See
/// `plans/specs/2026-05-16-mlp2-025-spoof-cross-check-control-lane.md`
/// §7 for the trust-model rationale.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LineageAnchor {
    /// Launcher process id.
    pub pid: u32,
    /// Launcher process start time as Unix seconds since epoch.
    /// Paired with [`Self::pid`] to defeat PID-reuse spoofs.
    pub pid_starttime: u64,
}

/// MLP2-026: audit context recorded by the daemon when an operator
/// clears a cascade via `IpcCommand::UnblockCascade`. Populated
/// server-side from the IPC peer credentials at the moment the verb
/// is received; never trusted from the client payload directly (a
/// client-supplied `OperatorContext` on the wire is silently
/// overwritten). See
/// `plans/specs/2026-05-16-mlp2-026-fence-cascade-control-lane.md`
/// §3.3.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperatorContext {
    /// Effective UID of the operator-acting process. `None` when
    /// the credential read failed — the daemon still clears the
    /// cascade (spec §7 — credential gaps record the gap; the
    /// clear-side authority is the existing UID gate at
    /// socket-accept, NOT the `OperatorContext`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uid: Option<u32>,
    /// Peer process id from the same syscall. Always populated on
    /// Linux + macOS (the daemon already captures `peer_pid` for
    /// MLP2-025b); `None` on Windows / platforms where the read
    /// is undefined.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pid: Option<u32>,
    /// Result of `gethostname(3)` at the daemon side. Best-effort.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hostname: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn agent_tag_round_trips_through_json() {
        let tag = AgentTag::new("anvil-run", "claude-code-1", 1_700_000_042);
        let line = serde_json::to_string(&tag).expect("serialise");
        let back: AgentTag = serde_json::from_str(&line).expect("deserialise");
        assert_eq!(back, tag);
    }

    #[test]
    fn env_var_names_match_planning_contract() {
        assert_eq!(ANVIL_AGENT_TAG_ENV, "ANVIL_AGENT_TAG");
        assert_eq!(ANVIL_TASK_ID_ENV, "ANVIL_TASK_ID");
    }

    /// ACTMO-014: the durability predicate keys off the activation-spine
    /// `claimed_agent_id`. A live agent session (any other id) is not durable
    /// membership, so it keeps the TTL lease.
    #[test]
    fn activation_spine_tag_is_durable_membership() {
        let spine = AgentTag::new("anvil-start", ACTIVATION_SPINE_CLAIMED_AGENT_ID, 0);
        assert!(spine.is_durable_membership());

        let live = AgentTag::new("anvil-run", "claude-code-1", 1_700_000_000);
        assert!(!live.is_durable_membership());

        // Pin the wire value: a rename here would silently break the daemon's
        // persisted-set predicate and the CLI producer in lockstep.
        assert_eq!(ACTIVATION_SPINE_CLAIMED_AGENT_ID, "activation-spine");
    }

    /// Pinned invariant: tags with different `pid_starttime` values
    /// compare unequal under `Eq`, so the daemon's session-registry
    /// `HashMap` treats them as distinct keys per MLP-014's
    /// `(WorktreeKey, AgentTag)` plan. (Hash collisions are allowed
    /// by `HashMap` — `Eq` is what guarantees key separation.)
    #[test]
    fn distinct_pid_starttimes_produce_distinct_tags() {
        let a = AgentTag::new("anvil-run", "claude-1", 1_700_000_000);
        let b = AgentTag::new("anvil-run", "claude-1", 1_700_000_001);
        assert_ne!(a, b);
    }
}
