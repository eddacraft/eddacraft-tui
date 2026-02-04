<!-- APS: See https://github.com/EddaCraft/anvil-plan-spec for format reference -->
<!-- Executable only if tasks exist and status is Ready. -->

# Policy Lifecycle Management

| Scope | Owner | Priority | Status |
| ----- | ----- | -------- | ------ |
| POLLC | —     | medium   | Draft  |

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

- `opa-architecture-integration` — Policy loading and evaluation
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

## Tasks

### POLLC-001: Policy version schema

- **Intent:** Define version metadata for policy artefacts
- **Expected Outcome:** Schema supports semver, author, timestamp, and changelog entry
- **Scope:** `packages/anvil/contracts/src/types/`
- **Non-scope:** Storage backend
- **Validation:** `nx test contracts --testNamePattern="policy-version"`
- **Confidence:** high

### POLLC-002: Lifecycle state machine

- **Intent:** Enforce valid lifecycle transitions with guard conditions
- **Expected Outcome:** State machine prevents invalid transitions and logs each change
- **Scope:** `packages/anvil/policy/src/`
- **Non-scope:** UI rendering
- **Dependencies:** POLLC-001
- **Validation:** `nx test policy --testNamePattern="lifecycle-state"`
- **Confidence:** high

### POLLC-003: Canary rollout selector

- **Intent:** Target a subset of repositories for gradual policy activation
- **Expected Outcome:** Canary selector matches repos by glob, tag, or percentage
- **Scope:** `packages/anvil/policy/src/`
- **Non-scope:** Notification delivery
- **Dependencies:** POLLC-002
- **Validation:** `nx test policy --testNamePattern="canary-selector"`
- **Confidence:** medium

### POLLC-004: Grace period enforcer

- **Intent:** Convert policy errors to warnings during a defined transition window
- **Expected Outcome:** Violations downgraded to warnings until grace period expires
- **Scope:** `packages/anvil/runtime/src/gate/`
- **Non-scope:** Policy evaluation logic
- **Dependencies:** POLLC-002
- **Validation:** `nx test runtime --testNamePattern="grace-period"`
- **Confidence:** high

### POLLC-005: Policy changelog generator

- **Intent:** Produce human-readable changelogs from version diffs
- **Expected Outcome:** Changelog includes added, removed, and modified rules per version
- **Scope:** `packages/anvil/policy/src/`
- **Non-scope:** Notification delivery
- **Dependencies:** POLLC-001
- **Validation:** `nx test policy --testNamePattern="changelog-generator"`
- **Confidence:** high

### POLLC-006: CLI lifecycle commands

- **Intent:** Expose lifecycle management through the CLI
- **Expected Outcome:** `promote`, `deprecate`, and `rollback` commands function correctly
- **Scope:** `apps/anvil-cli/src/commands/`
- **Non-scope:** TUI visualisation
- **Dependencies:** POLLC-002, POLLC-003, POLLC-004
- **Validation:** `nx test cli --testNamePattern="policy lifecycle"`
- **Confidence:** high

### POLLC-007: Gate runner lifecycle integration

- **Intent:** Gate evaluation respects lifecycle state and grace periods
- **Expected Outcome:** Only active and canary policies are evaluated; grace periods applied
- **Scope:** `packages/anvil/runtime/src/gate/`
- **Non-scope:** Hierarchy resolution
- **Dependencies:** POLLC-002, POLLC-004
- **Validation:** `nx test runtime --testNamePattern="lifecycle-gate"`
- **Confidence:** medium
