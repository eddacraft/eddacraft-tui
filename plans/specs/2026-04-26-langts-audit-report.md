# LANGTS Audit Report — TypeScript at T3 (Anchor Item Zero)

**Date:** 2026-04-26
**Author:** LANGTS audit (anchor item zero)
**Spec context:** [2026-04-08 Language and Coverage Design](./2026-04-08-language-and-coverage-design.md) §7.3, §8.1, §16.5
**Companion artefact:** [T3 Acceptance Checklist](./2026-04-26-t3-acceptance-checklist.md)
**Module:** [lang-ts-audit](../archive/modules/lang-ts-audit.aps.md)

> Honest gap surface, not a rubber stamp. Where the kernel is not currently at
> T3 for TypeScript, this report names it. The companion T3 acceptance
> checklist turns those gaps into the bar Rust (RSTLAN) and Python (PYLAN)
> must hit before they can credibly be called T3 anchors.

---

## 1. Summary

TypeScript is the language Anvil already partially supports through the
Rust kernel parser at `crates/anvil-kernel/src/parser/`. The parser wires
`tree-sitter-typescript` and `tree-sitter-javascript`, extracts a useful
subset of the symbol/import surface, and feeds the architecture validator at
`crates/anvil-architecture/`. By the spec §7.3 capability table TS reads as
"very close to T3" — and that framing is broadly correct.

It is also misleading on its own. The capability table answers "does
TypeScript support exist?" but does not answer the kernel-prereq question
the council raised in §16.5 #3 / C-003..C-005, C-026, C-027: *is the
substrate that supports TypeScript shaped well enough to carry a second and
third anchor language at T3?* The audit's answer is **no, not yet** — five
named kernel-prereq gaps must close before "T3" is a verifiable claim
rather than a TypeScript-shaped happy path.

The audit therefore produces three outputs as required by spec §7.3:

1. **Definitive statement of TS's current tier (§3 below):** TypeScript is
   functionally at **T3-minus-extraction-completeness**. Grammar, cache,
   suppression, antipattern catalogue, layer enforcement and drift baseline
   all reach `.ts` / `.tsx` / `.js` / `.jsx` / `.mjs` / `.cjs` files. Symbol
   extraction has named gaps (§3.2). Zod-creep rules are absent from the
   antipattern catalogue (§3.4).
2. **List of TS gaps to close (§4):** seven items, all bounded.
3. **T3 acceptance checklist:** a separate artefact at
   [`2026-04-26-t3-acceptance-checklist.md`](./2026-04-26-t3-acceptance-checklist.md).
   Cross-linked from §6 of this report.

The audit's value is in §5 — the kernel-prereq gaps. Five items (C-003,
C-004, C-005, C-026, C-027) currently named in the spec but not in any
APS task. Each is described with current state + recommended fix +
estimated complexity.

---

## 2. Scope and Method

**In scope.** The kernel parser and adjacent crates that carry T3 substrate
weight for TypeScript:

- `crates/anvil-kernel/src/parser/` — tree-sitter wrappers, AST cache,
  symbol/import extractor.
- `crates/anvil-kernel/src/graph/` — symbol graph, dependency graph,
  incremental updates, trust annotation.
- `crates/anvil-architecture/` — boundary definitions, layer assignment,
  baseline, validator.
- `crates/anvil-checks/src/antipattern/` — antipattern catalogue,
  registry, suppression parser.
- The TypeScript antipattern registry at `patterns/compiled/registry.json`
  (rules `AP-001`..`AP-007`).

**Out of scope.** Anything not currently load-bearing for TS at T3 — Rust
extraction, Python extraction, surface modules, packs, OPA policy
integration beyond confirming reachability, drift schema migration
(OPSUP). Any solution proposal beyond *naming current state + recommended
fix*; design decisions live in ADRs and downstream modules per spec
§17.2.

**Method.** Code-first. Each capability is checked against actual source
in the repo, not aspirational definitions. Where the spec hedges
("Confirm it reaches TS specifically"), the audit either confirms with a
file-line citation or names the gap.

---

## 3. Current TS Implementation State

### 3.1 Parser layer

