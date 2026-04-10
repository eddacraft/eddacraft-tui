<!-- APS: Design spec for FLAGS module -->

# Feature Flagging Design

Date: 2026-04-09
Module: `FLAGS`
Status: Draft

## Goal

Define a shared feature-flagging system for Anvil that supports tier-based
feature access, audience and environment targeting, progressive rollout, and
emergency disable without coupling application code to a specific vendor.

## Context

Anvil already references feature flags in several places, especially Rust engine
rollout and policy orchestration, but there is no shared model for flag
definition, evaluation, rollout, or retirement. The immediate need is to put
Anvil features behind flags so beta users and future tiers can receive the right
capabilities while environment promotion stays controlled.

The first proving paths are CLI licence-gated actions and docs access. The
longer-term direction is to stay ready for Featureboard once its SDKs support
OpenFeature.

## Decision Summary

- OpenFeature is the application-facing flag API
- Anvil uses a custom provider backed by versioned snapshots
- Evaluation happens locally in TypeScript and Rust runtimes
- Flag targeting is vendor-neutral and carried via OpenFeature evaluation context
- Targeting supports both audience and environment dimensions from day one
- Fallback behaviour is class-based rather than one global fail-open/fail-closed
- Default observability is minimal OTEL usage metrics at session start with no
  PII; detailed traces are debug-only
- CLI licence-gated actions and docs access are the first exemplars

## Why OpenFeature

OpenFeature gives Anvil a stable application-facing contract now while keeping
provider choice open later. That means:

- application call sites depend on OpenFeature, not a vendor SDK
- provider replacement is isolated to the provider boundary and snapshot mapping
- TypeScript and Rust can align around the same concepts: flag keys, evaluation
  context, resolution details, hooks, and events

OpenFeature is a portability layer, not full semantic portability. Anvil still
needs its own canonical targeting and snapshot model so behaviour remains stable
even if providers differ.

## Architecture

### High-level model

1. Anvil defines flags in a canonical manifest
2. A publisher produces versioned snapshots for runtime consumption
3. TypeScript and Rust runtimes load snapshots locally
4. A custom OpenFeature provider resolves flags in-process
5. Application code consumes flags through OpenFeature APIs only

### Components

- `FeatureFlagManifest`
  - source-of-truth schema for flag metadata
  - includes owner, intent, class, default, targeting, expiry, and rollout notes
- `FeatureFlagSnapshot`
  - versioned payload derived from the manifest and active rollout state
  - distributed to runtimes for local evaluation
- `OpenFeatureProvider`
  - local provider implementing evaluation against the snapshot
  - shared semantics across TypeScript and Rust
- `EvaluationContextAdapter`
  - maps runtime/session/user/environment data into a canonical evaluation context
- `FeatureFlagTelemetry`
  - emits minimal OTEL usage stats and optional debug traces

## Implementation Targets

### TypeScript

- Manifest and targeting schemas:
  - `packages/anvil/contracts/src/types/feature-flags.ts`
  - `packages/anvil/contracts/src/schemas/feature-flags.schema.ts`
  - export from `packages/anvil/contracts/src/index.ts`
- Runtime resolution and snapshot loading:
  - `packages/anvil/runtime/src/feature-flags/manifest.ts`
  - `packages/anvil/runtime/src/feature-flags/context.ts`
  - `packages/anvil/runtime/src/feature-flags/resolver.ts`
  - `packages/anvil/runtime/src/feature-flags/snapshot.ts`
  - `packages/anvil/runtime/src/feature-flags/provider.ts`
  - `packages/anvil/runtime/src/feature-flags/index.ts`
- Telemetry:
  - `packages/anvil/runtime/src/feature-flags/telemetry.ts`

### Rust

- Shared types:
  - `crates/anvil-kernel-types/src/feature_flags.rs`
  - export from `crates/anvil-kernel-types/src/lib.rs`
- Runtime evaluation and snapshot loading:
  - `crates/anvil-kernel/src/feature_flags/mod.rs`
  - `crates/anvil-kernel/src/feature_flags/context.rs`
  - `crates/anvil-kernel/src/feature_flags/resolver.rs`
  - `crates/anvil-kernel/src/feature_flags/snapshot.rs`
  - `crates/anvil-kernel/src/feature_flags/provider.rs`
