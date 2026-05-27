# ADR-053: json-render TUI engine in eddacraft-tui; Anvil catalogue in anvil-tui

## Status

Proposed

## Date

2026-05-27

## Context

[`tui-dashboard-render`](../modules/tui-dashboard-render.aps.md) (TUIDASH) plans
a Rust-native [json-render](https://github.com/vercel-labs/json-render)
interpreter for Ratatui: parse the same flat-element JSON spec the web dashboard
uses (`@json-render/core` v0.19, via the web-side `@eddacraft/render` package at
`packages/libs/render/`), and render it in the terminal. The value proposition
is write-once / render-anywhere — one spec, web and TUI surfaces.

No such Rust interpreter exists yet — as of this ADR, `eddacraft-tui` is `0.2.2`
with modules animation/compat/keyboard/pretext/shell/surface/theme/widgets and no
spec interpreter, and no other Rust crate provides one. (It is easy to assume the
TUI port shipped alongside the pretext work; it did not.) So we get to choose
where to build it.

The original TUIDASH plan put the whole interpreter in a new Anvil-owned crate,
`crates/anvil-tui-render/`. But json-render is, by design, **generic
infrastructure** — the TUI counterpart to `@json-render/core` + `@json-render/
react`, which are framework-generic. The component *catalogue* (which concrete
components exist and how they map to widgets) is the product-specific layer — the
counterpart to `@json-render/shadcn`. Vercel's own packaging encodes exactly this
core/renderer-vs-catalogue split.

`eddacraft-tui` is explicitly the "shared Ratatui component library for the
eddacraft product family." Its canonical source is now vendored in `anvil-001`
(`crates/eddacraft-tui/`) and mirrored out to the standalone repo, so code
authored there reaches the whole family without a cross-repo dance.

The forces:

- A spec→Ratatui engine is reusable across every eddacraft TUI product, not just
  Anvil — it wants a shared home.
- `eddacraft-tui` must stay product-family-generic; Anvil domain semantics
  (gate results, drift, suppressions, plans) must not leak into it.
- `eddacraft-tui` currently has a deliberately minimal dependency set (no
  `serde`); a spec parser needs `serde` + `serde_json`.
- `anvil dashboard` already exists (the TDASH native-dashboards module shipped
  it: architecture/drift/suppressions surfaces + a picker), so TUIDASH no longer
  *creates* that command — it extends it with a spec-rendering path.

## Decision

Split the json-render TUI work across two layers along the generic/specific
boundary:

1. **Generic engine → `crates/eddacraft-tui/`, behind a `json-render` Cargo
   feature.** This is the spec types + serde parser, the component `Registry`
   trait, the depth-first tree walker, the generic data-path binding mechanism,
   and the Ratatui rendering plumbing — plus a **base catalogue** of generic
   components that map directly to eddacraft-tui widgets (layout, metric, table,
   chart, badge, code/text). No Anvil semantics. `serde`/`serde_json` enter
   `eddacraft-tui` only under the `json-render` feature, keeping the base widget
   library lean for products that don't opt in.

2. **Anvil component catalogue + surface → `crates/anvil-tui/` (+ `anvil-cli`).**
   The Anvil-domain components (GateResultCard, WarningList, DriftIndicator,
   PlanCard, SuppressionRequest, EvidenceEntry), the `.anvil/` data-context
   loader that feeds the generic binding mechanism, and the dashboard surface
   that extends the existing `anvil dashboard` command to render saved
   `.anvil/dashboards/*.json` specs.

The standalone `crates/anvil-tui-render/` crate from the original TUIDASH plan is
**not created**; its responsibilities divide between the two layers above.

Dependency direction stays one-way: `anvil-tui → eddacraft-tui`, never the
reverse.

## Rationale

Why a shared, feature-gated engine in `eddacraft-tui` over an Anvil-owned crate:
the engine is the "render-anywhere" half of "write-once, render-anywhere" — its
whole point is reuse across surfaces and products. Anchoring it in the shared
library (which already owns the widgets, theme, and `Surface` trait it renders
into, and is mirrored to the family) makes it reusable by construction and keeps
the spec format the single contract. Feature-gating contains the `serde` cost.
The Anvil catalogue stays in `anvil-tui` because gate/drift/plan components are
domain semantics that must not enter a generic library.

### Alternatives Considered

| Option | Pros | Cons |
|--------|------|------|
| **Engine in `eddacraft-tui` (feature-gated) + catalogue in `anvil-tui` (chosen)** | Reusable across the family by construction; mirrors vercel's core/renderer vs catalogue split; engine lives with the widgets/theme it renders; clean one-way dep | Adds `serde` to eddacraft-tui (mitigated: feature-gated); engine and Anvil catalogue evolve in two crates |
| Standalone Anvil crate `anvil-tui-render` (original plan) | Self-contained; no eddacraft-tui dep change | Makes the *generic* engine Anvil-owned — other eddacraft products can't reuse it without depending on Anvil; contradicts the render-anywhere goal |
| Everything in `eddacraft-tui` (engine **and** Anvil catalogue) | One crate | Anvil domain semantics leak into the shared family library — violates its generic charter |
| Separate shared crate `eddacraft-json-render` depending on eddacraft-tui | Strictest separation; mirrors vercel's package boundary exactly | Extra crate + mirror overhead for a still-small engine; can be extracted later if the engine grows |

## Consequences

- **Positive:** the engine is reusable across the eddacraft TUI family from day
  one; the spec format is the single web/TUI contract; `eddacraft-tui` stays
  generic; the base catalogue covers generic components so Anvil only writes its
  domain components.
- **Positive:** TUIDASH stops being blocked on a misremembered prerequisite —
  its real prerequisites (spec format pin `@json-render/core ^0.19.0`, the
  catalogue at `packages/libs/render/`, the 3 template specs) all exist.
- **Negative:** `eddacraft-tui` grows a `serde`/`serde_json` dependency (under a
  feature); the work spans two crates rather than one.
- **Risks:** the base-catalogue / Anvil-catalogue boundary could blur (a generic
  component implemented as Anvil-specific, or vice versa); the engine, living in
  a mirrored crate, must never reference Anvil types.
- **Mitigations:** the `json-render` feature gate + `unsafe_code = "forbid"` and
  the existing generic-only convention in eddacraft-tui; a registry-parity check
  (TUIDASH-010) catches catalogue drift; ADR review enforces the dependency
  direction.

## References

- APS modules: TUIDASH (`tui-dashboard-render`), DASHAI (`dashboard-ai-builder`,
  web json-render), TDASH (`native-tui-dashboards`, shipped `anvil dashboard`)
- Related: ADR-026 (TS scanner retirement), the json-render brainstorm
  (`plans/brainstorms/json-render-dashboard.md`)
- External: [vercel-labs/json-render](https://github.com/vercel-labs/json-render),
  `@json-render/core` v0.19; web-side `@eddacraft/render` (`packages/libs/render/`)
