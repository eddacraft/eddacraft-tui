# MLP2-026 — `degraded:fence-cascade` mode at 5 fences in 60s

## 1. Identifiers and status

| Field | Value |
| ----- | ----- |
| Spec id | `2026-05-16-mlp2-026-fence-cascade-control-lane` |
| Status | Draft |
| Date | 2026-05-16 |
| Owners | @aneki |
| Work item | MLP2-026 (`degraded:fence-cascade` mode at 5 fences in 60s) |
| Supersedes | — |
| Council tier | `mini` for the spec review; `quick` per impl subtask thereafter |

Companion artefacts:

- APS: `plans/modules/multilayer-protection-v2.aps.md` MLP2-026 block (lines 1353–1556).
- Template: `plans/specs/INTEGRATION-SPEC-TEMPLATE.md`.
- Prior art: `plans/specs/2026-05-16-mlp2-025-spoof-cross-check-control-lane.md` (the first spec using this template — same shape, same conventions).

## 2. Problem statement

The APS body specifies **what** MLP2-026 needs to ship; this spec answers the integration-time questions the APS prose left implicit:

1. **Lock-ordering contract.** `register()` holds the registry lock for the full operation today (`registry.rs:447`). The new `is_cascaded` check has to acquire the fence-store's snapshot without nesting the two locks — what is the precise acquire/release sequence and what happens if the snapshot is stale by the time `register()` resumes?
2. **Restart semantics.** Cascade is a security boundary; the APS says daemon restart restores engaged state. What exactly persists (just the `since_unix` flag? the in-memory rate-window contents?), and what doesn't?
3. **`OperatorContext` provenance.** The APS names the type but doesn't say which IPC peer-credential calls feed which field, on which platforms, with what fail-closed semantics.
4. **`WorktreeStatus` wire-compat.** Two field additions on both `WorktreeStatus` (in-memory) and `WorktreeStatusV1` (proto). What's the exact serde guard, the manual conversion, and the round-trip test?
5. **Clear-side authority.** Who can invoke `IpcCommand::UnblockCascade`? The daemon's existing UID gate accepts any same-UID peer. Is that sufficient for cascade-clear or does the verb need stricter authorisation?

This spec resolves each before TDD starts.

## 3. Data shapes

### 3.1 `CascadeRecord` — new persisted record on `FenceFile`

New type in `crates/anvil-intercept/src/fence.rs` alongside the existing `FenceRecord` (`fence.rs:60–67`):

```rust
/// MLP2-026: per-worktree cascade engaged-state record. Persisted
/// inside `FenceFile.cascades` so daemon restart preserves the
/// security-relevant engaged flag — only the in-memory
/// `RateWindow` resets on restart, which is the correct posture:
/// the engaged flag stays sticky, the firing window is rebuilt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CascadeRecord {
    /// Canonical worktree path (matches `FenceRecord::worktree`
    /// canonicalisation convention).
    pub worktree: PathBuf,
    /// Engage timestamp as Unix seconds — mirrors
    /// `FenceRecord::fenced_at_unix` (`fence.rs:66`).
    pub since_unix: u64,
    /// Always `DEGRADED_FENCE_CASCADE` for v1. Stored explicitly
    /// so structured-log consumers do not need to look up the
    /// constant.
    pub reason: String,
}
```

### 3.2 `FenceFile.cascades` — wire-additive field

Extend `FenceFile` (`fence.rs:116–120`):

