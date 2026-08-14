<!-- APS: Design spec for the acknowledgements starter kit Node cold-adopt wave (ATTRIB-027..-034) -->

# Acknowledgements starter kit — Node cold-adopt path

Date: 2026-08-14
Module: `ATTRIB` (adds ATTRIB-027..-034)
Status: Accepted
Coordinates with:
[`plans/modules/acknowledgements-kit-hardening.aps.md`](../modules/acknowledgements-kit-hardening.aps.md),
[`tools/starters/acknowledgements/README.md`](../../tools/starters/acknowledgements/README.md)

## Goal

A stranger following the kit README for a **Node-only** repo can generate,
`--check`, and ship a notice without Rust files, a global `license-checker`,
or a version-pinned self-exclusion.

Operator-approved 2026-08-14 after a live cold-adopt of `v1.2.0` against
`@eddacraft/nxrust`. The generator core held; the documented first-copy path
did not.

## Decisions

1. Template markers are `<!-- BEGIN AUTO-GENERATED {{BLOCK_NAME}} -->`. The
   adopter replaces `{{BLOCK_NAME}}` with the block `name`.
2. Ship `licences.toml.template`. The expander requires only the consumer
   files that exist; `about.toml` and `deny.toml` are optional.
3. Resolve `license-checker` from `node_modules/.bin` walking up from the
   manifest, then `PATH`.
4. Always exclude the manifest's own `name@version`. Keep `exclude` for extras.
5. The freshness snippet is generator `--check` plus commented per-ecosystem
   setup. No Rust fixture.
6. The adoption checklist says to include `ACKNOWLEDGEMENTS.md` in the
   package or release artefact.

## Non-goals

- ATTRIB-025 (`--version`).
- New ecosystem drivers.
- Changing the `attribution.toml` schema or marker syntax.
