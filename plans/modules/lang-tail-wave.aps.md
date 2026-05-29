<!-- APS: See https://github.com/eddacraft/anvil-plan-spec for format reference -->
<!-- Executable only if tasks exist and status is Ready. -->

# Tail Language T1 Wave (Track 2)

| ID       | Owner | Status |
| -------- | ----- | ------ |
| LANGTAIL | —     | Draft  |

**Last reviewed:** 2026-04-26

## Purpose

Bring a batched set of tail languages to **T1 (Parsed)** in a single sprint
per [2026-04-08 Language and Coverage Design](../specs/2026-04-08-language-and-coverage-design.md)
§5.1, §8.2. T1 means: tree-sitter grammar wired, file detected, basic symbol
extraction (functions, classes, imports), file appears in the symbol graph.
**No** per-language anti-pattern catalogues, suppression syntax, or policy
hooks — that is T2/T3 anchor work, not tail-wave work.

The wave amortises per-language cost (test harness, fixtures, "does the graph
include this" validation) instead of paying it once per language. Doing the
tail piecemeal later costs more in aggregate.

This module **merges** the previous per-language placeholders for Dart, Go,
Java, Kotlin, .NET/C#, and C/C++ (now archived).

## In Scope

- Wire the following tree-sitter grammars through the extractor abstraction
  produced by `lang-ts-audit`:

  | Language | Grammar | Demand | Pack potential | Promotion lever |
  |----------|---------|--------|----------------|-----------------|
  | Dart | `tree-sitter-dart` | 1 (User B mobile) | Flutter | Second Dart user or Flutter pack demand |
  | Go | `tree-sitter-go` | 0 | Cobra, `net/http` | First Go user |
  | Java | `tree-sitter-java` | 0 | Spring | First Java user |
  | C# / .NET | `tree-sitter-c-sharp` | 0 | ASP.NET | First .NET user |
  | Kotlin | `tree-sitter-kotlin` | 0 | Ktor, Android | First Kotlin user |
  | C / C++ | `tree-sitter-c`, `tree-sitter-cpp` | 0 | — | First systems-code user |

- File detection per language extension(s).
- Basic symbol extraction (top-level functions, classes/types, imports) — no
  language-deep semantics.
- Inclusion in `architecture-validate` symbol graph.
- One fixture test per language.
- Grammar maturity audit per council finding C-005 — confirm crates.io
  availability + ABI stability before adding each grammar.

## Out of Scope

- T2 anti-pattern catalogues per language (anchor work, not tail wave).
- Suppression syntax per language.
- Policy hooks per language.
- Layer/boundary enforcement per language.
- Drift baseline per language.
- Anything that would promote a tail language to T2 — promotion is a
  separate module decision triggered by demand signal.

## Interfaces

**Depends on:**

- [`lang-ts-audit`](./lang-ts-audit.aps.md) — extractor abstraction;
  grammar version in cache key (council §16.5 #3 / C-004).
- Existing kernel parser, symbol graph.

**Exposes:**

- Six (or fewer) tail languages parsed and graph-included.
- Per-grammar maturity findings — informs whether any tail language is a
  promotion candidate for a future T2 module.

## Prerequisites

- `lang-ts-audit` complete.
- Grammar maturity audit complete for each candidate before that grammar
  ships in the wave.

## Ready Checklist

Change status to **Ready** when:

- [ ] LANGTS complete.
- [ ] Grammar maturity audit complete; final wave membership confirmed.
- [ ] At least one fixture file identified per included language.
- [ ] Owner named.

## Work Items

Tasks will be defined when this module moves to Ready. Anticipated shape:

- LANGTAIL-001: Grammar maturity audit; finalise wave membership.
- LANGTAIL-002: Wire `tree-sitter-dart`; basic symbol extraction; fixture.
- LANGTAIL-003: Wire `tree-sitter-go`; basic symbol extraction; fixture.
- LANGTAIL-004: Wire `tree-sitter-java`; basic symbol extraction; fixture.
- LANGTAIL-005: Wire `tree-sitter-kotlin`; basic symbol extraction; fixture.
- LANGTAIL-006: Wire `tree-sitter-c-sharp`; basic symbol extraction; fixture.
- LANGTAIL-007: Wire `tree-sitter-c` and `tree-sitter-cpp`; basic symbol
  extraction; fixture. **At-risk** — drop from wave if grammar quality
  blocks the batch (spec §12.3, council C-005).
- LANGTAIL-008: Wave-level acceptance: all included languages parse
  real-world files without panicking, appear in the graph, all fixtures
  green.

## Risks

| Risk | Impact | Mitigation |
| ---- | ------ | ---------- |
| `tree-sitter-cpp` C++20/23 partial-parse issues stall the batch (council C-005) | High | Hard rule: drop C/C++ from the wave rather than let it stall; document as cut-from-wave |
| `tree-sitter-dart` ABI publication gaps (council C-005) | Medium | Audit before include; pin to last-known-good crate version |
| `tree-sitter-kotlin` community-maintained regressions (council C-005) | Medium | Pin version; fixture must include known-regression cases |
| Binary size and LTO cost from 6 grammars (council C-005) | Medium | Measure before/after; gate on a budget agreed during Ready |
| Parser thread-safety with N workers × M grammars (council C-026) | High | Inherit thread-safety strategy from LANGTS prereq work; do not invent here |
| Half-batch ships and the wave never closes | Medium | Wave-level acceptance is all-or-some; if some are dropped, the dropped languages re-enter only on demand signal |

## Open Questions

- [ ] Final wave membership after grammar maturity audit — likely 5 of 6.
- [ ] Binary size / LTO budget agreed for Ready promotion.
- [ ] Where does the per-language fixture corpus live in the repo?
