# Adoption Trust Surface

<!-- Executable only if tasks exist and status is Ready. -->

| ID    | Owner  | Status | Progress |
| ----- | ------ | ------ | -------- |
| ADTRUST | @aneki | In Progress | 0/6 done |

**Last reviewed:** 2026-05-14 (promoted **Proposed → Ready** alongside
acceptance of
[`plans/specs/2026-05-14-release-plan-v0.7.0-sit-on.md`](../specs/2026-05-14-release-plan-v0.7.0-sit-on.md).
Module-level `Ready` means "ready to begin Wave 3B"; individual tasks
remain `Draft` until each is picked up — matches the INTL precedent.
ADTRUST-001 is the load-bearing item; the other five extend its render
path or sit alongside it.)

## Purpose

`MLP-009` proves the protection-claim vocabulary is well-formed in fixtures.
This module proves the claim is **legible and verifiable during sustained daily
use** — when the daemon has been running for hours, when the user has not
thought about Anvil since breakfast, when something has silently degraded.

The success test: a senior engineer who has not read the Anvil docs runs
`anvil status` once and can correctly answer two questions out loud:

1. Is Anvil protecting this project right now?
2. If not fully, what is degraded and what should I do about it?

Without this surface, the protection claim becomes a documentation problem
rather than a product feature — and sustained-use trust never forms.

## In Scope

- `anvil status` plain-mode output: legible at a glance, one screen, no
  jargon, no false confidence
- `anvil status --json` schema: pinned and versioned for editor/CI consumption
- Degraded-state surfacing: visible banner during normal use within 60s of next
  save-time interaction when the claim is anything other than `full`
- `anvil doctor` diagnose-and-fix recovery paths for the common bad states
  enumerated by `MLP-009`
- Daemon-down auto-recovery UX: hooks detect, re-arm; `anvil start` is
  idempotent and safe to invoke from a stale shell
- First-run claim summary printed by `anvil start` that the user can verify
  themselves (e.g. "make a test edit, you should see X")

## Out of Scope

- The closed-set claim vocabulary itself (owned by `MLP-009`)
- Render path inside the TUI watch surface (owned by `WATCHUX-005`)
- Configuration of which rules are enforced vs advisory (owned by `WATCHUX-007`)
- Server-side / org-level protection claim aggregation (Horizon 2)
- Editor extension UI (separate driver-framework module)

## Interfaces

- **Depends on:**
  - `crates/anvil-cli/src/commands/status.rs`
  - `crates/anvil-cli/src/commands/doctor.rs`
  - `crates/anvil-cli/src/commands/start.rs`
  - `crates/anvil-cli/src/activation/render.rs`
  - `crates/anvil-kernel-types::protection_claim` (the closed-set vocabulary
    shipped by `MLP-009`)
  - `MLP-018` v1-deferrals (consumer-side render wiring)
  - `WATCHUX-007` (rule-mode summary copy)
- **Exposes:**
  - Stable `anvil status --json` schema (versioned)
  - `anvil doctor --fix` recovery actions for documented bad states
  - First-run claim summary copy reused by `anvil start` and `anvil welcome`

## Tasks

### ADTRUST-001: `anvil status` Plain-Mode Legibility

- **Intent:** Make the default `anvil status` output understandable in one read
  by a developer who has not read the docs.
- **Expected Outcome:** Output fits a 24-row terminal, names the current
  protection state from the `MLP-009` closed set, lists the layers (L0–L5)
  with one-word status each, names the daemon PID + uptime if running, names
  the last witness commit + age, and ends with a single next-action line.
- **Files:**
  - `crates/anvil-cli/src/commands/status.rs`
  - `crates/anvil-cli/src/activation/render.rs`
- **Validation:**
  - `cargo test -p eddacraft-anvil commands::status::tests::plain_mode_fits_24_rows`
  - `cargo test -p eddacraft-anvil commands::status::tests::names_protection_state`
  - Manual: a teammate who has not seen Anvil before reads the output and
    correctly answers "are you protected?" without prompting
