<!--
APS Module: Operational Supplement
==================================
Cross-cutting infrastructure for the Language & Coverage tracks. Owns shared
operational work items but coordinates with Track 3 governance surfaces and
Track 4 semantic packs.

Cross-cutting convention: see plans/aps-rules.md#cross-cutting-modules.
-->

# Operational Supplement (Cross-Track Infrastructure)

| ID    | Owner   | Status      | Progress |
| ----- | ------- | ----------- | -------- |
| OPSUP | OpenCode | In Progress | 2/7      |

**Last reviewed:** 2026-05-17

## Cross-cutting convention

This module follows the cross-cutting convention. The normative spec lives in
[`plans/aps-rules.md#cross-cutting-modules`][rules]; OPSUP owns only the shared
operational prerequisites it lists below. Surface and pack modules own their own
rule catalogues, validation runs, and task counts.

[rules]: ../aps-rules.md#cross-cutting-modules

Task bodies that depend on OPSUP slices should use `Blocks on:` or
`Coordinates with:` callouts and close those callouts when the dependent task is
completed, per ADR-034.

> Note (2026-04-27): the old spec-level reference to a hardcoded
> `AVAILABLE_CHECKS` array in `gate.rs` is stale. The current Rust CLI
> has `crates/anvil-cli/src/commands/check_catalog.rs` as a central
> catalogue for user-facing check names, but it is not yet the stable
> check-ID registry OPSUP requires: it does not assign durable IDs,
> alias renamed checks for a transition window, declare file-presence
> guards or wall-time budgets, or cover drift schema migration.
> `SCHEMA_VERSION` constants still live in `drift.rs`, `init.rs`, and
> `doctor.rs`.

## Purpose

