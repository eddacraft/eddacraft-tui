# Active Reviews

| Type   | Authority | Owner  | Status | Freshness                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| ------ | --------- | ------ | ------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| README | Advisory  | DOCGOV | Live   | Last reviewed 2026-09-10 Reviewed against JREL-012 fail-closed journey verification and testing.md command updates. Prior review 2026-09-09 for the documentation-governance cross-link and docs-workflow skill reminder that /dev-loop must settle docs-owed Freshness before CI; no authority, trigger, exemption, or diagram contract changed. Prior review 2026-09-06 for CIB-411 and CIB-412; the diagram-impact collector now retains infra/\*\* and both rename endpoints, and no authority, trigger, exemption, or diagram contract changed. Prior review 2026-09-04 JSDoc for generatorFor now names the unnamed-marker placeholder and the null hand-maintained return Prior review 2026-09-03 for the `docs:redate` change that makes it refuse generated views; no authority, trigger, exemption, or metadata rule moved, so the diagrams stand. Prior review 2026-09-01 for the documentation-governance addition recording that review cascades and naming `pnpm docs:redate`; no authority, trigger, exemption, or metadata rule changed. Prior review 2026-08-31 for the `classify-changes.sh` project-config path class and for documentation-governance freshness after an archive link repoint (the governance rules themselves are unchanged); previously reviewed 2026-08-30 for work-item claim-issue documentation-governance freshness against ADR-123 and `docs/guides/documentation-governance.md` |

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
