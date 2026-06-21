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
- [ ] 092c orphan-`.snap` sweep is empty-guarded but **not wired** to a daemon
      call site (no faithful registered-set source at cold boot). Track wiring
      it to a post-session-attach/periodic reclaim before relying on bounded
      `graph-cache/` growth. (human decision)

## Net-new tracked follow-up (not in this PR)

- [ ] **N7** — suffix-match import resolution in `incremental.rs` can re-bind a
      relative import to a lookalike file in a deeply-nested monorepo
      (resolution-correctness, not a privilege bypass). Filed as a CIB-093
      follow-up; deferred to its own change. (human triage)

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
- Deferred-in-PR: 092d snapshot **write**-side `openat2` anchor (no shipped
  dirfd-create helper; read-side done, write-side tracked under CIB-092d); 092h notification
  delivery (fanout hard-denies session-less envelopes by INTD-015 design — WARN +
  metrics log is the operator signal).
