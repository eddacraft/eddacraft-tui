# GV2-032 — Batch Council Review Record

**Date:** 2026-06-24
**Branch:** `feat/gv2-032-symbol-spans`
**Session:** `council-03fe51e3` (batch, standard pack)
**Panel:** council-reviewer (general quality), security-analyst (privacy),
adversarial-reviewer
**Scope:** GV2-032 — `SymbolNode.span` (offsets-only) + per-file FNV-1a
`content_hash` on the resident graph, populated by the TS/Rust/Python extractors
and round-tripped in the snapshot (`SNAPSHOT_BACKING_SCHEMA_VERSION` 1→2).

## Verdict

**PASS — no critical or major findings.** All three reviewers independently
confirmed the load-bearing design is correct:

- The snapshot version bump (1→2) invalidates v1 on-disk snapshots **before**
  any postcard decode, so a daemon cold-rebuilds rather than mis-decoding.
- Removing `skip_serializing_if` from `SnapshotNode.span` is correct (postcard is
  non-self-describing — the field must always encode); keeping it on
  `SymbolNode.span` is safe (SymbolNode is never directly postcard-encoded; the
  GraphDelta IPC path is serde_json).
- No source text can be serialised anywhere: `ByteRange` is offsets-only,
  `content_hash` is a u64 digest, the no-leak guard's top-level allowlist was
  extended with `file_hashes`, and `file_hashes` keys are `check_relative`'d at
  build time.
- FNV-1a constants/op-order are correct; the file-hash lifecycle (stamp on
  update, clear on remove / `None` re-extraction) is consistent.

## Findings & resolutions

| # | Sev | Finding | Resolution |
|---|-----|---------|------------|
| 1 | minor | Spec requires span/hash tests for TS **and** Rust **and** Python; only TS had one | **Fixed** — added `populates_symbol_spans_and_content_hash_gv2_032` to `rust.rs` + `python.rs` |
| 2 | minor | No snapshot round-trip test for a non-`None` span (postcard `Some(ByteRange)` path untested) | **Fixed** — `snapshot_round_trips_non_none_span_gv2_032` |
| 3 | minor | `content_hash` lacked a known-vector test (only proof the constants are right) | **Fixed** — `content_hash_matches_known_fnv1a_vectors` (empty / `a` / `foobar`) + determinism/sensitivity test |
| 4 | nit | Misleading `serde(default)` "forward-compat" doc on `SnapshotNode.span` + `file_hashes` (postcard can't default absent fields) | **Fixed** — doc corrected; version bump named as the real guard |
| 5 | minor | No-leak string-scan never traverses a populated `file_hashes` key (build-time `check_relative` is the real guard) | **Fixed** — `from_graphs_rejects_non_relative_file_hash_key_gv2_032` negative test |
| 6 | info | Tail-language span/hash deferral needs a tracked follow-up before GCTX-021 ships | **Tracked** — added a "Follow-up (tracked)" bullet to the GV2-032 item |
| 7 | info | Release-build zip desync only `debug_assert`-guarded | **Waived** — inherited GCALL-002 pattern; no desync path exists (every push is paired) |
| 8 | info | `set_file_hash` called unconditionally even if all symbol inserts fail | **Waived** — benign; the consumer resolves a symbol before reading the file hash |
| 9 | info | `ByteRange::from_range` saturates at `u32::MAX` for >4 GiB files | **Waived** — parse-size caps (DSV-006) reject such files well below 4 GiB |

## Validation

- `cargo test --workspace` — **7243 passed / 0 failed**
- `cargo clippy --workspace --all-targets -- -D warnings` — clean
- `cargo fmt --all --check` — clean
- APS: `index-counts --check`, `active-lint` — clean

## Deferred (out of scope, documented)

- Tail-language (Dart/Go/Java/Kotlin/C#/C/C++) span + content-hash population
  (the T1 `tail_common` extractors carry no parallel span vec) — see the GV2-032
  follow-up bullet.
- An ADR-031 `call_lift`-style budget bench — the lift is one O(n) zip + a linear
  hash over already-read bytes; CI bench gating tracked as a follow-up.
