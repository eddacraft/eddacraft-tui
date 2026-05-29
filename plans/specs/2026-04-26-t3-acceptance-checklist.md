# T3 Acceptance Checklist (Anchor Languages)

**Date:** 2026-04-26
**Authoritative source:** [LANGTS audit report](./2026-04-26-langts-audit-report.md)
**Spec context:** [2026-04-08 Language and Coverage Design](./2026-04-08-language-and-coverage-design.md) §7.3, §8.1, §16.5
**Owning module:** [lang-ts-audit](../modules/lang-ts-audit.aps.md)
**Version:** v1 (2026-04-26)

> The bar a programming language must clear before Anvil claims it as a
> **T3 anchor**. Calibrated against the TypeScript implementation that
> exists today plus the kernel-prereq gaps that close inside LANGTS;
> referenced from [lang-rust](../modules/lang-rust.aps.md) (RSTLAN),
> [lang-python](../modules/lang-python.aps.md) (PYLAN), every Track 4
> pack module, and the pack registry per
> [ADR-027](../decisions/027-pack-architecture.md).
>
> **This checklist is versioned.** Material changes are ADR-level
> decisions per the LANGTS Risks table. Cosmetic edits and clarifying
> notes happen in place; semantic shifts cut a v2 of the artefact and
> existing anchors are re-validated against the new bar.

---

## 0. How to use this checklist