- Surface integration points:
  - `crates/anvil-cli/` command/session entry points that resolve gated access

### Why these locations

- `packages/anvil/contracts` already holds shared TS contracts and schemas
- `packages/anvil/runtime` is the right TS runtime home for evaluation,
  snapshot loading, and telemetry
- `crates/anvil-kernel-types` is the right Rust home for portable types
- `apps/anvil-cli`, `apps/docs-site`, and `packages/anvil/runtime` are the
  natural first integration points because the initial exemplars are licence and
  docs access gating rather than engine selection

## Runtime Choices

### TypeScript

- Use the OpenFeature JavaScript SDK as the application-facing API
- Wrap it with an internal snapshot-backed provider in `packages/anvil/runtime`
- Keep the manifest/targeting model independent from any provider SDK

### Rust

- Treat OpenFeature compatibility as a contract Anvil owns at the provider
  boundary rather than blocking on a perfect Rust ecosystem match
- Mirror the OpenFeature concepts Anvil needs now: flag evaluation, evaluation
  context, provider abstraction, resolution details, and hooks/events where
  useful
- Keep the Rust API shape aligned with the TypeScript provider so a future
  vendor-backed provider remains feasible

### Provider model

The default provider is local and snapshot-backed. Request-time remote evaluation
is explicitly out of scope.

Why:

- deterministic local behaviour
- no vendor/network dependency in hot paths
- easier parity testing between TypeScript and Rust
- cleaner future migration to Featureboard or another provider

## Canonical Flag Model

### Flag classes

- `rollout`
  - temporary progressive enablement for incomplete or risky features
- `entitlement`
  - gating by tier, plan, or licence-derived capability
- `ops_kill_switch`
  - emergency disable for operational safety

This module does not treat all arbitrary configuration as flags.

### Required metadata

Every production flag must define:

- `key`
- `owner`
- `intent`
- `class`
- `defaultVariant`
- `expiryOrReviewDate`
- `createdFor`
  - linked APS module or work item
- `status`
  - draft, active, retiring, retired

## Targeting Model

### Environment targeting

Environment targeting is first-class and vendor-neutral. Initial dimensions:

- `environment`
  - `local`, `preview`, `dev`, `staging`, `prod`
- `channel`
  - e.g. development, beta, rc, stable
- `deploymentRing`
  - optional rollout ring when needed

### Audience targeting

Audience targeting is also first-class. Initial dimensions:

- `accountTier`
- `licencePlan`
- `organisationId`
- `userRole`
- `cohort`

No raw PII is required for normal evaluation. Where identifiers are needed for
stable rollout bucketing, they should use stable internal IDs rather than email
or other user-facing identifiers.

### Supported operators

Keep the shared rule model deliberately small:

- equals
- not-equals
- in-set
- not-in-set
- percentage rollout
- segment membership

This is enough for the current use cases without hard-wiring vendor DSLs into
the design.

## Resolution Contract

Resolution order should be explicit and portable:

1. emergency override / kill switch
2. local operator override where explicitly allowed
3. snapshot targeting rules
4. manifest default

Equivalent inputs must resolve the same way in TypeScript and Rust.

## Snapshot Model

Snapshots are versioned evaluation payloads published from the canonical model.

They must include:

- manifest version
- snapshot version
- issue timestamp
- freshness metadata
- resolved flag definitions and targeting data needed for local evaluation

Refresh should be asynchronous and cache-safe. Runtime startup must define what
happens if the snapshot is missing, stale, or incompatible.

## Failure Policy

Failure behaviour is class-based:

- `ops_kill_switch`
  - fail closed
- `entitlement`
  - fail closed unless explicitly documented otherwise
- `rollout`
  - default to safe/disabled path unless the feature is already universally on

The goal is predictable degradation rather than silent exposure.

## Observability

### Default telemetry

At session start, emit minimal OTEL usage metrics for features actually used.

Properties:

