#!/usr/bin/env node
// Surface stub: as-built source path existence.
//
// This surface validates that every source-path reference inside an as-built
// document resolves to a file that exists at the cited tag/SHA. The freshness
// template that defines those references lands in DOCGOV-006. Until then this
// surface is a deliberate no-op so the `pnpm docs:check` orchestrator has a
// stable place to plug it in.

const SURFACE = 'asbuilt-paths';

console.log(
  `[${SURFACE}] pending DOCGOV-006 — as-built source path existence check is not yet implemented`
);
process.exit(0);
