#!/usr/bin/env node
// Surface stub: generated-index freshness.
//
// This surface validates that hand-edited documentation indexes are
// regenerated from document metadata before each commit. The mechanism that
// generates and signs those indexes ships in DOCGOV-007 (`pnpm docs:index` /
// `pnpm docs:index:check`). Until then this surface is a deliberate no-op so
// the `pnpm docs:check` orchestrator has a stable place to plug it in.

const SURFACE = 'index-freshness';

console.log(
  `[${SURFACE}] pending DOCGOV-007 — generated-index freshness check is not yet implemented`
);
process.exit(0);