Single home for the cross-cutting operational concerns surfaced by the
[2026-04-08 Language and Coverage Design](../specs/2026-04-08-language-and-coverage-design.md)
council review (§16.5 #7). Every Track 3 governance surface and Track 4
semantic pack pulls these in as prerequisites; without one shared module
each new module would re-design the same operational story differently.

Specifically owns:

- **Check registry with stable IDs** (council C-008) — promotes the
  current `check_catalog.rs` naming catalogue into a durable registry
  with stable IDs, aliases, and per-check metadata. `--skip_checks` must
  resolve against the registry, and newly shipped checks must be
  skippable without a binary downgrade.
- **Drift baseline schema versioning + `anvil drift migrate`** (council
  C-009) — the spec adds 7 new surfaces each with new baseline fields. The
  hardcoded `SCHEMA_VERSION = "1.0.0"` cannot absorb this silently. Owns
  the schema version field, the migration command, and the on-upgrade
  upgrade path so existing user baselines do not break.
- **Per-track feature flags** (council C-020) — every binary upgrade
  currently ships all tracks. Per-track flags let a user disable
  surfaces/packs that are noisy on their codebase without rolling back the
  whole release. Defaults: each new track ships behind a flag for one
  release, then flips on.
- **CI wall-time budget + file-presence guards** (council C-023) — a repo
  with no `.sql` files should pay zero cost for the SQL surface. Each
  surface and pack declares the file shapes it needs; if absent, the
  check short-circuits before doing work.
- **FP reporting channel** (council C-024) — Anvil's own repo is one
  controlled data point. Production signal currently arrives only via
  support tickets. Owns the reporting mechanism (CLI `anvil report-fp`,
  telemetry channel, anonymisation policy) so users have a non-anecdotal
  way to flag false positives.

## In Scope

- Stable check-ID registry crate or module, including:
  - ID assignment scheme (e.g. `ANV-SURF-SQL-001`)
  - Skip/disable resolution against the registry
  - Migration of existing `check_catalog.rs` entries to the registry
- Drift baseline schema versioning:
  - `SCHEMA_VERSION` becomes a versioned enum
  - `anvil drift migrate` command for on-upgrade migration
  - Per-surface/pack baseline-field declarations
- Per-track feature flag taxonomy:
  - Flag naming convention (e.g. `track.surface.sql`, `track.pack.pulumi`)
  - Default-state policy (new tracks start opt-in for one release)
  - Integration with the existing flag governance per
    [feature-flag-governance.md](../../docs/guides/feature-flag-governance.md)
- CI runtime budget framework:
  - Per-check declared file-shape needs
  - File-presence guards short-circuit before work
  - Per-check wall-time cap with surfaceable timeout reason
- FP reporting channel:
  - CLI command (`anvil report-fp <check-id> <file:line>`)
  - Telemetry destination (TBD — almost certainly the existing Kindling
    pipeline)
  - Anonymisation: file path hashing, no source content shipped by default

## Out of Scope

- Concrete surface or pack rules (those live in their own modules).
- Replacement of the existing feature-flag system — OPSUP layers on top of
  it, does not replace it.
- Telemetry-data analytics dashboards (the channel exists; what gets done
  with the data is a separate dashboard module concern).
- Backwards-compatibility of legacy check-name aliases beyond a
  one-release transition window.

## Interfaces

**Depends on:**

- Existing `check_catalog.rs` naming catalogue (migrates from).
- Existing `SCHEMA_VERSION` in `drift.rs` (migrates from).
- Existing feature-flag system per
  [`feature-flag-catalogue`](./feature-flag-catalogue.aps.md) (FLAGS and FLAGM
  are archived; FLAGCAT is the live catalogue module).
- Kindling pipeline (likely host for FP telemetry).

**Exposes:**

- Check-ID registry — referenced by every Track 3 surface and Track 4 pack
  module.
- Drift schema versioning + `anvil drift migrate` — referenced by every
  module that adds a baseline field.
- Per-track flag taxonomy — referenced by every Track 3/4 module.
- File-presence guard helpers — referenced by every Track 3/4 module.
- FP reporting CLI command — referenced from CLI surface modules.

## Prerequisites

None — this module unblocks others, not vice versa.

## Registry Slice Readiness

- [x] Owner named for OPSUP-001.
- [x] Check-ID scheme drafted and reviewed for the registry slice.

## Remaining Slice Readiness

Remaining non-registry slices move to **Ready** when:

- [ ] Drift schema migration policy drafted and reviewed.
- [ ] Per-track flag taxonomy aligns with existing flag governance.
- [ ] FP reporting destination confirmed (Kindling vs other).

## Tasks

### OPSUP-001 — Check-ID registry

- **Status:** Done
- **Intent:** Promote the existing Rust check catalogue into a durable
  check-ID registry with stable IDs, aliases, and migrated metadata for current
  checks.
- **Expected Outcome:** Every current check has a stable `ANV-*` ID, current
  user-facing names continue to resolve, legacy aliases are explicit, and the
  registry has tests guarding uniqueness and lookup behaviour.
- **Files:** `crates/anvil-cli/src/commands/check_catalog.rs`,
  `crates/anvil-cli/src/commands/gate.rs`,
  `crates/anvil-cli/src/commands/gate_config.rs`,
  `plans/modules/operational-supplement.aps.md`, `plans/index.aps.md`
- **Validation:** `cargo test -p eddacraft-anvil commands::check_catalog && cargo test -p eddacraft-anvil commands::gate_config && cargo test -p eddacraft-anvil commands::gate::tests::normalize_gate_check_set_accepts_stable_ids_and_aliases && cargo test -p eddacraft-anvil commands::gate::tests::read_anvilrc_checks_parses_stable_ids`

### OPSUP-002 — Registry-backed skip and disable resolution

- **Status:** Ready
- **Intent:** Resolve skip and disable paths against the stable check registry
  wherever durable IDs are required.

### OPSUP-003 — Drift baseline schema versioning

- **Status:** Draft
- **Intent:** Replace ad hoc schema constants with a versioned drift baseline
  schema model and per-field declarations.

### OPSUP-004 — Drift migration command

- **Status:** Draft
- **Intent:** Add `anvil drift migrate` and an on-upgrade migration path for
  existing baselines.

### OPSUP-005 — Per-track feature flag taxonomy

- **Status:** Draft
- **Intent:** Define per-track flag naming, defaults, and governance alignment
  for new surfaces and packs.

### OPSUP-006 — File-presence guards and wall-time caps

- **Status:** Released/Shipped via v0.7.0-beta (d7873161 · 2026-05-21)
- **Intent:** Provide reusable check guards and runtime budgets so absent file
  shapes short-circuit before expensive work.
- **Expected Outcome:** `CheckDefinition` declares optional file-shape globs
  and a wall-time budget; the gate dispatcher consults these to short-circuit
  checks whose declared file shapes are absent and surface a timeout-reason
  note when any check exceeds its declared budget. All current core checks
  default to unguarded so existing behaviour is unchanged. A reusable
  `check_guards` module exposes the pure evaluators with thorough tests so
  future Track 3/4 surface and pack modules can opt in without re-deriving
  the framework.
- **Files:**
  `crates/anvil-cli/src/commands/check_catalog.rs`,
  `crates/anvil-cli/src/commands/check_guards.rs`,
  `crates/anvil-cli/src/commands/gate.rs`,
  `crates/anvil-cli/src/commands/mod.rs`,
  `plans/modules/operational-supplement.aps.md`,
  `plans/index.aps.md`
- **Validation:** Validation passed 2026-05-17 with `cargo test -p eddacraft-anvil commands::check_guards && cargo test -p eddacraft-anvil commands::check_catalog && cargo test -p eddacraft-anvil commands::gate::tests`.
- **Validation Evidence:** Passed 2026-05-17 with `cargo test -p eddacraft-anvil commands::check_guards && cargo test -p eddacraft-anvil commands::check_catalog && cargo test -p eddacraft-anvil commands::gate::tests`.

### OPSUP-007 — False-positive reporting channel

- **Status:** Draft
- **Intent:** Define the CLI and telemetry path for users to report false
  positives without shipping source content by default.

## Risks

| Risk | Impact | Mitigation |
| ---- | ------ | ---------- |
| Check-ID renames cascade through user `--skip_checks` configs | High | One-release deprecation window with old IDs aliased to new ones |
| Drift schema migrations corrupt existing baselines | Critical | Migration is one-way write-with-backup; backup file retained for one release |
| Per-track flag explosion (one flag per surface + pack = 12+) | Medium | Hierarchical flags (`track.surface.*` umbrella) with per-leaf overrides |
| FP telemetry leaks user code | High | Anonymisation by default; opt-in for source snippets; never enabled in default config |
| OPSUP becomes a long-running prerequisite that delays every Track 3/4 module | High | Deliver in slices — check registry first (unblocks IDs), then drift versioning, then flags, then FP reporting; surfaces can move to Ready against partial OPSUP delivery if their needs are met |

## Open Questions

- [ ] Hosting the FP telemetry — Kindling pipeline reuse or new endpoint?
- [ ] Per-track flag default policy — opt-in for one release then flip on,
      or opt-in until an explicit promotion decision?
- [ ] How do legacy check names map to the new registry IDs — via
      alias table or one-shot rename in a major release?
- [x] Wall-time budget — OPSUP-006 chose per-check soft budgets; no global
      default is declared for current core checks.
