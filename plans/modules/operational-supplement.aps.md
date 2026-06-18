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
| OPSUP | OpenCode | In Progress | 3/7      |

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

- [x] Drift schema migration policy drafted and reviewed — OPSUP-003 (schema
      versioning) + OPSUP-004 (`anvil drift migrate`) authored to Ready
      quality 2026-05-28.
- [x] Per-track flag taxonomy aligns with existing flag governance — OPSUP-005
      authored against FLAGCAT + `feature-flag-governance.md`; default-policy
      open question recorded inline.
- [ ] FP reporting destination confirmed (Kindling vs other) — OPSUP-007 CLI
      surface + anonymisation policy authored, but the telemetry **destination**
      remains a design decision (stays Draft until resolved).

## Work Items

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
- **Expected Outcome:** `--skip-checks` and the `.anvil.<ext>` `checks:` list
  resolve every entry against the OPSUP-001 registry: stable `ANV-*` IDs,
  current user-facing names, and explicit legacy aliases all map to the same
  canonical check. An unknown identifier produces a deterministic error that
  names the closest registered ID rather than silently skipping nothing. A
  newly shipped check is skippable by ID without a binary downgrade.
- **Scopes:** skip/disable resolution paths in the gate dispatcher; no new
  check definitions.
- **Non-scope:** introducing new checks; changing default-enabled check sets;
  the FP reporting path (OPSUP-007).
- **Files:**
  - `crates/anvil-cli/src/commands/gate.rs` (skip-set normalisation)
  - `crates/anvil-cli/src/commands/gate_config.rs` (`.anvil.<ext>` checks list)
  - `crates/anvil-cli/src/commands/check_catalog.rs` (registry lookup surface)
- **Validation:**
  - `cargo test -p eddacraft-anvil commands::gate::tests::normalize_gate_check_set_accepts_stable_ids_and_aliases`
  - New test: an unknown skip ID errors with a registry-resolved suggestion
  - New test: a check absent from the legacy name map but present by `ANV-*`
    ID resolves and is skippable
- **Dependencies:** OPSUP-001 (Done)
- **Confidence:** high

### OPSUP-003 — Drift baseline schema versioning

- **Status:** Merged 2026-06-17 via PR #2694
- **Intent:** Replace ad hoc schema constants with a versioned drift baseline
  schema model and per-field declarations.
- **Expected Outcome:** The single `SCHEMA_VERSION = "1.0.0"` string constant
  in `drift.rs` is replaced by a versioned schema model that records the
  baseline schema version on write and reads it back on load. Each Track 3/4
  surface or pack declares the baseline fields it contributes, so adding a new
  surface advances the schema version additively rather than mutating `1.0.0`
  in place. Loading a baseline whose version is newer than the running binary
  understands fails with a clear "upgrade anvil" message instead of silently
  dropping fields.
- **Scopes:** drift baseline read/write schema; per-surface field declaration
  registry.
- **Non-scope:** the migration command itself (OPSUP-004); concrete surface
  baseline fields (owned by each surface module).
- **Files:**
  - `crates/anvil-cli/src/commands/drift.rs` (`SCHEMA_VERSION` → versioned model)
  - `crates/anvil-kernel-types/src/` (drift baseline schema types, if the
    version model is shared)
- **Validation:**
  - New test: a current-version baseline round-trips byte-stable
  - New test: a future-version baseline fails to load with an upgrade message
  - New test: an additive surface field declaration advances the version
    without breaking an older baseline read path
- **Dependencies:** none (unblocks OPSUP-004 and every surface baseline)
- **Confidence:** high

### OPSUP-004 — Drift migration command

- **Status:** Ready
- **Intent:** Add `anvil drift migrate` and an on-upgrade migration path for
  existing baselines.
- **Expected Outcome:** `anvil drift migrate` upgrades an existing drift
  baseline from an older schema version (per OPSUP-003) to the current one,
  writing a backup of the original before any in-place write. Migration is
  one-way write-with-backup; the backup is retained for one release. Running
  `anvil drift` against an out-of-date baseline surfaces a one-line hint
  pointing at `anvil drift migrate` rather than failing opaquely. Migrating an
  already-current baseline is a no-op that reports "already current".
- **Scopes:** the `drift migrate` subcommand and the on-load upgrade hint.
- **Non-scope:** the schema version model (OPSUP-003 owns it); destructive
  rewrites without a backup.
- **Files:**
  - `crates/anvil-cli/src/commands/drift.rs` (`migrate` subcommand + on-load hint)
  - `crates/anvil-cli/src/commands/mod.rs` (subcommand wiring)