- one usage stat per feature used in the session
- no PII in attributes
- include only low-risk dimensions such as feature key, environment, runtime,
  snapshot version, and coarse tier/channel where appropriate

### Debug telemetry

Detailed evaluation reasoning is not emitted by default. It is available only
through explicit debug paths for rollout investigation and incident response.

## First Exemplars

CLI licence-gated actions and docs access are the first adoption paths.

Why:

- both are already meaningful user-visible gates in the product
- both exercise entitlement-aware access rather than purely technical rollout
- both provide a concrete migration path from bespoke checks to the shared model

The shared model must stay generic enough that later adopters such as policy
orchestration and tier-based product access do not need a redesign.

## Proposed Flagged Features

This is the initial list of Anvil features and surfaces expected to move behind
the shared flagging system. It is a planning list, not a promise that all of
them ship in the first implementation wave.

### Already gated today

- CLI licence-gated actions
  - Meaningful CLI features already depend on licence/session state; this is the
    clearest existing entitlement gate to migrate onto the shared system
- `/anvil` docs access
  - `/anvil` documentation/product-entry surfaces are already restricted to
    authenticated beta users and pending users are blocked from meaningful access

### First migration wave

- CLI licence-gated actions
  - Move existing licence checks onto the shared manifest, targeting model, and
    snapshot-backed OpenFeature flow
- `/anvil` docs access
  - Move existing docs auth gating onto the same shared entitlement and
    environment-aware rules

### Next likely gated capabilities

- OPA agent orchestration rollout
  - Gate orchestration features during staged adoption and keep an operational
    kill switch
- Tier-based product capabilities
  - Gate Anvil capabilities by beta, plan, or future paid tier so only the right
    users receive the feature set

### Later expansion candidates

- Web dashboard capabilities
  - Gate specific dashboard views or advanced tooling by tier, audience, or
    environment
- Dashboard AI builder
  - Gate AI-assisted dashboard generation separately from baseline dashboard
    access
- Interactive tutorial and advanced TUI surfaces
  - Roll out new guided or premium terminal experiences to selected audiences
- Policy lifecycle and governance UX
  - Gate advanced policy-management capabilities for org-level users or staged
    beta cohorts

### Selection criteria

Features are good candidates for flagging when they need one or more of:

- staged rollout across environments
- audience segmentation or beta-only access
- tier or entitlement gating
- operational kill-switch protection
- safe migration from an existing implementation path to a new one

### First exemplar boundaries

Included:

- resolving CLI access for meaningful gated commands through the shared manifest
  and snapshot-backed resolver
- resolving `/anvil` docs access through the same shared targeting model
- environment and audience targeting needed for beta access and staged promotion
- safe fallback to deny access when entitlement resolution is unavailable or stale
- minimal OTEL usage emission when a flagged feature path is actually used

Excluded:

- migrating unrelated product surfaces in the same wave
- inventing product-tier UX beyond the resolution contract inputs
- introducing remote request-time evaluation for access checks

## Featureboard Readiness

Anvil should be ready to adopt Featureboard later if its SDKs support
OpenFeature. That readiness means:

- application code already uses OpenFeature
- targeting semantics live in Anvil's canonical model, not provider-specific code
- provider swap is treated as a snapshot/provider migration, not an application
  rewrite

Non-goal:

- guaranteeing one-to-one semantic portability for every future vendor feature

## Risks

- provider portability may be less complete than expected
- snapshot staleness may produce confusing rollouts
- percentage rollout consistency can drift if hashing inputs differ by runtime
- audience targeting may accidentally expand into business policy/config if not
  scoped tightly
- dead flags can accumulate without enforced metadata and review gates

## Proposed Delivery Shape

1. Define manifest and targeting schema
2. Define resolution and snapshot contract
3. Define OTEL/debug telemetry contract
4. Define rollout/promotion/governance policy
5. Prove the model with Rust engine selection
6. Classify existing ad-hoc flags for later migration

## Ready Criteria

The module can move to `Ready` once:

- this design is accepted
- file targets for manifest/provider/snapshot implementation are agreed
- OpenFeature package/runtime choices are agreed for TypeScript and Rust
- First exemplar boundaries are agreed
