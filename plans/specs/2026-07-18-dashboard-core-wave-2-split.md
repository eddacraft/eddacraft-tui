# Dashboard Core Wave 2 Data-Truth Split

## Status

Approved by the owner on 2026-07-18.

## Goal

Begin DASH Wave 2 with a useful current-state health summary while preserving
the dashboard's evidence-honesty contract and reserving historical charts for
an authoritative retained-history capability.

## Context

DASHCORE-001 was written before DASH Wave 1 established the durable seam:

```text
Rust dashboard capability
  -> OpenAPI
  -> generated TypeScript client
  -> TanStack Query
  -> dashboard module
```

Its original acceptance mixed current facts with 30-day pass-rate, drift, and
suppression trends. The shipped `ProtectionOverview` capability can prove the
live save-time state and latest persisted gate summary, but the canonical
`GateSnapshot` deliberately contains no retained runs, timestamps, diagnostic
history, affected-file history, drift history, or suppression history.

The old module also cited archived drift-reporting and anti-pattern-library
modules. Current authority is split between Rust-owned dashboard capabilities,
`anvil-kernel-types::GateSnapshot`, and the compiled anti-pattern registry
loaded by `anvil-checks`.

## Decision

### DASHCORE-001: current-state health cards

Render five compact, responsive cards from the existing typed
`ProtectionOverview` resource:

1. save-time protection state;
2. latest gate score and result;
3. active warnings, retaining complete/partial/unavailable semantics;
4. workspace assurance coverage or state;
5. evidence freshness.

The component is a pure presentation adapter. It does not fetch, infer domain
state, read local files, or create a second data contract. Essential values are
visible without hover or colour. Missing facts render explicit unavailable
copy rather than `0`, `100%`, or an invented timestamp.

No sparkline renders in DASHCORE-001 because the shipped server currently
provides no genuine retained series.

### DASHCORE-002: retained history and trends

DASHCORE-002 owns the backend read model and every historical visual. The Rust
dashboard server must source dated series from the owning evidence stores and
publish them through OpenAPI before the browser renders trends.

Sparkline and chart windows follow the owner-approved actual-range rule:

- use every genuine retained point available;
- target at least 30 days;
- include a longer retained range when present rather than truncating it;
- show a shorter available range honestly;
- label the actual covered dates;
- never pad missing days or invent samples.

## Visualisation Contract

- **Analytical job:** monitoring/current-state scan for DASHCORE-001;
  time-change analysis for DASHCORE-002.
- **Primary artefact:** directly labelled metric cards now; compact sparklines
  and detailed trend charts only after retained history exists.
- **Renderer ownership:** React owns the card layout; Recharts remains the
  declared chart primitive for genuine series. No Canvas/WebGL is justified.
- **Interaction:** current cards are readable without interaction. Later
  ranges use explicit controls and URL-backed state where selection affects
  navigation or shareability.
- **Mobile:** cards use one column at narrow widths and preserve the same
  protection -> gate -> warnings -> assurance -> freshness reading order.
- **Accessibility:** text is redundant with colour; unavailable and partial
  states are named; charts require text summaries and keyboard/touch access.
- **Performance:** five cards, one typed resource, no additional request, and
  no chart bundle work in DASHCORE-001.
- **Fallback:** when a fact is absent, retain the card and explain its state;
  do not hide the gap.

## Non-goals

- Browser reads of `.anvil/` or user-scoped evidence files.
- Fixture-only production visuals.
- A new client-side warning, drift, suppression, or pattern authority.
- Gate-history, warning-detail, or suppression-management pages in
  DASHCORE-001.

## Validation

- Component tests cover complete, partial, unavailable, zero, and missing
  current facts.
- The full dashboard test, lint, typecheck, and build targets pass.
- Visual QA covers desktop and mobile sibling layouts against the approved
  Nordic Terminal baseline.
- APS active lint, index checks, docs checks, formatting, and diff checks pass.
