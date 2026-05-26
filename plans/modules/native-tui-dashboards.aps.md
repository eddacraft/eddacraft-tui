<!-- APS: See https://github.com/eddacraft/anvil-plan-spec for format reference -->
<!-- Executable only if work items exist and status is Ready. -->

# Native TUI Dashboards

| ID    | Owner      | Status | Progress |
| ----- | ---------- | ------ | -------- |
| TDASH | joshuaboys | Done   | 4/4      |

**Last reviewed:** 2026-05-26

## Purpose

Ship read-only native Ratatui dashboards that render persisted `.anvil/` state
directly in the terminal, following the `anvil plan dashboard` precedent
(APSCAN-011 / `crates/anvil-tui/src/surfaces/plan_dashboard/`).

This module delivers the "see Anvil state in the terminal" value that
[`tui-dashboard-render`](./tui-dashboard-render.aps.md) (TUIDASH) was meant to
provide, but **without** the json-render spec interpreter (spec parser →
component registry → tree walker) or the AI spec-generation path
([`dashboard-ai-builder`](./dashboard-ai-builder.aps.md), DASHAI). Each
dashboard is a hand-written Ratatui layout reading domain data via existing Rust
file readers — the same approach already proven by `anvil plan dashboard`.

TUIDASH stays the write-once/render-anywhere json-render path. The two are
complementary: TDASH ships fixed dashboards for data that is persisted today;
TUIDASH later renders arbitrary (incl. AI-generated) specs. If json-render lands,
these native surfaces can be reframed as catalogue components, but nothing here
blocks on that.

## In Scope

