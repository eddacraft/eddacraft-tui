<!-- APS: See https://github.com/eddacraft/anvil-plan-spec for format reference -->
<!-- Executable only if tasks exist and status is Ready. -->

# C# / .NET Language Anchor (Track 1)

| ID    | Owner | Status |
| ----- | ----- | ------ |
| DNLAN | —     | Draft  |

**Last reviewed:** 2026-08-06 (module created on owner direction under
[ADR-118](../decisions/118-csharp-anchor-promotion-t2-t3.md), promoting C# out
of the Track 2 tail. **Demand is 0** — the §8.2 promotion lever ("first .NET
user") has not fired; the ADR records that override honestly. No work item has
started; the module is Draft until the Ready Checklist below is met.)

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
catalogue, suppression coverage, entry-point detection, namespace→file import
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

  | Rule | Pattern | Default |
  |---|---|---|
  | CS-001 | `#pragma warning disable` without a justification comment | on |
  | CS-002 | `[SuppressMessage(...)]` without a `Justification:` argument | on |
  | CS-003 | Empty or blanket swallow — `catch { }` / `catch (Exception) { }` | on |
  | CS-004 | `async void` outside an event-handler signature | on |
  | CS-005 | Sync-over-async — `.Result` / `.Wait()` / `.GetAwaiter().GetResult()` | on |
  | CS-006 | `// ReSharper disable` without justification | on |
  | CS-007 | `dynamic` type usage (the C# parallel of TS `any`) | opt-in |
  | CS-008 | `Console.Write` / `Console.WriteLine` in non-console projects | opt-in |

  CS-004 and CS-005 are additions to the archived placeholder's list, on blast
  radius: `async void` loses exceptions to the synchronisation context, and
  sync-over-async deadlocks production services. CS-007/CS-008 ship opt-in for
  the same noise reason `PY-006`/`PY-007` did.

- **Suppression syntax** — `// @anvil-ignore CS-NNN -- <reason>`. The ADR-029
  parser already accepts the `//` prefix, so this is coverage and proof, not
  new parsing (the `PYLAN-004` shape).
- **Entry-point detection** — `Program.cs` top-level statements,
  `static void Main` / `static async Task Main`, and `.csproj`
  `<OutputType>Exe</OutputType>`, mirroring `detect_rust_entry_points`
  (RSTLAN-004) and `detect_python_entry_points` (PYLAN-005).
- **Namespace→file import resolution** — map a `using` target to a
  workspace-relative `.cs` file, the C# parallel of `resolve_rust_import` /
  `resolve_python_import`. Unresolvable targets (BCL, NuGet, namespaces
  declared nowhere in the tree) return `None` and the edge is **dropped**.
- **Layer/boundary enforcement** reaching C# namespaces and projects.
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
  [`lang-python`](./lang-python.aps.md) — the resolver, entry-point, catalogue,
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

- [ ] Owner named for the anchor work.
- [ ] Catalogue tier decided — regex/RE2 (the `PYLAN-003` default, save-time
      safe) vs the ADR-071 AST tier — per rule, with CS-005's `.Result`
      ambiguity resolved either way.
- [ ] FP-bar **N** agreed for C# (PYLAN accepted N = 1%), and the external
      corpus named. C# measured **6.9% error-trees** in `LANGTAIL-008`; confirm
      that recovery rate is good enough for the resolver before committing.
- [ ] Namespace→file resolution strategy decided — folder convention,
      `.csproj` `RootNamespace`, or declared-namespace index (see Open
      Questions).
- [ ] T3 acceptance checklist re-read and confirmed applicable unchanged.

## Work Items

All items are **Draft**. None may start before the module is Ready.

The T2 slice is DNLAN-001..004; the T3 slice is DNLAN-005..007; DNLAN-008 is
wave-level acceptance. Per §8.1 there are **no partial anchors** — shipping
DNLAN-001..004 alone is C#-at-T2 and must be described as such, never as
".NET support".

#### DNLAN-001: C# T2 anti-pattern catalogue

- **Status:** Draft
- **Intent:** Govern the C#-specific anti-patterns that carry real blast
  radius, not merely the ones that are easy to match.
- **Expected Outcome:** A `csharp-reliability` family fires on C# via `anvil
  check` / `gate` / drift and the save-time daemon; the noisy rules ship
  opt-in. Final rule set is decided here against the candidate table above.
- **Validation:** Per-rule positive and justified-negative tests; RE2-compile
  clean (lookahead is dropped **silently** — the `PYLAN-003` trap); extension
  scoping to `.cs`; opt-in gating asserted; `registry_compile_diagnostics`
  regression guard.
- **Files:** `patterns/csharp-reliability/*.anvil`,
  `patterns/compiled/registry.json`,
  `crates/anvil-checks/src/antipattern/types.rs`,
  `crates/anvil-checks/tests/csharp_antipatterns.rs`
- **Dependencies:** —
- **Confidence:** medium — rule selection is settled, per-rule tier is not.

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

#### DNLAN-005: C# namespace→file import resolution

- **Status:** Draft
- **Intent:** Turn a `using` target into a file the boundary checker can reason
  about, without ever inventing an edge.
- **Expected Outcome:** `resolve_csharp_import` maps a namespace to a
  workspace-relative `.cs` file. BCL / NuGet / declared-nowhere namespaces
  return `None` and the edge is dropped — conservative, never a false boundary
  violation.
- **Validation:** Unit tests for folder-convention resolution, `RootNamespace`
  override, a namespace declared across several files, `using static` and alias
  targets, BCL drop, unresolvable drop, and malformed-input drop.
- **Files:** `crates/anvil-architecture/src/csharp_resolve.rs`,
  `crates/anvil-architecture/src/lib.rs`
- **Dependencies:** —
- **Confidence:** **low** — the hardest item here. C# namespaces are
  conventionally but not necessarily folder-aligned, one file may declare
  several, and several files routinely share one. This is strictly harder than
  the Rust module tree or Python package path.

---

#### DNLAN-006: C# layer/boundary enforcement

- **Status:** Draft
- **Intent:** Let layer and boundary rules reach C# namespaces and projects —
  the same enforcement Rust, Python, and TS get.
- **Expected Outcome:** Cross-layer C# `using` directives surface as boundary
  violations with verbatim `.cs` paths; external and BCL imports never do.
- **Validation:** Gate integration tests proving an end-to-end cross-layer C#
  violation, and that `using System.*` drops out rather than reporting.
- **Files:** `crates/anvil-architecture/src/validator.rs`,
  `crates/anvil-cli/src/commands/gate.rs`
- **Dependencies:** DNLAN-005
- **Confidence:** medium — mechanical once DNLAN-005 lands, and blocked on it.

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
| Namespace→file resolution is undecidable by convention alone (DNLAN-005) | **High** — without it, DNLAN-006/-007 cannot land, and T3 is unreachable | Resolve conservatively: build an index of *declared* namespaces from the symbol graph rather than guessing from folders; drop the edge on ambiguity. Decide the strategy at Ready, not mid-item |
| C#'s 6.9% error-tree rate (LANGTAIL-008) degrades AST-tier rules and edge extraction | Medium | Prefer the regex tier; treat partial parses as recoverable-symbols-only; measure resolver coverage on the DNLAN-008 corpus before trusting it |
| CS-005 `.Result` / `.Wait()` fires on any member named `Result` | Medium | Code-scope the rule (`rule_is_code_scoped`, the PYLAN-009 fix), require `await`-adjacent or `Task`-typed context, or escalate to the AST tier; ship opt-in if it cannot clear the bar |
| Zero confirmed .NET demand means the anchor rots unnoticed | Medium | ADR-118 records the override; re-evaluate at the next §16.5 #8 re-scoring gate. If the FP bar cannot be met on external corpora, stop at T2 and say so |
| ASP.NET / EF patterns leak into the anchor catalogue | Medium | Strict separation — frameworks live in packs, and packs need T2 first |
| "C# support" gets claimed at T2 | **High** — the §8.1 trust-burning failure | Partial anchors are not anchors. The module reports the tier it has actually reached |

## Open Questions

- [ ] Is namespace→file resolution driven by folder convention, `.csproj`
      `RootNamespace`, or an index of declared namespaces built from the symbol
      graph? (Blocks DNLAN-005; the third is the most robust and the most work.)
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
