<!-- APS: See https://github.com/eddacraft/anvil-plan-spec for format reference -->
<!-- Executable only if tasks exist and status is Ready. -->

# C# / .NET Language Anchor (Track 1)

| ID    | Owner      | Status |
| ----- | ---------- | ------ |
| DNLAN | joshuaboys | Draft  |

**Last reviewed:** 2026-08-06 (module created on owner direction under
[ADR-118](../decisions/118-csharp-anchor-promotion-t2-t3.md), promoting C# out
of the Track 2 tail. **Demand is 0** — the §8.2 promotion lever ("first .NET
user") has not fired; the ADR records that override honestly. No work item has
started; the module is Draft until the Ready Checklist below is met.

**Owner decisions 2026-08-06 (second pass):** owner named — **joshuaboys**;
catalogue tier is **regex *and* AST, split per rule** (see Detection Tier
below), not regex-first-with-escalation; namespace resolution is the
**declared-namespace index** (see DNLAN-005/-009). Three of the five Ready
gates are now closed — the FP-bar **N** and the T3-checklist re-read remain.)

## Purpose

Bring C# to **T3 (Governed)** per
[2026-04-08 Language and Coverage Design](../specs/2026-04-08-language-and-coverage-design.md)
§5.1, §8.1 — the same bar Rust (`RSTLAN`) and Python (`PYLAN`) cleared.

C# is already at **T1 (Parsed)**, shipped as `LANGTAIL-006` in the tail wave
(Merged 2026-06-18 via PR #2757). This module owns the T2 and T3 delta on top
of that, and nothing that T1 already delivers.

**What T1 already gives us** (`crates/anvil-kernel/src/parser/extract/csharp.rs`,
213 lines):

- `.cs` detected via `Language::from_path`; `tree-sitter-c-sharp` 0.23.5 bound.
- Symbols: `class`/`struct`/`record`/`record struct` → `Class`, `interface` →
  `Interface`, `enum` → `Enum`, methods qualified as `Owner.method`. Visibility
  is `Public` on the `public` modifier, else `Internal`.
- Recursion through block-scoped **and** file-scoped `namespace` declarations.
- One `ImportEdge` per `using` directive, target = the namespace path. Handles
  `using System.Text;`, `using static System.Math;`, and
  `using Alias = System.Text;` (resolves to the **target**, not the alias).

**The gap to T3** is everything §5.1 requires beyond that: an anti-pattern
catalogue, suppression coverage, entry-point detection, namespace import
resolution, layer/boundary enforcement, drift baseline, and inclusion in
`architecture-validate`.

This module **supersedes the scope of** the archived
[`lang-dotnet`](../archive/modules/lang-dotnet.aps.md) placeholder, whose
content is regex-era (it assumed `using`-extraction regexes and a since-archived
`HTMLCSS-001` prerequisite). Tree-sitter reality changes the implementation
shape; the archived file stays the historical record.

## In Scope

- **T2 anti-pattern catalogue** (§5.1 wants 5–10 patterns). Candidate set,
  finalised in DNLAN-001 — not fixed by this section:

  | Rule | Pattern | Tier | Default |
  |---|---|---|---|
  | CS-001 | `#pragma warning disable` without a justification comment | regex | on |
  | CS-002 | `[SuppressMessage(...)]` without a `Justification:` argument | regex | on |
  | CS-003 | Empty or blanket swallow — `catch { }` / `catch (Exception) { }` | AST | on |
  | CS-004 | `async void` outside an event-handler signature | AST | on |
  | CS-005 | Sync-over-async — `.Result` / `.Wait()` / `.GetAwaiter().GetResult()` | AST | on |
  | CS-006 | `// ReSharper disable` without justification | regex | on |
  | CS-007 | `dynamic` type usage (the C# parallel of TS `any`) | AST | opt-in |
  | CS-008 | `Console.Write` / `Console.WriteLine` in non-console projects | regex | opt-in |

  CS-004 and CS-005 are additions to the archived placeholder's list, on blast
  radius: `async void` loses exceptions to the synchronisation context, and
  sync-over-async deadlocks production services. CS-007/CS-008 ship opt-in for
  the same noise reason `PY-006`/`PY-007` did. The **Tier** column is decided —
  see Detection Tier below for why each rule sits where it does.

- **Suppression syntax** — `// @anvil-ignore CS-NNN -- <reason>`. The ADR-029
  parser already accepts the `//` prefix, so this is coverage and proof, not
  new parsing (the `PYLAN-004` shape).
- **Entry-point detection** — `Program.cs` top-level statements,
  `static void Main` / `static async Task Main`, and `.csproj`
  `<OutputType>Exe</OutputType>`, mirroring `detect_rust_entry_points`
  (RSTLAN-004) and `detect_python_entry_points` (PYLAN-005).
- **Namespace import resolution via a declared-namespace index** — resolve a
  `using` target to the **set** of workspace-relative `.cs` files declaring
  that namespace. Deliberately *not* the `resolve_rust_import` /
  `resolve_python_import` single-file shape: a C# `using` imports a namespace,
  not a file, and one namespace routinely spans many files. Targets declared
  nowhere in the tree are external (BCL, NuGet) and the edge is **dropped**.
  Design and rejected alternatives: DNLAN-005; extractor prerequisite:
  DNLAN-009.
- **Layer/boundary enforcement** reaching C# namespaces and projects —
  including the collapse from that file set to a single layer verdict, with
  layer-straddling namespaces dropped rather than guessed (DNLAN-006).
- **Drift baseline default-on for `.cs`** — `.cs` joins
  `AntipatternCheckConfig::default()`'s extension set.
- **`architecture-validate` includes C# projects** — `.cs` in the validator's
  `include_extensions` so the CLI / MCP / dashboard surfaces enumerate and
  layer-assign C# files.
- **T3 dogfood + FP bar** per §16.5 #9, on public OSS (Anvil has no C#).

## Out of Scope

- **`.vb` / `.fs` / `.csx`** — C# only, retaining the archived module's own
  boundary. None are detected by `Language::from_path` today, and ADR-118 does
  not admit them.
- **NuGet / MSBuild dependency-graph analysis** — `.csproj` is read for entry
  points and project layout only, never for package resolution (that shape
  belongs to `config-intelligence`).
- **Roslyn analyser integration** — Anvil is not a Roslyn host.
- **ASP.NET, Entity Framework, LINQ semantics** — pack territory (§8.4), and
  gated on this module reaching T2 first.
- **`.razor` / `.cshtml`** — templating surfaces, not C# source.
- **Call-site extraction** — `EdgeType::Calls` for C# is `GCALL` substrate
  work, not a T3 requirement under §5.1.
- **Solution-wide (`.sln`) project graph modelling** — a `.sln` may inform
  project roots, but modelling the solution graph is out.

## Interfaces

**Depends on:**

- [`lang-tail-wave`](./lang-tail-wave.aps.md) — `LANGTAIL-006` delivered the T1
  substrate (grammar, `Language::CSharp`, `CSharpExtractor`) this builds on.
- [`lang-ts-audit`](../archive/modules/lang-ts-audit.aps.md) — the authoritative
  T3 acceptance checklist.
- [`lang-rust`](../archive/modules/lang-rust.aps.md) /
  [`lang-python`](../archive/modules/lang-python.aps.md) — the resolver, entry-point, catalogue,
  and FP-bar patterns to copy rather than reinvent.
- Existing kernel parser, architecture analysis, policy pipeline, drift
  baseline, ADR-029 suppression parser.

**Exposes:**

- C# at T3 — the substrate an ASP.NET pack requires (§8.4 needs ≥ T2).
- A third worked example of the T1→T3 promotion path, for whichever tail
  language is promoted next.

## Prerequisites

- `LANGTAIL-006` merged (**met** — PR #2757).
- `lang-ts-audit` Complete, so the T3 checklist exists (**met**).
- `lang-python` T3 landed, so the resolver/catalogue/FP-bar pattern is proven
  (**met** — all nine PYLAN items Merged).

## Ready Checklist

Change status to **Ready** when:

- [x] Owner named for the anchor work — **joshuaboys** (2026-08-06).
- [x] Catalogue tier decided — **both tiers, split per rule**, per the
      Detection Tier section below (2026-08-06). CS-005's `.Result` ambiguity
      is resolved by putting it on the AST tier, where the receiver's type is
      knowable, rather than shipping it as a regex that matches any member
      named `Result`.
- [ ] FP-bar **N** agreed for C# (PYLAN accepted N = 1%), and the external
      corpus named. C# measured **6.9% error-trees** in `LANGTAIL-008`; confirm
      that recovery rate is good enough for the resolver before committing.
- [x] Namespace resolution strategy decided — **declared-namespace index**
      (2026-08-06). Not folder convention, not `.csproj` `RootNamespace`; see
      DNLAN-005 for the design and DNLAN-009 for its extractor prerequisite.
- [ ] T3 acceptance checklist re-read and confirmed applicable unchanged.

## Detection Tier (decided 2026-08-06)

The catalogue uses **both** tiers, chosen per rule by what the rule actually
needs to see — not regex-first with escalation on FP failure. Rules that only
need to spot a textual directive stay on the cheap save-time-safe tier; rules
that need block structure or type context start on the AST tier, because no
amount of regex tuning gives them what they need.

| Tier | Rules | Why |
|---|---|---|
| **Regex / RE2** (`Detection::Regex`, save-time hot path, the `PYLAN-003` shape) | CS-001 `#pragma warning disable`, CS-002 `[SuppressMessage]`, CS-006 `// ReSharper disable`, CS-008 `Console.Write*` | Each is a literal directive or call token. Line-oriented matching is sufficient and correct; these can run on every keystroke-adjacent save |
| **AST** (ADR-071 `anvil-checks-ast`, gate-time) | CS-003 empty/blanket `catch`, CS-004 `async void`, CS-005 sync-over-async, CS-007 `dynamic` | Each needs structure a regex cannot see: CS-003 needs the catch **block body** (`PY-004` documents this exact limitation — a `pass` on the next line escapes the regex); CS-004 needs modifier + return type on a method declaration; CS-005 needs the **receiver's type** to tell `Task.Result` from any other `Result` member; CS-007 needs type position to avoid matching the word in comments and identifiers |

Consequence to accept knowingly: the AST-tier rules run at **gate time, not
save time** (that is what ADR-071 bought), so four of the eight rules will not
fire in the save-time daemon. That is the correct trade — a wrong answer
delivered fast is worse than a right answer at the gate — but it means "C# is
governed on save" is **not** a claim this module can make for the AST four.
Note the tier split in the DNLAN-008 evidence so the surface difference is
recorded rather than discovered.

## Work Items

All items are **Draft**. None may start before the module is Ready.

Nine items. The T2 slice is DNLAN-001..004; the T3 slice is **DNLAN-009 →
005 → 006 → 007** (that is execution order — DNLAN-009 was filed after the
module merged and keeps its number so existing references stay valid);
DNLAN-008 is wave-level acceptance. Per §8.1 there are **no partial anchors** —
shipping DNLAN-001..004 alone is C#-at-T2 and must be described as such, never
as ".NET support".

#### DNLAN-001: C# T2 anti-pattern catalogue

- **Status:** Draft
- **Intent:** Govern the C#-specific anti-patterns that carry real blast
  radius, not merely the ones that are easy to match.
- **Expected Outcome:** A `csharp-reliability` family fires on C#. The four
  regex rules reach `anvil check` / `gate` / drift **and** the save-time
  daemon; the four AST rules reach gate time per ADR-071. The noisy rules ship
  opt-in. Final rule set is decided here against the candidate table above.
- **Scope note:** This item spans **both** detection tiers, so it is the
  largest in the module and may split into a regex half and an AST half at
  start time. Split on tier, not on rule count — the two tiers have different
  test harnesses and different latency budgets.
- **Validation:** Per-rule positive and justified-negative tests. *Regex half:*
  RE2-compile clean (lookahead is dropped **silently** — the `PYLAN-003` trap),
  extension scoping to `.cs`, `registry_compile_diagnostics` regression guard.
  *AST half:* per-rule node-shape tests, and a **partial-parse test per rule** —
  given C#'s 6.9% error-tree rate, each AST rule must degrade to "no finding"
  rather than panicking or firing wrongly on a recovered tree. Opt-in gating
  asserted for both halves.
- **Files:** `patterns/csharp-reliability/*.anvil`,
  `patterns/compiled/registry.json`,
  `crates/anvil-checks/src/antipattern/types.rs`,
  `crates/anvil-checks/tests/csharp_antipatterns.rs`,
  `crates/anvil-checks-ast/` (the AST half, per ADR-071)
- **Dependencies:** —
- **Confidence:** medium — rule selection and per-rule tier are both settled
  (2026-08-06); what is unproven is AST-rule behaviour on partial parses.

---

#### DNLAN-002: `//`-comment suppression coverage for C#

- **Status:** Draft
- **Intent:** Let C# findings be suppressed with a reason, exactly as TS, Rust,
  and Python can.
- **Expected Outcome:** `// @anvil-ignore CS-NNN -- reason` suppresses the
  matching C# finding; a wrong-ID suppression does **not** silence it.
- **Validation:** Suppression tests in `csharp_antipatterns.rs`, including the
  wrong-ID negative case.
- **Files:** `crates/anvil-checks/tests/csharp_antipatterns.rs` (behaviour
  expected to already exist in `crates/anvil-checks/src/antipattern/scanner.rs`
  — the ADR-029 parser already accepts `//`)
- **Dependencies:** DNLAN-001
- **Confidence:** high — `PYLAN-004` was a test-only item for the same reason.

---

#### DNLAN-003: Drift baseline default-on for `.cs`

- **Status:** Draft
- **Intent:** Make C# drift and anti-pattern scanning default-on rather than
  something a user must opt into.
- **Expected Outcome:** `.cs` is in the default scanned-extension set, so the
  C# rules and the already-`.cs`-eligible generic families fire across `anvil
  check` / `gate` / drift and the save-time daemon.
- **Validation:** Test asserts `.cs` in `AntipatternCheckConfig::default()`.
- **Files:** `crates/anvil-checks/src/antipattern/types.rs`
- **Dependencies:** DNLAN-001
- **Confidence:** high — one-line parallel of RSTLAN-006 / PYLAN-007.

---

#### DNLAN-004: .NET entry-point detection

- **Status:** Draft
- **Intent:** Surface a .NET (or mixed) repo's roots so baseline creation and
  the `anvil architecture` surfaces treat them the way they treat Rust
  `[[bin]]` and Python `__main__`.
- **Expected Outcome:** `EntryPoint`s for `Program.cs` top-level statements,
  `static void Main` / `static async Task Main`, and `.csproj`
  `<OutputType>Exe</OutputType>`, with confidence tiers. Output is
  workspace-relative, forward-slash, sorted, and de-duplicated — a declared
  `.csproj` entry wins over a bare `Main`.
- **Validation:** Unit tests per source, confidence tiers, dedup precedence,
  build/VCS directory pruning (`bin/`, `obj/`, `.vs/`), and determinism.
- **Files:** `crates/anvil-architecture/src/dotnet_detection.rs`,
  `crates/anvil-architecture/src/lib.rs`
- **Dependencies:** —
- **Confidence:** medium — `.csproj` is XML, unlike the TOML/INI sources the
  Rust and Python detectors parse.

---

#### DNLAN-009: Capture the declaring namespace in the C# extractor

> Filed 2026-08-06, after the module merged, once the namespace-index decision
> exposed a T1 gap. Numbered 009 rather than renumbering 005..008, so the
> references in PR #3657 and the index row stay valid — **numbering is not
> execution order**, and the `Dependencies` fields are authoritative. This item
> precedes DNLAN-005.

- **Status:** Draft
- **Intent:** Record *which namespace a symbol is declared in*, so namespace
  resolution has ground truth to read instead of a path convention to guess.
- **Context:** `LANGTAIL-006` recurses **through** `namespace_declaration` and
  `file_scoped_namespace_declaration` to reach the types inside, but discards
  the namespace name on the way past — `namespace App { class Service }` yields
  the symbol `Service`, not `App.Service`. That is correct and sufficient for
  T1; it is the single blocker for T3 namespace resolution.
- **Expected Outcome:** Every C# symbol carries its declaring namespace.
  Nested and file-scoped namespace forms both resolve to the same dotted path;
  a file declaring several namespaces attributes each symbol to the right one.
- **Validation:** Fixture tests for block-scoped, file-scoped, nested
  (`namespace A { namespace B { … } }`), dotted-in-one-declaration
  (`namespace A.B.C`), several namespaces in one file, and the global
  namespace (no declaration). Existing `LANGTAIL-006` tests keep passing.
- **Files:** `crates/anvil-kernel/src/parser/extract/csharp.rs`,
  `crates/anvil-kernel/src/parser/extract/mod.rs` (only if the symbol shape
  needs a namespace field rather than a qualified name)
- **Dependencies:** —
- **Confidence:** high — the walk already visits these nodes; this threads the
  name through, exactly as `owner` is already threaded for methods.
- **Note:** Decide qualified-name vs separate-field early — it is the one
  choice here that touches `anvil-kernel-types` and therefore other languages.

---

#### DNLAN-005: C# namespace resolution via a declared-namespace index

- **Status:** Draft
- **Intent:** Let the boundary checker answer "what does this `using` actually
  reach?" from what the code declares, not from what a folder name implies.
- **Design (decided 2026-08-06):** A `using` directive in C# imports a
  **namespace, not a file** — `using App.Services;` legitimately reaches every
  type in that namespace, across however many files declare it. One namespace
  spanning many files is the **normal** case in C#, not an edge case, so
  `Option<PathBuf>` — the `resolve_rust_import` / `resolve_python_import`
  shape — is the **wrong return type** here, and this item deliberately breaks
  that symmetry. Instead:
  1. A workspace pre-pass builds `namespace → {declaring files}` from
     DNLAN-009's declarations, over the same file list
     `gate::extract_import_edges` already walks.
  2. `using N` resolves to the **set** of files declaring `N`.
  3. A namespace declared nowhere in the tree is external — BCL, NuGet — and
     the edge is **dropped**. Conservative, never a false boundary violation
     (the `PYLAN-006` rule).
- **Expected Outcome:** `resolve_csharp_namespace` returns the set of
  workspace-relative `.cs` files declaring a namespace, empty for external
  ones. Deterministic and sorted.
- **Validation:** Unit tests for one namespace across several files, several
  namespaces in one file, `using static` and alias targets (both resolve to the
  target namespace), nested/file-scoped equivalence, BCL drop, unresolvable
  drop, malformed drop, and determinism across two runs.
- **Files:** `crates/anvil-architecture/src/csharp_resolve.rs`,
  `crates/anvil-architecture/src/lib.rs`
- **Dependencies:** DNLAN-009
- **Confidence:** medium — raised from **low**. The index reads declarations
  rather than guessing paths, which removes the failure mode that made this the
  module's blocking risk. What remains is cost and ambiguity handling, not
  correctness-in-principle.
- **Rejected alternatives:** *Folder convention* (`Foo.Bar` ↔ `Foo/Bar/`) and
  *`.csproj` `RootNamespace`* both derive one path from a name. Real C#
  violates the convention freely — extension-method files are the standard
  counter-example — and neither can express one-namespace-many-files at all.
  Nothing else in the module needs a namespace→path derivation: layer
  assignment is path-glob based (the `PYLAN-008` finding), so the index is
  sufficient on its own.

---

#### DNLAN-006: C# layer/boundary enforcement

- **Status:** Draft
- **Intent:** Let layer and boundary rules reach C# namespaces and projects —
  the same enforcement Rust, Python, and TS get.
- **Expected Outcome:** Cross-layer C# `using` directives surface as boundary
  violations with verbatim `.cs` paths; external and BCL imports never do.
- **Set-to-layer collapse:** DNLAN-005 returns a *set* of files, so this item
  owns the collapse to a verdict. All resolved files in one layer → one edge to
  that layer. Files spanning **several** layers → the namespace straddles a
  boundary; **drop conservatively** and do not report, because Anvil cannot
  tell which type the `using` was for without symbol-level resolution. Log the
  straddle behind the existing partial/diagnostic channel so it is measurable
  in DNLAN-008 rather than silent — if straddling turns out to be common, a
  "namespace spans layers" finding is a follow-up module decision, not a
  silent guess here.
- **Validation:** Gate integration tests proving an end-to-end cross-layer C#
  violation, that `using System.*` drops out rather than reporting, and that a
  layer-straddling namespace drops rather than reporting a coin-flip layer.
- **Files:** `crates/anvil-architecture/src/validator.rs`,
  `crates/anvil-cli/src/commands/gate.rs`
- **Dependencies:** DNLAN-005
- **Confidence:** medium — mechanical once DNLAN-005 lands, and blocked on it.
  The straddle case is the one real design call left in this item.

---

#### DNLAN-007: `architecture-validate` includes C# projects

- **Status:** Draft
- **Intent:** Surface C# projects and namespace graphs in `anvil architecture
  validate`.
- **Expected Outcome:** `.cs` files appear in the validate surface and their
  cross-layer edges are reported, with no silent "C# ignored" path.
- **Validation:** Validator tests — `.cs` layer assignment, and a C#
  cross-layer violation through `validate_with_files_and_edges`.
- **Files:** `crates/anvil-architecture/src/validator.rs`
- **Dependencies:** DNLAN-006
- **Confidence:** high — layer assignment is path-glob based, so no language
  gate is needed (the PYLAN-008 finding).

---

### Acceptance

#### DNLAN-008: Dogfood T3 acceptance + FP bar (§16.5 #9)

- **Status:** Draft
- **Intent:** Demonstrate the full C# T3 stack on real-world code at an
  acceptable false-positive rate, before anyone calls C# governed.
- **Expected Outcome:** ≥1 external-codebase run over public OSS C# with an FP
  rate below the agreed N; 0 panics; evidence recorded under `plans/reviews/`.
- **Validation:** TP-vs-FP classification in the evidence note; regression
  tests for every precision fix the run forces; full `anvil-checks` green.
- **Files:** `plans/reviews/YYYY-MM-DD-dnlan-008-external-validation.md`,
  plus whichever `patterns/csharp-reliability/*.anvil` the run corrects
- **Dependencies:** DNLAN-001, DNLAN-004, DNLAN-006
- **Confidence:** medium — the method is proven (RSTLAN-008, PYLAN-009,
  LANGTAIL-008); the unknown is how CS-005 and CS-007 score.
- **Note:** Anvil has no C# of its own, so the "own repo" half of the bar is
  discharged entirely via public OSS, as `PYLAN-009` did.

## Risks

| Risk | Impact | Mitigation |
| ---- | ------ | ---------- |
| ~~Namespace→file resolution is undecidable by convention alone~~ — **resolved 2026-08-06** by the declared-namespace index (DNLAN-005). Residual: the index needs DNLAN-009 first, and layer-straddling namespaces stay unresolvable | Medium (was High) | Read declarations, never guess paths; drop the edge when a namespace is declared nowhere **or** straddles layers. Measure straddle frequency in DNLAN-008 |
| C#'s 6.9% error-tree rate (LANGTAIL-008) degrades AST-tier rules and edge extraction | **Medium–High** — raised: half the catalogue is now AST-tier by decision, so this risk lands on more of the module than when the tier split was regex-first | Treat partial parses as recoverable-symbols-only; measure **both** AST-rule recall and namespace-index coverage on the DNLAN-008 corpus before trusting either. A rule whose recall the error rate visibly guts ships opt-in or not at all |
| A namespace declared in an unparseable file is missing from the index, silently weakening enforcement | Medium | The index is only as complete as the parse. Count files that failed to parse during the pre-pass and surface that count — an unknown-coverage index must not read as a clean bill of health |
| CS-005 `.Result` / `.Wait()` fires on any member named `Result` | Medium | **Resolved by tier choice** — CS-005 is AST-tier, where the receiver's type is knowable. If the type is not resolvable in a given expression, do not fire. Ship opt-in if it still cannot clear the bar |
| Zero confirmed .NET demand means the anchor rots unnoticed | Medium | ADR-118 records the override; re-evaluate at the next §16.5 #8 re-scoring gate. If the FP bar cannot be met on external corpora, stop at T2 and say so |
| ASP.NET / EF patterns leak into the anchor catalogue | Medium | Strict separation — frameworks live in packs, and packs need T2 first |
| "C# support" gets claimed at T2 | **High** — the §8.1 trust-burning failure | Partial anchors are not anchors. The module reports the tier it has actually reached |

## Open Questions

- [x] Is namespace resolution driven by folder convention, `.csproj`
      `RootNamespace`, or an index of declared namespaces? — **Declared-namespace
      index** (owner, 2026-08-06). A `using` imports a namespace, not a file, and
      one namespace routinely spans many files; only the index models that. See
      DNLAN-005, which also records why the other two were rejected.
- [ ] **New, from that decision:** should a namespace whose files straddle two
      layers be a *finding* ("namespace spans layers") rather than a dropped
      edge? Deferred until DNLAN-008 measures how often it happens — a rule
      nobody triggers is not worth the FP surface.
- [ ] How are `global using` directives (C# 10+) and `ImplicitUsings` treated —
      real edges, or noise that should not create edges at all? The archived
      module leaned toward detect-but-do-not-edge.
- [ ] Which OSS C# corpus for DNLAN-008, and what N? PYLAN accepted N = 1%.
- [ ] Does a `.sln` inform project roots for DNLAN-004, or is `.csproj`
      discovery sufficient?
- [ ] Do any candidate rules belong in the existing `guardrail-suppression`
      (CS-001/002/006), `error-visibility` (CS-003), or `type-system-evasion`
      (CS-007) families rather than a new `csharp-reliability` family? RSTLAN
      and PYLAN both chose a per-language family; confirm that still holds.
- [ ] Should `.csx` (C# script — same grammar, currently undetected) come along
      cheaply, or stay out per ADR-118's C#-only boundary?
