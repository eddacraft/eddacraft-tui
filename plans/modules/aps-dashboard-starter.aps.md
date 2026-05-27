<!-- APS: See https://github.com/eddacraft/anvil-plan-spec for format reference -->
<!-- Executable only if work items exist and status is Ready. -->

# APS Dashboard Starter Kit

| ID      | Owner      | Status      | Progress |
| ------- | ---------- | ----------- | -------- |
| APSDASH | joshuaboys | In Progress | 2/4      |

**Last reviewed:** 2026-05-27

**Blocks on:** [ADR-055](../decisions/055-aps-oss-carveout.md) — publishing
the kit relicenses proprietary Anvil source (`anvil-cli`/`anvil-tui`,
`LicenseRef-Proprietary` per ADR-018) as Apache-2.0 for the public
`anvil-plan-spec` repo. A Council review (2026-05-27) blocked publication on this
unresolved IP decision. The kit is staged and verified but **must not be
published** until ADR-055 is Accepted with legal sign-off.

## Purpose

Package Anvil's `anvil plan dashboard` (delivered by APSCAN-011) as an optional,
reusable starter kit under `tools/starters/aps-dashboard/`, following the
`tools/starters/acknowledgements/` precedent and the ADR-047 starter-kit
topology. The kit is a **seed copy** intended to be lifted once into a downstream
repo (the canonical `anvil-plan-spec`) and re-developed there into a standalone,
shippable terminal dashboard for canonical APS plan state.

The dashboard's render layer depends only on the public, crates.io-published
`eddacraft-tui` crate — its sole anvil-local import (`crate::surface::Surface`)
was a re-export of `eddacraft_tui::surface::Surface`, so the extraction is a near
clean lift rather than a decoupling project.

## In Scope

- A faithful seed copy of the dashboard sources (`snapshot` builder + Ratatui
  render layer) under `tools/starters/aps-dashboard/`.
- Standalone-crate glue as `.template` files: `Cargo.toml`, a reduced run loop,
  and a CLI entry point with the snapshot→render adapter.
- A README documenting the coupling map, the wire-up checklist, the three seams,
  and the re-develop-to-ship roadmap for the adopting repo.

## Out of Scope

- The downstream re-development itself (consolidating snapshot types, canonical
  status decoupling, branding, render snapshot tests) — that happens in
  `anvil-plan-spec`, tracked as APSDASH-002 and not closeable from this repo.
- Changing Anvil's in-repo `anvil plan dashboard`; the kit is a one-way fork, not
  a mirror of the internal surface.
- A public mirror repository / mirror workflow for the kit (deferred; the stated
  destination is direct adoption into `anvil-plan-spec`).

## Interfaces

**Depends on:**

- `crates/anvil-cli/src/plan_dashboard.rs`,
  `crates/anvil-tui/src/surfaces/plan_dashboard/*`,
  `crates/anvil-cli/src/commands/plan.rs`, `crates/anvil-cli/src/tui.rs` — the
  upstream sources seeded into the kit.