```rust
#[derive(Debug, Serialize, Deserialize)]
struct FenceFile {
    version: u8,
    fences: Vec<FenceRecord>,
    /// MLP2-026: cascade engaged-state records. Wire-additive
    /// via `#[serde(default, skip_serializing_if = "Vec::is_empty")]`,
    /// matching the `FenceRecord::aliases` precedent
    /// (`fence.rs:63`). `version` stays at 1.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    cascades: Vec<CascadeRecord>,
}
```

A pre-MLP2-026 fence file (no `cascades` key) deserialises with `cascades = vec![]`. A new daemon emitting a record with non-empty cascades is parsed by older daemons that tolerate unknown fields (they ignore the field, which is the safe degradation — cascade enforcement is loosened, not silently violated).

Wire-shape example:

```json
{
  "version": 1,
  "fences": [{"worktree":"/work/wt","reason":"rule violation","fenced_at_unix":1700000000}],
  "cascades": [{"worktree":"/work/wt","since_unix":1700000100,"reason":"degraded:fence-cascade"}]
}
```

### 3.3 `OperatorContext` — new type in proto

New type in `crates/anvil-intercept-proto/src/session.rs` (alongside `AgentTag` and `LineageAnchor`):

```rust
/// MLP2-026: audit context recorded by the daemon when an
/// operator clears a cascade via `IpcCommand::UnblockCascade`.
/// Populated server-side from the IPC peer credentials at the
/// moment the verb is received; never trusted from the client
/// payload directly (a client-supplied OperatorContext on the
/// wire is silently overwritten).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperatorContext {
    /// Effective UID from SO_PEERCRED (Linux) /
    /// LOCAL_PEERCRED (macOS). `None` when the credential read
    /// failed (the daemon still clears the cascade — see §7 —
    /// but the audit trail records the gap).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uid: Option<u32>,
    /// Peer process id from the same syscall. Always populated
    /// on Linux + macOS (we already capture peer_pid for
    /// MLP2-025b); `None` on platforms / paths where the read
    /// is undefined.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pid: Option<u32>,
    /// Result of `gethostname(3)` at the daemon side. Best-effort.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hostname: Option<String>,
}
```

### 3.4 `IpcCommand::UnblockCascade` — new variant

Extend `IpcCommand` in `crates/anvil-intercept-proto/src/lib.rs:64–120`:

```rust
/// MLP2-026: clear a worktree's `degraded:fence-cascade`
/// engaged state and reset its in-memory rate window. The
/// daemon overwrites any client-supplied `operator` field with
/// values it derives from the peer credentials of the connection
/// itself — clients SHOULD send `operator: None` or omit the
/// field; doing otherwise has no security implication but is
/// noise on the wire.
UnblockCascade {
    worktree: PathBuf,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    operator: Option<session::OperatorContext>,
},
```

Wire shape (request):

```json
{"jsonrpc":"2.0","method":"unblock-cascade","params":{"worktree":"/work/wt"},"id":"req-1"}
```

Response shape: same `{"ok": true}` pattern the existing register/unregister handlers use.

### 3.5 `RegistryError::WorktreeCascaded` — new variant

Extend `RegistryError` in `crates/anvil-intercept/src/registry.rs:91–153`:

```rust
/// MLP2-026: registration refused because the worktree is in
/// `degraded:fence-cascade` mode. Mirrors the
/// `SessionCapExceeded` precedent (`registry.rs:148–151`); the
/// wire layer maps to JSON-RPC `-32603 Internal error` with
/// the error message as data.
#[error("worktree is in degraded fence-cascade mode and refuses new sessions: {worktree:?}")]
WorktreeCascaded { worktree: PathBuf },
```

### 3.6 `WorktreeStatus` and `WorktreeStatusV1` — new fields

In-memory (`crates/anvil-intercept/src/status.rs:121–126`):

```rust
pub struct WorktreeStatus {
    pub worktree: std::path::PathBuf,
    pub session_id: anvil_intercept_proto::SessionId,
    pub fenced: bool,
    /// MLP2-026: `true` when the worktree is in
    /// `degraded:fence-cascade` mode. Distinct from `fenced` —
    /// a worktree can be cascaded without being individually
    /// fenced (cascade refuses NEW sessions; fence refuses
    /// enforcement on existing ones).
    pub cascaded: bool,
    /// MLP2-026: Unix seconds at which the cascade was engaged.
    /// `None` when not cascaded.
    pub cascade_since: Option<u64>,
}
```

Wire mirror (`crates/anvil-intercept-proto/src/status.rs:71–76`):

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorktreeStatusV1 {
    pub worktree: PathBuf,
    pub session_id: SessionId,
    pub fenced: bool,
    /// MLP2-026: see in-memory `WorktreeStatus::cascaded`.
    #[serde(default)]
    pub cascaded: bool,
    /// MLP2-026: see in-memory `WorktreeStatus::cascade_since`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cascade_since: Option<u64>,
}
```

