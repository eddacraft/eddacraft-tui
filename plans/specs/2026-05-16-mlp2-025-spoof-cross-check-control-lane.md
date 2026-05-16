# MLP2-025b — Daemon control-lane wire-up for env-tag spoof cross-check

## 1. Identifiers and status

| Field | Value |
| ----- | ----- |
| Spec id | `2026-05-16-mlp2-025-spoof-cross-check-control-lane` |
| Status | Draft |
| Date | 2026-05-16 |
| Owners | @aneki |
| Work item | MLP2-025b (Daemon control-lane wire-up for spoof cross-check) |
| Supersedes | — |
| Council tier | `mini` for the spec review; `quick` per impl subtask |

Companion artefacts:

- Phase 1 primitives merged via PR #1597 (commit `893cc2bb`).
- APS module: `plans/modules/multilayer-protection-v2.aps.md` (MLP2-025 + MLP2-025b).
- Execution plan: `plans/execution/2026-05-15-mlp2-025-026.steps.md`.
- Template: `plans/specs/INTEGRATION-SPEC-TEMPLATE.md`.

## 2. Problem statement

The Phase 1 primitives (`SessionRecord::daemon_issued_tag`, `tag_env::env_agent_tag`, `SessionRegistry::register_with_lineage` / `lookup_tag_for_lineage` / `cross_check_env_tag`) ship the library surface required to detect an out-of-lineage env-tag forgery. They are not yet wired to any production decision point. Phase 2 must answer four contract questions before TDD:

1. **Writer env-tag ingress.** `env_agent_tag()` reads the **daemon's** env. The cross-check needs the **writer's** env tag. The writer is an external process; the daemon must obtain its tag without making cross-process assumptions about `/proc`.
2. **Cross-check call site.** The 2026-05-15 council said "do the check at the control-lane caller that already holds the registry". Mid-implementation re-read showed `ScanBufferService` (write-time) has no `SessionRegistry` reference; `RegistryDispatcher` (register-time) has it but is on a different code path. The actual control-lane for write-time decisions is the JSON-RPC handler at `crates/anvil-intercept/src/ipc.rs:1989`.
3. **Block + fence combinator.** Today's fence path is register-time only. The write-time path has no precedent for "block this write AND record a worktree fence" as a single decision.
4. **Migration scope.** Production has exactly one `register()` call path (single trait dispatch through `dispatch_command`). Migration is small, not the 20-site sweep the original plan implied.

This spec answers all four. Each answer becomes a contract in the relevant section.

## 3. Data shapes

### 3.1 `ScanBufferRequest` — wire-additive `env_agent_tag`

Today, `crates/anvil-intercept/src/midedit.rs:47–52`:

```rust
pub struct ScanBufferRequest {
    pub path: PathBuf,
    pub text: String,
    pub version: u64,
    pub mode: ScanBufferMode,
}
```

Phase 2 adds **one** field, wire-additive:

```rust
pub struct ScanBufferRequest {
    pub path: PathBuf,
    pub text: String,
    pub version: u64,
    pub mode: ScanBufferMode,
    /// MLP2-025b: env-supplied `AgentTag` from the writer process,
    /// parsed via `anvil_attribution::env::agent_tag_from_env_value`.
    /// `None` for pre-MLP2-025 writers — the cross-check returns
    /// `Cross::Untagged` and enforcement proceeds unchanged.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub env_agent_tag: Option<AgentTag>,
}
```

Wire-additive precedent: matches `SessionRecord::agent_tag` (MLP2-023) and `SessionRecord::daemon_issued_tag` (MLP2-025 Phase 1).

Wire shape (legacy reader sees this and ignores the new field):

```json
{
  "path": "/work/src/x.rs",
  "text": "fn main() {}",
  "version": 42,
  "mode": "pre-write",
  "env_agent_tag": {"driver": "anvil-run", "agent": "claude-1", "pid_starttime": 1700000042}
}
```

A legacy `ScanBufferRequest` without `env_agent_tag` keeps deserialising with `env_agent_tag = None` because of `#[serde(default)]`.

### 3.2 `IpcCommand::RegisterSession` — wire-additive `pid` + `pid_starttime`

Today the IPC register path does not carry the registering PID. To populate `by_pid_lineage` in production (Phase 1 added the index; nothing writes to it yet), the register envelope must carry the launcher's PID anchor.

