# ADR-128: eddacraft-tui `flow` feature — rataflow-backed flow graphs

## Status

Proposed

## Date

2026-08-22

## Context

The anvil impact view (IMPV-001) needs interactive node-graph rendering in
the terminal, and the operator has declared a second consumer: planned use of
the same widget in APS tooling. The `spike-flow` validation spike (PRs #4074
and #4081) proved `rataflow` 0.1 — node-based flow graphs for ratatui 0.30,
MIT-licensed, with Sugiyama layout, pan/zoom, semantic zoom, selection, edge
creation, and parent-container hierarchies — against the workspace's exact
ratatui 0.30 / crossterm 0.29 pins, including a container-box boundary
layout and terminal-lifecycle behaviour (mouse capture, deferred fit-view).

TUIN's precedent (ADR-050 for `runner`/`lifecycle`) is that a new public
feature flag on the published `eddacraft-tui` crate lands with its own
decision record. TUIN-014 carries the work item; the IMPV reuse gate
("second consumer or demonstrated reuse case") is satisfied per the operator
authorisation of 2026-08-22 recorded in the TUIN module.

## Decision

Add an off-by-default `flow` Cargo feature to `eddacraft-tui`, following the
`image` feature template exactly (optional dependency, paired
`cfg`/`doc(cfg)` attributes, modules-table row, docs.rs all-features build).
The feature gates a `flow` module wrapping `rataflow`:

- `rataflow = "=0.1.0"` pinned **exact** (the `animate-core` precedent: 0.x
  crate, small maintainer pool; bumps go through `cargo audit` and manual
  review), `default-features = false` with only `sugiyama` + `crossterm`.
- Curated re-exports plus spike-proven helpers: themed construction
  (`Theme` → rataflow palette mapping), `zoom_to_read`, a layered
  container-box builder with a stable edge-id scheme, and a mouse-capture
  RAII guard (neither `ratatui::run` nor the `lifecycle` feature enables
  mouse reporting).
- The full upstream API stays reachable as `flow::raw` while the wrapper
  surface settles; every public item is graded `# Stability` **experimental**.

## Rationale

Consumers get graph surfaces without taking a direct dependency on a 0.x
crate, theming stays centralised through `Theme::role_style` conventions, and
the terminal-lifecycle traps the spike documented are solved once in the
library instead of per consumer.

### Alternatives Considered

| Option | Pros | Cons |
|--------|------|------|
| Feature-gated wrapper module (chosen) | Off-by-default, exact pin, curated surface, one place for lifecycle traps | Wrapper maintenance as upstream evolves |
| Consumers depend on rataflow directly | No wrapper layer | Each consumer re-solves theming + lifecycle; version skew across consumers; 0.x churn multiplied |
| Vendor/fork rataflow into eddacraft-tui | Full control | Maintenance burden out of proportion for a 0.1 engine still moving upstream |

## Consequences

- **Positive:** anvil impact view and APS tooling share one themed graph
  surface; `flow` stays out of default builds (core MSRV 1.88 unchanged —
  rataflow's floor is also 1.88); attribution recorded because the crate
  ships.
- **Negative:** the published crate's optional surface grows; docs.rs and
  all-features CI lanes now build rataflow.
- **Risks:** upstream 0.x breakage lands on the exact-pin bump, not
  silently; the experimental grade keeps downstream expectations honest.
- **Mitigations:** exact pin + audit-and-review bump policy; `flow::raw`
  escape hatch means wrapper gaps never block a consumer.

## References

- Related ADRs: ADR-050 (runner/lifecycle feature precedent), ADR-126
  (spike-crate gate exemption)
- APS modules: TUIN-014 (this feature), IMPV-001 (first consumer)
- External: PRs #4074 / #4081 (spike evidence), rataflow (MIT,
  <https://github.com/furkankly/rataflow>)
