<!-- APS: See https://github.com/eddacraft/anvil-plan-spec for format reference -->
<!-- Executable only if tasks exist and status is Ready. -->

# Feature Flagging

| Scope | Owner | Priority | Status |
| ----- | ----- | -------- | ------ |
| FLAGS | —     | high     | Complete (9/9) |

## Purpose

Establish one consistent feature-flagging system for Anvil so new capabilities
can be rolled out progressively, disabled quickly, and removed cleanly once
they are stable.

**Problem:** Feature flags are referenced across Rust rollout, policy
orchestration, licensing, and docs gating, but there is no shared module that
defines how flags are declared, resolved, observed, or retired. That creates a
risk of ad-hoc switches, unclear rollout rules, and stale flags persisting long
after a rollout is complete.

**Planning council direction:** Use OpenFeature as the primary runtime
abstraction, backed by a hybrid snapshot model so Anvil can evaluate flags
locally across TypeScript and Rust surfaces while staying ready for a future
provider swap to Featureboard once its SDKs support OpenFeature.

## In Scope

- OpenFeature-first flagging contract for TypeScript and Rust consumers
- Canonical manifest/schema for runtime and product flags
- Hybrid snapshot distribution model for local flag evaluation
- Flag resolution model across CLI, API, website, and Rust surfaces
- Audience-aware and environment-aware targeting rules
- Progressive rollout controls: explicit opt-in, cohort targeting, kill switch,
  and environment promotion
- Minimal OTEL telemetry at session start for static session/snapshot metadata,
  plus minimal usage metrics on first use or per evaluation for features
  actually used, with debug-on-demand tracing for deeper investigation
- Documentation and governance for adding and retiring flags
- Test strategy for enabled, disabled, and mixed-flag execution paths

## Out of Scope

- Pricing/entitlement policy design beyond consuming existing auth/licence claims
- Policy lifecycle rollout semantics already owned by `policy-lifecycle`
- Release cadence and channel management already owned by `release-management`
- Direct coupling to a vendor rule DSL or remote request-time flag evaluation
- Building every individual feature behind flags; this module defines the shared
  system they use

## Interfaces

**Depends on:**

- `beta-auth-streamline` — auth/licence claims for future entitlement-based gating
- `docs-auth-gating` — existing gated web surface patterns
- `opa-agent-orchestration` — rollout controls that need a shared flag system
- `rust-cli` / `rust-core-engine` — current rollout paths using ad-hoc flags
- `observability-foundation` — telemetry contract for flag evaluation events

**Exposes:**

- `OpenFeatureProvider` — primary runtime abstraction for flag evaluation
- `FeatureFlagManifest` — canonical definition of flags, owners, defaults,
  targeting, and expiry
- `FeatureFlagSnapshot` — versioned local-evaluation payload shared across runtimes
- `FeatureFlagResolver` — shared runtime resolution contract for all surfaces
- `RolloutPolicy` — progressive enablement rules, environment promotion, and
  emergency kill-switch semantics
- `FeatureFlagTelemetry` — standard event shape for evaluations and overrides
- `FlagGovernanceGuide` — rules for creation, rollout, sunset, and deletion

## Acceptance Criteria

- [ ] Every production flag has owner, intent, default, and sunset metadata
- [ ] Audience and environment targeting are part of the canonical contract,
      not ad-hoc per consumer
- [ ] A feature can be resolved consistently as enabled/disabled across CLI, API,
      website, and Rust entry points when given the same inputs
- [ ] OpenFeature is the application-facing API so a future provider swap does
      not require application-level call-site rewrites
- [ ] Snapshot publication and refresh rules are explicit enough to support local
      evaluation without request-time vendor dependence
- [ ] Emergency disable path exists without code changes or redeploying all
      surfaces
- [ ] First use of a feature within a session emits minimal OTEL metrics for
      that feature, without PII in event attributes
- [ ] Rollout state is observable enough to answer which code path executed,
      with detailed traces available on demand when needed
- [ ] Temporary rollout flags have an explicit retirement path and are prevented
      from becoming permanent dead configuration

## Constraints

- OpenFeature is the primary abstraction, not a thin compatibility layer
- Local evaluation must work from versioned snapshots in both TypeScript and Rust
- Targeting rules must stay vendor-neutral and portable to future providers
- Fallback behaviour is class-based: risky rollout and entitlement flags fail
  closed unless a safer rule is explicitly documented
- The first exemplars must be CLI licence-gated actions and docs access without
  baking surface-specific assumptions into the shared model

## Design Spec

`plans/specs/2026-04-09-feature-flagging-design.md`

## Ready Checklist

Change status to **Ready** when:

