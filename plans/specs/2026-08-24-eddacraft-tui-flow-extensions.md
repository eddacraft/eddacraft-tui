# eddacraft-tui `flow` extensions

| Type | Authority | Owner | Status | Freshness |
| ---- | --------- | ----- | ------ | --------- |
| Spec | Authoritative for the TUIN flow-extension wave | [TUIN](../modules/tui-next.aps.md) | Proposed | 2026-08-24 — filed from the 0.5.1 post-cut design pass |

| Upstream | Downstream |
| -------- | ---------- |
| [ADR-128](../decisions/128-eddacraft-tui-flow-feature.md); [TUIN-014](../modules/tui-next.aps.md); `crates/eddacraft-tui/src/flow.rs`; anvil impact view (`crates/anvil-tui/src/surfaces/impact/`) | TUIN-015..021; crate CHANGELOG on the next `eddacraft-tui` cut |

**Execution authority** is the TUIN-015..021 work-item set. This specification
records the approved direction for evolving the existing `flow` feature. It
does not authorise product code, promote any item to Ready, or make a crate
release claim.

## 1. Problem

`eddacraft-tui` 0.5.1 shipped a thin spike harvest behind `flow`: a theme map,
vertical Sugiyama from an edge list, a hand-rolled container grid,
`zoom_to_read`, and a mouse RAII guard. The interesting product behaviour —
drill-down, neighbourhood, degraded states — lives in the anvil impact view,
which rebuilds the whole `Flow` on every stack push and drops into `flow::raw`
for directional selection.

A second declared consumer (APS tooling) will re-solve the same problems unless
the wrapper grows the helpers.

## 2. Constraints

- Stay behind the existing off-by-default `flow` feature. No new Cargo feature
  flag unless a later ADR says otherwise (morph is the only likely exception).
- Public items remain `# Stability` **experimental** until a later grading pass.
- Exact `rataflow = "=0.1.0"` pin unchanged. Do not vendor or fork rataflow.
- TUIN still does not redesign Anvil-internal surfaces. Impact may *call* new
  helpers as proof; IMPV owns any impact UX change.
- Unicode display width, not `.chars().count()`, for node geometry.
- No silent `stable` API break. Additive helpers only.

## 3. Sequence

Wave 1 can run as two parallel items (015 and 016). Wave 2 consumes them.
Wave 3 waits for a second consumer or a dedicated spike.

```text
Wave 1 (parallel):  TUIN-015 spotlight cone
                    TUIN-016 preserve view + fit-then-read
Wave 2:             TUIN-017 role-styled specs + Unicode width
                    TUIN-018 graph diff (needs 017)
                    TUIN-019 FlowSession
Wave 3:             TUIN-020 elision portals (needs 016)
                    TUIN-021 layout-morph spike (needs 016)
```

015 does not have to wait for 017: spotlight can use today's
`set_edge_animated` plus `Theme` colours, then re-express through roles when
017 lands.

## 4. In-wrapper outcomes

### Spotlight cone (TUIN-015)

`flow::spotlight(&mut Flow, node_id, Spotlight::{Upstream, Downstream, Both})`
walks edges, mutes the complement, and animates the remaining edges. Tests
cover a small DAG snapshot and an unknown-id no-op.

### Preserve view + fit-then-read (TUIN-016)

Extract `ViewState { zoom, pan, selected }`. `rebuild_preserving_view` keeps
camera and selection across an edge-list change.

`zoom_to_read_after_layout(flow, node_id, width, height)` is the first-frame
hook. rataflow applies `request_fit_view` on the first render at a canvas
size and clears the request only on a second render at that same size. The
helper draws twice into an off-screen buffer of `(width, height)`, then
calls `zoom_to_read`. Tests must start from `request_fit_view` and assert
the read zoom survives a later same-size frame.

### Role-styled specs (TUIN-017)

`NodeSpec` / `EdgeSpec` carry a `Theme` `Role`. Existing
`themed_from_edges` / `container_flow` string signatures stay unchanged
(Rust has no overloads). Role-styled construction is a distinct additive
constructor: `themed_from_specs(nodes, edges, theme)`. Map `warning` per
node/edge (rataflow's palette has no warning slot). Size container cells
with `unicode-width`.

### Graph diff (TUIN-018)

`themed_from_diff(before, after, theme)` takes two edge-list descriptions,
not prior `Flow`s. Layout is one Sugiyama pass over the **union** of both
lists so removed nodes stay in the graph as `Error` ghosts occupying a
slot in that union layout. Added edges are `Success`, unchanged edges
muted. This is occupancy-stable (ghosts do not vanish), not a replay of
the previous frame's coordinates.

### FlowSession (TUIN-019)

RAII session that composes `MouseCaptureGuard`, the first-frame camera, and
optional `lifecycle::TerminalGuard`. Drop restores mouse and terminal. Impact
may keep keyboard-only; the session exists so APS and examples cannot leak
mouse mode.

### Elision portals (TUIN-020)

Caller supplies the library-owned budget (`max_visible`). The helper does
not read Anvil's impact `MAX_RENDERABLE_NODES`. Over-budget lowest-degree
nodes collapse to a portal (`… N crates`). `ElidedGraph` holds `portal_id`
and `collapsed` member ids; `elide_from_edges_keeping` expands those
members in place. Camera preservation is TUIN-016's job.

### Layout morph spike (TUIN-021)

Spike only: interpolate node positions with `animate-core` over ~250 ms
quad-out when the edge list changes. Ship/no-ship recorded; a yes spawns its
own implement item and an ADR (new feature coupling).

## 5. Non-goals

- Wrapping every rataflow method so consumers never import `flow::raw`.
- Horizontal Sugiyama as its own work item (trivial flag on 016/017 if needed).
- A minimap, vendoring rataflow, or a new published crate.
- Promoting `flow` items from experimental to stable.
- IMPV intent-capture or the policy-boundaries *view* (those stay IMPV).

## 6. Release

Crate semver stays independent of Anvil (D-TUIR-006). Wave 1+2 are additive
experimental API: patch bucket while 0.x minor remains the breaking bucket.
Do not cut a crate release from this spec; the next `eddacraft-tui` cut
consumes whatever has Merged.

## 7. TUIN-021 spike result

**Decision: no-ship.** Viewport lerp is well-defined (unit test
`morph_lerp_is_monotonic`) without coupling `flow` to `animate-core`. No
public morph API and no new Cargo feature. Duration/easing (~250 ms
quad-out) remains a candidate if revived. An ADR is required only if a
later item ships that coupling.
