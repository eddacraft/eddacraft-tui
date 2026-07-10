<!-- APS: See https://github.com/eddacraft/anvil-plan-spec for format reference -->
<!-- Executable only if tasks exist and status is Ready. -->

# Policy Lifecycle Management

| ID | Owner | Priority | Status |
| ----- | ----- | -------- | ------ |
| POLLC | —     | medium   | Draft  |

**Last reviewed:** 2026-07-11 (post-POLRESET downstream coherence review —
`plans/reviews/2026-07-11-polreset-downstream-coherence.md`: grace-period
vocabulary restated in the shipped two-axis terms, EXCEPT relationship pinned,
work items retargeted per ADR-098 AD-2).

> **Reset posture (POLRESET-010 / ADR-098, 2026-07-04; updated 2026-07-11):**
> post-first-slice expansion — not a prerequisite for first policy value. The
> first policy-value slice **has shipped** (POLRESET Done 10/10, 2026-07-05),
> so the live prerequisite is **ORGHIER** (itself Draft, demand-gated) —
> lifecycle applies per tier and has no meaning before the hierarchy exists.
> Coordinated by [`POLRESET`](./policy-value-enforcement-reset.aps.md).

> **Policy-solution validation (2026-06-24):** lifecycle state belongs above
> the ADR-040/POLENG runtime. Promotion, canary, grace, and rollback metadata
> should select which Rego packs are evaluated by the regorus facade; the module
> does not own or replace the policy engine.

## Purpose

Give organisations control over the full lifecycle of a policy — from draft
through active enforcement to deprecation and retirement. Policies are versioned
artefacts with defined rollout stages, sunset schedules, and migration paths so
that rule changes never surprise developers.

## In Scope

- Policy versioning with semantic version tags
- Lifecycle states: draft, canary, active, deprecated, retired
- Canary rollout that applies a policy to a subset of repos before fleet-wide
  activation
- Deprecation notices with sunset dates and migration guidance
- Grace periods: a time-boxed projection of veto-class outcomes
  (`Block`/`Fence`/`Interrupt`) to `Warn` in the shipped kernel-types
  `ControlDecision` vocabulary (ADR-098 AD-3 two-axis model) — not an
  "errors → warnings" severity flip, which predates the unification
- Policy changelog generation from version diffs
- CLI commands to manage lifecycle transitions
- Rollback to a previous policy version

## Out of Scope

- Automated policy generation or AI-assisted authoring — excluded here and
  everywhere today: the old "(see opa-enhancements)" pointer predates the
  2026-07-02 OPAE reset, and post-reset OPAE explicitly excludes
  natural-language policy generation too; no module owns it
- Approval workflows for state transitions (see policy-federation)
- Real-time notification delivery (use existing CI output)

## Interfaces

**Depends on:**

<!-- Audit 2026-04-26: opa-architecture-integration archived. Retargeted 2026-07-11: loading/evaluation is crates/anvil-policy-engine; crates/anvil-policy is deletion-slated (ADR-098 AD-2). -->
- `crates/anvil-policy-engine` — pack admission (`src/pack/`) and the regorus
  evaluation facade; lifecycle state acts as an **admission filter** over that
  shipped path (POLLC-007), never a parallel selection path
- `policy-pack-validation` — Validate packs before promotion (Done — the
  admission pipeline this composes with is shipped)
- `org-policy-hierarchy` — Lifecycle applies per tier (the live prerequisite)
- `git-native-exceptions` — the grace-period design must state whether
  `GracePeriodEnforcer` is an expiring pack-scoped exception under the shipped
  EXCEPT store (ADR-100 committed authority) or a distinct lifecycle
  attribute, so it does not re-invent the exception mechanism

**Exposes:**

- `PolicyVersionManager` — Version tagging and retrieval
- `LifecycleStateMachine` — State transitions with guards
- `CanarySelector` — Subset targeting for gradual rollout
- `GracePeriodEnforcer` — Time-boxed warning mode
- `anvil policy promote` — Advance a policy through lifecycle stages
- `anvil policy deprecate` — Mark a policy for retirement
- `anvil policy rollback` — Revert to a previous version

## Acceptance Criteria

- [ ] Each policy version is immutable once promoted past draft
- [ ] Canary targets are configurable by repo glob, team tag, or percentage
- [ ] Deprecated policies emit warnings with sunset date in every run
- [ ] Grace period projects veto-class `ControlDecision` outcomes to `Warn`
      for the configured duration (two-axis model, ADR-098 AD-3)
