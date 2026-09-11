# Post-merge: feat-tuidash-001-json-render-spec-parser

PR: #2068
Branch: `feat/tuidash-001-json-render-spec-parser`
APS: TUIDASH-001
Merged: <!-- filled by cleanup agent -->
Verified: <!-- filled by cleanup agent -->

## Steps

- [ ] No runtime post-merge verification required — the engine is behind a
      default-off `json-render` feature with no caller yet, fully covered by
      `cargo test -p eddacraft-tui --features json-render` (agent: yes).
- [ ] Advance TUIDASH-001 `Merged → Released/Shipped` only when an
      `eddacraft-tui` crates.io release that contains the `json_render` module
      ships (agent: yes, on release evidence).

## Notes

Deferred Council items (`council-baf95d3c`) to revisit in later TUIDASH work or
before an `eddacraft-tui` 1.0 — not blockers for this PR:

- **C-003 (waived):** `PropValue`/`Props` are intentional `serde_json` aliases
  (TUIDASH-002 renderer consumes `serde_json::Value`). Re-evaluate a newtype
  seam only if the public API is frozen at 1.0.
- **C-004 (deferred):** consider a `ValidationErrors` newtype impl'ing `Error`
  for `?`-composition once a real consumer (TUIDASH-003 renderer / `anvil
  dashboard`) needs it.
- **Fixture drift (TUIDASH-010):** `tests/fixtures/json_render/*.dashboard.json`
  are verbatim copies of `packages/libs/render/specs/`; catalogue/spec parity
  to source is TUIDASH-010's scope, not gated here.
