<!-- APS: See https://github.com/EddaCraft/anvil-plan-spec for format reference -->
<!-- Executable only if tasks exist and status is Ready. -->

# Organisational Policy Hierarchy

| Scope   | Owner | Priority | Status |
| ------- | ----- | -------- | ------ |
| ORGHIER | —     | high     | Draft  |

## Purpose

Enable multi-level policy governance so organisations can enforce baseline
standards across all repositories while allowing teams and projects to layer on
additional rules. Policies cascade from organisation → team → project, with
controlled override and exemption semantics at each level.

## In Scope

- Hierarchical policy resolution (org → team → project)
- Policy inheritance with merge strategies (append, override, replace)
- Scope selectors that bind policy sets to teams, repositories, or path globs
- Override permissions so project-level configs can relax org rules only when
  explicitly allowed
- Conflict detection when policies at different levels contradict
- CLI commands to inspect the effective policy set at any scope
- Configuration schema for `.anvilrc` and central policy repositories

## Out of Scope

- Authentication and identity management (use existing Git/CI credentials)
- Hosted policy registry service (file and bundle-based distribution only)
- Real-time sync across repositories (pull-based refresh)
- GUI for hierarchy management (CLI-first)

## Interfaces

**Depends on:**

- `opa-architecture-integration` — Policy loading and OPA execution
- `policy-pack-validation` — Validation of policy packs at each tier
- `opa-enhancements` — Remote bundle infrastructure (OPAE-034–036)

**Exposes:**

- `PolicyHierarchyResolver` — Merges policies across tiers
- `ScopeSelector` — Binds policy sets to targets
- `EffectivePolicySet` — Resolved, conflict-free policy collection
- `anvil policy effective` — CLI to show resolved policies
- `anvil policy hierarchy` — CLI to visualise inheritance chain

## Acceptance Criteria

- [ ] Org-level policies apply to all repos that reference the org bundle
- [ ] Team-level policies layer on top of org policies without duplication
- [ ] Project-level overrides only succeed when the parent tier permits it
- [ ] Conflicting rules across tiers produce a clear diagnostic
- [ ] `anvil policy effective` prints the merged policy set with provenance
- [ ] Hierarchy resolution completes in < 500ms for typical setups
- [ ] Configuration change at any tier triggers re-resolution on next run

## Risks & Mitigations

| Risk                                 | Mitigation                                      |
| ------------------------------------ | ----------------------------------------------- |
| Override abuse undermines org rules  | Override requires explicit `allow_override` flag |
| Deep hierarchies slow resolution     | Cache resolved sets; invalidate on config change |
| Conflicting policies confuse users   | Conflict report with provenance per rule        |
| Migration burden for existing setups | Auto-detect single-tier and skip hierarchy      |

## Tasks

### ORGHIER-001: Hierarchy configuration schema

- **Intent:** Define the configuration format for multi-tier policy bindings
- **Expected Outcome:** Schema supports org, team, and project tiers with merge strategy
- **Scope:** `packages/anvil/contracts/src/types/`
- **Non-scope:** Resolution logic
- **Validation:** `nx test contracts --testNamePattern="hierarchy-config"`
- **Confidence:** high

### ORGHIER-002: Scope selector engine

- **Intent:** Match repositories and paths to policy sets using selectors
- **Expected Outcome:** Selectors support repo name globs, team tags, and path patterns
- **Scope:** `packages/anvil/policy/src/`
- **Non-scope:** Policy evaluation
- **Validation:** `nx test policy --testNamePattern="scope-selector"`
- **Confidence:** high

### ORGHIER-003: Policy hierarchy resolver

- **Intent:** Merge policies from multiple tiers into an effective set
- **Expected Outcome:** Resolver applies inheritance, overrides, and conflict detection
- **Scope:** `packages/anvil/policy/src/`
- **Non-scope:** Bundle fetching
- **Dependencies:** ORGHIER-001, ORGHIER-002
- **Validation:** `nx test policy --testNamePattern="hierarchy-resolver"`
- **Confidence:** high

### ORGHIER-004: Override permission enforcement

- **Intent:** Prevent project-level configs from relaxing org rules without authorisation
- **Expected Outcome:** Overrides blocked unless parent tier sets `allow_override`
- **Scope:** `packages/anvil/policy/src/`
- **Non-scope:** Approval workflows
- **Dependencies:** ORGHIER-003
- **Validation:** `nx test policy --testNamePattern="override-enforcement"`
- **Confidence:** high

### ORGHIER-005: Conflict diagnostics

- **Intent:** Report contradictory policies across tiers with actionable guidance
- **Expected Outcome:** Conflicts include provenance, severity, and resolution hints
- **Scope:** `packages/anvil/policy/src/`
- **Non-scope:** Auto-resolution
- **Dependencies:** ORGHIER-003
- **Validation:** `nx test policy --testNamePattern="conflict-diagnostics"`
- **Confidence:** medium

### ORGHIER-006: CLI hierarchy commands

- **Intent:** Let users inspect effective policies and hierarchy chain
- **Expected Outcome:** `anvil policy effective` and `anvil policy hierarchy` work
- **Scope:** `apps/anvil-cli/src/commands/`
- **Non-scope:** TUI visualisation
- **Dependencies:** ORGHIER-003, ORGHIER-005
- **Validation:** `nx test cli --testNamePattern="policy hierarchy"`
- **Confidence:** high

### ORGHIER-007: Gate runner hierarchy integration

- **Intent:** Gate evaluation uses the resolved effective policy set
- **Expected Outcome:** Gate runner resolves hierarchy before policy check
- **Scope:** `packages/anvil/runtime/src/gate/`
- **Non-scope:** New check types
- **Dependencies:** ORGHIER-003
- **Validation:** `nx test runtime --testNamePattern="hierarchy-gate"`
- **Confidence:** medium