`cascaded` uses `#[serde(default)]` only (no skip-if) so the field is always present on the wire — operators reading status snapshots see `cascaded: false` explicitly, not absence. `cascade_since` is skip-if to keep the common case (`None`) compact.

### 3.7 `DEGRADED_FENCE_CASCADE` consts

New consts in `crates/anvil-intercept/src/telemetry.rs` alongside `DEGRADED_SPOOFED_ATTRIBUTION` (`telemetry.rs:27`):

```rust
pub const DEGRADED_FENCE_CASCADE: &str = "degraded:fence-cascade";
pub const DEGRADED_FENCE_CASCADE_CLEAR: &str = "degraded:fence-cascade-clear";
```

## 4. Message flow

### 4.1 Engage path

```
┌─────────┐                                          ┌────────────────┐
│ Caller  │                                          │ FenceStore     │
│ of      │                                          │ (in-memory     │
│ fence_  │                                          │  RateWindow +  │
│ worktree│                                          │  on-disk file) │
└────┬────┘                                          └────────┬───────┘
     │                                                         │
     │ 1. fence_worktree(worktree, reason)                     │
     │   [fence.rs:168 — unchanged signature]                  │
     │ ───────────────────────────────────────────────────────▶│
     │                                                         │
     │                                  2. load() current state│
     │                                     [fence.rs:133]      │
     │                                                         │
     │                                  3. [NEW] window.record(│
     │                                        Instant::now())  │
     │                                                         │
     │            ┌─ RateDecision::Allow ────────────────────┐ │
     │            │ 4a. upsert FenceRecord (unchanged)        │ │
     │            │ 5a. save() returns                        │ │
     │            └───────────────────────────────────────────┘ │
     │            ┌─ RateDecision::Throttle ──────────────────┐ │
     │            │ 4b. [NEW] write CascadeRecord into        │ │
     │            │     FenceFile.cascades; upsert            │ │
     │            │     FenceRecord (still fence the          │ │
     │            │     individual fire — cascade is          │ │
     │            │     additional, not replacing)            │ │
     │            │ 5b. [NEW] emit notification               │ │
     │            │     envelope_for_fence_transition(...,    │ │
     │            │     ActiveToFenced) with reason =         │ │
     │            │     DEGRADED_FENCE_CASCADE                │ │
     │            │ 6b. [NEW] tracing::warn!(target =         │ │
     │            │     "anvil_intercept::fence", reason,     │ │
     │            │     %worktree, since_unix, ...)           │ │
     │            └───────────────────────────────────────────┘ │
     │                                                         │
     │ 7. Result<FenceRecord, FenceStoreError>                 │
     │   (existing return shape — unchanged)                   │
     │ ◀───────────────────────────────────────────────────────│
     │                                                         │
```

### 4.2 Refuse path (during register)

```
┌──────────────┐                  ┌─────────────┐                  ┌──────────────┐
│ IPC handler  │                  │ FenceStore  │                  │ Session-     │
│ (dispatch_   │                  │             │                  │ Registry     │
│  command)    │                  │             │                  │              │
└──────┬───────┘                  └──────┬──────┘                  └──────┬───────┘
       │                                  │                                │
       │ 1. register(...) — IpcCommand    │                                │
       │   already validated              │                                │
       │ ─────────────────────────────────┼───────────────────────────────▶│
       │                                  │                                │
       │                                  │  2. [NEW] snapshot:            │
       │                                  │     is_cascaded(&worktree)     │
       │                                  │◀───────────────────────────────│
       │                                  │  (acquire & release            │
       │                                  │   fence_store internal lock    │
       │                                  │   — no nesting with registry   │
       │                                  │   lock; cascade-before-        │
       │                                  │   registry ordering)           │
       │                                  │                                │
       │                                  │                                │  3. on cascaded=true:
       │                                  │                                │     return Err(
       │                                  │                                │     RegistryError::
       │                                  │                                │     WorktreeCascaded {...})
       │                                  │                                │     — registry lock NEVER
       │                                  │                                │     acquired in this branch
       │                                  │                                │
       │                                  │                                │  4. on cascaded=false:
       │                                  │                                │     proceed with existing
       │                                  │                                │     register() body
       │                                  │                                │     (lines 447–500)
       │ ◀─────────────────────────────────────────────────────────────────│
       │                                                                   │
```

