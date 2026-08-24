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
camera and selection across an edge-list change. A first-frame helper applies
pending `request_fit_view` *after* layout so `zoom_to_read` is not silently
undone. This deletes the footgun documented in `flow.rs` today.

### Role-styled specs (TUIN-017)

`NodeSpec` / `EdgeSpec` carry a `Theme` `Role`. `container_flow` and
`themed_from_edges` accept specs as well as bare strings. Map `warning` into
the rataflow palette or an equivalent per-edge/node style. Size nodes with
`unicode-width`.

### Graph diff (TUIN-018)

`themed_from_diff(before, after, theme)` keeps layout stable: added edges
`Success`, removed edges `Error` as ghosts occupying the old positions,
unchanged edges muted. Ghost nodes do not disappear between frames.

### FlowSession (TUIN-019)

RAII session that composes `MouseCaptureGuard`, the first-frame camera, and
optional `lifecycle::TerminalGuard`. Drop restores mouse and terminal. Impact
may keep keyboard-only; the session exists so APS and examples cannot leak
mouse mode.

### Elision portals (TUIN-020)

Over-budget clusters collapse to a portal node (`… N crates`). Selecting a
portal expands in place. Library-level, not a consumer `if nodes > N` that
degrades the whole surface.

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