```rust
pub enum IpcCommand {
    RegisterSession {
        session_id: SessionId,
        worktree: PathBuf,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        agent_tag: Option<AgentTag>,
        /// MLP2-025b: launcher PID for the lineage anchor.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pid: Option<u32>,
        /// MLP2-025b: launcher pid_starttime for the lineage anchor.
        /// Paired with `pid`; one without the other is treated as
        /// "no lineage info supplied" and the registration takes the
        /// pre-MLP2-025 path.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pid_starttime: Option<u64>,
    },
    // … other variants unchanged
}
```

Both fields are `Option` and additive. Old `anvil-run` launchers continue to register without populating them; their sessions don't get a lineage anchor, and any env-tag from a child of theirs will hash as `Cross::Spoofed` — the safe default.

### 3.3 `ScanBufferResponse` — new `Blocked` outcome

Today's response (verify exact name during impl) carries diagnostics + decision. Phase 2 adds a new explicit `Blocked` outcome for the spoof case, distinct from the existing `Allow`/`Warn`/`Interrupt` decisions:

```rust
pub enum ScanBufferOutcome {
    Allowed { diagnostics: Vec<Diagnostic> },
    Warned { diagnostics: Vec<Diagnostic> },
    Interrupted { reason: String, diagnostics: Vec<Diagnostic> },
    /// MLP2-025b: write blocked because the env-supplied AgentTag
    /// did not match the writer's PID lineage. Also fences the
    /// worktree as a side effect (recorded server-side; no client
    /// action needed beyond surfacing the reason).
    BlockedSpoofedAttribution { reason: String, fenced_worktree: PathBuf },
}
```

`Interrupted` already exists for hard-rule failures; `BlockedSpoofedAttribution` is distinct because the reason and recovery path differ — the worktree fence is the operator-touch surface, not the diagnostics list. **Exact variant naming verified during impl** if the existing response type differs from this sketch.

### 3.4 Telemetry payload — `SpoofedAttributionEvent`

Notification envelope carries the structured payload at engage:

```rust
pub struct SpoofedAttributionEvent {
    pub worktree: PathBuf,
    pub writer_pid: u32,
    pub env_tag: AgentTag,
    pub registered_tag: Option<AgentTag>, // None when no ancestor was registered at all
    pub reason: &'static str, // = DEGRADED_SPOOFED_ATTRIBUTION
}
```

`pub const DEGRADED_SPOOFED_ATTRIBUTION: &str = "degraded:spoofed-attribution";` in `crates/anvil-intercept/src/telemetry.rs`.

## 4. Message flow

Greenfield additions noted with `[NEW]`. Existing arrows kept for context.

```
┌─────────┐                                               ┌─────────────────┐
│ Writer  │                                               │ Daemon          │
│ process │                                               │ (anvil-intercept) │
└────┬────┘                                               └────────┬────────┘
     │                                                              │
     │ 1. read ANVIL_AGENT_TAG from own env                          │
     │    [tag_env::env_agent_tag() — already lives in writer side]│
     │                                                              │
     │ 2. ScanBufferRequest{path,text,version,mode,env_agent_tag} [NEW field]
     │ ─────────────────────────────────────────────────────────────▶
     │                                                              │
     │                                  3. handle_scan_buffer_jsonrpc
     │                                     (ipc.rs:1989) deserialises
     │                                     request, extracts writer
     │                                     pid from peer credentials
     │                                     [NEW: thread peer pid
     │                                     from socket accept]
     │                                                              │
     │                                  4. registry.cross_check_env_tag(
     │                                       request.env_agent_tag.as_ref(),
     │                                       writer_pid,
     │                                     ) -> Cross
     │                                     [NEW call site;
     │                                     impl is Phase 1, this is
     │                                     the wire-up]
     │                                                              │
     │            ┌─ Cross::Match or Cross::Untagged ────────────┐  │
     │            │ 5a. scan_buffer.scan_buffer(request)         │  │
     │            │     (midedit.rs:194) — unchanged path        │  │
     │            └──────────────────────────────────────────────┘  │
     │            ┌─ Cross::Spoofed ──────────────────────────────┐ │
     │            │ 5b. fence_store.fence_worktree(               │ │
     │            │       worktree, DEGRADED_SPOOFED_ATTRIBUTION  │ │
     │            │     ) [NEW: write-time fence record]          │ │
     │            │ 6b. emit notification + tracing::warn!        │ │
     │            │     (telemetry.rs) [NEW emission]             │ │
     │            │ 7b. short-circuit: do NOT call scan_buffer    │ │
     │            └──────────────────────────────────────────────┘ │
     │                                                              │
     │ 8. ScanBufferResponse (Allowed/Warned/Interrupted/           │
     │    BlockedSpoofedAttribution)                                │
     │ ◀─────────────────────────────────────────────────────────── │
     │                                                              │
```

