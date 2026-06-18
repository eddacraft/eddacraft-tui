# json-render fixture specs

These three files are **verbatim copies** of the `@eddacraft/render` template
specs (primarily dashboard/monitoring surfaces) at
`packages/libs/render/specs/`:

- `gate-summary.dashboard.json`
- `watch-session.dashboard.json`
- `architecture-health.dashboard.json`

They are vendored here so `eddacraft-tui` stays self-contained when mirrored out
of the monorepo (ADR-047) and its tests do not reach across package boundaries.

They exercise the json-render parser (`tests/json_render_specs.rs`) for
round-trip fidelity and base-catalogue validation, and the engine renderer.
Refresh by re-copying from `packages/libs/render/specs/` when the templates
change.

(Note: while the current fixtures are dashboard-oriented, the json-render engine
and format are general-purpose and not limited to dashboards.)

## `catalog-names.json`

A verbatim mirror of the component type names registered by `@eddacraft/render`
(`packages/libs/render/src/catalog-registry.ts`). It is the web source of truth
for the catalogue-parity test (`tests/catalog_parity.rs`, **TUIDASH-010**),
which fails if the in-crate `Catalog::base()` mirror drifts from it or if the
component registry stops mapping any name it lists. Refresh it when the web
catalogue changes. Fully automated diffing against a TS-exported JSON Schema is
gated on DASHAI-002.