`crates/anvil-kernel/src/parser/` — five files, ~530 LoC excluding tests.

| File | Purpose | Notes |
| --- | --- | --- |
| `mod.rs` | `Parser` struct + `parse_bytes` / `parse_file` API | Caches per-language `tree_sitter::Parser` instances in a `HashMap<Language, tree_sitter::Parser>`. Single-threaded by construction (`tree_sitter::Parser` is not `Send`). |
| `languages.rs` | `Language` enum with five variants: `TypeScript`, `Tsx`, `JavaScript`, `Jsx`. | Extension matcher covers `.ts`, `.tsx`, `.js`, `.jsx`, `.mjs`, `.cjs`. `ts_language()` returns the tree-sitter `LANGUAGE` constant. |
| `cache.rs` | `AstCache` keyed on `(PathBuf, content_hash)` with FNV-1a content hash. | **Grammar version is not part of the cache key (gap, §5.2).** |
| `extract.rs` | Symbol/import extractor. Handles `function_declaration`, `class_declaration`, `export_statement` (named + default + clauses + re-exports), `import_statement`, `call_expression` for `require(...)`, CJS `module.exports.*`, lexical arrow functions. | Handles fewer kinds than the design implies — see §3.2. |
| `queries/typescript.scm`, `queries/javascript.scm` | Tree-sitter S-expression queries. | Used elsewhere; not directly exercised by `extract.rs` today. |

The implementation works. Tests pass (`cargo test -p eddacraft-anvil-kernel
--lib parser` covers the named-export / default-export / CJS /
lexical-require shapes with positive + regression cases).

### 3.2 Extraction completeness

The extractor currently emits symbols of kind `Function`, `Class`,
`Export`. It does **not** currently extract:

- `interface_declaration` (TypeScript interfaces).
- `type_alias_declaration` (TypeScript `type X = ...`).
- `enum_declaration` (TS enums).
- `method_definition` (class methods are not surfaced as separate
  symbols).
- `dynamic_import` (`import('...')` expressions).
- Namespace exports (`export * as ns from './m'`).
- Re-export-only exports without local declaration (handled when there is
  a `source` field, but the symbol kind is `Export` with name `*` rather
  than per-specifier names).

These are not necessarily T3 blockers — the pack rules that currently
matter (Pulumi `acl: "public-read"`, Drizzle `.delete()` without
`.where()`, Hono `c.req.parseBody()` without size limit) operate on
import edges + call expressions, not on type aliases. But the T3
acceptance checklist must call out which of these *are* required for a
language to be considered T3, otherwise Rust and Python end up using the
TS extractor's accidental shape as the bar.

### 3.3 Symbol graph

`crates/anvil-kernel/src/graph/` — four files.

- `symbol_graph.rs` — `SymbolGraph` over `petgraph::DiGraph<SymbolNode,
  SymbolEdge>`. Symbol IDs are `u64`, indexed via `HashMap<u64,
  NodeIndex>`. Per-file index for `symbols_in_file()` and `remove_file()`
  (handles petgraph's swap-remove correctly with descending-index
  removal — regression-tested).
- `incremental.rs` — `update_file`, `remove_file`, `re_resolve_imports`,
  `GraphDelta`. Operates on `FileSymbols` produced by the extractor.
- `dependency.rs` — file-level dependency graph derived from import edges.
- `trust.rs` — annotates `TrustLevel` on symbol nodes from external
  signal (e.g. node_modules path).

The graph is generic over `SymbolNode` / `SymbolEdge` defined in
`anvil-kernel-types`, so adding new symbol kinds is data-driven, not
schema-changing.

### 3.4 Antipattern catalogue (T2 layer)

`crates/anvil-checks/src/antipattern/` plus
`patterns/compiled/registry.json`. Seven rules ship today:

| ID | Coverage |
| --- | --- |
| AP-001 | `eslint-disable` block-form / line-form |
| AP-002 | `@ts-ignore` / `@ts-expect-error` (deferred-debt family) |
| AP-003 | TypeScript `: any` / `as any` / `<any>` |
| AP-004 | Bare `catch {}` |
| AP-005 | `console.log` in production paths |
| AP-006 | TODO/FIXME comment with no ticket |
| AP-007 | Hardcoded credential-shaped strings |

