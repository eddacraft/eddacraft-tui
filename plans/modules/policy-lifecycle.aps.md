<!-- APS: See https://github.com/eddacraft/anvil-plan-spec for format reference -->
<!-- Executable only if tasks exist and status is Ready. -->

# Policy Lifecycle Management

| ID | Owner | Priority | Status |
| ----- | ----- | -------- | ------ |
| POLLC | —     | medium   | Draft  |

**Last reviewed:** 2026-07-04 (POLRESET-010 enterprise backlog reset).

> **Reset posture (POLRESET-010 / ADR-098, 2026-07-04):** post-first-slice
> expansion — not a prerequisite for first policy value. Later enterprise
> expansion; requires ORGHIER and a shipped first policy-value slice. Coordinated by
> [`POLRESET`](./policy-value-enforcement-reset.aps.md).

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
- Grace periods that convert violations to warnings before they become errors
- Policy changelog generation from version diffs
- CLI commands to manage lifecycle transitions
- Rollback to a previous policy version

## Out of Scope

- Automated policy generation or AI-assisted authoring (see opa-enhancements)
- Approval workflows for state transitions (see policy-federation)
- Real-time notification delivery (use existing CI output)

## Interfaces

**Depends on:**

<!-- Audit 2026-04-26: opa-architecture-integration archived; policy now lives in crates/anvil-policy. -->
- `crates/anvil-policy` — Policy loading and evaluation
- `policy-pack-validation` — Validate packs before promotion
- `org-policy-hierarchy` — Lifecycle applies per tier

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
- [ ] Grace period converts errors to warnings for the configured duration
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
- **Scope:** `crates/anvil-policy/src/`
- **Non-scope:** UI rendering
- **Dependencies:** POLLC-001
- **Validation:** `cargo test -p eddacraft-anvil-policy -- lifecycle_state`
- **Confidence:** high

### POLLC-003: Canary rollout selector

- **Intent:** Target a subset of repositories for gradual policy activation
- **Expected Outcome:** Canary selector matches repos by glob, tag, or percentage
- **Scope:** `crates/anvil-policy/src/`
- **Non-scope:** Notification delivery
- **Dependencies:** POLLC-002
- **Validation:** `cargo test -p eddacraft-anvil-policy -- canary_selector`
- **Confidence:** medium

### POLLC-004: Grace period enforcer

- **Intent:** Convert policy errors to warnings during a defined transition window
- **Expected Outcome:** Violations downgraded to warnings until grace period expires
- **Scope:** `crates/anvil-policy/src/` (gate hooks via `crates/anvil-cli/src/commands/gate.rs`)
- **Non-scope:** Policy evaluation logic
- **Dependencies:** POLLC-002
- **Validation:** `cargo test -p eddacraft-anvil-policy -- grace_period`
- **Confidence:** high

### POLLC-005: Policy changelog generator

- **Intent:** Produce human-readable changelogs from version diffs
- **Expected Outcome:** Changelog includes added, removed, and modified rules per version
- **Scope:** `crates/anvil-policy/src/`
- **Non-scope:** Notification delivery
- **Dependencies:** POLLC-001
- **Validation:** `cargo test -p eddacraft-anvil-policy -- changelog_generator`
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
- **Scope:** `crates/anvil-policy/src/` (gate hooks via `crates/anvil-cli/src/commands/gate.rs`)
- **Non-scope:** Hierarchy resolution
- **Dependencies:** POLLC-002, POLLC-004
- **Validation:** `cargo test -p eddacraft-anvil-policy -- lifecycle_gate`
- **Confidence:** medium