Key seams:

- **Arrow 2** is the wire change (§3.1).
- **Arrow 3** extracts `writer_pid` from socket peer credentials. On Linux: `SO_PEERCRED` already used at socket accept (verified in `validate_connected_peer_for_client`). The PID is currently dropped after UID validation; this spec keeps it.
- **Arrow 4** is the new call site. `SessionRegistry::cross_check_env_tag` was added in Phase 1; this arrow finally invokes it.
- **Arrow 5b** is the new write-time fence-recording site. No precedent — greenfield.
- **Arrow 7b** is the short-circuit: spoofed writes don't reach `EnforcementPipeline`.

## 5. Function signatures

### 5.1 New: `handle_scan_buffer_with_spoof_check`

Or: extend the existing `handle_scan_buffer_jsonrpc` directly. Decision in §10 Q1.

If extended:

```rust
async fn handle_scan_buffer_jsonrpc<D, S>(
    map: &serde_json::Map<String, Value>,
    method: &str,
    params: &Value,
    response_id: Option<Value>,
    traceparent: Option<&str>,
    is_notification: bool,
    scan_buffer: &ScanBufferService,
    registry: &SessionRegistry,        // NEW
    fence_store: &Arc<FenceStore>,     // NEW
    peer_pid: u32,                     // NEW — threaded from socket accept
) -> Option<Value>
```

**Pre-conditions:** `peer_pid` is the validated peer credential PID from the accepted socket (UID already gated upstream).

**Post-conditions:**

- On `Cross::Match` or `Cross::Untagged`: `scan_buffer.scan_buffer(...)` was called exactly once.
- On `Cross::Spoofed`: `fence_store.fence_worktree(...)` was called exactly once; `scan_buffer.scan_buffer(...)` was NOT called; a `BlockedSpoofedAttribution` response was returned; a notification + tracing event was emitted.

**Errors:** existing JSON-RPC error variants unchanged; new `BlockedSpoofedAttribution` outcome surfaces in the `result` field, not as a JSON-RPC error.

**Lock ordering:** `cross_check_env_tag` takes the registry lock briefly (Phase 1 already documents this). `fence_store.fence_worktree` takes its own lock. Ordering: registry → fence_store. Document this with a comment at the call site.

### 5.2 New: `RegistryDispatcher::register_with_lineage_from_envelope`

Wraps `SessionRegistry::register_with_lineage` (Phase 1) for the IPC `RegisterSession` path:

```rust
impl SessionDispatcher for RegistryDispatcher {
    fn register(
        &self,
        id: &SessionId,
        worktree: &Path,
        agent_tag: Option<&AgentTag>,
        // NEW two args:
        pid: Option<u32>,
        pid_starttime: Option<u64>,
    ) -> Result<(), RegistryError> {
        // …fence check existing…
        match (pid, pid_starttime) {
            (Some(p), Some(ts)) => self.registry.register_with_lineage(
                id, worktree, agent_tag, agent_tag, p, ts, Instant::now()
            ).map(|_| ()),
            _ => SessionDispatcher::register(
                self.registry.as_ref(), id, worktree, agent_tag
            ),
        }
    }
    // …
}
```

**Pre-conditions:** `(pid, pid_starttime)` either both `Some` (lineage path) or both `None` (legacy path). Mixed is treated as legacy.

**Post-conditions:** when both are `Some`, the lineage index now has `(pid, pid_starttime) -> session_id` and the session's `daemon_issued_tag` is set to the client-supplied `agent_tag` (the daemon is trusting the launcher's claim here — see §7 trust model).

**Note:** widens the `SessionDispatcher` trait. One implementor (`RegistryDispatcher`); one consumer (`dispatch_command` at `ipc.rs:2529`). Tests using a stub dispatcher will need the two new arguments.

