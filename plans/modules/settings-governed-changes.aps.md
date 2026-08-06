<!-- APS: See https://github.com/eddacraft/anvil-plan-spec for format reference -->
<!-- Executable only if tasks exist and status is Ready. -->

# Settings Governed Changes and Audit

| ID     | Owner | Priority | Status | Progress |
| ------ | ----- | -------- | ------ | -------- |
| SETGOV | —     | low      | Draft  | 0/9      |

**Last reviewed:** 2026-08-06 — module created from the operator-supplied
`/settings` specification v1.1
([`plans/specs/2026-08-06-settings-truth-surface.md`](../specs/2026-08-06-settings-truth-surface.md),
spec §22 Slice 3). Post-v0.1 work; deliberately Draft.

> **Release gate (spec §24).** Nothing here ships until the shared proposal,
> authority, persistence, activation and audit path passes the governed-change
> acceptance criteria. Until then `/settings` shows `Settings | Status |
> Sources` only, and Class B/C values stay read-only with a link to the
> established policy-change workflow.

## Purpose

Make protection-affecting configuration changes reviewable rather than
convenient: an immutable proposal describing exactly what would change, a
consequence analysis of the specific transition, the approvals that transition
actually requires, verified activation afterwards, and a durable record of what
happened — including changes Anvil did not make.

This module is where `/settings` stops being an inspection surface and becomes a
governed mutation surface. Its hardest requirement is not the UI: it is that
persistence, activation and audit outcomes stay **distinct and visible**, so a
successful write is never reported as active enforcement.

## In Scope

- Immutable, expiring proposals bound to catalogue, source, constraint and
  runtime revisions
- Transition-aware consequence analysis returning base class, impact
  (strengthened / neutral / weakened / unknown), required authority, required
  workflow, affected components and activation consequences
- Class B flow: target scope, exact diff, validation, explicit confirmation,
  restart and reactivation reporting
- Class C flow: approval routing, higher-level authorities, and a reviewable
  repository change artifact when the controlling source is version-controlled
- Governed reset for Class B/C declarations
- Session overrides with explicit authority and expiry, bounded by constraints
- Activation verification, durable intent and idempotent reconciliation/recovery
- Audit event store, `Audit` view, `anvil settings audit`, MCP audit read, and
  honest coverage/retention/gap reporting
- Detection and recording of externally initiated configuration changes

## Out of Scope

- Natural-language proposal authoring (SETNL)
- Remote organisation administration from a local project session
- Non-interactive CLI mutation — still unspecified; a future contract must reuse
  this service, scope rules and authority model
- Replacing the established policy-change workflow — `/settings` links to it and
  must not create an alternative mutation path
- Building a new event store if Anvil's durable append-only store fits
  (SETGOV-007 decides reuse vs extend)

## Interfaces

**Depends on:**

- [settings-truth-contract](./settings-truth-contract.aps.md) (SETCON) —
  consequence classes, constraints, revisions, runtime evidence, redaction
- [settings-safe-preferences](./settings-safe-preferences.aps.md) (SETPREF) —
  atomic write, scope validation and concurrency detection plumbing
- `crates/anvil-policy-engine` — approval requirements from policy
- `crates/anvil-witness`, `crates/anvil-observability` — candidate durable
  append-only event storage and hash-linking primitives
- `crates/anvil-intercept` — activation requests and post-write verification

**Exposes:**

- Proposal contract and governed mutation API on the settings service
- Material-event audit stream and its inspection surfaces

**Coordinates with:**

- [org-policy-hierarchy](./org-policy-hierarchy.aps.md) (ORGHIER) and
  [policy-lifecycle](./policy-lifecycle.aps.md) (POLLC) — authority model
- [git-native-exceptions](./git-native-exceptions.aps.md) — existing
  version-controlled governance-change conventions
- [compliance-evidence-workspace](./compliance-evidence-workspace.aps.md) —
  export/checkpoint of audit events to a customer evidence store

## Constraints

- **Confirmation is not approval** — confirming a proposal and satisfying policy
  approval are separate acts, and language such as "enable", "merge" or "make it
  work" is never authority to bypass policy.
- **No one-key Class C** — governance changes never change through a toggle or
  an ordinary confirmation alone.
- **Unknown impact is conservative** — an `unknown` consequence cannot silently
  fall back to a lower-risk workflow.