- `anvil dashboard [name]` command dispatching to native dashboard surfaces,
  with `--json` and non-TTY plain-text fallbacks (mirroring `anvil plan
  dashboard`'s `GlobalArgs` handling)
- A shared read-only dashboard surface scaffold in `crates/anvil-tui/` (header/
  footer chrome, empty-state, a dashboard index/picker listing what is available)
- Native dashboard surfaces for state that is **already persisted** under
  `.anvil/`: architecture health, drift snapshots, suppressions
- Snapshot tests per surface (insta), matching the existing surface test pattern

## Out of Scope

- The json-render spec interpreter, component registry, and tree walker — owned
  by TUIDASH (TUIDASH-001..003)
- AI-generated dashboard specs and the prompt interface — owned by DASHAI
- The web dashboard (DASH / DASHCORE / DASHARCH / DASHOPS / DASHAI)
- Write actions from a dashboard (approve plan, suppress finding) — read-only only
- **Gate-summary and watch-session dashboards** — their inputs are not persisted
  to `.anvil/` yet (gate/watch surfaces are live-run only). These need a
  data-persistence prerequisite first; see Prerequisites and the forward note in
  TUIDASH. Tracked as future work, not in this module's count.

## Interfaces

**Depends on:**

- `crates/anvil-tui/` — `Surface` trait, `tui::run_surface`, and the
  `plan_dashboard` surface as the structural precedent
- `crates/eddacraft-tui/` (`eddacraft-tui` 0.2.x) — themed widgets
  (`DataTable`, `Container`, `StatusBadge`, header/shell chrome) and `Theme`
- `crates/anvil-cli/` — command dispatch and `GlobalArgs` (`--json`, `--no-tui`,
  TTY detection)
- Persisted `.anvil/` stores read directly from Rust:
  - `.anvil/architecture.json` (architecture health)
  - `.anvil/snapshots/` + `.anvil/baseline.json` (drift)
  - `.anvil/suppressions.json` (suppressions)

**Exposes:**

- `anvil dashboard [name]` command family (`architecture`, `drift`,
  `suppressions`)
- A reusable read-only dashboard surface scaffold in `crates/anvil-tui/` that
  later dashboards (incl. gate-summary/watch-session, once their data persists)
  can compose

## Decisions

**D-TDASH-001:** Rendering approach — native surfaces vs json-render

- **Options:** (a) Hand-written native Ratatui surfaces per dashboard,
  (b) Build the json-render spec interpreter now and render specs (TUIDASH),
  (c) Minimal hardcoded interpreter over the 3 template JSON specs
- **Resolution:** Option (a). Native surfaces ship value immediately against
  data that already persists, reuse the proven `plan_dashboard` pattern, and
  carry zero new dependency or AI surface. The json-render interpreter (b)
  remains TUIDASH's charter; (c) was rejected because a one-off interpreter is
  throwaway work that neither ships faster than (a) nor advances TUIDASH.
- **Status:** Resolved

**D-TDASH-002:** Data source

- **Options:** (a) Read persisted `.anvil/` artefacts directly from Rust,
  (b) Shell out to `anvil <cmd> --json` and parse, (c) Query anvil-api over HTTP
- **Resolution:** Option (a). The kernel already has file readers for these
  stores; reuse them for zero-dependency, deterministic data binding. Mirrors
  D-TUIDASH-002.
- **Status:** Resolved

**D-TDASH-003:** Command shape

- **Options:** (a) Top-level `anvil dashboard [name]` with a default picker,
  (b) Per-domain nesting (`anvil architecture dashboard`, `anvil drift
  dashboard`), (c) A single `anvil dashboard` that always opens the picker
- **Resolution:** Option (a). One discoverable entry point; `anvil dashboard`
  with no name opens the picker, `anvil dashboard architecture` jumps straight
  in. Consistent with how `anvil plan dashboard` is reached and keeps domain
  command groups uncluttered.
- **Status:** Resolved

## Constraints

- Read-only: a dashboard must never mutate `.anvil/` state.
- Must render at 80x24 minimum (reuse `eddacraft-tui::compat` min-size check).
- Missing/empty data renders an empty-state, never a panic or an error screen.
- Non-TTY (`--no-tui`, piped, `--json`) must produce useful output without a
  terminal, matching `anvil plan dashboard`.
- No new third-party dependencies; compose existing `eddacraft-tui` widgets.

## Risks

| Risk | Impact | Mitigation |
|------|--------|------------|
| Native surfaces and a future json-render catalogue diverge in look/behaviour | medium | Reuse `eddacraft-tui` widgets + theme so both render through the same primitives; treat these surfaces as candidate catalogue components |
| `.anvil/` store schemas change and break readers | low | Read through existing kernel readers, not ad-hoc parsing; snapshot tests catch drift |
| Scope creep toward gate/watch before their data persists | medium | Explicitly out of scope until a persistence prerequisite lands; documented in Prerequisites |

## Prerequisites

- Architecture, drift, and suppressions dashboards are unblocked — their data is
  persisted today.
- Gate-summary needs the last gate run persisted to `.anvil/` (a separate
  prerequisite item, not in this module). Watch-session needs persisted
  `WatchStats` (the contract referenced in the TUIDASH-009 sweep note).

## Ready Checklist

Status is **Ready** because:

- [x] Purpose and scope are clear
- [x] Dependencies identified and confirmed present in-tree
- [x] At least one work item defined
- [x] Decisions (D-TDASH-001 through D-TDASH-003) resolved
- [x] Target data confirmed persisted under `.anvil/` for every in-scope surface

## Wave

**Wave 3** — Structurally unblocked (anvil-tui, eddacraft-tui, RCLI complete).
Parallel to TUIDASH; neither blocks the other. TDASH-001 must land before the
per-domain surfaces.

## Work Items

### TDASH-001: `anvil dashboard` command + shared read-only surface scaffold

- **Status:** Merged 2026-05-26 via PR #1981
- **Intent:** Add the `anvil dashboard [name]` command and a reusable read-only
  dashboard surface scaffold in `crates/anvil-tui/`, with JSON/plain/TUI
  fallbacks and a dashboard picker listing available dashboards
- **Expected Outcome:** `anvil dashboard` opens a picker of available
  dashboards; `anvil dashboard <name>` selects one; `--json` and non-TTY emit
  the dashboard registry/empty state without a terminal. No domain data yet —
  the scaffold renders chrome, empty-state, and the picker.
- **Files:**
  - `crates/anvil-cli/src/commands/dashboard.rs`
  - `crates/anvil-tui/src/surfaces/dashboard/mod.rs`
  - `crates/anvil-tui/src/surfaces/dashboard/render.rs`
- **Dependencies:** —
- **Validation:** `anvil dashboard --json` lists registered dashboards;
  `anvil dashboard` with no TTY prints the plain picker; snapshot test for the
  picker surface
- **Confidence:** high

### TDASH-002: Architecture-health dashboard surface

- **Status:** Merged 2026-05-26 via PR #1986
- **Intent:** Render architecture health (score, layers, boundary violations,
  rule compliance) natively from `.anvil/architecture.json`
- **Expected Outcome:** `anvil dashboard architecture` renders a health-score
  header, a violations table, and rule-compliance metrics. Empty/missing data
  renders an empty-state. JSON/plain fallbacks supported.
- **Files:**
  - `crates/anvil-tui/src/surfaces/dashboard/architecture.rs`
- **Dependencies:** TDASH-001
- **Validation:** Snapshot test with a representative `architecture.json`;
  `anvil dashboard architecture --json` returns the structured snapshot
- **Confidence:** high

### TDASH-003: Drift-snapshots dashboard surface

- **Status:** Merged 2026-05-26 via PR #1988
- **Intent:** Render drift snapshots and baseline comparison natively from
  `.anvil/snapshots/` and `.anvil/baseline.json`
- **Expected Outcome:** `anvil dashboard drift` lists snapshots with timestamps,
  shows new-edge counts vs baseline, and highlights the latest delta. Empty
  snapshot dir renders an empty-state. JSON/plain fallbacks supported.
- **Files:**
  - `crates/anvil-tui/src/surfaces/dashboard/drift.rs`
- **Dependencies:** TDASH-001
- **Validation:** Snapshot test with a representative snapshots dir + baseline;
  `anvil dashboard drift --json` returns the structured snapshot
- **Confidence:** high

### TDASH-004: Suppressions-overview dashboard surface

- **Status:** Merged 2026-05-26 via PR #1989
- **Intent:** Render the suppressions inventory natively from
  `.anvil/suppressions.json`
- **Expected Outcome:** `anvil dashboard suppressions` lists suppressions with
  scope, justification, and approver, plus a count summary. Empty file renders
  an empty-state. JSON/plain fallbacks supported.
- **Files:**
  - `crates/anvil-tui/src/surfaces/dashboard/suppressions.rs`
- **Dependencies:** TDASH-001
- **Validation:** Snapshot test with a representative `suppressions.json`;
  `anvil dashboard suppressions --json` returns the structured snapshot
- **Confidence:** high

## Notes

Recorded 2026-05-26 after confirming (a) the json-render work shipped on the
**web** side as `@eddacraft/render` (`packages/libs/render/`), not in
`eddacraft-tui`, so TUIDASH's Rust-side interpreter is still unbuilt; and (b)
`anvil plan dashboard` (APSCAN-011) already proves a native Ratatui surface
reading local state with JSON/plain/TUI fallbacks. The three in-scope surfaces
read data that is persisted under `.anvil/` today; gate-summary and
watch-session are deferred until their inputs are persisted.