### 4.3 Clear path

```
┌─────────┐                ┌──────────────┐               ┌────────────┐
│ CLI:    │                │ IPC handler  │               │ FenceStore │
│ anvil   │                │ (new         │               │            │
│ intercept│               │  unblock-    │               │            │
│ unblock │                │  cascade)    │               │            │
└────┬────┘                └──────┬───────┘               └─────┬──────┘
     │                            │                              │
     │ 1. canonicalise worktree   │                              │
     │   (lookup_path)            │                              │
     │ 2. send                    │                              │
     │   IpcCommand::             │                              │
     │   UnblockCascade {         │                              │
     │     worktree, operator:    │                              │
     │     None,                  │                              │
     │   }                        │                              │
     │ ──────────────────────────▶│                              │
     │                            │                              │
     │                            │ 3. [NEW] read peer creds     │
     │                            │   from connection            │
     │                            │   (uid, pid, hostname);      │
     │                            │   build OperatorContext      │
     │                            │   server-side (overwrites    │
     │                            │   any client-supplied        │
     │                            │   value)                     │
     │                            │                              │
     │                            │ 4. [NEW] clear_cascade(      │
     │                            │   &worktree)                 │
     │                            │ ────────────────────────────▶│
     │                            │                              │ 5. [NEW] load(),
     │                            │                              │    remove cascade
     │                            │                              │    record,
     │                            │                              │    reset in-memory
     │                            │                              │    RateWindow for
     │                            │                              │    this worktree,
     │                            │                              │    save()
     │                            │ 6. [NEW] emit notification   │
     │                            │   envelope_for_fence_        │
     │                            │   transition(...,            │
     │                            │   FencedToActive) reason =   │
     │                            │   DEGRADED_FENCE_CASCADE_    │
     │                            │   CLEAR; tracing::info!      │
     │                            │   with ?operator context     │
     │                            │                              │
     │ 7. {"ok": true}            │                              │
     │ ◀──────────────────────────│                              │
     │                            │                              │
```

## 5. Function signatures

### 5.1 `FenceStore::fence_worktree` — body change, signature unchanged

```rust
pub fn fence_worktree(
    &self,
    worktree: &Path,
    reason: impl Into<String>,
) -> Result<FenceRecord, FenceStoreError>
```

**Pre-conditions:** unchanged from today (`fence.rs:168`).

**Post-conditions (new):**

- Fire recorded through the per-worktree `RateWindow::new(4, Duration::from_secs(60))` exactly once.
- On `RateDecision::Throttle`: a `CascadeRecord { worktree, since_unix: now, reason: DEGRADED_FENCE_CASCADE.into() }` is persisted in `FenceFile.cascades`; the engage notification + `tracing::warn!` are emitted exactly once.
- Subsequent fires on an already-cascaded worktree do NOT re-emit (idempotent — pinned by `cascade_state_persists_until_acknowledged` test in the APS validation list).

