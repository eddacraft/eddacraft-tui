# Post-merge: fix-v090-council-survivors

PR: #2852
Branch: `fix/v090-council-survivors`
APS: CIB-091..095 (continuous-improvement-backlog)
Merged: <!-- filled by cleanup agent -->
Verified: <!-- filled by cleanup agent -->

## Steps

- [ ] Full `Cross` matrix incl. Windows/macOS — `gh workflow run rust.yml --ref main`
      after merge; confirm all legs green. CIB-091/093 touched the trust pass,
      the parser extractor, and `openat2`/`O_PATH` Linux-gated snapshot I/O
      (092d) — cross-platform risk. (agent: yes — dispatch + poll)
- [ ] `release-readiness.yml` green on the merge SHA. (agent: yes)
- [ ] `ACKNOWLEDGEMENTS` freshness — no new Rust/transitive deps were added
      (092e *removed* `sha2`); confirm the package-list drift gate is green and
      regen only if it flags. (agent: yes)
- [ ] 092b soak readout — confirm the cumulative `SnapshotMetrics` shutdown
      `info!` line (`anvil_intercept::snapshot`) is scrapeable in a real daemon
      run with `ANVIL_PERSIST_GRAPH=1`, so the §7b "zero `SnapshotLoadError::Corrupt`"
      graduation criterion can actually be evaluated before any default-on flip.
      (human required — needs a soak environment)
_(092c is no longer a standalone step here — it is tracked as CIB-096 in the
section below, to avoid a duplicate checkbox that could drift out of sync.)_

## Deferred sub-parts — follow-up status

CIB trackers for the sub-parts deferred at the #2852 merge, plus the one net-new
item that has since landed (N7 is **not** a CIB item, listed here only to close
it out):

- [x] **CIB-096** — orphan-`.snap` sweep wired via companion root-file + existence
      check (092c) — **DONE** (Merged 2026-06-22 via PR #2870).
- [x] **CIB-097** — anchor the snapshot **write** path to a validated
      `O_DIRECTORY` dirfd (092d) — **DONE** (Merged 2026-06-22 via PR #2865;
      `fstat`-validated, real fsync-able fd).
- [ ] **CIB-098** — deliver the persist-failure degradation signal to opted-in
      operators (092h; fanout currently hard-denies session-less envelopes).
- [x] **N7** _(not a CIB item)_ — suffix-match import re-bind in `incremental.rs`
      — **DONE** in PR #2852 (exact-match-only resolution).

## Notes

- This branch fixes the v0.9.0-beta release-council survivors (CIB-091..095) plus
  net-new items N1–N6, N8 from the `anvil:2.1` cross-ref
  (`plans/audits/2026-06-21-v090-netnew-crossref.md`). Full per-item status:
  `plans/audits/2026-06-21-v090-council-survivors.md`.
- **CE-3 (091a)** is the v0.9.0-beta cut-blocker and is closed + council-verified.
- **095b** closed a real privilege-certify hole (restore→reconcile window).
- Local gates green at PR time: `cargo fmt --all --check`,
  `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`
  (7195 passed / 0 failed), and a final convergent council.
- Deferred-in-PR (each now a standalone tracker): 092d snapshot **write**-side
  `openat2` anchor (read-side done) → **CIB-097**; 092c sweep daemon-wiring →
  **CIB-096**; 092h notification delivery (fanout hard-denies session-less
  envelopes by INTD-015 design — WARN + metrics log is the interim signal) →
  **CIB-098**.
