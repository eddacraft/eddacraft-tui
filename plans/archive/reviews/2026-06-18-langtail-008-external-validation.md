# LANGTAIL-008 — external-codebase validation evidence

**Date:** 2026-06-18
**Scope:** Discharge the LANGTAIL-008 acceptance bar — "all included languages
parse real-world files without panicking, appear in the graph" — against real
OSS corpora, at the same scale Rust (RSTLAN-008, 571 files) and Python
(PYLAN-009, ~270 files) were held to today.

## Why this run

The merged wave's acceptance (`tests/langtail_wave_acceptance.rs`) used 8
hand-authored fixtures — representative but synthetic. RSTLAN/PYLAN were
validated against real external codebases at scale. This run closes that gap
for the six tail languages. The **FP-rate half** of the Rust/Python bar does
**not** apply: T1 ships no anti-pattern catalogue, suppression, or policy, so
there is nothing to false-positive on. **Parse robustness at scale** is the
load-bearing evidence here.

## Method

Harness: `crates/anvil-kernel/tests/langtail_external_validation.rs` (gated by
`LANGTAIL_CORPUS`, `#[ignore]` so CI is unaffected). For every file under the
corpus whose extension maps to a tail-wave `Language`, it runs the production
parse + symbol-extraction path (`Parser::parse_bytes` + `extract_symbols`)
inside `catch_unwind`, tallying per language: files, **panics**, **error-trees**
(`root.has_error()`), **unreadable** files, **parse_bytes failures**, and
extracted symbols/imports. Run once per repo so signals stay isolated. The
harness counts unreadable + `parse_bytes` failures as their own conservative
"not-cleanly-parsed" categories (never silent skips); both were **0** in this
run, so the per-language error-tree rate equals the not-cleanly-parsed rate.
The panic hook is restored via a `Drop` guard so an early test panic can't
leak a suppressed hook.

## Corpus

| Language | Repo | Files |
| --- | --- | --- |
| Dart | [`flutter/samples`](https://github.com/flutter/samples) | 483 |
| Go | [`gin-gonic/gin`](https://github.com/gin-gonic/gin) | 99 |
| Java | [`google/gson`](https://github.com/google/gson) | 262 |
| Kotlin | [`square/okhttp`](https://github.com/square/okhttp) | 569 |
| C# | [`JamesNK/Newtonsoft.Json`](https://github.com/JamesNK/Newtonsoft.Json) | 945 |
| C | [`redis/redis`](https://github.com/redis/redis) | 783 |
| C++ | [`nlohmann/json`](https://github.com/nlohmann/json) (`.hpp`) | 473 |
| C++ | [`fmtlib/fmt`](https://github.com/fmtlib/fmt) (`.cc` + `.h`) | 72 |

~3,700 tail-language files — same order of magnitude as the Rust/Python runs.

## Results — primary signal (each repo's own language)

| Language | Files | Panics | Error-trees | Err-tree % | Symbols | Verdict |
| --- | ---: | ---: | ---: | ---: | ---: | --- |
| Go | 99 | **0** | 0 | 0.00% | 1,491 | ✅ clean |
| Java | 262 | **0** | 0 | 0.00% | 3,574 | ✅ clean |
| Dart | 483 | **0** | 0 | 0.00% | 8,188 | ✅ clean |
| Kotlin | 569 | **0** | 7 | 1.23% | 6,378 | ✅ clean |
| C# | 945 | **0** | 65 | 6.88% | 8,437 | ⚠️ acceptable |
| C | 783 | **0** | 345 | 44.1% | 15,661 | ⚠️ at-risk (C-005) |
| C++ (`.hpp`) | 473 | **0** | 147 | 31.1% | 2,790 | ⚠️ at-risk (C-005) |
| C++ (`.cc`/`.h`) | 72 | **0** | 41 | 56.9% | 2,474 | ⚠️ at-risk (C-005) |

**Zero panics across every repo and every language** — the hard LANGTAIL-008
safety bar (the parse path is load-bearing for a long-running daemon and must
never abort) is met on ~3,700 real-world files.

## Interpretation

- **Tier A — Go, Java, Dart, Kotlin (≤1.2% error-trees).** Production-quality
  T1 on real codebases. Symbol yield is healthy (Go ~15/file, Java ~14/file,
  Dart ~17/file, Kotlin ~11/file). The 7 Kotlin error-trees are bleeding-edge
  syntax the `-ng` grammar doesn't yet cover; recovery still extracts symbols.

- **Tier B — C# (6.9%).** The 0.23 `tree-sitter-c-sharp` grammar trips on some
  newer C# (the error-trees cluster in the test suite's modern syntax), but
  yields ~9 symbols/file and never panics. Acceptable for T1.

- **Tier C — C / C++ (31–57%).** This is the real-world evidence behind the
  council **C-005 at-risk** flag. The error-trees are **not** in vendored
  `deps/` — 48 of 50 sampled redis error-files are redis's own `src/`
  (`server.c`, `sds.c`, `util.c`, …). The cause is inherent to *static* parsing:
  `tree-sitter-c`/`-cpp` do **not** expand the C preprocessor, so macro-heavy
  systems C and bleeding-edge C++ (templates/concepts) produce error nodes.
  Crucially, error-recovery still extracts symbols at scale (C ~20/file, C++
  ~6/file) and **never panics** — so C/C++ remain *usable* at T1 (symbols flow
  into the graph) but at **degraded parse fidelity**. The documented `.h`→C
  mapping is visible in fmt: 23 of its 25 C++ `.h` headers error when parsed by
  the C grammar — expected, not a regression.

## Verdict

- **LANGTAIL-008 robustness bar: PASS** — 0 panics, ~3,700 real files, 7
  languages. Every language appears in the symbol graph with real symbol yield.
- **Wave membership unchanged** — C/C++ stay **in** at T1 (no panics, symbols
  extracted), with the C-005 at-risk flag now backed by data: a future C/C++
  **T2** promotion must address preprocessor-aware parsing before the catalogue
  work, or accept the partial-parse fidelity ceiling. Go/Java/Dart/Kotlin are
  the strongest T2 candidates on this evidence.

## Reproduce

```sh
# clone the corpus (shallow) into /tmp/langtail-corpus/<lang>, then:
for d in go dart java kotlin csharp c cpp cpp_json; do
  LANGTAIL_CORPUS=/tmp/langtail-corpus/$d \
    cargo test -p eddacraft-anvil-kernel --test langtail_external_validation \
    -- --ignored --nocapture
done
```