**Errors:** unchanged. `FenceStoreError::Write`/`Read`/`InsecureStoreParent` propagate. A failure to persist the cascade record is logged via `tracing::error!` but does NOT prevent the underlying fence from being recorded (defence in depth: the per-fire fence is still active even if the cascade engaged-state couldn't be persisted).

**Lock-acquisition note:** `FenceStore` does not hold a lock today (just a `PathBuf`); the `RateWindow` it embeds uses its own `Mutex<Inner>`. The on-disk file is the serialisation point — `load`/`save` are atomic via the existing temp+rename pattern (`fence.rs:234–243`).

### 5.2 `FenceStore::is_cascaded(&Path) -> bool`

```rust
/// MLP2-026: snapshot accessor for the cascade engaged-state.
/// Reads the current on-disk fence file via `load()` and returns
/// `true` iff a `CascadeRecord` exists for any path that
/// canonicalises to the supplied worktree (uses the same
/// `lookup_path` canonicalisation as `unblock_worktree`).
///
/// Used by `SessionRegistry::register` to refuse new sessions
/// on a cascaded worktree (§4.2). The snapshot semantics are
/// intentional: the caller drops this lock before acquiring the
/// registry lock (cascade-before-registry ordering, §6 inv-2).
///
/// Returns `false` on `load()` failure rather than propagating
/// the error — the call site is on the registration hot path
/// and a degraded fence-store I/O is a separate concern that
/// FenceStateUnavailable already surfaces on other paths.
#[must_use]
pub fn is_cascaded(&self, worktree: &Path) -> bool
```

### 5.3 `FenceStore::clear_cascade(&Path) -> Result<bool, FenceStoreError>`

```rust
/// MLP2-026: operator-clear of a cascade engaged-state. Removes
/// the matching `CascadeRecord` from the on-disk file, resets
/// the in-memory `RateWindow` for the worktree, persists the
/// change.
///
/// Returns `Ok(true)` when a cascade record existed and was
/// removed, `Ok(false)` when no cascade was engaged for the
/// worktree (idempotent operator-clear). `Err(_)` only on
/// underlying I/O failure.
///
/// Pre-condition: caller has already canonicalised the worktree
/// path (the daemon's IPC handler does this; the CLI mirrors
/// the canonical form before dispatch — see §4.3).
///
/// Post-conditions:
/// - On `Ok(true)`: the engage flag is cleared; the rate-window
///   is reset; subsequent `fence_worktree` calls start counting
///   from zero again.
/// - The clear notification + `tracing::info!` are emitted by
///   the IPC handler, NOT by this function — separation keeps
///   the store API observation-free for direct test usage.
pub fn clear_cascade(&self, worktree: &Path) -> Result<bool, FenceStoreError>
```

### 5.4 IPC handler for `UnblockCascade`

Added inside `dispatch_command` (`ipc.rs:2517–2592` region):

```rust
IpcCommand::UnblockCascade {
    worktree,
    operator: _client_supplied,  // overwritten with daemon-derived
} => {
    let daemon_operator = read_operator_context_from_peer(&peer_creds);
    let cleared = fence_store
        .clear_cascade(worktree)
        .map_err(|err| err.to_string())?;
    if cleared {
        emit_cascade_clear_notification(&telemetry_ctx, worktree);
        tracing::info!(
            target: "anvil_intercept::fence",
            reason = DEGRADED_FENCE_CASCADE_CLEAR,
            %worktree.display(),
            ?daemon_operator,
            "cascade cleared by operator",
        );
    }
    Ok(json!({"ok": cleared}))
}
```

**Pre-conditions:** peer credentials already validated upstream (existing UID gate). `fence_store` is in scope (matches existing `dispatch_command` signature pattern).

**Post-conditions:** `OperatorContext` is built server-side; client-supplied value is dropped. `cleared` reflects whether a record was removed; subsequent calls for the same worktree are idempotent.

**Lock ordering:** `clear_cascade` takes `FenceStore`'s internal lock briefly; no registry lock is acquired in this path. Safe per cascade-before-registry rule.

### 5.5 CLI `anvil intercept unblock --acknowledge-cascade <worktree>`

New subcommand in `crates/anvil-cli/src/commands/intercept.rs:21–30`:

```rust
enum InterceptCommand {
    Start(StartArgs),
    Status(StatusArgs),
    /// MLP2-026: clear a `degraded:fence-cascade` engaged-state.
    /// The `--acknowledge-cascade` flag is required as a UX
    /// affordance — it documents operator intent on the command
    /// line. The audit-of-record is the `OperatorContext` the
    /// daemon derives from the IPC peer credentials; the flag
    /// itself is not on the wire.
    Unblock(UnblockArgs),
}

#[derive(Debug, clap::Args)]
struct UnblockArgs {
    worktree: PathBuf,
    /// Required affordance — confirms operator intent.
    #[arg(long)]
    acknowledge_cascade: bool,
}
```

**Pre-conditions:** worktree path canonicalised via the same `lookup_path` guard the fence store uses (already exported as a helper).

**Errors:** clap surfaces `--acknowledge-cascade` missing as a parse error. IPC errors propagate via the existing client-side error display.

## 6. Lifecycle and invariants

### Creation

- A `CascadeRecord` is created exclusively inside `FenceStore::fence_worktree` when `RateWindow::record` returns `Throttle`.
- An `OperatorContext` is created exclusively inside the IPC handler for `UnblockCascade` (or, in future verbs, any operator action that needs an audit trail).

### Liveness

- `CascadeRecord` persists on disk until `clear_cascade(&path)` removes it.
- The in-memory `RateWindow` for a worktree is rebuilt empty on daemon restart — the engage flag survives, the firing window does not.

### Consistency invariants

- **inv-1:** at any observable point, `is_cascaded(&path)` returns `true` iff a `CascadeRecord` exists for the canonical form of `path` in the current `FenceFile`.
- **inv-2:** `cascade-before-registry` lock ordering. The fence-store snapshot is acquired and released BEFORE `SessionRegistry::Inner::Mutex` is acquired. Cited at both call sites (`registry.rs:447` and the new is_cascaded probe just above it). Violating this introduces a potential deadlock with future code paths that hold the registry lock and call into fence operations.
- **inv-3:** `clear_cascade` is idempotent. Multiple calls return `Ok(false)` after the first `Ok(true)`. The in-memory rate window for the worktree is reset on every successful clear; on idempotent `Ok(false)` it is also reset (defensive — if there was leftover window state without an engaged record, that's drift we want to reset).
- **inv-4:** daemon restart preserves the engage flag. After a clean restart, `is_cascaded(&path)` returns the same value it would have returned before the restart. The in-memory window is empty; the next `fence_worktree` call would record the first fire of a new window but the engage flag stays `true` until explicit clear.

## 7. Error channel

| Path | Error / outcome | Wire surface | Verdict |
| ---- | --------------- | ------------ | ------- |
| Fence under threshold | none | existing `Result<FenceRecord>` | allow (existing fence behaviour unchanged) |
| Fence triggers cascade | none | existing `Result<FenceRecord>` (cascade is a side effect, not an error) | allow per-fire; cascade engaged for future writes |
| `clear_cascade` succeeds | none | `{"ok": true}` | allow |
| `clear_cascade` idempotent miss | none | `{"ok": false}` | allow (operator is informed nothing was cleared) |
| `register` on cascaded worktree | `RegistryError::WorktreeCascaded` | JSON-RPC `-32603` with the typed error message | **deny** — refuse the registration. Mirrors `SessionCapExceeded`. |
| `FenceStore::load` fails during `is_cascaded` | swallowed, returns `false` | n/a | **allow** the registration. Reasoning: `is_cascaded` is on the hot register path; failing-closed here would cause an operator-visible refusal whenever the fence file is briefly unreadable (e.g. during the temp+rename window). The per-fire fence path remains intact and is the security primary surface. |
| `FenceStore::save` fails during cascade engage | logged via `tracing::error!`; fence still recorded | n/a | **allow** the in-memory engage flag if subsequent operations can read it; subsequent `fence_worktree` calls retry the save. |
| `clear_cascade` underlying I/O fails | `FenceStoreError::Write` | JSON-RPC `-32603` | **propagate** to operator — they should see the failure and retry. |
| Peer-credential read fails during `UnblockCascade` | `OperatorContext` fields default to `None` | request still succeeds | **allow** the clear; the audit trail records the gap (`uid: None`). The clear-side authority gate is the existing UID-match at socket accept, NOT the OperatorContext — losing peer credentials at clear time does not bypass anything that wasn't already required at connect time. |

**Trust model.** Same trust zone as MLP2-025b: same-UID peers can issue any IPC command. `UnblockCascade` does not introduce a NEW trust boundary — it is a same-UID operator action. The `OperatorContext` is an audit field, not an authorisation field. Cascade clear is intentionally permissive within the trust zone; the operator-on-call signal is the requirement of running `anvil intercept unblock --acknowledge-cascade` (the affordance, not the wire). A future tighter model can ratchet by adding a `requires_ack: bool` flag to `CascadeRecord` and refusing clear without a matching `OperatorContext.uid`, but that is out of scope for v1.

## 8. Observability contract

### Notification envelope

Both transitions reuse the existing `envelope_for_fence_transition` helper (`telemetry.rs:330–365`):

| Event | Transition | Priority | Reason |
| ----- | ---------- | -------- | ------ |
| Engage | `ActiveToFenced` | `Critical` | `DEGRADED_FENCE_CASCADE` |
| Clear  | `FencedToActive` | `Normal`   | `DEGRADED_FENCE_CASCADE_CLEAR` |

The priority asymmetry mirrors the existing `FenceTransition` mapping at `telemetry.rs:339–342` (Critical on engage, Normal on clear).

### Tracing

```rust
// Engage (inside fence_worktree, on RateDecision::Throttle):
tracing::warn!(
    target: "anvil_intercept::fence",
    reason = DEGRADED_FENCE_CASCADE,
    %worktree.display(),
    since_unix,
    "cascade engaged after 5 fences in 60s",
);

// Clear (inside UnblockCascade handler, after clear_cascade returns Ok(true)):
tracing::info!(
    target: "anvil_intercept::fence",
    reason = DEGRADED_FENCE_CASCADE_CLEAR,
    %worktree.display(),
    ?operator,  // OperatorContext (Debug)
    "cascade cleared by operator",
);
```

### Both channels are required

The notification envelope drives real-time operator surfaces (`anvil intercept status`, future TUI watchers); the tracing channel drives structured-log collectors. A cascade event must be visible on both — operators using either surface should see it.

### Constants

Two new `pub const` literals in `telemetry.rs`:

```rust
pub const DEGRADED_FENCE_CASCADE: &str = "degraded:fence-cascade";
pub const DEGRADED_FENCE_CASCADE_CLEAR: &str = "degraded:fence-cascade-clear";
```

Single find-target for the future enum migration (MLP2-058 follow-on).

## 9. Migration plan

The cascade integration is mostly transparent: existing `fence_worktree` callers don't change their call shape, only their side effects.

| Site | File:line | Action | Reason |
| ---- | --------- | ------ | ------ |
| `dispatch_command::register` | `crates/anvil-intercept/src/ipc.rs:2517+` (the dispatcher branch for `IpcCommand::RegisterSession`) | **migrate** | Add the `is_cascaded` snapshot probe before invoking the dispatcher's `register`. New `WorktreeCascaded` error path. |
| `SessionRegistry::register` | `crates/anvil-intercept/src/registry.rs:438–500` | **migrate** | Document `cascade-before-registry` lock ordering at the call site (cited comment); no signature change. |
| Daemon-side spoof control-lane fence | `crates/anvil-intercept/src/ipc.rs:2537` (via `fence_worktree_for_spoof` → `fence_worktree`) | **unchanged** | `fence_worktree` body changes; the call site keeps its current signature. Cascade engage is now an additional side effect. |
| Daemon-side spoof fence (alt site) | `crates/anvil-intercept/src/unregistered.rs:130` | **unchanged** | Same reasoning. |
| Restart + live fence | `crates/anvil-intercept/src/lib.rs:1164` + `1187` | **unchanged** | Same reasoning. |
| `IpcCommand` enum | `crates/anvil-intercept-proto/src/lib.rs:64–120` | **migrate** | Add the new `UnblockCascade` variant. Wire-additive; pre-MLP2-026 daemons fail unknown-method which is the correct refusal. |
| `RegistryError` | `crates/anvil-intercept/src/registry.rs:91–153` | **migrate** | Add `WorktreeCascaded` variant. New error path, no signature break. |
| `WorktreeStatus` / `WorktreeStatusV1` conversion | `crates/anvil-intercept/src/status.rs:160–171` | **migrate** | Thread `cascaded` + `cascade_since` through the manual conversion. |
| `render_status` | `crates/anvil-intercept/src/status.rs:436–482` | **migrate** | Insert `cascade: engaged since <ts>` line after fences when applicable. |
| `anvil intercept` CLI subcommands | `crates/anvil-cli/src/commands/intercept.rs:21–30` | **migrate** | Add `Unblock(UnblockArgs)` variant and dispatch handler. |
| FenceStore unit tests | `crates/anvil-intercept/src/fence.rs:541–775` | **migrate** | Existing fixtures that build `FenceFile` literals or assert on its serde shape may need updating once `cascades` is added. The `#[serde(default)]` guard should mean most fixtures remain valid; explicit shape-pinning tests need the new field. |

**Greenfield (no migration; new code only):**

- `CascadeRecord` type.
- `FenceStore::is_cascaded`, `clear_cascade`.
- `OperatorContext` type in `proto/session.rs`.
- `WorktreeCascaded` error variant body.
- New IPC handler arm for `UnblockCascade`.
- CLI `Unblock` subcommand.
- Two new telemetry consts.

## 10. Open questions

### Q1 — Restart with corrupt fence file: refuse startup or auto-recover?

- **Candidates:**
  - **(a)** Daemon refuses startup with the existing `FenceStoreError::Parse` path. Operator must manually edit / delete the file. Same posture as today for any other fence-file corruption.
  - **(b)** Daemon logs the corruption, replaces the file with an empty `FenceFile { version: 1, fences: vec![], cascades: vec![] }`, and continues. Cascade engage flags lost.
- **Chosen:** **(a)** — preserve existing behaviour. The fence file is already treated as a security artefact today; cascade records inherit the same protection. Auto-recovery would silently clear cascades, which is the exact failure mode the persistence is designed to prevent.

### Q2 — Should cascade-clear take a wire-supplied `OperatorContext` at all if the daemon overwrites it?

- **Candidates:**
  - **(a)** Keep `operator: Option<OperatorContext>` on the wire as documented (clients SHOULD send `None`; daemon overwrites). Minor noise on the wire when populated.
  - **(b)** Drop the field from the wire entirely — daemon derives the context, period.
- **Chosen:** **(a)**. The field-on-the-wire pattern allows a future variant where the daemon HONOURS a client-supplied operator context for cross-host audit (e.g. an ssh-tunnelled CLI invoking via a forwarded socket). v1's "overwrite" is a defensive default, not a permanent restriction. The cost of carrying the field on the wire is one optional JSON key when populated; the cost of changing the wire later is a proto bump. The wire-additive precedent wins.

### Q3 — Engage idempotency: re-emit if cascade is already engaged?

- **Candidates:**
  - **(a)** Re-emit notification + tracing every time the rate window fires `Throttle` while a cascade is already engaged. Operator sees the noise; structured logs over-fill.
  - **(b)** Emit exactly once per engage. After the first `Throttle`, subsequent throttles on the same already-engaged worktree silently do not re-emit (still propagate to caller as `Throttle` for accounting, just no observability events).
- **Chosen:** **(b)**. The operator surface is for "something changed" events; a worktree already in cascade mode doesn't need re-notification on every excess fire. The rate-window itself counts the drops via `RateDecision::Throttle { drops }`; if a future operator surface wants "how many fires during cascade" that's a separate metric, not a noise channel. APS `cascade_state_persists_until_acknowledged` test pins this behaviour.

### Q4 — Cascade engage during `unblock_worktree`: race or coexistence?

- **Candidates:**
  - **(a)** Cascade clear and individual unblock are independent — running `anvil intercept unblock <wt>` (the existing per-fence unblock) does NOT clear a cascade. Operator must run `anvil intercept unblock --acknowledge-cascade <wt>` explicitly.
  - **(b)** Per-fence unblock implicitly clears the cascade if engaged. Single command suffices.
- **Chosen:** **(a)**. The cascade engaged-state is a distinct affordance — it signals "this worktree has been misbehaving repeatedly" and the operator-touch surface should be explicit. Conflating the two commands silently downgrades the security signal. Documented in the CLI help text.

### Q5 — Per-task vs per-worktree cascade key?

- **Already resolved in APS (Council 2026-05-15, Q2 verdict, MLP2-025b spec §10):** worktree-only for v1; per-tag escalation `(WorktreeKey, AgentTag)` deferred. The cascade engage path here keys on worktree path; per-task cascade tracking is a future axis if usage patterns demand it.

**No `BLOCKING` open questions remain.** Spec is `Draft` → ready for `mini` Council review.

---

## Reviewer checklist

- [ ] §3 — every new wire field has `#[serde]` attrs and an explicit additive guarantee. ✓
- [ ] §4 — every arrow has a file:line citation or a `[NEW]` label. ✓
- [ ] §5 — every new signature has pre-conditions, post-conditions, errors, lock-ordering note. ✓
- [ ] §6 — invariants stated as `inv-N:` so they can be cited from §7. ✓
- [ ] §7 — every error has a deny/allow/record verdict. ✓
- [ ] §9 — every migration site has a chosen action. ✓
- [ ] §10 — no `BLOCKING` questions remain. ✓
