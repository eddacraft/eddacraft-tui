<!-- APS: See https://github.com/eddacraft/anvil-plan-spec for format reference -->
<!-- Executable only if tasks exist and status is Ready. -->

# TUI Impact View — interactive boundary/impact graph

| ID   | Owner | Status | Progress |
| ---- | ----- | ------ | -------- |
| IMPV | —     | In Progress | 0/1      |

**Last reviewed:** 2026-08-22 (created from the `spike-flow` validation spike in
[PR #4074](https://github.com/eddacraft/anvil-001/pull/4074); findings updated
from the spike's second pass in
[PR #4081](https://github.com/eddacraft/anvil-001/pull/4081) — intent layer,
scriptable CLI surface, and policy boundaries view). No existing module owned
this surface: ACTTUI is Done
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

The spike (`crates/spike/src/flow.rs`, `spike-flow` bin) settled, across two
passes (PR #4074, PR #4081), the questions that would otherwise have to be
answered by building:

1. **Rendering.** `rataflow` 0.1 (node-based flow graphs for ratatui 0.30, MIT,
   `ratatui` + `thiserror` + `rust-sugiyama`; `petgraph`/`indexmap` already
   in-workspace) renders Sugiyama-laid-out graphs in-terminal with pan, zoom,
   semantic zoom, selection, and edge creation, against our exact ratatui 0.30 /
   crossterm 0.29 pins.
2. **Data.** The warm per-worktree graph-cache snapshot (`ANVILGC1`, ADR-069),
   decoded through `anvil-graph-cache`, is sufficient to derive a crate-level
   graph of **used** imports (not declared Cargo dependencies) plus per-crate
   internal module graphs — with **zero daemon changes**.
3. **Write-side semantics.** Graph gestures record **intent**, never code
   mutations: flag-with-note, planned node (renders in the graph before the
   crate exists), retire-intent (marked, still visible until reality catches
   up), proposed edge. Intent persists as deterministic JSON (`BTreeMap`
   serialization; an unparseable file is preserved as `<path>.invalid`, never
   overwritten) in `.anvil/impact-notes.json` — ADR-073 local runtime state —
   and is reconciled against the actual graph on every load ("pending" →
   "now real ✓", "still present" → "gone ✓"). The same store is scriptable
   (`--flag`/`--unflag`/`--plan`/`--retire`/`--propose` with `--note`;
   `--report` is deterministic and always exits 0), so agents, CI, and the
   TUI share one intent surface.
4. **Boundary lens.** A policy file mirroring anvil-architecture's layer model
   (member patterns + `depends_on`, most-specific-match precedence: exact
   beats prefix, longer prefix beats shorter) supports both `⚠` violation
   reporting over actual used-import edges and a **boundaries view** drawing
   each layer as a titled rataflow parent-container box — members gridded
   inside, layers stacked dependents-above-dependencies, violating edges
   animated. Key layout finding: do **not** compose containers with Sugiyama.
   The dependency lens (Sugiyama, "what depends on what") and the boundary
   lens (policy-driven geometry, "is everything where it belongs") are two
   views one keypress apart, sharing selection and intent state. The
   productised version reads the real policy engine, not a sidecar file.

Terminal-lifecycle gotchas the productisation inherits: `ratatui::run` does not
enable mouse capture (the shell layer must own it, with release on every exit
path); `request_fit_view()` is deferred to the next render, so programmatic
zoom before the first frame is silently overridden; semantic zoom hides labels
when zoomed out, which is what makes zoom-to-read necessary; rataflow's default
zoom clamp (0.5–2.0) caps how far fit-view can zoom out on large graphs.

What remains is productisation: turning a spike binary into a surface in
`crates/anvil-tui/` that a user can open against their own repository, with the
honesty, snapshot-testing, and terminal-lifecycle discipline the shipped TUI
surfaces already hold. Settled-by-the-spike is not the same as in-scope-for-
IMPV-001: the first item ships the **read lenses only** (findings 1, 2, and 4's
reporting side). Intent capture (finding 3) and the boundaries *view* are
spike-proven and inform the design, but enter as later items — intent capture
only after the graduation question in Out of Scope is decided.

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

- Graduating intent beyond local state. The spike persists flags, planned
  nodes, retire-intent, and proposed edges to `.anvil/impact-notes.json`
  (gitignored, ADR-073) and proves the reconcile-against-reality loop; what
  graduates to a **shared or committed** intent surface — and whether a
  proposed edge ever feeds architecture rules — is a separate decision with
  its own authority question.
- Reading the real policy engine for the boundary lens. The spike's sidecar
  policy file proves the view; wiring it to the actual policy/architecture
  engine is part of productisation design, not more spike work.
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

- **Status:** In Progress
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
- **Confidence:** medium-high — rendering, data source, write-side intent
  semantics, and the two-lens view split (Sugiyama dependency lens vs
  policy-driven boundary lens) are all spike-proven; the open questions are
  the entry-point shape (own command vs panel in an existing surface), whether
  the snapshot is read in-process or over the daemon, and how large a graph
  the layout stays usable on.
- **Risks:** A repo whose graph is an order of magnitude larger than anvil's own
  may exceed what Sugiyama layout renders usefully in a terminal (the boundary
  lens is policy-driven geometry and degrades differently); the degraded state
  has to be designed, not discovered. `rataflow` is a 0.1 dependency — the
  exact pin and a shipped-crate attribution entry are load-bearing, and an
  upstream break is a real maintenance cost this module accepts on the consumer
  side before any `eddacraft-tui` promotion is considered.

---

IMPV-001 was promoted to Ready by the operator on 2026-08-22, with the spike
(PRs #4074/#4081) as validation evidence and the scope line drawn above: read
lenses first, intent capture and the boundaries view as later items.
