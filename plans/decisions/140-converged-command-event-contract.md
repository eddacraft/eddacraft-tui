# ADR-140: Converged application command and event contract

## Status

Proposed. Design contract for incremental implementation; no new runtime,
wire API, desktop release or allomorph dependency is delivered by this ADR.

## Date

2026-09-05

## Context

The [two-track strategy](../specs/2026-08-07-two-track-ui-strategy.md)
assigns the shared command/event/run design to converged-app ADR-B.
[allomorph EXP-002](https://github.com/eddacraft/allomorph/blob/acb0ce2c90ac88ed83d13c676b8d22b6791044f9/plans/experiments/exp-002-report.md)
now supplies executable evidence for one typed command projected into plain
text, JSONL, inline Flow, a static terminal workspace and generated agent schema.
Its nine conformance tests passed before
[PR #8](https://github.com/eddacraft/allomorph/pull/8) was rebase-merged.

The experiment is bounded and process-local. Its arena handles are not durable
application identities; its approvals cannot survive a runtime instance; its
replay is complete-file inspection, not crash recovery or authenticated evidence.
It has no delivery retry or durable resume. External human comprehension remains
untested. Accepting these findings does not accept its wire representation as
anvil's production protocol.

anvil already has daemon discovery, protection, settings and evidence contracts.
The app must compose those authorities, rather than installing a second command
authority, governance archive or daemon lifecycle alongside them.

## Decision

### 1. Adopt semantics through anvil-owned contracts

A typed application command owns each mutation; CLI, TUI, native, web, tray and
agents are clients of the same handler and typed result. Clients read projections.
The command catalogue and input/output schemas are generated from those contracts.
No renderer, Flow/Scene type or allomorph crate enters domain/application crates.

Integrated operations use the existing daemon-backed application boundary.
"One local control plane" means the [ADR-036](036-daemon-scope-discovery-and-boundaries.md)
execution scope, not one universal process across machines, users, containers
or Windows/WSL. Preserve discovery, peer checks and unsupported-boundary refusal.
Existing handlers remain authoritative behind adapters; do not create a parallel
implementation merely to demonstrate the new envelope.

[ADR-064](064-intercept-graph-cache-crate-boundary.md) and
[ADR-067](067-daemon-symbol-feed-parse-hook.md) remain binding: the intercept
crate does not acquire the application domain, UI or parser implementation.
Use the existing composition root and injected ports where needed. A daemon
process assembled by the CLI can contain the injected parser without violating
the crate boundary. This ADR does not authorise a new daemon binary or process.

### 2. Logical command envelope

These are required semantics, not a published Rust layout or transport schema.
CONV-002 owns exact DTO/schema placement and executable compatibility fixtures.

| Field | Contract |
| --- | --- |
| schema_version | Explicit application contract version, negotiated independently of transport version |
| command_id | Durable request identity assigned once, preserved across transport retries |
| command_type / command_version | Registered operation and versioned typed payload contract |
| target | Typed project/repository/worktree/object references, validated against the authenticated scope |
| payload | Validated typed input with defaults resolved before digesting |
| actor / source_surface | Server-established principal and ingress; caller claims are separate, untrusted metadata |
| correlation_id | Stable workflow correlation, preserved across related commands |
| causation_id | Reference to the actual parent command/event when known; absent rather than invented |
| traceparent | Existing ADR-035 cross-pipe trace context; never authorisation |
| expected_version | Required aggregate version or content hash for conflict-sensitive writes |
| idempotency_key | Required for retriable mutations, scoped to principal, execution scope and command type |
| requested_at / accepted_at | Caller timestamp is advisory; server acceptance time is authoritative |

A surface name is not proof that an actor is human or trusted. CLI and MCP
are ingress descriptions, not permission grants. Apply the current OS-account
trust boundary and validated registration. Do not promise isolation from hostile
code already running as the same UID/SID. A repository file cannot grant authority.

Reuse [ADR-036](036-daemon-scope-discovery-and-boundaries.md) project identity
and [ADR-116](116-kindling-product-profiles-and-governance-record.md)
repository/worktree/run/event identity where those concerns apply. Never expose
arena slot/generation pairs as durable anvil identity. Keep work item, workspace,
run, session, operation and tab distinct. A retried delivery retains command and
operation identity; a genuinely new execution attempt receives a new attempt/run
identity with an explicit predecessor reference.

### 3. Acceptance, idempotency and failure

Results distinguish Completed, Accepted(operation_id), Rejected, Conflict,
Unavailable and Failed. A lost reply is an unknown delivery outcome, not proof
the operation failed. Query by command/operation identity before retrying.

Persist the idempotency reservation and operation admission atomically before
acknowledging Accepted or starting a side effect. Concurrent duplicate requests
share one admission. Reuse of a key with a different canonical input, target,
command version or expected version is a Conflict. Return the existing operation
or result only after rechecking access; a cached result is not an authorisation
bypass. The identity/digest encoding must be deterministic across supported clients.

Do not claim exactly-once external effects. After a crash between effect and
receipt, reconcile through the provider's idempotency mechanism or report an
indeterminate outcome requiring explicit recovery; never blindly replay a write.
Deduplication retention must cover the advertised retry window. An expired key
cannot silently authorise the same command again: return an explicit expired or
unknown outcome and require a new authorised attempt. CONV-003 must specify
bounded tombstones or an equivalent expired-command recognition mechanism;
deleting every trace of an expired key cannot satisfy this guarantee.

Structured failures distinguish validation, policy, capability, version conflict,
document conflict, unavailable dependency, capacity, redaction and indeterminate
execution. Semantic outcomes are authoritative. CLI adapters preserve existing
numeric exit contracts, including [ADR-132](132-settings-truth-contract.md)
and [anvil-run](../../crates/anvil-run/src/exit_codes.rs).
Do not copy EXP-002's 0/1/2/3/4/130 table: its 3/4 meanings conflict with anvil.

### 4. Events, projections and recovery

Events identify event, command, operation and run where applicable, with contract
version, authenticated actor/scope, correlation, causation, trace context,
server timestamp and a monotonic sequence within the declared stream.
Ordering is per stream, never a fabricated global total order.
Domain statuses remain distinct; mappings into common presentation lanes are
explicit. Operational completion is not verification success or APS completion.

Preserve [ADR-035](035-three-pipe-observability-rule.md):
governance facts use Kindling under ADR-116, user-visible changes use the
notification contract, and debugging uses tracing. The application envelope
provides correlation and typed projection semantics; it is not a fourth
authoritative observability store. Diagnostic events are not automatically
governance facts. Saved client JSONL is an inspectable export, not authenticated
proof or the source of recovered runtime authority.

Subscriptions use a versioned snapshot plus a cursor from the same logical
boundary. Deltas after that cursor cannot be lost between snapshot and subscribe.
Duplicate event IDs are safe to discard. A retention gap or expired cursor
returns explicit resynchronisation-required, followed by a fresh authoritative
snapshot; clients never infer success from a partial stream. Access is rechecked
at subscribe, replay and reconnect.

Operational status, admission/idempotency records and recovery metadata require
durable application storage. This does not mandate full event sourcing or duplicate
governance content. CONV-003 must settle storage placement, transactional boundaries,
numeric retention and global/per-scope limits before implementation.
Slow clients cannot block save-time validation or cause unbounded producer memory.
Reserve terminal-state capacity, cap payloads and queues, and make overflow visible.

### 5. Lifecycle and approval ownership

Closing a tab, losing a subscriber or exiting a client does not cancel an admitted
daemon-owned operation. Cancellation is a separately authorised, idempotent command.
A cancellation acknowledgement means requested; terminal cancellation means owned
tasks/processes have been reconciled and the supported cleanup guarantee is met.
If cleanup cannot be established, expose that uncertainty rather than successful
cancellation. Restart recovery must identify unfinished work and preserve run/session
history. Provider reconnect capability must be proven before claiming resumability.

Approval binds the exact typed action and preview digests, command version,
authenticated requesting and approving actors, scope, request version, expiry,
policy/capability context and permitted attempt/retry policy. Consumption is durable
and atomic with execution admission. Revocation, expiry or changed context requires
revalidation before effects; consumed approvals cannot authorise a new attempt.
TTY presence alone cannot grant approval or turn an agent into a human.
Durable approval schema/storage and crash-boundary tests belong to CONV-003.
No serialised allomorph token is accepted as an anvil credential.

### Tray-only operation

The tray is a first-class graphical client. Supported operation must include
the tray with no main desktop window, no running desktop shell and no browser
session. The tray distribution must supply or provision its supported runtime
without requiring installation or launch of the full desktop UI; exact packaging
is decided by CONV-006.

Tray startup discovers and authenticates to the correct execution-scope daemon
and uses the existing authorised lifecycle/startup policy when it is absent.
Login startup is an explicit preference governed by that policy, not an
unconditional new auto-spawn path. Failure, incompatibility or missing attestation
produces an honest unavailable/degraded state and an actionable recovery route.
A visible tray icon alone never means protection is active.

Status, attention counts and approvals come from shared authoritative projections.
The tray can show a compact details/approval window on demand without creating
a main application window. That surface must provide the exact action, scope,
preview, requesting actor and consequences needed for the decision, and use the
same authenticated commands and durable approval rules. Notification clicks open
context; they do not grant approval. Dismissing a notification does not settle
the underlying request.

A tray-only installation must support its essential status, attention, approval
and recovery journey without a full desktop or browser dependency. When an action
cannot be safely represented by the compact UI, leave it pending with a clear
reason and offer an explicit available richer surface; never silently approve,
lose the request or link only to an unavailable client. Such unsupported actions
must be declared in the tray capability matrix before claiming tray-only parity.
The main desktop and web views are optional handoffs, not background authorities.

Closing a compact window or quitting/crashing the tray leaves admitted
daemon-owned work intact. Quitting the tray, cancelling an operation and stopping
the daemon are separate actions with explicit labels and existing authorisation.
After restart, the tray restores status and pending attention from the daemon,
including recovery/resynchronisation when its cursor is stale. An OS notification
delivery failure cannot erase durable attention.

### 6. Existing product authorities remain in force

- [ADR-092](092-mcp-optional-activation-spine.md): MCP remains optional;
  protection claims come from daemon attestation, not client connection or graph warmth.
- [ADR-132](132-settings-truth-contract.md): the settings service remains the
  sole settings writer/read model. Configured/requested/resolved/active and
  unknown/stale/failed/drift states are preserved in projections.
- [ADR-116](116-kindling-product-profiles-and-governance-record.md):
  governance admission, quotas, recording gaps and explicit prune receipts remain
  authoritative. An app event cannot introduce automatic governance expiry.
- Redaction occurs before publication, persistence or export under the relevant
  owning contract. Suppression or redaction failure must not fabricate a successful
  complete evidence record. References to artefacts are scoped and authorised on read.
- Capabilities are enforced server-side; feature flags only control exposure.
  Existing entitlements and feature catalogue rules are reused.
- Standalone APS remains possible without proprietary runtime dependencies.
  Canonical source/public mirror changes require the separate ADR-A decision.

### Existing browser dashboard

[ADR-104](104-dashboard-host-server-module-boundary.md) already selects the
React/Vite dashboard host, loopback Rust read-only server and OpenAPI-generated
client seam. Preserve those choices for the existing dashboard. This ADR does
not enable browser writes, authenticated remote control, real-time channels or
replace that server. The native framework spike must evaluate reuse of existing
components; a change to the dashboard stack needs an explicit ADR-104 amendment.
CONV-006 owns the browser auth/transport/write design gate if that scope is selected.

### 7. Adoption and verification gates

Adopt the semantics incrementally in one real, bounded anvil vertical slice,
selected in CONV-002. Do not mass-rewrite existing RPCs. Exact schemas remain
unpublished until the fixtures below pass and the contract is explicitly versioned.

| Evidence | Required demonstration |
| --- | --- |
| Shared command | CLI and a second client exercise the same real handler and typed result; generated schema invokes it without handwritten payload duplication |
| Identity/auth | Cross-user/scope/root refusal; spoofed actor and surface cannot elevate; IDs survive restart |
| Idempotency | Concurrent duplicate, lost reply, changed payload, expired key and crash-after-effect cases produce no blind duplicate execution |
| Tray-only | Start with full desktop UI absent and no browser; connect/recover daemon, inspect status, safely approve/deny and restore attention; closing/quitting/crashing tray leaves work alive; notification loss does not erase attention; optional full UI opens only on explicit request |
| Lifecycle | Client close leaves work alive; authorised cancel cleans descendants; restart exposes unfinished/indeterminate work honestly |
| Approval | Changed input/preview/version/policy, expiry, revocation and concurrent/restarted consumption cannot reuse authority |
| Projection | Atomic snapshot/subscribe boundary, duplicate delivery, cursor expiry, gaps and bounded slow-client recovery |
| Compatibility | Version mismatch, supported additive changes and structured rejection; current CLI exit behaviour preserved |
| Evidence | Correct ADR-035 routing, ADR-116 admission/gap accounting, authorised artefact access; settings redaction failure emits no payload on any channel; absent/stale/untrusted attestation cannot render active |
| Isolation | No UI/allomorph dependencies in domain contracts; standalone APS and existing daemon boundary tests remain green |

The converged app stays behind the default-off umbrella described in the
two-track strategy. No allomorph runtime import before Gate E. EXP-003 and
human API evaluation are not prerequisites for anvil's own contract work.
Human evaluation in allomorph remains explicitly untested; this ADR supplies
no substitute pass.

## Rationale

The experiment establishes that shared command semantics are practical. Existing
anvil authorities supply the production constraints. Adopting the semantics while
retaining those authorities avoids both a research dependency and competing state.

### Alternatives Considered

| Option | Benefit | Cost / disposition |
| --- | --- | --- |
| Anvil-owned contract informed by EXP-002 | Reuses proven approach and existing product boundaries | Requires durable production fixtures; chosen direction |
| Import allomorph now | Immediate experimental implementation | Violates Gate E rule; process-local guarantees are insufficient |
| Copy EXP-002 JSONL unchanged | Fast wire format | Loses durable identity, retries, cross-pipe ownership and exit compatibility |
| Wait for allomorph UI research | More framework evidence | Unnecessarily blocks the main product track |
| Full event sourcing for every domain | Uniform replay story | Unjustified migration and retention burden |

## Consequences

- **Positive:** One contract can drive a real application slice without choosing
  the desktop framework or waiting for research.
- **Negative:** Durable admission, approval and reconnect are additional product
  work; the experiment does not implement them.
- **Risk:** A design document could be mistaken for shipped behaviour.
- **Mitigation:** CONV separates this decision from executable fixtures,
  implementation and release exposure; as-built documents remain unchanged.

## References

- [Convergence decision register](../specs/converged-app/05-decisions-risks-and-open-questions.md)
- [CONV work programme](../modules/converged-app-decisions.aps.md)
- [EXP-002 envelope](https://github.com/eddacraft/allomorph/blob/acb0ce2c90ac88ed83d13c676b8d22b6791044f9/plans/experiments/command-event-envelope-v0.md)