- [x] Purpose and scope are clear
- [x] Dependencies identified
- [x] Design spec written
- [x] OpenFeature package/runtime choices agreed for TypeScript and Rust
- [x] File locations agreed for manifest, provider, snapshot, and telemetry code
- [x] First exemplar boundaries agreed

## Risks & Mitigations

| Risk | Mitigation |
| ---- | ---------- |
| Flags accumulate and become permanent complexity | Require sunset metadata, owner, and retirement task before promotion |
| Different runtimes resolve flags differently | Define one manifest and one resolution contract with parity tests |
| Snapshot refresh lag causes stale decisions | Version snapshots, define freshness rules, and expose staleness diagnostics |
| Vendor portability is overestimated | Keep targeting schema vendor-neutral and document Featureboard swap limits explicitly |
| Percentage rollouts vary by runtime | Standardise hashing inputs and parity-test the implementation across TS and Rust |
| Kill switch is too slow during incidents | Prefer centrally supplied overrides and cache-safe polling/refresh rules |
| Telemetry leaks user or cohort data | Restrict default metrics to minimal OTEL usage counts with no PII and gate detailed tracing behind explicit debug paths |
| Rollouts are opaque to support and review | Emit standard telemetry and document cohort/override inspection |

## Tasks

### FLAGS-001: Define feature flag taxonomy and OpenFeature-aligned manifest schema

- **Intent:** Standardise what kinds of flags exist and the metadata each one must
  carry.
- **Expected Outcome:** A documented manifest/schema distinguishes temporary
  rollout flags, operational kill switches, and durable product gates with
  required owner/default/sunset fields and OpenFeature-compatible value types.
- **Scope:** `plans/`, `packages/anvil/contracts/`, `crates/anvil-kernel-types/`
- **Non-scope:** Runtime storage or admin UI
- **Validation:** `pnpm test -- --runInBand feature-flag-manifest`
- **Confidence:** high

### FLAGS-002: Define audience and environment targeting contract

- **Intent:** Standardise how flags target environments and audiences without
  embedding vendor-specific rule syntax into application code.
- **Expected Outcome:** A canonical schema exists for environment dimensions,
  audience attributes, supported operators, and percentage-rollout inputs.
- **Scope:** `packages/anvil/contracts/`, `crates/anvil-kernel-types/`, `plans/`
- **Non-scope:** Provider-specific segment management UIs
- **Dependencies:** FLAGS-001
- **Validation:** `pnpm test -- --runInBand feature-flag-targeting && cargo test feature_flag_targeting`
- **Confidence:** high

### FLAGS-003: Define shared flag resolution contract

- **Intent:** Specify how all runtimes resolve a flag from defaults, env,
  auth/licence claims, local overrides, snapshots, and rollout cohorts.
- **Expected Outcome:** A single precedence model exists so equivalent inputs
  produce equivalent decisions across TypeScript and Rust surfaces through an
  OpenFeature-facing API.
- **Scope:** `packages/anvil/runtime/`, `apps/anvil-cli/`, `apps/website/`, `crates/anvil-cli/`
- **Non-scope:** Vendor-specific remote config integrations
- **Dependencies:** FLAGS-001, FLAGS-002
- **Validation:** `pnpm test -- --runInBand feature-flag-resolution && cargo test feature_flag_resolution`
- **Confidence:** high

### FLAGS-004: Define snapshot publication and refresh model

- **Intent:** Deliver flag state to runtimes in a versioned form that supports
  local evaluation and future provider swaps.
- **Expected Outcome:** Snapshot shape, publication flow, freshness policy, and
  staleness handling are specified for TypeScript and Rust consumers.
- **Scope:** `packages/anvil/runtime/`, `apps/anvil-api/`, `crates/anvil-cli/`, `docs/guides/`
- **Non-scope:** Selecting a long-term hosted vendor before needed
- **Dependencies:** FLAGS-001, FLAGS-002, FLAGS-003
- **Validation:** `pnpm test -- --runInBand feature-flag-snapshots && cargo test feature_flag_snapshots`
- **Confidence:** medium

### FLAGS-005: Define rollout, promotion, and kill-switch policy

- **Intent:** Establish the operational rules for enabling features gradually and
  disabling them safely.
- **Expected Outcome:** Rollouts support explicit opt-in, bounded cohorts,
  environment promotion, and emergency disable semantics with documented
  operator steps.
- **Scope:** `docs/guides/`, `plans/modules/`, `plans/reviews/post-merge/`
- **Non-scope:** Specific product feature implementations
- **Dependencies:** FLAGS-001, FLAGS-002, FLAGS-003, FLAGS-004
- **Validation:** `grep -q "kill switch" docs/guides/feature-flag-governance.md`
- **Confidence:** medium

### FLAGS-006: Define telemetry and audit contract for flag evaluation

