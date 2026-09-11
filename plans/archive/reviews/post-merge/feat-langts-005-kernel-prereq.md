# Post-merge test plan — LANGTS-005 (feat/langts-005-kernel-prereq)

PR: https://github.com/eddacraft/anvil-001/pull/2096

Kernel-prerequisite refactor (K1–K4). Unit/integration coverage lands with the
PR; the items below are post-merge verifications that the refactor holds under
conditions the in-PR tests cannot fully exercise (real grammar bumps, real
multi-language scans, daemon longevity).

## K1 — extractor trait / dispatch

- [ ] Run a real multi-file `anvil gate` / embedded scan over a mixed TS + JS
      project and confirm the symbol graph and import edges are unchanged
      vs the pre-merge binary (no symbol count or visibility drift).
- [ ] Confirm `extract::extract_symbols` callers (`watch.rs`, `embedded.rs`,
      `gate.rs`, benches) still link without source edits (compile-time check;
      already green in CI).

## K2 — grammar-versioned cache

- [ ] On the next `tree-sitter-typescript` / `tree-sitter-javascript` bump,
      verify that a warm-cache reparse of unchanged files reports `cached:
      false` (forced re-parse) rather than serving the old-grammar tree.
      Spot-check by diffing the symbol graph across the bump on a fixed corpus.
- [ ] Confirm `Language::grammar_version()` changes across the bump (log it
      once at scan start during the upgrade PR).

## K3 — thread-safety

- [ ] Run a large concurrent scan (rayon, many workers) under
      `cargo test`-style sanitizer if available, or at minimum a sustained
      `anvil watch` session over a churning workspace, and confirm no parser
      panics / crashes (`Parser` confined per worker).

## K4 — non-panicking parse path (daemon mode)

- [ ] In daemon/`watch` mode, confirm that a forced grammar-load failure
      (e.g. an intentionally mismatched grammar build in a throwaway branch)
      surfaces a `ParseError::LanguageInit` and the daemon keeps running rather
      than aborting the process.