- **Validation:**
  - New test: migrating an older-version baseline writes a backup then upgrades
  - New test: migrating a current baseline is a no-op
  - New test: `anvil drift` on a stale baseline emits the migrate hint
- **Dependencies:** OPSUP-003
- **Confidence:** medium

### OPSUP-005 — Per-track feature flag taxonomy

- **Status:** In Progress
- **Intent:** Define per-track flag naming, defaults, and governance alignment
  for new surfaces and packs.
- **Expected Outcome:** A documented flag taxonomy lets a user disable a noisy
  Track 3 surface or Track 4 pack without rolling back the whole release.
  Flags are hierarchical (`track.surface.*` / `track.pack.*` umbrella with
  per-leaf overrides, e.g. `track.surface.sql`, `track.pack.pulumi`) so the
  flag count does not explode one-per-surface. Each new track ships opt-in
  behind its leaf flag for one release, then the default flips on. The taxonomy
  registers through the existing flag system per
  [feature-flag-governance.md](../../docs/guides/feature-flag-governance.md)
  and through [`feature-flag-catalogue`](./feature-flag-catalogue.aps.md)
  (FLAGCAT), not a parallel flag system; every flag carries `createdFor`
  linking to its owning surface/pack work item and a sunset/review date per
  flag governance.
- **Scopes:** flag naming convention, default-state policy, FLAGCAT catalogue
  entries for the track flags.
- **Non-scope:** the surface/pack rule logic each flag gates; replacing the
  existing flag system.
- **Files:**
  - `crates/anvil-kernel-types/src/feature_flags.rs` (flag taxonomy types, if
    the hierarchy needs a type)
  - FLAGCAT manifest entries (coordinated with the catalogue's manifest layout)
  - `docs/guides/feature-flag-governance.md` (taxonomy reference, if extended)
- **Validation:**
  - New test/consistency check: each `track.*` flag has `createdFor` + a
    sunset/review date and resolves through the hierarchical umbrella
  - `pnpm`-side flag consistency check (FLAGCAT) passes with the new entries
- **Dependencies:** FLAGCAT manifest layout (FLAGCAT-001 Complete)
- **Confidence:** medium
- **Resolution (2026-06-18, confirms documented design; owner-overridable):**
  default policy is **opt-in for one release then auto-flip** — each track
  surface/pack ships `defaultVariant: disabled`, and the default flips to
  `enabled` in a follow-up change after a clean release. This confirms the
  Expected Outcome above (authored 2026-05-28) rather than introducing a new
  policy; if the owner prefers opt-in-until-explicit-promotion, this is a
  one-line manifest convention change with no code impact. The default state is
  intentionally **not** hard-enforced by `track_flag_violations` (which guards
  only the permanent invariants: rollout class, sunset date, `createdFor`,
  umbrella-group resolution) so the flip path cannot break CI.

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
- **Expected Outcome (CLI surface, deterministic):** `anvil report-fp
  <check-id> <file:line>` records a structured false-positive report keyed on
  the OPSUP-001 stable check ID. The report includes the check ID, a hashed
  file path (no plaintext path), and the rule context — never source content by
  default. Source snippets are opt-in only and never enabled in the default
  config (fail-closed on anonymisation).
- **Scopes:** the `report-fp` CLI command and the anonymisation policy.
- **Non-scope:** analytics dashboards over the collected data (a dashboard
  module concern); the registry itself (OPSUP-001).
- **Files:**
  - `crates/anvil-cli/src/commands/` (new `report_fp` command)
  - `crates/anvil-cli/src/commands/mod.rs` (subcommand wiring)
- **Validation:**
  - New test: `report-fp` hashes the path and omits source content under the
    default config
  - New test: an unknown check ID is rejected against the registry
- **Dependencies:** OPSUP-001 (Done)
- **Confidence:** medium
- **Blocked-on (design):** the telemetry **destination** is unresolved —
  Kindling pipeline reuse vs a new endpoint (see Open Questions). The CLI
  surface and anonymisation policy above are decidable now; the destination is
  a cross-cutting infra decision that gates this item moving to Ready.

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
- [x] Per-track flag default policy — **resolved 2026-06-18** (owner-overridable):
      opt-in for one release then flip on (OPSUP-005 Resolution).
- [ ] How do legacy check names map to the new registry IDs — via
      alias table or one-shot rename in a major release?
- [x] Wall-time budget — OPSUP-006 chose per-check soft budgets; no global
      default is declared for current core checks.