For an anchor module to declare its language at T3, every section
below must have all required boxes checked. "N/A with documented
exclusion" is acceptable for items the language can't honour for
intrinsic reasons (e.g. dynamic-import shape doesn't exist in Rust);
each exclusion needs a one-line rationale stored in the anchor module's
acceptance section.

The checklist applies to both new anchors (RSTLAN, PYLAN) and to the
TypeScript anchor itself (LANGTS — items §1, §2, §6, §7 below are the
LANGTS-002 / LANGTS-005 closing list, audit-derived).

The audit report (companion) names which gaps already close TS to
this bar and which gaps remain. Future anchors don't need to re-read
the audit; they read this checklist.

---

## 1. Parser layer requirements

The substrate that turns source bytes into a tree-sitter AST.

- [ ] **Grammar wired.** A tree-sitter grammar crate is registered in
      `crates/anvil-kernel/src/parser/languages.rs` with both
      extension matching (`Language::from_path`) and grammar binding
      (`Language::ts_language()`). Grammar crate version pinned in
      `Cargo.toml`.
- [ ] **Grammar maturity gate passed** (audit §5.5 / K5):
  - [ ] ABI version stable (tree-sitter ≥ 0.20 ABI) and crate is
        published on crates.io.
  - [ ] Upstream maintenance signal exists (commit in the last 12
        months OR explicit "stable, maintenance-mode" upstream
        statement).
  - [ ] Grammar parses at least one large real-world fixture
        (≥ 50 KB representative file from a popular open-source
        project) without aborting.
  - [ ] Error-recovery behaviour on malformed input does not panic
        the host process (errors surface as `tree.has_error()` or
        equivalent, not as a tree-sitter abort).
- [ ] **Cache key includes grammar version** (audit §5.2 / K2). The
      `AstCache` `CacheEntry` carries a `grammar_version` field
      derived from the grammar crate's `LANGUAGE_VERSION` plus
      build-time crate version. A grammar bump invalidates cache
      entries deterministically. Cache statistics surface
      "invalidated due to grammar bump" events for doctor /
      observability.
- [ ] **No panics on the parse path** (audit §5.4 / K4). No
      `.expect()` / `.unwrap()` in non-test code on any path
      reachable from `Parser::parse_bytes` or `Parser::parse_file`.
      Grammar version mismatches surface as a typed
      `ParseError::GrammarVersionMismatch`. A clippy lint or
      equivalent encodes the rule.
- [ ] **Parser thread-safety strategy documented** (audit §5.3 /
      K3). One of: per-thread parser pool via `thread_local!`, or
      worker-scope parsing (parse on main thread, ship `Tree` to
      workers). Choice recorded as a code comment in
      `parser/mod.rs` plus a regression-net bench in
      `crates/anvil-intercept/benches/` exercising concurrent parses
      across at least two languages.
- [ ] **Extractor implements the canonical trait** (audit §5.1 /
      K1). The language has its own implementation of
      `LanguageExtractor` (or whatever the trait ends up named in
      the implementing ADR) under
      `crates/anvil-kernel/src/parser/extract/<lang>.rs`. The
      orchestrator (`extract_symbols`) is language-agnostic — it
      routes `(Language, Tree, source)` through the trait, not via
      a string-kind match.

---

## 2. Symbol-graph requirements

What the kernel turns into nodes and edges.

The `SymbolNode` / `SymbolEdge` types live in `anvil-kernel-types`. T3
requires the language's extractor to populate them with the shapes
below, with cardinality matching the language's actual surface.

### 2.1 Required symbol kinds

Each anchor language extracts the following symbol kinds (or
documents an N/A exclusion with rationale):

- [ ] **Function** — top-level functions and named function
      expressions.
- [ ] **Class / struct / record** — the language's primary
      type-definition shape (TS class, Rust struct/enum/trait,
      Python class).
- [ ] **Method** — class/impl methods surfaced as separate symbols
      with a `parent` link to the owning class/struct (audit
      gap TS-G2; closes inside LANGTS-002 for TS).
- [ ] **Module / namespace** — file-level module symbol when the
      language has explicit module syntax (Rust `mod`, Python
      `__init__.py`). N/A acceptable for TS where files *are*
      modules.
- [ ] **Type alias / interface / trait** — the language's
      type-shape declaration form (TS `type` / `interface`, Rust
      `trait` + `type`, Python `TypeAlias`). Closes audit gap
      TS-G1 for TS.
- [ ] **Enum** — where the language has enums (TS, Rust, Python via
      `enum.Enum`).
- [ ] **Export / public surface marker** — symbols on the public API
      surface have `Visibility::Public` (TS `export`, Rust `pub`,
      Python module-level names absent the `_` prefix convention or
      via `__all__`).

### 2.2 Required edge types

The graph must populate the following edge types (from
`anvil-kernel-types::EdgeType`) where the language's source
expresses them:

- [ ] **Imports** — module-to-module import edges, one per
      `import` / `use` / `require` / `from import` statement, with
      1-based line number.
- [ ] **Calls** — call-expression edges from caller to callee where
      both are resolvable to graph nodes. Best-effort for symbols
      whose definition is outside the scanned tree (external imports
      remain edges to a synthetic external node).
- [ ] **Inherits / implements** — class-extends-class, struct/enum
      implements-trait, Python class inheritance.
- [ ] **References** — type-position references (e.g. `Foo` used as
      a parameter type) where extracting them is cheap. May be N/A
      if the language requires whole-program type-checker work to
      resolve; document the call.

### 2.3 Cardinality expectations

- [ ] **Symbol count per file scales linearly with declaration
      count.** Spot-checked on at least three real-world files of
      different sizes (small / medium / large per
      [ADR-031](../decisions/031-validation-latency-rubric.md)
      `latency-corpus-v1` shape).
- [ ] **Import edges captured for every static import statement.**
      Verified by fixture test asserting count of edges == count of
      import statements in the fixture.
- [ ] **Re-exports preserve named specifiers.** `export { foo } from
      './m'` produces an Export edge for `foo` plus a re-export
      edge to `./m`. Audit gap TS-G6 — currently TS emits a single
      `*` symbol on re-export; closing this is part of the LANGTS
      gap-close pass.
- [ ] **Dynamic / runtime imports (e.g. `import('...')` in TS,
      `__import__` in Python) captured as edges where statically
      resolvable; documented N/A otherwise.** Closes TS-G3 partly.

### 2.4 Deferred gaps (first T3 iteration)

LANGTS-002 closed the two Medium audit gaps — **TS-G1** (interface /
type-alias / enum symbols) and **TS-G2** (class methods as `Owner.method`
symbols) via PR #2106. The remaining audit gaps are **deferred for the
first T3 iteration** so RSTLAN / PYLAN are **not** required to extract the
analogous shapes while this note stands:

- **TS-G3 — dynamic `import()` edges.** Static `import` edges are captured;
  runtime `import('…')` call-site edges are deferred.
- **TS-G4 — namespace re-exports** (`export * as ns from './m'`). Deferred.
- **TS-G6 — per-specifier re-export names.** `export { foo } from './m'`
  still emits a single re-export symbol rather than one per specifier;
  deferred.
- **TS-G7 — entry-point auto-detection.** Documentation-only for the first
  T3 iteration; no extraction work required.

TS-G2's owning-class link is encoded in the method symbol name
(`Owner.method`); a structural parent edge is deferred (the graph carries no
symbol-edge channel for it yet).

---

## 3. Architecture analysis requirements

What the architecture validator (`crates/anvil-architecture/`)
detects and resolves for the language.

- [ ] **File extensions registered.** `validator.rs::include_extensions`
      includes the language's primary source extensions.
- [ ] **Layer assignment reaches the language.** Files of this
      language placed in a directory matched by an
      `architecture.yml` layer rule are assigned to that layer.
      Verified by fixture test.
- [ ] **Boundary violations detected.** An import that crosses a
      boundary forbidden by `architecture.yml` produces a
      `BoundaryViolation` for this language's import edges. Verified
      by fixture test using a real language-specific import shape
      (TS `import`, Rust `use`, Python `from … import`).
- [ ] **Entry-point detection works.** Files declared as entry
      points in `architecture.yml` (or via the language's
      conventional entry-point shape — TS `bin` field, Rust `main.rs`
      / `lib.rs`, Python `if __name__ == "__main__"`) are recognised
      as such. Detection may be declarative (config-only) for v1;
      auto-detection is a follow-up.
- [ ] **Scope resolution.** Imports resolve through the language's
      conventional resolution rules (TS extensionless-import + ESM
      `.js` swap; Rust `mod.rs` / `lib.rs`; Python relative-import +
      namespace packages). Resolution rules are documented in the
      anchor module.
- [ ] **Drift baseline included by default.** A baseline written
      against `architecture-validate` covers files of this language
      without extra config. Ensures upgrades catch *new edges
      only*, per the planless-first principle.

---

## 4. Anti-pattern catalogue requirements (T2 layer)

Every T3 anchor *also* covers T2 — language-level anti-patterns. The
catalogue is shipped in `crates/anvil-checks/src/antipattern/` and
the `patterns/` registry.

- [ ] **At least five language-specific anti-pattern rules** in the
      registry, each with: id, family, severity, confidence,
      explanation, suggestion, definition_ref, and a fixture
      exercising both positive and negative cases.
- [ ] **Each rule documented.** Rule definition references a markdown
      page under `patterns/<family>/` (the `definition_ref` field
      points there). Documentation explains the smell, the fix, and
      at least one example.
- [ ] **Suppression syntax tested.** Per
      [ADR-029](../decisions/029-suppression-parser-authority.md),
      suppression goes through the Rust authoritative parser. Each
      anti-pattern rule has at least one fixture exercising
      `// @anvil-ignore <ID> -- reason` (or the language's
      equivalent comment style — `#`, `/*`, `<!--`, `--`).
- [ ] **Comment-style coverage.** The language's primary comment
      syntax is recognised by `parse_suppression` in
      `crates/anvil-checks/src/antipattern/scanner.rs`. Adding new
      styles per ADR-029 is a Rust-side change; TS retirement
      window not extended.
- [ ] **Per-rule fixture coverage minimum.**
  - [ ] One **positive** fixture per rule (rule fires).
  - [ ] One **negative** fixture per rule (rule does not fire on
        legitimate code that resembles the smell).
  - [ ] One **suppression** fixture per rule (rule fires, but the
        warning carries a `Suppression` because of an inline
        directive).
- [ ] **Cross-cutting language patterns folded in.** Examples: TS
      Zod-creep (`z.any()`, `z.unknown()`, `.passthrough()`); Rust
      Serde hygiene (`#[serde(deny_unknown_fields)]`,
      `#[serde(flatten)]` without bounds); Python `# type: ignore`
      / bare `except`. The cross-cutting set is named in the anchor
      module's task list.

---

## 5. False-positive rate target

Per council §16.5 #9 and finding C-014, the previous
"zero FPs on Anvil's own repo" bar is replaced by:

- [ ] **FP rate < 5 % on Anvil's own repo** measured as
      `false_positive_warnings / total_warnings_for_this_language`
      across a clean run. Anvil-specific suppression files are not
      counted toward the FP denominator (i.e. you cannot game the
      bar by mass-suppressing).
- [ ] **≥ 1 external codebase validation run.** At least one
      open-source repository written predominantly in this language
      runs through `anvil scan` at warn level without crashes or
      partial results. Selected repository documented in the
      anchor module. Suggested anchor candidates:
  - TS — Anvil's own `apps/website` (Next.js) and a non-Anvil repo
    such as `microsoft/vscode-eslint` or `vercel/next.js`.
  - Rust — Anvil's own `crates/anvil-kernel` and a non-Anvil repo
    such as `tokio-rs/tokio` or `BurntSushi/ripgrep`.
  - Python — once PYLAN starts, candidates named in PYLAN module.
- [ ] **External-repo FP rate is reported, not gated.** The hard
      gate is the Anvil-repo number; the external number is a
      sanity check that surfaces "we tuned for our own
      style" failures.

The measurement methodology lives next to the OPSUP FP-reporting
channel work; until OPSUP names a tool, the anchor module records
the numbers manually in its acceptance evidence.

---

## 6. Performance target

Per [ADR-031](../decisions/031-validation-latency-rubric.md), all
latency claims for save-time and mid-edit validation use one rubric.
T3 anchors honour the same budgets:

- [ ] **Mid-edit p95 within budget.** On a warm daemon,
      `mode = midEdit` `validation.roundtrip` p95 <= 80 ms on
      `latency-corpus-v1` (per ADR-031), with the language's typical
      file size class included in the corpus or a documented
      analogue. If the budget is exceeded, scope the hot path before
      loosening the SLO.
- [ ] **Save-time p95 within budget.** `mode = save`
      `validation.roundtrip` p95 <= 120 ms on a warm daemon over
      `latency-corpus-v1`.
- [ ] **Service latency recorded.** `validation.service` p95 is
      recorded for the same run so regressions can be attributed to
      daemon work versus driver / transport work.
- [ ] **Tail context reported.** p50 and p99 are reported alongside
      p95, and cold-start samples are reported separately from the
      warm-daemon percentiles.
- [ ] **Bench in `crates/anvil-intercept/benches/`** exercising the
      language's representative fixture (criterion + the
      observability span layout from ADR-031). The bench files are
      committed; baseline is recorded under
      `crates/anvil-intercept/benches/baselines/`.