- [ ] `anvil policy promote` validates the pack before advancing state
- [ ] `anvil policy rollback` restores the previous active version
- [ ] Changelog is generated automatically from version metadata diffs

## Risks & Mitigations

| Risk                                  | Mitigation                                    |
| ------------------------------------- | --------------------------------------------- |
| Version sprawl across organisation    | Auto-retire versions older than N generations |
| Canary false confidence on small sets | Require minimum canary population threshold   |
| Grace period abused to defer fixes    | Grace period max duration enforced by schema  |
| Rollback causes policy gap            | Rollback re-activates previous version atomic |

## Work Items

### POLLC-001: Policy version schema

- **Intent:** Define version metadata for policy artefacts
- **Expected Outcome:** Schema supports semver, author, timestamp, and changelog entry
- **Scope:** `crates/anvil-kernel-types/src/`
- **Non-scope:** Storage backend
- **Validation:** `cargo test -p eddacraft-anvil-kernel-types -- policy_version`
- **Confidence:** high

### POLLC-002: Lifecycle state machine

- **Intent:** Enforce valid lifecycle transitions with guard conditions
- **Expected Outcome:** State machine prevents invalid transitions and logs each change
- **Scope:** `crates/anvil-policy-engine/src/`
- **Non-scope:** UI rendering
- **Dependencies:** POLLC-001
- **Validation:** `cargo test -p eddacraft-anvil-policy-engine -- lifecycle_state`
- **Confidence:** high

### POLLC-003: Canary rollout selector

- **Intent:** Target a subset of repositories for gradual policy activation
- **Expected Outcome:** Canary selector matches repos by glob, tag, or percentage
- **Scope:** `crates/anvil-policy-engine/src/`
- **Non-scope:** Notification delivery
- **Dependencies:** POLLC-002
- **Validation:** `cargo test -p eddacraft-anvil-policy-engine -- canary_selector`
- **Confidence:** medium

### POLLC-004: Grace period enforcer

- **Intent:** Project veto-class policy outcomes (`Block`/`Fence`/`Interrupt`)
  to `Warn` during a defined transition window (kernel-types `ControlDecision`
  vocabulary, ADR-098 AD-3); design decides the EXCEPT-store relationship (see
  Interfaces)
- **Expected Outcome:** Veto outcomes downgraded to `Warn` until the grace
  period expires; the true decision stays auditable
- **Scope:** `crates/anvil-policy-engine/src/` (gate hooks via `crates/anvil-cli/src/commands/gate.rs`)
- **Non-scope:** Policy evaluation logic
- **Dependencies:** POLLC-002
- **Validation:** `cargo test -p eddacraft-anvil-policy-engine -- grace_period`
- **Confidence:** high

### POLLC-005: Policy changelog generator

- **Intent:** Produce human-readable changelogs from version diffs
- **Expected Outcome:** Changelog includes added, removed, and modified rules per version
- **Scope:** `crates/anvil-policy-engine/src/`
- **Non-scope:** Notification delivery
- **Dependencies:** POLLC-001
- **Validation:** `cargo test -p eddacraft-anvil-policy-engine -- changelog_generator`
- **Confidence:** high

### POLLC-006: CLI lifecycle commands

- **Intent:** Expose lifecycle management through the CLI
- **Expected Outcome:** `promote`, `deprecate`, and `rollback` commands function correctly
- **Scope:** `crates/anvil-cli/src/commands/`
- **Non-scope:** TUI visualisation
- **Dependencies:** POLLC-002, POLLC-003, POLLC-004
- **Validation:** `cargo test -p eddacraft-anvil -- policy_lifecycle`
- **Confidence:** high

### POLLC-007: Gate runner lifecycle integration

- **Intent:** Gate evaluation respects lifecycle state and grace periods
- **Expected Outcome:** Only active and canary policies are evaluated; grace periods applied
- **Scope:** `crates/anvil-policy-engine/src/` (gate hooks via `crates/anvil-cli/src/commands/gate.rs`)
- **Non-scope:** Hierarchy resolution
- **Dependencies:** POLLC-002, POLLC-004
- **Validation:** `cargo test -p eddacraft-anvil-policy-engine -- lifecycle_gate`
- **Confidence:** medium
