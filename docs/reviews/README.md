# Active Reviews

| Type   | Authority | Owner  | Status | Freshness                                                                  |
| ------ | --------- | ------ | ------ | -------------------------------------------------------------------------- |
| README | Advisory  | DOCGOV | Live   | Last reviewed 2026-08-16 against `docs/guides/documentation-governance.md` |

| Upstream                                  | Downstream            |
| ----------------------------------------- | --------------------- |
| `docs/guides/documentation-governance.md` | Review note discovery |

This directory is for review notes that still have open follow-up work.

Use `docs/reviews/` for:

- active council or adversarial review notes
- review summaries attached to work still in progress
- temporary review tracking that still informs code changes
- CLI command-truth audits (`cli-command-truth-review.md` — living WIP, APS
  CLICT): runtime registry of all 45 command families + per-family drift slices
- shipped product code-review map and session tracker
  (`shipped-codebase-review-checklist.md`): chunked checklist over the pure-Rust
  binary and related surfaces
- GCTX dogfood failure points (2026-08-16) —
  (`2026-08-16-gctx-dogfood-failure-points.md`): measured `not_ready` /
  scan-timeout / graph-base spawn / two-client handshake leftovers; executable
  follow-up is CIB-341..344

Move review documents to `docs/archive/reviews/` once their follow-up work is
merged, superseded, or no longer actionable.