- [ ] **Regression policy honoured.** CI fails if interactive
      `validation.roundtrip` p95 exceeds the ADR-031 SLO. The owning
      benchmark task may add stricter baseline-relative gates.

If a language cannot meet the mid-edit budget, the anchor module
records the boundary and dimensions responsible, then narrows the
active rule set or files a follow-up ADR revising the budget.

---

## 7. Documentation requirements

T3 means a user can *understand* the support without reading kernel
code.

- [ ] **Each rule documented.** `definition_ref` in the registry
      entry points at a markdown page under `patterns/<family>/`.
      The page explains: smell, why it matters, fix, example, and
      suppression syntax.
- [ ] **Suppression syntax documented for the language.** A short
      page (or section in the anchor module's docs) shows the
      comment styles that work for this language, the
      `@anvil-ignore <ID> -- reason` shape, and the
      `@anvil-ignore-until DATE` extension from ADR-004.
- [ ] **Anchor capabilities page.** A single doc per anchor
      enumerating: extracted symbol kinds, captured edge types,
      anti-pattern rule list with one-line summary, performance
      bench reference, FP-rate measurement, external-repo
      validation result. Lives at
      `docs/architecture/anvil-<lang>-t3.md` (or wherever the
      LANGTS Ready-decision pins the artefact location).
- [ ] **Index entry updated.** `plans/index.aps.md` reflects the
      language's status flip to "T3 capable" with a link to the
      anchor capabilities page.

---

## 8. Test coverage minima

The minimum test substrate for T3 — both as a quality bar and as a
regression net for substrate work that touches multiple anchors
(extractor trait, cache, etc.).

- [ ] **Per-rule fixture coverage** (§4 above): positive + negative
      + suppression for each anti-pattern rule.
- [ ] **Symbol-graph fidelity tests.** Unit tests in
      `crates/anvil-kernel/src/parser/extract/<lang>.rs` (or its
      `tests/` equivalent) covering each of §2.1's symbol kinds
      that the language supports.
- [ ] **Edge-type tests.** Unit tests covering each §2.2 edge type
      the language supports — at least one positive case per kind.
- [ ] **Architecture violation tests.** At least one fixture in
      `crates/anvil-architecture/` exercising a boundary violation
      using a real import shape from this language.
- [ ] **Suppression parser tests.** Per ADR-029, the language's
      comment styles each have a `parse_suppression` test in
      `crates/anvil-checks/src/antipattern/scanner.rs`.
- [ ] **Latency bench** (§6) — committed.
- [ ] **External-repo smoke test.** A CI job (or manual smoke
      script committed under `tests/external/<lang>/`) clones the
      anchor module's named external repository and runs
      `anvil scan`. Reports counts of warnings, panics, and
      partial-result events. Failures gate.

---

## 9. Pack-substrate gating

ADR-027 declares packs are tier-gated against this checklist. For an
anchor module to support packs:

- [ ] **Anchor module declares its tier.** The anchor module's
      header table reads "Status: Complete | Tier: T3" only when
      every section above is checked.
- [ ] **Pack registry refuses activation below declared tier.** Per
      [ADR-027](../decisions/027-pack-architecture.md) §Decision
      item 5, `crates/anvil-packs/` blocks activation of packs
      declaring `min_substrate_tier = T3` until the substrate
      anchor reports T3. The check is data-driven from this
      checklist's outputs (the anchor capabilities page is the
      machine-readable side).