`scanner.rs` runs these via rayon over file paths matched against
`LEGACY_JS_TS_EXTENSIONS = [".ts", ".tsx", ".js", ".jsx", ".mjs",
".cjs"]` (line 12). Suppression is parsed via `parse_suppression` (line
233) supporting `//`, `/*`, `#`, `<!--`, `--` comment styles per
ADR-029.

The Zod-creep rules named in the LANGTS module — `z.any()`,
`z.unknown()`, `.passthrough()` — are **absent** from the registry.
They are mentioned in the existing AP-003 *suggestion* prose ("Use a
runtime validator (Zod, io-ts, ArkType) at the boundary to narrow
`unknown` to your interface") but no rule fires on the anti-pattern
shapes themselves. This is the explicit LANGTS-004 gap.

### 3.5 Suppression syntax

`parse_suppression` in `crates/anvil-checks/src/antipattern/scanner.rs:233`
honours all five comment styles per ADR-029. A second suppression parser
exists at `packages/anvil/core/src/suppression/parser.ts` (TS scanner
side); ADR-029 declares the Rust parser authoritative for new comment
styles. **No gap.**

### 3.6 Architecture / layer enforcement

`crates/anvil-architecture/src/validator.rs`:

- `include_extensions = ["ts", "tsx", "js", "jsx", "mjs", "cjs", "rs"]`
  (line 347) — TS plus Rust, hardcoded.
- ESM `.js` → `.ts` import resolution at line 238.
- Layer assignment + baseline persisted under `.anvil/` per ADR-027.

For TypeScript this is comprehensive. The fact that `rs` is in the
extension list but Rust extraction is not yet wired (see C-003) is a
RSTLAN concern, not LANGTS.

### 3.7 Drift baseline + `architecture-validate` inclusion

Drift baseline is written under `.anvil/` and read by
`crates/anvil-architecture/src/baseline.rs`. The TS extension set is
default-included via `validator::collect_source_files()`. **No
TS-specific gap** — drift schema versioning is an OPSUP concern (council
C-009).

### 3.8 Policy hook integration

OPA integration is reachable from the antipattern pipeline via the
shared `Warning` schema; warnings carry `pattern`, `family`, `severity`,
`confidence`, `definition_ref`, `spectrum_position` fields that the
policy layer consumes. Reachability **confirmed** for TS-emitted
warnings; depth of policy-rule coverage is a separate concern out of
scope here.

### 3.9 Entry-point detection

The architecture validator detects entry points via the `EntryPoint`
type in `crates/anvil-architecture/src/types.rs` and the `entry_points`
section of `architecture.yml` (or its template). Detection is
declarative, not parser-driven, so adding TS entry points is a config
concern. **No gap** — but the spec §7.3 line "Confirm" should be
considered confirmed: declared, not auto-discovered.

---

## 4. TS-specific Gaps (LANGTS work items)

| # | Gap | Severity | Owner task | Evidence |
| --- | --- | --- | --- | --- |
| TS-G1 | Interface / type-alias / enum extraction missing. | Medium — not blocking for currently-shipping packs but T3 checklist needs an explicit decision (extract or excuse). | LANGTS-002 | §3.2 |
| TS-G2 | Method-level symbols not emitted. Class symbols carry no `methods` collection; methods are lost from the graph. | Medium — Drizzle / Pulumi rules don't need it today; Hono `c.req.*` chain awareness will. | LANGTS-002 | §3.2 |
| TS-G3 | Dynamic-import (`import('...')`) not captured as an import edge. | Low for governance, medium for packs reasoning about lazy-loaded provider SDKs (LLM Provider pack). | LANGTS-002 | §3.2 |
| TS-G4 | Namespace re-exports (`export * as ns from './m'`) not surfaced as named exports. | Low — re-export `to_source` is captured. | LANGTS-002 | §3.2 |
| TS-G5 | Zod-creep rules not in registry. | Medium — explicit LANGTS-004 deliverable; user-visible. | LANGTS-004 | §3.4 |
| TS-G6 | Re-export-without-local-declaration emits a single `Export` symbol with name `*` instead of per-specifier names. | Low — affects symbol-graph fidelity, not warnings. | LANGTS-002 | §3.2 |
| TS-G7 | `entry_points` documented as declarative; no auto-detection of TS entry shapes (`bin` field of `package.json`, root `index.ts`, `main` / `module` fields). | Low — declarative is acceptable; document-and-move-on. | LANGTS-003 (checklist note) | §3.9 |

None of TS-G1..G7 individually blocks Phase 1 ship. Together they shape
*what "T3" means* for Rust and Python — which is exactly what the T3
acceptance checklist (companion artefact) pins down.

---

## 5. Kernel-Prereq Gaps (council §16.5 #3 work)

These are the load-bearing items. Each is currently named in the LANGTS
module's anticipated tasks but **not yet** in any APS task with
Validation criteria. The spec's §17.2 row 1 explicitly assigns this
work to "Track 1 item 0.5, in the TS-audit APS module" — i.e. here.

### 5.1 Extractor refactor — one canonical extractor trait

**Council finding:** C-003.

**Current state.** `crates/anvil-kernel/src/parser/extract.rs` is a
JS/TS-shaped recursive AST walker (528 LoC). It dispatches on
tree-sitter node kind strings (`"function_declaration"`,
`"class_declaration"`, `"import_statement"`, `"call_expression"`, etc.)
that are TS/JS grammar-specific. There is no `LanguageExtractor` trait;
adding Rust extraction would mean either (a) bolting an `if lang ==
Rust { ... }` cascade onto `extract_from_node`, or (b) writing a parallel
file. Neither is acceptable for three anchor languages plus a tail wave.

The function signatures expose the leak directly: `extract_from_node`
takes a `tree_sitter::Node` and matches on string kinds, with no
language parameter. The implicit assumption is "the kinds are TS/JS
kinds". For Rust the kinds (`function_item`, `impl_item`, `mod_item`,
`use_declaration`) are different strings and different shapes. The
existing extractor cannot be extended by adding match arms.