- **Status:** In Progress
- **changeType:** feature
- **releaseIntent:** candidate
- **releaseScope:** minor
- **releaseNote:**
  - audience: user
  - type: changed
  - text: "`anvil status` now fits a single screen and reports the protection
    state from the closed-set vocabulary."

### ADTRUST-002: Degraded-State Surfacing During Normal Use

- **Intent:** When the protection claim drops below `full`, tell the user
  visibly within 60 seconds of their next save-time interaction.
- **Expected Outcome:** Watch TUI and pre-commit/hook output emit a single
  terse banner naming the degraded state (e.g. `degraded-protection`,
  `multi-daemon-detected`) and a `anvil doctor` command to investigate.
  Banner is rate-limited to ≤1 per 60s per terminal session. No additional
  noise during the silent middle.
- **Files:**
  - `crates/anvil-tui/src/surfaces/watch/render.rs`
  - `crates/anvil-cli/src/commands/hook.rs`
  - `crates/anvil-kernel/src/watch.rs`
- **Validation:**
  - `cargo test -p eddacraft-anvil-tui watch::tests::degraded_banner_rate_limited`
  - `cargo test -p eddacraft-anvil-kernel watch::tests::degraded_emits_within_60s`
- **Status:** Draft
- **Dependencies:** ADTRUST-001
- **changeType:** feature
- **releaseIntent:** candidate
- **releaseScope:** minor
- **releaseNote:**
  - audience: user
  - type: added
  - text: "Watch and hooks now surface degraded protection states with a
    pointer to `anvil doctor`."

### ADTRUST-003: `anvil doctor` Diagnose-And-Fix Recovery

- **Intent:** Make recovery from the common bad states a single-command
  operation, not a knowledge-base scavenger hunt.
- **Expected Outcome:** `anvil doctor` enumerates the documented bad states
  (`degraded-protection`, `multi-daemon-detected`, `path-uncertain`,
  `cross-boundary-mixed`, daemon-missing, hooks-missing, witness-corrupt,
  baseline-stale) and reports per-state findings. `anvil doctor --fix`
  performs the safe subset of corrections (re-arm hooks, restart daemon,
  refresh baseline) with explicit per-step user-visible confirmation.
- **Files:**
  - `crates/anvil-cli/src/commands/doctor.rs`
  - `docs/runbooks/anvil-doctor-states.md` (NEW)
- **Validation:**
  - `cargo test -p eddacraft-anvil commands::doctor::tests::enumerates_all_documented_states`
  - `cargo test -p eddacraft-anvil commands::doctor::tests::fix_is_idempotent`
- **Status:** Draft
- **changeType:** feature
- **releaseIntent:** candidate
- **releaseScope:** minor
- **releaseNote:**
  - audience: user
  - type: added
  - text: "`anvil doctor --fix` recovers from the common documented bad
    states."

### ADTRUST-004: Daemon-Down Auto-Recovery

- **Intent:** Daemon crashes happen. The user experience when they happen
  should be silent re-arm, not silent failure.
- **Expected Outcome:** Pre-commit / pre-push / save-time hooks detect daemon
  unavailability, fall back to the embedded-correctness path, emit the
  documented `degraded-protection` claim state once, and trigger re-arm on
  next `anvil start` or `anvil status`. `anvil start` is idempotent and safe
  to run while a healthy daemon is already up.
- **Files:**
  - `crates/anvil-cli/src/commands/start.rs`
  - `crates/anvil-cli/src/commands/hook.rs`
  - `crates/anvil-hook/src/*`
  - `crates/anvil-kernel/src/*` (embedded fallback path)