This row is the load-bearing reason the checklist is durable: pack
authors can't hand-wave "the substrate is at T3" — they refer to
this artefact and the registry refuses the activation if the
substrate's anchor module hasn't completed it.

---

## 10. Versioning and amendments

This artefact is **v1 (2026-04-26)**. Material changes — additions
or removals of items in §1–§8, changes to FP-rate bar in §5, changes
to the latency budget in §6, changes to the pack-gate in §9 — are
ADR-level decisions. The change is recorded as a new ADR and a v2
of this artefact replaces v1; existing anchors re-validate against
v2 within their next anchor-rescoring cycle per
[anchor-rescoring-process](../../docs/guides/anchor-rescoring-process.md).

Cosmetic edits (typos, broken cross-links, clarifying notes that
don't shift the bar) happen in place; the version stays v1 with a
`Last edited` date in the header.

---

## 11. Cross-references

- [LANGTS audit report](./2026-04-26-langts-audit-report.md) —
  the evidence base for the items in §1, §2, §6 of this checklist.
- [ADR-026](../decisions/026-rust-scanner-authoritative.md) —
  Rust scanner authoritative; underpins §1 K1.
- [ADR-027](../decisions/027-pack-architecture.md) — pack
  architecture; consumes this checklist via the pack registry tier
  gate (§9).
- [ADR-029](../decisions/029-suppression-parser-authority.md) —
  suppression parser authority; underpins §4 and §8.
- [ADR-031](../decisions/031-validation-latency-rubric.md) —
  validation latency rubric; underpins §6.
- [anchor-rescoring-process](../../docs/guides/anchor-rescoring-process.md)
  — the process every anchor module runs against this checklist.

---

**End of T3 acceptance checklist v1.**