- **Stale proposals die** — any change to a relevant source, constraint,
  approval or runtime precondition invalidates the proposal.
- **Persistence ≠ activation** — success is never reported from a successful
  write alone.
- **Convergence** — for recoverable failures within the supported persistence
  model, every interrupted mutation converges to the verified previous state or
  the verified proposed state; an ambiguous state stays blocked and visible.
- **Honest history** — audit never implies completeness it cannot evidence, and
  hash linking is presented as local tamper evidence, not proof against a
  privileged actor.

## Acceptance Criteria

- [ ] Class B and C changes cannot bypass the shared proposal and mutation service
- [ ] Every proposal contains target scope, exact redacted patch, revisions,
      predicted result, consequence analysis and validation result
- [ ] Risk classification reflects the transition and context, not only the key
- [ ] A stale proposal cannot be applied
- [ ] Class C changes cannot be applied by a one-key toggle or ordinary
      confirmation alone
- [ ] Failed validation produces no partial write
- [ ] Persistence, activation and audit outcomes remain distinct and visible
- [ ] Activation failure produces `failed` and never reports the new value as
      active; attested mismatch produces `drift`
- [ ] Audit view and CLI report retention boundaries and observation gaps
- [ ] An externally initiated change is recorded and distinguishable from an
      Anvil-managed mutation

## Ready Checklist

Change status to **Ready** when:

- [ ] `/settings` v0.1 (SETCON + SETINS + SETPREF) has shipped
- [ ] Audit storage decision made (reuse Anvil's append-only store vs extend)
- [ ] Approval-authority model agreed with ORGHIER/POLLC owners
- [ ] Actor-identity confidence levels wired to a real identity source
- [ ] Recovery strategy per target writer documented

## Work Items

### SETGOV-001: Proposal contract

- **Intent:** Make a requested change an immutable, expiring artifact bound to
  the world it was computed against.
- **Expected Outcome:** A proposal carries identifier and expiry, actor and
  session, target scope and source, catalogue version, source/constraint/runtime
  revisions, the exact redacted patch, the predicted resolved result, consequence
  analysis, validation results, required approvals, affected components and
  activation consequences; immediately before apply the service rechecks
  authority, approvals, revisions, constraints and validation, and any relevant
  change invalidates it.
- **Non-scope:** Consequence computation (SETGOV-002); approval routing
  (SETGOV-004)
- **Dependencies:** SETPREF-005
- **Validation:** `cargo test -p anvil-config settings_proposal`
- **Confidence:** medium
- **Status:** Draft

### SETGOV-002: Transition-aware consequence analysis

- **Intent:** Judge risk from what is actually changing, not from the key's
  reputation.
- **Expected Outcome:** The evaluator returns base class, impact
  (strengthened / neutral / weakened / unknown), required authority, required
  workflow, affected components and activation consequences for the specific
  transition, target scope and current context; `unknown` impact routes to the
  more conservative workflow; the same evaluation drives both the UI badge and
  the workflow selection.
- **Non-scope:** Presentation polish
- **Dependencies:** SETCON-005, SETGOV-001
- **Validation:** `cargo test -p anvil-config settings_consequence`
- **Confidence:** low
- **Status:** Draft

### SETGOV-003: Class B change flow

- **Intent:** Let operational configuration change with a visible diff and an
  explicit decision.
- **Expected Outcome:** A Class B change shows target scope and exact diff,
  validates before apply, requires explicit confirmation, reports restart and
  reactivation consequences, and fails closed if validation, policy evaluation or
  consequence analysis cannot complete.
- **Non-scope:** Class C approvals and VCS artifacts
- **Dependencies:** SETGOV-002
- **Validation:** `cargo test -p anvil-tui settings_class_b`
- **Confidence:** medium
- **Status:** Draft

### SETGOV-004: Class C authority and approval routing

- **Intent:** Ensure governance changes collect every approval policy demands.
- **Expected Outcome:** Class C changes never apply from a toggle or a single
  confirmation; the flow shows whether the transition strengthens, preserves,
  weakens or has unknown effect on protection; every applicable higher-level
  approval is required and routed; proposal confirmation stays distinct from
  policy approval; session overrides require explicit authority, carry an expiry
  and cannot exceed constraints.
- **Non-scope:** Building an identity provider
- **Dependencies:** SETGOV-002
- **Validation:** `cargo test -p anvil-config settings_authority`
- **Confidence:** low
- **Status:** Draft

### SETGOV-005: Version-controlled change artifact

- **Intent:** Route governance changes through review where the source is
  version-controlled.
- **Expected Outcome:** A Class C change targeting a version-controlled source
  produces a reviewable change artifact by default and does not modify the
  active branch or open a remote pull request implicitly; creating a branch,
  native VCS change or pull request is a separate explicit action on the
  established change workflow; creating the artifact is explicitly not a change
  to active configuration, and merge plus later activation are observed and
  audited as separate events.
- **Non-scope:** Replacing the repository's existing change workflow
- **Dependencies:** SETGOV-004
- **Validation:** `cargo test -p anvil-config settings_vcs_artifact`
- **Confidence:** medium
- **Status:** Draft

### SETGOV-006: Activation verification and recovery

- **Intent:** Close the loop between writing a value and proving the system
  enforces it.
- **Expected Outcome:** Material mutations record a durable intent before
  persistence and a completion, rejection or recovery outcome afterwards;
  post-write resolution and activation are verified; activation failure yields
  `failed` and an attested mismatch yields `drift`, never a success claim; if
  audit finalisation or activation verification fails the service returns a
  non-success outcome, exposes the resulting state and supplies a deterministic
  recovery action; interrupted mutations converge to the verified previous or
  verified proposed state, and ambiguous states stay blocked and visible.
- **Non-scope:** Per-component attestation implementation (owning modules)
- **Dependencies:** SETCON-006, SETGOV-001
- **Validation:** `cargo test -p anvil-config settings_activation`
- **Confidence:** low
- **Status:** Draft

### SETGOV-007: Audit event store

- **Intent:** Give material change activity a durable, honest home.
- **Expected Outcome:** Material events (proposed, rejected, approved,
  persisted, activation-requested, activated, activation-failed, rolled-back,
  recovered, externally-modified) are appended to Anvil's durable append-only
  store with hash-linked records; each event records actor identity with a
  confidence level (`verified` / `asserted` / `unknown`) and never presents a
  weak identity as verified; durability, tamper-evidence expectations, retention
  and rotation, offline behaviour, interrupted-write recovery, redaction rules
  and linkage to commits, policy events and runtime evidence are documented;
  sensitive values are omitted in favour of an approved classified digest;
  default local retention is 90 days, extendable by policy, and retention never
  removes an event under an active hold or unresolved recovery.
- **Non-scope:** Enterprise export/replication targets (coordinate, don't build)
- **Dependencies:** SETGOV-001
- **Validation:** `cargo test -p anvil-config settings_audit_store`
- **Confidence:** low
- **Status:** Draft

### SETGOV-008: Audit surfaces

- **Intent:** Expose the record without overclaiming what it covers.
- **Expected Outcome:** An `Audit` tab joins the top-level surface and
  `anvil settings audit` plus a read-only MCP audit view expose the same events;
  each entry shows timestamp, event type, actor and identity confidence, session,
  target scope, changed canonical keys without sensitive values, consequence
  classification, validation/approval/persistence/activation results and a
  reference to the related proposal, commit, pull request, policy event or
  recovery record; every surface reports the earliest retained event, retention
  boundary, current observation coverage and known gaps.
- **Non-scope:** Adding Audit to `/settings` before SETGOV-007 lands — the tab
  appears with the store, not before
- **Dependencies:** SETGOV-007
- **Validation:** `cargo test -p anvil-cli settings_audit`
- **Confidence:** medium
- **Status:** Draft

### SETGOV-009: External-change observation

- **Intent:** Record configuration changes Anvil did not make, without pretending
  to see everything.
- **Expected Outcome:** Externally observed configuration changes are detected
  where possible and recorded as `externally-modified`, always distinguishable
  from an Anvil-managed mutation; periods when Anvil was not running or a source
  could not be observed are reported as coverage gaps rather than silently
  omitted.
- **Non-scope:** Continuous filesystem surveillance of unmanaged sources
- **Dependencies:** SETGOV-007
- **Validation:** `cargo test -p anvil-config settings_external_change`
- **Confidence:** low
- **Status:** Draft
