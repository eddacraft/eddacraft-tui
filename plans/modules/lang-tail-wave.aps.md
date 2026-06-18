<!-- APS: See https://github.com/eddacraft/anvil-plan-spec for format reference -->
<!-- Executable only if tasks exist and status is Ready. -->

# Tail Language T1 Wave (Track 2)

| ID       | Owner       | Status      |
| -------- | ----------- | ----------- |
| LANGTAIL | joshuaboys  | In Progress |

**Last reviewed:** 2026-06-18

## Grammar Maturity Audit (LANGTAIL-001)

Audited 2026-06-18 against the workspace `tree-sitter` 0.26 runtime. Every
candidate grammar crate resolves on crates.io and **binds + parses a
representative multi-line snippet without an error tree** (pinned as a
permanent regression guard in `parser::languages::tests::tail_wave_grammars_bind_and_parse`):

| Language | Grammar crate | Version | ABI bind | Verdict |
|----------|---------------|---------|----------|---------|
| Dart | `tree-sitter-dart` | 0.2.0 | ✓ | Include |
| Go | `tree-sitter-go` | 0.25.0 | ✓ | Include |
| Java | `tree-sitter-java` | 0.23.5 | ✓ | Include |
| Kotlin | `tree-sitter-kotlin-ng` | 1.1.0 | ✓ | Include (maintained fork of `tree-sitter-kotlin`) |
| C# / .NET | `tree-sitter-c-sharp` | 0.23.5 | ✓ | Include |
| C | `tree-sitter-c` | 0.24.2 | ✓ | Include |
| C++ | `tree-sitter-cpp` | 0.23.4 | ✓ | Include |

**Final wave membership: all 6 languages (7 grammars).** The spec anticipated
"likely 5 of 6"; empirically all bind and parse, so none were cut. The C/C++
at-risk flag (council C-005, spec §12.3) is retained as a *future* concern —
the deep C++20/23 partial-parse risk only surfaces on advanced syntax beyond
T1; the basic-symbol fixtures parse cleanly. The Kotlin community-maintenance
risk is mitigated by using the maintained `-ng` fork and by realistic
multi-line fixtures (single-line bodies trip the newline-sensitive grammar —
the audit's one transient ERROR was a snippet artifact, not a grammar defect).

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

- [`lang-ts-audit`](../archive/modules/lang-ts-audit.aps.md) — extractor abstraction;
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

Promoted to **Ready** (then **In Progress**) 2026-06-18 — all met:

- [x] LANGTS complete (`lang-ts-audit` Complete, v0.7.3-beta).
- [x] Grammar maturity audit complete; final wave membership confirmed (all 6).
- [x] At least one fixture file identified per included language
      (`crates/anvil-kernel/tests/fixtures/langtail/`).
- [x] Owner named.

## Work Items

Delivered as a single wave (one branch/PR) per the module's amortisation
intent — the languages share `Language`/dispatch hot files, so a batch lands
cleaner than per-language PRs. All eight items **Merged 2026-06-18 via PR
#2757**; the module stays In Progress until a release tag ships them
(Released/Shipped → Complete), per the APS lifecycle.

- **LANGTAIL-001** — Grammar maturity audit; finalise wave membership.
  **Merged 2026-06-18 via PR #2757** (see audit table above).
- **LANGTAIL-002** — Wire `tree-sitter-dart`; symbol/import extraction
  (`parser/extract/dart.rs`); fixture. **Merged 2026-06-18 via PR #2757**.
- **LANGTAIL-003** — Wire `tree-sitter-go` (`parser/extract/go.rs`); fixture.
  **Merged 2026-06-18 via PR #2757**.
- **LANGTAIL-004** — Wire `tree-sitter-java` (`parser/extract/java.rs`);
  fixture. **Merged 2026-06-18 via PR #2757**.
- **LANGTAIL-005** — Wire `tree-sitter-kotlin-ng` (`parser/extract/kotlin.rs`);
  fixture. **Merged 2026-06-18 via PR #2757**.
- **LANGTAIL-006** — Wire `tree-sitter-c-sharp` (`parser/extract/csharp.rs`);
  fixture. **Merged 2026-06-18 via PR #2757**.
- **LANGTAIL-007** — Wire `tree-sitter-c` + `tree-sitter-cpp`
  (`parser/extract/clike.rs`); fixture. **Merged 2026-06-18 via PR #2757** —
  not cut; both bind and parse the T1 fixtures (at-risk flag retained for
  future deep-C++ work).
- **LANGTAIL-008** — Wave-level acceptance: every included language parses a
  real-world fixture without panicking, symbols appear in the kernel symbol
  graph, all fixtures green (`tests/langtail_wave_acceptance.rs`).
  **Merged 2026-06-18 via PR #2757**. External-corpus robustness validation
  (the RSTLAN-008/PYLAN-009 equivalent) run 2026-06-18 over ~3,700 real OSS
  files across all 6 languages — **0 panics**; Go/Java/Dart/Kotlin parse clean
  (≤1.2% error-trees), C# 6.9%, C/C++ 31–57% (validates the C-005 at-risk flag:
  un-preprocessed macro/template syntax → partial parses, recovery still
  extracts symbols). Evidence:
  `plans/reviews/2026-06-18-langtail-008-external-validation.md`.

Supporting change (outside the numbered items, required for graph inclusion):
the kernel parseable-extension gate (`FileFilter::is_parseable`) and the
daemon warm-up + architecture-validate discovery lists now delegate to
`Language::from_path`, so every supported language — closing the latent
Rust/Python omission as well — reaches the symbol graph, not just JS/TS.

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

- [x] Final wave membership after grammar maturity audit — **all 6** included
      (audit found every grammar binds + parses; none cut).
- [x] Where does the per-language fixture corpus live in the repo? —
      `crates/anvil-kernel/tests/fixtures/langtail/`, exercised by
      `tests/langtail_wave_acceptance.rs`.
- [ ] Binary size / LTO budget — measured cost is 7 small grammar crates (all
      MIT, no new transitive runtime deps; ACKNOWLEDGEMENTS +7). Formal budget
      sign-off deferred to release prep; not a blocker for T1 landing.
