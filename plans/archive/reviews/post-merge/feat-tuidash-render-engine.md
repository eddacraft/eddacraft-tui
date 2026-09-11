# Post-merge: feat-tuidash-render-engine

PR: #NNN
Branch: `feat-tuidash-render-engine`
APS: TUIDASH
Merged: <!-- filled by cleanup agent -->
Verified: <!-- filled by cleanup agent -->

## Steps

- [ ] Build the release binary and run `anvil dashboard` in a TTY; confirm the
      two-pane picker lists the native dashboards and renders the mini-preview
      pane for any saved spec (human required — interactive TUI).
- [ ] Author a sample spec at `.anvil/dashboards/demo.json` (copy a template
      from `packages/libs/render/specs/`), run `anvil dashboard demo`, and
      confirm it renders through the engine and that `r` refreshes `.anvil/`
      data binding (human required).
- [ ] Confirm the catalogue-parity test runs in CI and fails loudly if the
      vendored `catalog-names.json` drifts from `@eddacraft/render`
      (`cargo test -p eddacraft-tui --features json-render --test catalog_parity`)
      (agent: yes).
- [ ] Confirm the no-panic and escape-sanitisation regression tests run in CI
      (`cargo test -p eddacraft-tui --features json-render`) (agent: yes).

## Notes

This branch completes the TUIDASH json-render TUI engine (TUIDASH-003..-012) on
top of the merged parser/registry foundation (TUIDASH-001/-002). Per ADR-054 the
generic engine + base/chart components live in `eddacraft-tui` behind the
`json-render` feature; the Anvil-domain catalogue, `.anvil/` data context, spec
surface, and `anvil dashboard` CLI wiring live in `anvil-tui`/`anvil-cli`.

Follow-ups (not blocking, tracked here):

- **DASHAI-002 dependency** — automated catalogue-parity diffing against a
  TS-exported JSON Schema is gated on DASHAI-002 (which currently has no schema
  export and lives in the Draft `dashboard-ai-builder` module). Until then parity
  is enforced against the vendored `catalog-names.json` mirror. When DASHAI-002
  lands its schema export, replace the vendored mirror with a generated check.
- **`visible` conditions** — spec `visible` expressions are parsed but not yet
  evaluated by the renderer (every element renders); wiring conditional
  visibility is future work.
- **Chart components** (`LineChart`/`BarChart`/`SparklineChart`) and the Anvil
  domain components are registered ahead of the web `@eddacraft/render`
  catalogue; they surface as `tui_only` extras in the parity check until the web
  catalogue adds them.