**Recommended fix.** Introduce a `LanguageExtractor` trait (name and
shape to be designed; suggested interface lives in the T3 acceptance
checklist §3). Each language registers an implementation that knows
its kind strings, field names, and symbol shapes. The orchestration
layer (`extract_symbols`) becomes language-agnostic — it asks the
trait object what the symbols are.

- `extract.rs` is re-shaped into a trait + per-language modules
  (`extract/typescript.rs`, `extract/rust.rs`, `extract/python.rs`).
- `Parser::parse_bytes` returns a `ParseResult` that already carries
  language; the symbol-graph orchestration layer then dispatches to the
  registered extractor.
- Existing TS behaviour is preserved by porting the current walker to
  the TS implementation of the trait.

**Estimated complexity.** Medium. ~1 sprint for the trait + TS port
with parity tests; Rust/Python implementations are then per-anchor work
in RSTLAN/PYLAN. Risk: the "JS/TS-shaped" assumption leaks into
`anvil-kernel-types::SymbolNode` (e.g. `kind: SymbolKind::Function`
mapped from `function_declaration` only). Audit during the refactor —
likely additive (`SymbolKind::Method`, `SymbolKind::Trait`,
`SymbolKind::Module`, `SymbolKind::TypeAlias`) rather than breaking.

### 5.2 Grammar version in cache key

**Council finding:** C-004.

**Current state.** `AstCache` in `crates/anvil-kernel/src/parser/cache.rs`
keys on `(PathBuf, content_hash)`. `tree_sitter::Tree` node kind IDs
are grammar-version-specific — upgrading any tree-sitter grammar crate
silently returns trees whose node-kind integers don't match the new
extractor expectations. Today this hits TS only, so the failure surface
is bounded; with 9+ grammars on the roadmap (TS, TSX, JS, JSX, Rust,
Python, Dart, Go, Java, Kotlin, .NET, C/C++) the latent-corruption
window grows linearly.