- [`eddacraft-tui`](https://github.com/eddacraft/eddacraft-tui) 0.2.x — the
  public substrate the extracted component consumes (ADR-047).
- `tools/starters/acknowledgements/` — the starter-kit layout precedent.

**Exposes:**

- `tools/starters/aps-dashboard/` — canonical source of the kit in this repo.

## Constraints

- Read-only: the dashboard never mutates plan state.
- The kit must not become a cargo workspace member — `.rs` sources stay inert in
  this repo (manifest ships as `Cargo.toml.template`) so workspace build / clippy
  / fmt gates and `ACKNOWLEDGEMENTS` regeneration are untouched.

## Ready Checklist

Status is **In Progress** because APSDASH-001 is staged in this branch:

- [x] Purpose and scope are clear
- [x] Upstream sources and the public dependency identified
- [x] Coupling map confirmed (single `use`-line seam in the render layer)
- [x] At least one work item defined

## Work Items

### APSDASH-001: Decide the OSS/IP boundary for the APS viewer (ADR-055)

- **Status:** Done
- **Intent:** Resolve whether a read-only APS plan viewer may move into the OSS
  surface, given the source is proprietary (`anvil-cli`/`anvil-tui` per ADR-018).
- **Expected Outcome:** An ADR is filed proposing a narrow carve-out — a
  read-only viewer of the public APS format, built only on the public OSS
  surface, exposing no product internals — bounded by a three-test rule, with a
  legal sign-off gate before it can advance to Accepted.
- **Files:** `plans/decisions/055-aps-oss-carveout.md`,
  `plans/decisions/DECISION-LOG.md`.
- **Validation:** `pnpm docs:check` (ADR + index-freshness surfaces pass);
  Council review (2026-05-27) confirms the IP problem is captured.
- **Confidence:** high — the decision artifact is filed; its Accepted state is
  gated on legal, tracked in the DECISION-LOG.

### APSDASH-002: Stage the verified seed kit

- **Status:** Done
- **Intent:** Land a faithful, build-verified seed copy of the dashboard under
  `tools/starters/aps-dashboard/`.
- **Expected Outcome:** The kit holds the copied `snapshot` builder and render
  layer (with the one `Surface` import repointed at the public crate), the
  `Cargo.toml` / run-loop / entry-point templates, and a README. The kit was
  assembled as a standalone crate against the published `eddacraft-tui` 0.2.2 and
  its 30 copied unit tests pass.
- **Files:** `tools/starters/aps-dashboard/**`, `plans/index.aps.md`.
- **Validation:** `pnpm format:check`, `pnpm docs:check`, `pnpm aps:drift --json`;
  standalone `cargo test` (30/30) against crates.io `eddacraft-tui`.
- **Confidence:** high

### APSDASH-003: Scrub and publish the seed to `anvil-plan-spec`

- **Status:** Blocked
- **Blocks on:** APSDASH-001 / ADR-055 Accepted + legal sign-off.
- **Intent:** Once the IP decision clears, run the ADR-055 pre-publication
  checklist and lift the seed into `anvil-plan-spec`.
- **Expected Outcome:** Apache-2.0 headers applied; Anvil-internal references
  scrubbed (internal crate paths in comments, status-dialect framing, fixture
  strings — internal branch names, SHAs, PR numbers, version tags, non-existent
  crate names); hardcoded `"Anvil APS Work Dashboard"` branding neutralised; a
  self-test CI gate added in the home repo; the scrubbed copy re-reviewed by
  Council before the lift.
- **Validation:** Council sign-off on the scrubbed copy; the published component
  builds + tests green against crates.io `eddacraft-tui` in its home-repo CI.
- **Confidence:** medium — mechanically clear, but gated on the legal decision.

### APSDASH-004: Re-develop into a standalone shippable component

- **Status:** Proposed
- **Intent:** In `anvil-plan-spec`, turn the seed into a polished, general
  component: collapse the two snapshot types, target canonical APS status
  semantics (treat `Merged` / `Released/Shipped` as configurable extensions),
  set branding, port the panic-restore hook, and add render snapshot tests.
- **Also fold in (PR #2013 Copilot review, declined upstream to keep the seed a
  faithful copy):** render unknown `done`/`total` in the progress-mismatch
  diagnostic explicitly (e.g. `?`/`-`) rather than `0/N`; precompute a completed-ID
  `HashSet` for the blocked-dependency check instead of the nested scan.
- **Expected Outcome:** A standalone `aps-dashboard` crate in `anvil-plan-spec`
  that builds against the published `eddacraft-tui`, renders canonical APS state,
  and ships independently of Anvil.
- **Validation:** Determined downstream in `anvil-plan-spec`; out of scope for
  Anvil CI.
- **Confidence:** medium — the seam analysis is done, but downstream CLI/branding
  conventions and the snapshot-type consolidation are design calls for that repo.