- **Intent:** Make flag decisions inspectable during rollout, debugging, and
  incident response.
- **Expected Outcome:** Session start emits minimal OTEL usage metrics for
  features actually used, without PII in attributes. Detailed evaluation traces
  remain available on demand for debugging and incident response.
- **Scope:** `packages/anvil/contracts/`, `packages/anvil/runtime/`, `crates/anvil-kernel-types/`, `plans/modules/observability-foundation.aps.md`
- **Non-scope:** Dashboard visualisations
- **Dependencies:** FLAGS-003, FLAGS-004
- **Validation:** `pnpm test -- --runInBand feature-flag-telemetry`
- **Confidence:** medium

### FLAGS-007: Add flag governance and retirement workflow

- **Intent:** Prevent temporary rollout flags from surviving indefinitely after a
  feature stabilises.
- **Expected Outcome:** Every new flag requires an owning module/work item, a
  sunset trigger, and a removal checkpoint that can be verified in review.
- **Scope:** `docs/guides/`, `plans/aps-rules.md`, `AGENTS.md`
- **Non-scope:** Automated codemods for flag removal
- **Dependencies:** FLAGS-001, FLAGS-005
- **Validation:** `grep -q "sunset" docs/guides/feature-flag-governance.md && grep -q "feature flag" plans/aps-rules.md`
- **Confidence:** high

### FLAGS-008: Implement CLI licence gating and docs access as the first exemplars — Complete

- **Intent:** Prove the shared flagging model on the clearest existing
  entitlement-gated surfaces: CLI licence-gated actions and `/anvil` docs
  access.
- **Expected Outcome:** CLI feature access and `/anvil` docs access are driven
  through the shared manifest, targeting contract, and snapshot-backed
  OpenFeature flow rather than bespoke per-surface checks alone.
- **Scope:** `plans/archive/modules/rust-cli.aps.md`, `plans/modules/docs-auth-gating.aps.md`, `apps/anvil-cli/`, `apps/docs-site/`, `apps/anvil-api/`, `packages/anvil/runtime/`
- **Non-scope:** Migrating unrelated product surfaces in the same work item
- **Dependencies:** FLAGS-003, FLAGS-004, FLAGS-005
- **Validation:** `pnpm test -- --runInBand feature-flag-exemplars`
- **Confidence:** medium
- **Files:** `crates/anvil-cli/src/feature_flags.rs`, `crates/anvil-cli/src/main.rs`, `crates/anvil-cli/src/commands/auth.rs`, `apps/docs-site/lib/feature-flags.ts`, `apps/docs-site/middleware.ts`
- **Outcome:** CLI `whoami` now evaluates the shared `cli.licence-gate` flag
  via `anvil_kernel::feature_flags::resolve_flag` and surfaces the variant;
  docs-site middleware routes `/anvil` access through the inline `docs.access`
  evaluator (Vercel edge runtime cannot import the workspace runtime package
  yet). Both surfaces follow the shared model exercised in
  `packages/anvil/runtime/src/feature-flags/exemplars.test.ts`, with
  intentional compatibility differences documented alongside the code: the
  Rust `cli.licence-gate` path keeps its existing default variant of
  `enabled` rather than the exemplar's `disabled`, and the docs inline
  evaluator still allows a missing `tier` rather than defaulting that case
  to `disabled`. Full fail-closed cutover is scoped to FLAGM-002/FLAGM-004.

### FLAGS-009: Map current ad-hoc flags and rollout toggles onto the shared model — Complete

- **Intent:** Inventory existing feature-flag-like controls so the new system
  starts from real usage rather than a clean-room design.
- **Expected Outcome:** CLI licence gating, policy rollout controls, docs/auth
  gating, and future entitlement hooks are classified as migrate/adopt/defer,
  including what a later Featureboard provider swap would change.
- **Scope:** `plans/archive/modules/rust-cli.aps.md`, `plans/modules/opa-agent-orchestration.aps.md`, `plans/modules/docs-auth-gating.aps.md`, `docs/specs/`, `plans/decisions/`
- **Non-scope:** Migrating every consumer in the same work item
- **Dependencies:** FLAGS-001, FLAGS-002, FLAGS-003
- **Validation:** `grep -q "migrate" docs/guides/feature-flag-inventory.md`
- **Confidence:** medium
- **Files:** `docs/guides/feature-flag-inventory.md`
- **Outcome:** Inventory landed in commit `c3a217dc` (2026-04-14) with a
  migrate/adopt/defer summary table, four new migrate/defer entries
  (ANVIL_DEV bypass, admin key, beta scopes, policy profiles, per-policy
  toggles), four adopt entries for future capabilities, and an exclusions
  table for env vars that are operational config rather than feature gates.