**Recommended fix.** Extend the cache key to include a grammar-version
identifier. The simplest correct shape is `CacheEntry { content_hash:
u64, grammar_version: u64, tree: tree_sitter::Tree }` and a cache hit
requires both hashes match. The `grammar_version` value is a hash of
the grammar crate's `LANGUAGE_VERSION` constant exposed by tree-sitter
(every crate exposes a numeric ABI version) plus the crate's `pkg`
version string at build time. On grammar bump the hash changes; the
cache invalidates; reparses occur. No silent corruption.

- Disk-backed cache (if added later for multi-process scenarios)
  serialises the grammar version next to the tree.
- Cache statistics surface "invalidated due to grammar bump" so doctor
  / observability can flag a mass-invalidation event.

**Estimated complexity.** Small. < 1 day for the cache change + tests.
The wider question — when does grammar bump propagate — is a
release-engineering decision orthogonal to this fix.

### 5.3 Parser thread-safety guarantees

**Council finding:** C-026.

**Current state.** `tree_sitter::Parser` is **not** `Send` (it owns a
language-specific FFI handle that is not thread-safe). The `Parser`
struct in `mod.rs` holds `parsers: HashMap<Language, tree_sitter::Parser>`,
which is therefore not `Send` either. The antipattern scanner already
parallelises across files via rayon (`scanner.rs:3`); today it gets
away with it because the antipattern pass is regex-only — it does not
touch the tree-sitter parser. The moment any parser-driven check runs
inside the rayon worker, the borrow checker will refuse the layout.

The spec's roadmap (Track 4 packs) explicitly assumes packs operate on
the *symbol graph* per ADR-027 — which means parsers feed the graph
from somewhere, and that "somewhere" must support concurrent feeders
once the workspace size grows past the in-process single-core bound.

**Recommended fix.** Pin a thread-locality strategy and document it.
Two viable shapes:

1. **Per-thread parser pool.** Each rayon worker holds a `RefCell<Parser>`
   in `thread_local!` storage. Cost: `N_workers × N_languages` parser
   instances; for 16 workers × ~10 languages = 160 parsers, ~5–10 MB
   memory. Acceptable.
2. **Worker-scope parsing.** Parse on the main thread (single Parser
   instance), distribute the resulting `tree_sitter::Tree` (which *is*
   Sync + Send when accessed read-only) to rayon workers for
   extraction/check work. Cost: serialised parse step; OK for save-time
   but probably unacceptable for the watcher.

Recommended path: option (1), with a documented escape hatch to (2)
behind a config flag. Either way, the choice gets a one-line comment in
`parser/mod.rs` and a property test in
`crates/anvil-intercept/benches/` once the rubric harness lands per
ADR-031.

**Estimated complexity.** Small for the wrapper (< 1 day). Medium for
the regression net (need a multi-language fixture corpus exercising
concurrent parses; piggybacks on `latency-corpus-v1` from ADR-031).

### 5.4 Panic removal — parsers surface errors as `Result`

**Council finding:** C-027.

**Current state.** `Parser::get_parser()` in
`crates/anvil-kernel/src/parser/mod.rs:54` calls
`parser.set_language(&lang.ts_language()).expect("language version
mismatch")`. Inside the long-running watcher / daemon (INTD), an
`expect()` aborts the whole process — every other surface watching
that daemon goes dark with no recovery path beyond restart. For
batch-mode CLI use this is merely ugly; for the daemon it's a
launch-blocker.

The same pattern recurs in `cache.rs` test code (acceptable — tests)
but NOT in non-test parser code beyond line 54 — confirmed via grep.
So the fix surface is small.

**Recommended fix.** Replace the `expect` with a propagated `Result`:

- `Parser::get_parser` returns `Result<&mut tree_sitter::Parser,
  ParseError>` with a new `ParseError::GrammarVersionMismatch(Language,
  String)` variant.
- `parse_bytes` propagates the error.
- The daemon translates the error into a JSON-RPC error response per
  ADR-031's `comp.daemonRpc` boundary; the watcher logs it and
  continues serving other languages.

A second pass should walk the kernel for any remaining `.expect()` /
`.unwrap()` in non-test code on the parse hot path; today the count is
1 (the line above) but RSTLAN/PYLAN extraction will introduce more.
The T3 checklist captures "no panics on the parse path" as a gate.

