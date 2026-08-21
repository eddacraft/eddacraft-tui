<!-- APS: See https://github.com/eddacraft/anvil-plan-spec for format reference -->
<!-- Executable only if tasks exist and status is Ready. -->

# TUI Impact View — interactive boundary/impact graph

| ID   | Owner | Status | Progress |
| ---- | ----- | ------ | -------- |
| IMPV | —     | Draft  | 0/1      |

**Last reviewed:** 2026-08-21 (created from the `spike-flow` validation spike in
[PR #4074](https://github.com/eddacraft/anvil-001/pull/4074), branch
`feat/spike-flow-graph`). No existing module owned this surface: ACTTUI is Done
and scoped to `anvil start` activation, and TUIN explicitly excludes
"Anvil-internal TUI surface redesign (`crates/anvil-tui/`, `crates/anvil-cli/`)"
— it owns the shared crate's contract, not its consumers' surfaces. This module
is the consumer-side home; any eventual `eddacraft-tui` widget promotion is
filed against TUIN, not here.

## Purpose

Anvil is a graph product with no graph view. The boundary and impact answers
that the kernel already computes — used-import edges between crates, module-level
structure inside a crate, the blast radius of a change — are reachable today only
through MCP tools, JSON, or prose. This module gives a person the same answers
interactively, in the terminal they are already in.

The spike (`crates/spike/src/flow.rs`, `spike-flow` bin) settled the two
questions that would otherwise have to be answered by building:

1. **Rendering.** `rataflow` 0.1 (node-based flow graphs for ratatui 0.30, MIT,
   `ratatui` + `thiserror` + `rust-sugiyama`; `petgraph`/`indexmap` already
   in-workspace) renders Sugiyama-laid-out graphs in-terminal with pan, zoom,
   semantic zoom, selection, and edge creation, against our exact ratatui 0.30 /
   crossterm 0.29 pins.
2. **Data.** The warm per-worktree graph-cache snapshot (`ANVILGC1`, ADR-069),
   decoded through `anvil-graph-cache`, is sufficient to derive a crate-level
   graph of **used** imports (not declared Cargo dependencies) plus per-crate
   internal module graphs — with **zero daemon changes**.

What remains is productisation: turning a spike binary into a surface in
`crates/anvil-tui/` that a user can open against their own repository, with the
honesty, snapshot-testing, and terminal-lifecycle discipline the shipped TUI
surfaces already hold.

## In Scope

- An impact view surface in the anvil TUI consumer layer (`crates/anvil-tui/`,
  entry point via `crates/anvil-cli/`) for the current repository.
- Crate-level used-import graph and per-crate internal module graphs derived
  from the warm graph-cache snapshot, read in-process or over the daemon —
  whichever the item's design pass settles, without new substrate.
- Drill-down navigation (neighbourhood, internals, back) with a breadcrumb, and
  zoom controls including zoom-to-read for dense graphs.
- `rataflow` as the rendering engine, pinned exact in the consuming crate (the
  `animate` precedent), with attribution recorded because this ships.
- Mouse capture, alt-screen entry/exit, and panic restore owned by the shell
  layer, not by the view.
- Honest empty and degraded states: no snapshot, cold cache, unsupported
  language, or a graph too large to lay out must say so rather than render an
  empty canvas.

## Out of Scope

- Persisting proposed edges. The spike's session-only edge editing proves the
  interaction; writing a proposed dependency edge back to architecture rules or
  the graph is a separate decision with its own authority question.
- New graph substrate, new daemon RPC surface, or changes to the ADR-069
  snapshot format.
- Promoting a graph widget into `eddacraft-tui` behind an off-by-default feature
  flag. That is a possible follow-up and belongs to
  [TUIN](./tui-next.aps.md) (`eddacraft-tui` crate contract), pinned exact like
  the `animate` dependency, and only after this surface has a second consumer or
  a demonstrated reuse case.
- Web dashboard architecture graphs — owned by
  [DASHARCH](./dashboard-architecture-views.aps.md).
- Editor-side projection of the same graph — owned by
  [LSPNAV](./lsp-graph-navigation.aps.md).

## Interfaces

**Depends on:**

- `crates/anvil-graph-cache/` — snapshot decode (ADR-069 `ANVILGC1`), the
  spike's proven data source.
- `crates/anvil-tui/` + `crates/anvil-cli/` — consumer surface and entry point;
  `eddacraft-tui` lifecycle/theme conventions apply as they do for the other
  shipped surfaces.
- `rataflow` 0.1 (MIT) — rendering engine; validated in PR #4074.
- [`plans/specs/anvil-ultimate-ui/`](../specs/anvil-ultimate-ui/00-index.md) and
  [`plans/specs/2026-08-07-two-track-ui-strategy.md`](../specs/2026-08-07-two-track-ui-strategy.md)
  — the ultimate-ui track this surface informs. Ultimate UI itself is a gated
  research track in a separate repository; nothing here waits on it.

**Exposes:**

- An interactive impact view a user can open against their own repository, and
  the first real evidence of whether a terminal graph view changes how people
  read their own architecture.

## Work Items

### IMPV-001: Interactive impact view in the anvil TUI

- **Status:** Draft
- **Intent:** A person can open an interactive boundary/impact graph of the
  repository they are working in, from the anvil TUI, and navigate from
  crate-level structure down to the internals of one crate.
- **Expected Outcome:** The impact view opens from the anvil TUI for the current
  repository and renders the crate-level used-import graph derived from the warm
  graph-cache snapshot. Selection drills into a crate's neighbourhood and into
  its internal module graph, with a visible way back and a breadcrumb naming
  where you are. Zoom controls include zoom-to-read, so labels are legible in a
  dense graph. The view is read-only: proposed-edge editing is not persisted.
  Mouse capture and terminal lifecycle are owned by the shell layer, so the view
  cannot leave a terminal in raw mode. Absent, cold, or unrenderable graph state
  is named on screen rather than shown as an empty canvas. Snapshot coverage
  pins at least the crate-level render, one drill-down level, and one degraded
  state.
- **Validation:** `cargo test -p eddacraft-anvil-tui`,
  `cargo clippy --workspace --all-targets -- -D warnings`, plus a headless
  render check of the new surface (the spike's `--snapshot WxH` TestBackend
  path is the precedent).
- **Files:** `crates/anvil-tui/`, `crates/anvil-cli/`, `Cargo.toml`,
  `ACKNOWLEDGEMENTS.md`
- **Dependencies:** —
- **Confidence:** medium — rendering and data source are both spike-proven; the
  open questions are the entry-point shape (own command vs panel in an existing
  surface), whether the snapshot is read in-process or over the daemon, and how
  large a graph the layout stays usable on.
- **Risks:** A repo whose graph is an order of magnitude larger than anvil's own
  may exceed what Sugiyama layout renders usefully in a terminal; the degraded
  state has to be designed, not discovered. `rataflow` is a 0.1 dependency — the
  exact pin and a shipped-crate attribution entry are load-bearing, and an
  upstream break is a real maintenance cost this module accepts on the consumer
  side before any `eddacraft-tui` promotion is considered.

---

Promoting IMPV-001 to Ready is an operator decision. The spike in PR #4074 is
validation evidence, not execution authority.