### 5.3 New: `FenceStore::fence_worktree_for_spoof`

A typed convenience over the existing `fence_worktree(worktree, reason: &str)`. Same body; named for the call site:

```rust
impl FenceStore {
    /// MLP2-025b: record a worktree-level fence with the
    /// `degraded:spoofed-attribution` reason. Convenience over
    /// `fence_worktree` that pins the reason string to the
    /// `DEGRADED_SPOOFED_ATTRIBUTION` `pub const`.
    pub fn fence_worktree_for_spoof(
        &self,
        worktree: &Path,
    ) -> Result<(), FenceStoreError> {
        self.fence_worktree(worktree, telemetry::DEGRADED_SPOOFED_ATTRIBUTION)
    }
}
```

Adds zero new behaviour to `FenceStore`; just pins the reason string at the only caller.

## 6. Lifecycle and invariants

### Creation

- `by_pid_lineage` entries are created at register time when both `pid` and `pid_starttime` are supplied (Phase 1 already supports this).
- A spoofed-attribution fence is created at write time on `Cross::Spoofed` (Phase 2 — new lifecycle event).

### Liveness

- Lineage anchors live until session unregister or evict-stale (Phase 1 already handles both).
- Spoofed-attribution fences persist until explicit `anvil intercept unblock` (existing behaviour; cascading-fence work belongs to MLP2-026, not this spec).

### Consistency invariants

- **inv-1:** every entry in `by_pid_lineage` has a corresponding `SessionRecord` in `sessions` (Phase 1 unregister/evict guarantees this).
- **inv-2:** `Cross::Match` implies the writer's PID lineage contains a registered session whose `daemon_issued_tag` equals the env tag. Verified by Phase 1 tests.
- **inv-3:** on `Cross::Spoofed`, the daemon emits exactly one notification, exactly one `tracing::warn!`, and records exactly one fence per `(worktree, writer_pid)` per scan-buffer call. Duplicate suppression is out of scope — the rate-window primitive (MLP2-009) is not consulted here.
- **inv-4:** `ScanBufferOutcome::BlockedSpoofedAttribution` is never returned alongside diagnostics; the short-circuit precedes the rule engine.

## 7. Error channel

| Path | Error / outcome | Wire surface | Verdict |
| ---- | --------------- | ------------ | ------- |
| `Cross::Untagged` | none | request continues to scan_buffer | allow (pre-MLP2-025 path) |
| `Cross::Match` | none | request continues to scan_buffer | allow (attribution preserved) |
| `Cross::Spoofed` | new outcome | `ScanBufferOutcome::BlockedSpoofedAttribution` | **deny + fence** |
| `cross_check_env_tag` panics | registry lock poisoned | JSON-RPC `-32603 Internal error` | **deny** (fail-closed) |
| `fence_store.fence_worktree_for_spoof` fails | IO error | logged via tracing::error!; spoofed write **still blocked** | **deny** even if fence recording failed |
| Peer-credential PID unavailable | greenfield | new `IpcError::PeerPidUnavailable` | **deny** the request (fail-closed; cannot cross-check) |
| `env_agent_tag` deserialisation fails | malformed wire payload | JSON-RPC `-32602 Invalid params` | reject before any registry call |

**Trust model.** The daemon trusts the **launcher** (anvil-run / driver-client) to supply the `pid` and `pid_starttime` at register time (§3.2). The launcher is in the same trust zone as the daemon; if it lies about its own PID, the operator has bigger problems. The daemon does NOT trust **writer-process** env tags — the entire point of the cross-check is that env is forgeable by any same-UID process.

Trust boundary, restated for the spec:

- **Trusted:** the launcher's register-time claim about its own `(pid, pid_starttime)`.
- **Untrusted:** every env-tag a writer process supplies in a `ScanBufferRequest`.
- **Out of scope:** intra-lineage privilege escalation. A co-process inside a legitimate launcher's tree will pass the cross-check by design (already documented on `Cross::classify`).

**Fail-closed default.** Every error path verdict above is `deny`. There is no path on which a spoofed-or-uncertain attribution is silently allowed.

## 8. Observability contract

### Notification

`FenceTransition::ActiveToFenced { reason: "degraded:spoofed-attribution", worktree, ... }` — reuses the existing notification envelope from `telemetry.rs:339–354`. No new envelope variant needed; the existing fence-transition surface already handles "fenced for reason X".

