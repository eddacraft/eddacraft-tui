# LTW2-004 — Tail Wave 2 external-corpus validation

**Date:** 2026-06-30
**Item:** LTW2-004 (wave acceptance — external real-OSS-corpus smoke), the
LANGTAIL-008 equivalent for tail wave 2 (Zig + WebAssembly text, ADR-093).

## Goal

Prove the wave-2 grammars + extractors survive real-world source: parse a large
external corpus through the **production kernel path**
(`Parser::parse_bytes` → `extract_symbols`) and confirm the load-bearing
invariant — **zero panics** — on the save-time-critical parser. Error-tree rate
is reported for context (parse-recovery quality), not as a gate, exactly as
LANGTAIL-008 treated the C/C++ at-risk flag.

## Corpus

Real OSS source, cloned `--depth 1`:

- **Zig:** [`zigtools/zls`](https://github.com/zigtools/zls) + the
  [`ziglang/zig`](https://github.com/ziglang/zig) standard library (`lib/`).
- **WebAssembly text:** pure `.wat` from
  [`bytecodealliance/wasm-tools`](https://github.com/bytecodealliance/wasm-tools)
  tests, and `.wast` script files from the
  [WebAssembly spec testsuite](https://github.com/WebAssembly/testsuite) +
  wasm-tools.

Harness: a throwaway `examples/ltw2_004_smoke.rs` (not committed) walking the
corpus, parsing each file via the kernel and running the extractor, with every
file wrapped in `catch_unwind`.

## Results

| ext     | files | panics | parse_err | error-trees   | symbols |
| ------- | ----- | ------ | --------- | ------------- | ------- |
| `.zig`  | 1082  | **0**  | 0         | 73 (6.7%)     | 15 995  |
| `.wat`  | 554   | **0**  | 0         | 298 (53.8%)   | 1 323   |
| `.wast` | 888   | **0**  | 0         | 562 (63.3%)   | 8 923   |
| `.zon`  | 3     | **0**  | 0         | 3 (100%)      | 0       |

**TOTAL PANICS: 0** over ~2 527 files. ✅ The acceptance invariant holds.

## Interpretation

- **Zig — clean.** 6.7% error-trees over the zls + zig-stdlib corpus, ~16k
  symbols extracted. Comparable to the cleaner LANGTAIL-008 languages
  (Go/Java/Dart). The extractor handles real stdlib breadth without panicking.
- **WAT / WAST — high error-trees, expected, 0 panics.** The vendored grammar is
  the 2022-era ABI-13 `wat/` (module) grammar. The corpus exercises far beyond
  it: `.wast` carries script-only syntax (`assert_return`, quoted modules) the
  module grammar does not model; the wasm-tools `.wat` set includes
  newer-proposal syntax (GC, SIMD, EH, threads) and **deliberately-invalid**
  error-path fixtures. So a majority error-tree rate is the honest, expected
  outcome — and recovery still extracted >10k symbols across both. This mirrors
  LANGTAIL-008's C/C++ result (31–57% error-trees, 0 panics): partial parses,
  not crashes. T1 is "parsed without panicking + symbols in the graph", which
  holds.

## Finding (follow-up, not a blocker)

- **`.zon` should not map to the Zig source grammar.** Zig Object Notation
  (`build.zig.zon`) is a data-manifest format (a bare anonymous-struct literal),
  not a Zig *source file*, so it 100%-error-trees and yields **0 symbols** under
  the `source_file` grammar. Mapping `.zon → Language::Zig` (LTW2-003) therefore
  adds no value and only pollutes the error-tree signal. Recommend dropping
  `.zon` from `Language::from_path` (a 1-line change + the `detects_zig` test
  assertion), or filing it as a small CIB item. No panic risk — purely a
  no-value mapping. **Owner call: fix inline later or file CIB.**

## Verdict

LTW2-004 **passes**: zero panics across the wave-2 corpus; symbols populate the
graph; error-tree rates are explained and acceptable for T1. The `.zon` mapping
is the one cleanup the run surfaced.