- **Validation:**
  - `cargo test -p eddacraft-anvil-hook fallback_path_emits_degraded_once`
  - `cargo test -p eddacraft-anvil commands::start::tests::idempotent_with_running_daemon`
  - Integration: kill daemon mid-session, make a save, verify hook does not
    fail and `anvil status` reports `degraded-protection` exactly once
- **Status:** Draft
- **changeType:** feature
- **releaseIntent:** candidate
- **releaseScope:** minor
- **releaseNote:**
  - audience: user
  - type: changed
  - text: "Anvil now degrades cleanly when the daemon is unavailable instead
    of silently failing."

### ADTRUST-005: Pin `anvil status --json` Schema

- **Intent:** Editor extensions and CI consumers need a stable contract
  before they invest in integrating.
- **Expected Outcome:** `anvil status --json` emits a schema-versioned
  document; the schema is committed as a JSON Schema in
  `schemas/anvil-status.v1.json`; the CLI honours the schema across patch
  releases and bumps it explicitly on minor releases; the contract test
  pins both the schema and a fixture.
- **Files:**
  - `crates/anvil-cli/src/commands/status.rs`
  - `schemas/anvil-status.v1.json` (NEW)
  - `crates/anvil-cli/tests/status_json_contract.rs` (NEW)
- **Validation:**
  - `cargo test -p eddacraft-anvil --test status_json_contract`
  - `pnpm run validate:schemas` (extend to cover the new schema)
- **Status:** Draft
- **Dependencies:** ADTRUST-001
- **changeType:** feature
- **releaseIntent:** candidate
- **releaseScope:** minor
- **releaseNote:**
  - audience: developer
  - type: added
  - text: "`anvil status --json` is now schema-pinned at `anvil-status.v1` for
    stable editor/CI consumption."

### ADTRUST-006: First-Run Claim Summary

- **Intent:** When `anvil start` lands, print a short, accurate summary of
  what protection the user just turned on, plus a verification recipe they
  can run themselves.
- **Expected Outcome:** `anvil start` output names the current claim state,
  lists the active layers in one line each, and prints a verification
  recipe ("make a test edit to a file → expect a warning on …"). Recipe
  matches a real fixture the integration tests run.
- **Files:**
  - `crates/anvil-cli/src/commands/start.rs`
  - `crates/anvil-cli/src/commands/welcome.rs`
  - `crates/anvil-cli/src/activation/render.rs`
- **Validation:**
  - `cargo test -p eddacraft-anvil commands::start::tests::first_run_recipe_matches_fixture`
  - Manual: a fresh checkout + `anvil start` + recipe steps reproduces the
    described behaviour
- **Status:** Draft
- **changeType:** feature
- **releaseIntent:** candidate
- **releaseScope:** minor
- **releaseNote:**
  - audience: user
  - type: added
  - text: "`anvil start` now prints a verification recipe so new users can
    confirm protection is real."

## Sequencing

1. **ADTRUST-001** establishes the legible default output; everything else
   reuses its render path.
2. **ADTRUST-002** layers degraded-state surfacing onto the render path from
   -001.
3. **ADTRUST-005** pins the JSON contract once -001 is settled.
4. **ADTRUST-003**, **ADTRUST-004**, **ADTRUST-006** are parallel after -001.

## Release Notes

This module's items collectively justify a "Anvil is now trustworthy under
sustained daily use" line in the `v0.7.0-beta` release notes. The individual
items do not need separate user-facing notes beyond the per-task
`releaseNote` text above.

## Cross-References

- Coordinates with: [`MLP-009`](multilayer-protection.aps.md) (closed-set
  vocabulary), [`MLP-018`](multilayer-protection.aps.md) (consumer-side
  render wiring), [`WATCHUX-005`](watch-ux-advisory-rules.aps.md) (advisory
  rendering language), [`WATCHUX-007`](watch-ux-advisory-rules.aps.md) (rule
  mode summary in `anvil status`).
- Blocks on: none at module level; individual tasks note dependencies inline.