### Tracing

At the cross-check call site in `handle_scan_buffer_jsonrpc`:

```rust
tracing::warn!(
    target: "anvil_intercept::ipc",
    reason = telemetry::DEGRADED_SPOOFED_ATTRIBUTION,
    %worktree,
    writer_pid,
    env_tag = ?env_tag,
    registered_tag = ?registered_tag,
    "blocking spoofed-attribution write and fencing worktree",
);
```

Mirrors the priority convention from `status.rs:393–398`. `warn!` for the engage event; `info!` is not paired here because the **clear** is the existing `anvil intercept unblock` path which already emits its own clear telemetry.

### Constants

```rust
// telemetry.rs
pub const DEGRADED_SPOOFED_ATTRIBUTION: &str = "degraded:spoofed-attribution";
```

Single find-target for a future enum migration (Council Q1 verdict, deferred per APS).

### Both channels are required

The notification channel is the real-time operator surface; the tracing channel is the structured-log surface. A spoofed write must emit on both — operators using `anvil intercept status` should see the fence (via notification) and engineers reading collector logs should see the event (via tracing). Neither alone is sufficient.

## 9. Migration plan

The original execution plan estimated ~20 register() callers needing migration. The explore agent on 2026-05-16 corrected this: **production has one register call path**. Test sites are unchanged.

| Site | File:line | Action | Reason |
| ---- | --------- | ------ | ------ |
| `dispatch_command` → `dispatcher.register(...)` | `crates/anvil-intercept/src/ipc.rs:2529` | **migrate** | The single production register path. Update to forward `pid` + `pid_starttime` from `IpcCommand::RegisterSession`. |
| `SessionDispatcher` trait method | `crates/anvil-intercept/src/registry.rs:250` (approx) | **migrate** | Widen the trait surface; one implementor + one consumer. |
| `RegistryDispatcher::register` | `crates/anvil-intercept/src/lib.rs:98` | **migrate** | Implementor of widened trait. Branches on `(Some, Some)` vs legacy. |
| All `tests/*.rs` and `#[cfg(test)] mod tests` registry users | various | **unchanged** | Test sites use the concrete `SessionRegistry::register` (not the trait) and continue to work without lineage. |
| Embedded path (`crates/anvil-intercept/src/embedded.rs`) | embedded.rs | **unchanged** | Embedded mode does not go through the IPC dispatcher; pre-MLP2-025 behaviour preserved. The spoof cross-check is daemon-only. |
| Launcher (`anvil-run`) | external crate | **migrate** | Update register-session call to include `pid` + `pid_starttime` from the launcher's own PID. Same for `driver-client`. Tracked under a separate APS sub-item if it grows. |

**Greenfield additions (not migrations):**

- `FenceStore::fence_worktree_for_spoof` — new method (§5.3).
- `ScanBufferRequest::env_agent_tag` field — new (§3.1).
- `IpcCommand::RegisterSession::{pid, pid_starttime}` fields — new (§3.2).
- `ScanBufferOutcome::BlockedSpoofedAttribution` variant — new (§3.3).
- `DEGRADED_SPOOFED_ATTRIBUTION` `pub const` — new (§8).

## 10. Open questions

### Q1 — Extend `handle_scan_buffer_jsonrpc` in place, or add a wrapper?

- **Candidates:**
  - **(a)** Extend the existing function with three new parameters (`registry`, `fence_store`, `peer_pid`). One call site changes; one extra branch.
  - **(b)** Add a new outer function `handle_scan_buffer_with_spoof_check` that calls the existing handler after the cross-check. Cleaner separation; one more function in the dispatch chain.
- **Chosen:** **(a)** — extend in place. The cross-check is structurally part of the IPC dispatch; splitting it into a wrapper hides the seam from the file that already owns the dispatch. The three new parameters are local to one call site at `ipc.rs:1836–1846`.

### Q2 — Where does `peer_pid` actually come from?

- **Candidates:**
  - **(a)** Thread it from socket-accept through every layer down to `handle_scan_buffer_jsonrpc`. Touches the listener loop, dispatcher, JSON-RPC handler. Cleanest data flow.
  - **(b)** Read peer credentials lazily inside the JSON-RPC handler via a closure or socket reference. Avoids parameter creep but couples the handler to socket internals.