**Estimated complexity.** Trivial for the named line (< 1 hour).
Medium-small for the policy-level audit ("no `.expect()` /
`.unwrap()` in non-test parser code") plus a clippy lint
configuration that encodes the rule.

### 5.5 Grammar maturity audit

**Council finding:** C-005.

**Current state.** The kernel currently depends on
`tree_sitter_typescript` (TypeScript + TSX) and `tree_sitter_javascript`
— both upstream-supported, both pinned. **Zero audit cost today**
because the substrate is the language we already support.

The cost arrives with the Track 2 tail wave. From C-005:

- `tree-sitter-dart` lacks stable 0.26 ABI publication.
- `tree-sitter-kotlin` is community-maintained with known regressions.
- `tree-sitter-cpp` has partial-parse issues on C++20/23 syntax.
- `tree-sitter-c-sharp`, `tree-sitter-go`, `tree-sitter-java` —
  upstream-stable as of 2026-04-26, but binary size and LTO impact
  unaccounted for.

For TypeScript at T3, no current gap. For *the substrate the next
anchors land on*, the audit needs to happen before any grammar enters
the kernel build. Belongs in LANGTAIL (Track 2) and PYLAN/RSTLAN
Ready Checklists, not in LANGTS execution. **Surfaced here so it is
not forgotten.**

**Recommended fix.** Add a one-page maturity rubric to the T3
acceptance checklist (companion artefact §6) covering: ABI version
stability, upstream maintenance signal, error-recovery behaviour on
malformed input, and "does the grammar parse at least one large
real-world fixture without aborting". Apply the rubric before any new
grammar is added to `Cargo.toml`.

**Estimated complexity.** Small per grammar (< 1 day). Recurring
governance cost, not one-off.

### 5.6 Summary table

| # | Gap | Council ref | Severity | Estimated complexity | Lands in |
| --- | --- | --- | --- | --- | --- |
| K1 | Extractor trait — one canonical shape | C-003 | High (load-bearing for RSTLAN, PYLAN) | Medium (~1 sprint) | LANGTS-005 (or split LANGTS-prereq) |
| K2 | Grammar version in cache key | C-004 | High (latent corruption) | Small (< 1 day) | LANGTS-005 |
| K3 | Parser thread-safety strategy | C-026 | Medium (becomes blocker at watcher scale) | Small wrapper + medium regression net | LANGTS-005 |
| K4 | Panic removal on parse path | C-027 | High for daemon mode | Trivial single-line + small policy audit | LANGTS-005 |
| K5 | Grammar maturity rubric | C-005 | Low for TS, High for tail wave | Small per grammar | T3 checklist + LANGTAIL / RSTLAN / PYLAN |

---

## 6. Recommended Outputs and Cross-links

**Audit outputs (this report).** §3 + §4 + §5 above. No solution-design
beyond named recommendations.

**Companion artefact.** [T3 Acceptance Checklist](./2026-04-26-t3-acceptance-checklist.md)
— the re-usable bar referenced from RSTLAN, PYLAN, every Track 4 pack
module, and (data-driven) the pack registry per ADR-027.

The two artefacts are intentionally split:

- The **audit report** is point-in-time evidence. It captures what TS
  looks like on 2026-04-26. It does not need to be re-read after
  RSTLAN starts.
- The **acceptance checklist** is durable. It outlives the audit and
  becomes the gate for every future anchor.

When RSTLAN or PYLAN run their re-scoring gate per
[anchor-rescoring-process](../../docs/guides/anchor-rescoring-process.md),
they reference the checklist, not this report.

---

## 7. Open Questions

1. **Are TS-G1..G4 pre-anchor blockers or post-anchor follow-ups?** The
   audit names them; the call on whether they ship inside LANGTS-002
   or get parked behind a "T3 with documented exclusions" flag is a
   LANGTS Ready-decision, not an audit finding. Recommended default:
   close TS-G1 (interfaces / type aliases) and TS-G2 (methods) inside
   LANGTS-002; defer TS-G3 / TS-G4 / TS-G6 with an explicit follow-up
   note in the T3 checklist saying "TS-G3..G6 are deferred for the
   first iteration; RSTLAN/PYLAN are not required to extract analogous
   shapes if the corresponding deferral note exists."
2. **K1 (extractor trait) inside LANGTS or split into LANGTS-prereq?**
   The LANGTS module Ready Checklist already poses this as an open
   question. The audit's recommendation: **keep K1 inside LANGTS** if
   the trait can land alongside K2..K4 within one sprint; split into
   `lang-ts-prereq.aps.md` if scope grows. The Risks section of the
   LANGTS module already names this exact failure mode ("kernel
   prerequisite work entangles anchor zero indefinitely").
3. **Is a new ADR required for the extractor trait shape?** Probably
   yes — the trait surface is referenced from RSTLAN and PYLAN, and
   ADR-026 (Rust scanner authoritative) does not cover the shape of
   the kernel-side extractor abstraction. See §8 below.

---

## 8. ADRs the Audit Believes Are Missing

| ADR candidate | Reason | Triggered by |
| --- | --- | --- |
| **ADR for extractor trait shape** | K1 reshapes the kernel-side language abstraction. ADR-026 declares Rust scanner authoritative but says nothing about how the extractor is split per language. RSTLAN and PYLAN both depend on the trait surface; pinning it in an ADR keeps the surface stable across anchor work. | C-003 / K1 |
| **ADR for grammar maturity gate** | K5 is recurring governance, not one-off. An ADR ("a tree-sitter grammar enters the kernel only if it passes the maturity rubric") would let the LANGTAIL wave move to Ready without re-arguing the criteria each time. Could live as an extension of ADR-026 or stand alone. | C-005 / K5 |
| **ADR for parser thread-locality strategy** *(conditional — see below)* | K3 is bounded enough that a single-line code comment + checklist entry may suffice. Promote to ADR if the implementation choice diverges from option (1) or if multi-process daemon scenarios materialise. | C-026 / K3 |

**Decision 2026-04-26 (the three flagged ADRs):**

- **Extractor trait shape** — defer until RSTLAN starts. Author at the
  point RSTLAN's first work item surfaces real shape requirements;
  premature locking risks the wrong abstraction.
- **Grammar maturity gate** — fold into `operational-supplement`
  (OPSUP). Maturity rubrics are operational-supplement-shaped (per-track
  flags, FP reporting, cross-track governance). No standalone ADR;
  capture as an OPSUP slice when LANGTAIL admission becomes a real ask.
- **Parser thread-locality** — **conditional defer.** Trigger to author
  the ADR: INTD-001 review surfaces a parser-concurrency question. If
  the daemon's first concurrency model picks option (1) `thread_local!`
  cleanly with no disagreement, no ADR needed — capture inline in
  INTD-001's spec instead. If the choice proves contentious, or
  multi-process daemon scenarios appear, then write the ADR. INTD-001
  carries a forward-reference (see `plans/archive/modules/intercept-daemon.aps.md`
  INTD-001 task) so the trigger is not lost.

---

## 9. Appendix — Evidence Index

Files cited in this audit, with stable line references:

- `crates/anvil-kernel/src/parser/mod.rs:35` — `Parser` struct definition
- `crates/anvil-kernel/src/parser/mod.rs:54` — `expect("language version
  mismatch")` (K4)
- `crates/anvil-kernel/src/parser/cache.rs:11` — `CacheEntry { content_hash,
  tree }` (K2)
- `crates/anvil-kernel/src/parser/extract.rs:59` — `match node.kind()` JS/TS
  string match (K1)
- `crates/anvil-kernel/src/parser/extract.rs:60–67` — kind strings:
  `function_declaration`, `class_declaration`, `export_statement`,
  `import_statement`, `call_expression`, `assignment_expression`,
  `lexical_declaration`
- `crates/anvil-architecture/src/validator.rs:347` — `include_extensions`
  (TS + Rust)
- `crates/anvil-checks/src/antipattern/scanner.rs:12` —
  `LEGACY_JS_TS_EXTENSIONS`
- `crates/anvil-checks/src/antipattern/scanner.rs:229` — suppression regex
  covering `//`, `/*`, `#`, `<!--`, `--`
- `patterns/compiled/registry.json` — AP-001..AP-007 registry (no
  Zod-creep rules)

---

**End of audit report.**