- **Chosen:** **(a)** — thread from socket accept. The existing `validate_connected_peer_for_client` at `ipc.rs:258` already reads `SO_PEERCRED` (Linux) / `LOCAL_PEERCRED` (macOS) / `GetNamedPipeServerProcessId` (Windows). It currently drops the PID after UID validation; this spec keeps it. Lazy reads have race conditions if the peer exits mid-request.

### Q3 — Should `env_agent_tag` on `ScanBufferRequest` be `Option<AgentTag>` or `Option<String>` (raw env value)?

- **Candidates:**
  - **(a)** `Option<AgentTag>` — writer parses, daemon trusts the parse.
  - **(b)** `Option<String>` — writer forwards raw, daemon parses (rejects malformed at the boundary).
- **Chosen:** **(b)** — `Option<String>`. The daemon is the authoritative parser; if a malformed env value reaches the wire (e.g. corrupted by an upstream tool), the daemon classifies it as `Cross::Spoofed` rather than as a deserialisation error. This is the existing convention from `agent_tag_from_env_value` (Empty + Malformed both fold to `None`). **Spec note:** the field is `Option<String>` on the wire; the daemon decodes to `Option<AgentTag>` internally.

  Updated §3.1:
  ```rust
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub env_agent_tag: Option<String>,
  ```

### Q4 — Should `IpcCommand::RegisterSession::{pid, pid_starttime}` be one merged `Option<LineageAnchor>` struct or two separate fields?

- **Candidates:**
  - **(a)** Two fields, paired-or-nothing (current §3.2).
  - **(b)** `Option<LineageAnchor { pid, pid_starttime }>` — one optional struct, neither-or-both is encoded in the type.
- **Chosen:** **(b)** — single nested struct. Forecloses the "one supplied, the other not" mis-pairing that §3.2 currently handles defensively. Cheaper to evolve later (add fields to `LineageAnchor` without bumping `RegisterSession`).

  Updated §3.2 sketch:
  ```rust
  pub struct LineageAnchor {
      pub pid: u32,
      pub pid_starttime: u64,
  }
  pub enum IpcCommand {
      RegisterSession {
          session_id: SessionId,
          worktree: PathBuf,
          #[serde(default, skip_serializing_if = "Option::is_none")]
          agent_tag: Option<AgentTag>,
          #[serde(default, skip_serializing_if = "Option::is_none")]
          lineage: Option<LineageAnchor>,
      },
      // …
  }
  ```

### Q5 — Does the spoofed-write block return a JSON-RPC error or a successful response with the new outcome variant?

- **Candidates:**
  - **(a)** JSON-RPC error (e.g. `-32000` application error) — clear failure signal.
  - **(b)** Successful response with `ScanBufferOutcome::BlockedSpoofedAttribution` — clear semantic distinction; not a protocol error, an enforcement outcome.
- **Chosen:** **(b)**. The block is a deliberate enforcement decision, not a transport error. Treating it as a JSON-RPC error conflates two concerns and breaks clients that distinguish "the daemon refused this write" from "the request was malformed". Existing `Interrupted` outcome already uses this pattern.

### Q6 — Should we add a separate `MLP2-025c` for `anvil-run` and `driver-client` launcher migration?

- **Candidates:**
  - **(a)** Fold the launcher migration into MLP2-025b (this work item).
  - **(b)** Split out as MLP2-025c, leaving MLP2-025b daemon-only.
- **Chosen:** **(b)** — split. The daemon-side wire-up can ship and exercise the path with synthetic IPC fixtures (PR tests). The launcher migration is a separate concern in `crates/anvil-run/` and `crates/anvil-driver-client/` (assuming the crate exists). Decoupling them lets MLP2-025b land and unblocks future launchers without coupling release cadence.

**No `BLOCKING` open questions remain.** Spec is `Draft` → ready for `mini` Council review.

---

## Reviewer checklist

- [ ] §3 — every new wire field has `#[serde(default, skip_serializing_if = "Option::is_none")]` and an explicit additive guarantee. ✓
- [ ] §4 — every arrow has a file:line citation or a `[NEW]` label. ✓
- [ ] §5 — every new signature has pre-conditions and post-conditions. ✓
- [ ] §7 — every error has a recovery verdict, not just a description. ✓
- [ ] §9 — every migration site has a chosen action. ✓
- [ ] §10 — no `BLOCKING` questions. ✓
