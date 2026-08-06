# ADR-118: C# / .NET promotion from tail T1 to anchor T2/T3

## Status

**Accepted** — 2026-08-06, Josh (owner). Records an owner-directed **promotion
of C# / .NET out of the Track 2 tail** (where it sits at T1, Parsed) into
Track 1 anchor work targeting **T3 (Governed)**, realised as a new active
module [`lang-dotnet-anchor`](../modules/lang-dotnet-anchor.aps.md) (scope
`DNLAN`).

## Date

2026-08-06

## Context

C# / .NET shipped at **T1 (Parsed)** as `LANGTAIL-006` in the Track 2 tail wave
(Merged 2026-06-18 via PR #2757): `tree-sitter-c-sharp` 0.23.5 bound,
`.cs` detected by `Language::from_path`, and
`crates/anvil-kernel/src/parser/extract/csharp.rs` emitting types
(class/struct/record → `Class`, interface, enum), qualified methods, and one
`ImportEdge` per `using` directive — including `using static` and
`using Alias = Target` (alias resolves to the target, not the alias).

The [2026-04-08 Language and Coverage Design](../specs/2026-04-08-language-and-coverage-design.md)
§8.2 is explicit that the tail wave stops there: *"No per-language anti-pattern
catalogues, no suppression syntax, no policy hooks. Just 'appears in the
graph'."* Promotion out of the tail is a **separate module decision triggered
by a demand signal**, and the recorded promotion lever for C# is literally
**"First .NET user"**.

A dedicated `lang-dotnet` module (scope `DNLAN`) predates the tail wave. It was
[archived 2026-04-22](../archive/modules/lang-dotnet.aps.md) — demand point 0,
its anchor-shaped Task Status entry retired, its content folded into
`lang-tail-wave` — and it survives only as a historical record of the intended
T2 scope (`#pragma warning disable`, `[SuppressMessage]`, `dynamic`, empty
`catch`, `Console.Write`, ReSharper directives, `.csproj` entry points,
namespace→folder boundary mapping).

This ADR exists so the promotion is **not silent**. §13's re-entry rule and
§8.2's promotion lever both require a demand signal, and the honest position is
that there is not one.

### §6 re-score (honest)

| Candidate | Demand | Blast | Strategic | Pack unlock | Note |
|---|---|---|---|---|---|
| C# / .NET | **0 confirmed** (unchanged since 2026-04-08; Anvil has no `.cs` files; no early-access user has brought a .NET codebase) | High — enterprise service code, and the sync-over-async / `async void` failure modes ship silent production deadlocks and swallowed exceptions | supports ("governs enterprise stacks"); weaker than the Python AI/ML narrative | +1 potential (ASP.NET, per §8.2's pack-potential column) | **Owner-directed.** The §8.2 promotion lever ("First .NET user") has **not** fired. |

C# does **not** clear the §6 bar on demand alone. It proceeds because the owner
has authority to override the tail's promotion gate; this record makes that
override explicit rather than dressing it as user demand. Consistent with the
[ADR-093](093-tail-wave-2-wasm-text-and-zig-reentry.md) precedent for the
owner-directed WAT/Zig additions.

The one material difference from ADR-093: that ADR added languages at **T1**,
which is cheap and batched. This promotes a language to **T3**, which is anchor
work — the heaviest per-language commitment Anvil makes. The cost is real and
is accepted knowingly.

## Decision

1. **Promote C# / .NET from Track 2 tail (T1) to Track 1 anchor work targeting
   T3 (Governed)**, superseding §8.2's "first .NET user" promotion lever for
   this language only. The lever stays in force for Dart, Go, Java, Kotlin,
   and C/C++.

2. **Create a new active module** `plans/modules/lang-dotnet-anchor.aps.md`,
   reusing scope **`DNLAN`**. The archived `lang-dotnet.aps.md` is **not**
   un-archived — it stays the historical record and gains a forward pointer,
   mirroring how [`lang-zig.aps.md`](../archive/modules/lang-zig.aps.md) was
   handled under ADR-093. The new module carries a distinct slug so the
   `modules/<slug>.aps.md` suffix match in `scripts/aps/index-counts.mjs`
   cannot resolve ambiguously against the archived path.

3. **Acceptance is the T3 checklist produced by `LANGTS`**, the same bar Rust
   (`RSTLAN`) and Python (`PYLAN`) passed. Per §8.1: *"No partials."* A state
   where the catalogue ships but boundary enforcement is unwired is **not**
   C#-at-T3, and must not be described as .NET support.

4. **The §16.5 #9 FP bar applies.** C# reaches T3 only after an
   external-codebase validation run with a false-positive rate below the
   accepted N (N = 1% for `PYLAN-009`), with evidence recorded under
   `plans/reviews/`. Anvil has no C# of its own, so the "own repo" half of the
   bar is discharged via public OSS, exactly as `PYLAN-009` did.

5. **Scope stays C#-only.** `.vb` and `.fs` are not in scope, are not detected
   by `Language::from_path` today, and do not enter on the back of this ADR —
   the archived module's own "start with C# only" boundary is retained.

## Rationale

The alternative framings were considered and rejected:

### Alternatives Considered

| Option | Pros | Cons |
|--------|------|------|
| **Chosen: promote to a dedicated T2/T3 anchor module under an ADR** | Honest about the missing demand signal; acceptance bound to the existing T3 checklist and FP bar; matches the RSTLAN/PYLAN shape reviewers already know | Anchor-tier cost for a language with 0 confirmed users; adds a fourth anchor to maintain |
| Leave C# at T1 until a .NET user appears | Costs nothing; respects §8.2's lever exactly as written | Declines an explicit owner directive; "we support .NET" would keep meaning "we parse it", which §8.1 names as the trust-burning failure mode |
| Add a T2 catalogue only, stop short of T3 | Cheaper; catches the high-blast C# bugs | §8.1 forbids partial anchors — policy, drift, and boundary silently not applying to `.cs` while the language is billed as governed is precisely the Tier-1-labelled-Tier-3 failure |
| Un-archive `lang-dotnet.aps.md` in place | Reuses the file and its ID naturally | Its scope predates tree-sitter (regex `using` extraction, retired `HTMLCSS-001` prerequisite); a same-basename file in both `modules/` and `archive/modules/` is ambiguous to the index-count suffix match |
| Fold C# T2/T3 into a third tail wave | Amortises harness cost | Category error — waves are a T1 batching device (§8.2); there is nothing to batch with |

## Consequences

- **Positive:** ".NET support" stops being a T1 claim dressed as a capability.
  C#'s highest-blast failure modes (sync-over-async deadlocks, `async void`
  exception loss, blanket `#pragma warning disable`) become governable. Unlocks
  a future ASP.NET pack, which §8.4 requires substrate ≥ T2.
- **Negative:** A fourth anchor to keep current, funded by owner direction
  rather than demand. Anvil has no C# to dogfood against, so every FP
  measurement depends on external corpora.
- **Risks:**
  - **Parse quality.** `LANGTAIL-008`'s external validation measured C# at
    **6.9% error-trees** over real OSS — clean enough for T1 symbol recovery,
    but the highest of the four clean-parsing tail languages. Boundary
    resolution and any AST-tier rule inherit that error rate.
  - **No namespace→file resolver exists.** C# namespaces are conventionally
    but not necessarily folder-aligned, and one file may declare several
    namespaces. This is strictly harder than the Rust and Python resolvers.
  - **FP-prone rules.** `.Result` / `.Wait()` matches any property named
    `Result`, not only `Task.Result`; `dynamic` and `Console.Write` are noisy
    by nature.
  - **Zero-demand drift.** With no user, the anchor can rot unnoticed.
- **Mitigations:**
  - Default to the **regex/RE2 tier** (the `PYLAN-003` precedent, save-time
    safe) and escalate only the rules that fail the FP bar to the ADR-071
    AST tier; rules that cannot clear the bar at either tier ship **opt-in**
    or not at all.
  - Resolver returns `None` and **drops the edge** when a namespace resolves
    nowhere — conservative, never a false boundary violation (the
    `PYLAN-006` rule).
  - The FP bar (consequence 4 above) is the gate, not a follow-up.

## References

- Related ADRs: [ADR-093](093-tail-wave-2-wasm-text-and-zig-reentry.md)
  (owner-directed language additions; §6 re-score precedent),
  [ADR-029](029-suppression-parser-authority.md) (suppression parser — already
  handles `//`), [ADR-071](071-ast-aware-antipattern-detection.md) (AST
  anti-pattern tier)
- APS modules: `DNLAN` ([`lang-dotnet-anchor`](../modules/lang-dotnet-anchor.aps.md)),
  `LANGTAIL-006` ([`lang-tail-wave`](../modules/lang-tail-wave.aps.md)),
  `PYLAN` ([`lang-python`](../modules/lang-python.aps.md)),
  `RSTLAN` ([archived](../archive/modules/lang-rust.aps.md)),
  archived [`lang-dotnet`](../archive/modules/lang-dotnet.aps.md)
- Spec: [2026-04-08 Language and Coverage Design](../specs/2026-04-08-language-and-coverage-design.md)
  §5.1 (tiers), §6 (scoring), §8.1 (anchors), §8.2 (tail + promotion levers),
  §13 (cut list), §16.5 #9 (FP bar)
- Evidence: [`plans/reviews/2026-06-18-langtail-008-external-validation.md`](../reviews/2026-06-18-langtail-008-external-validation.md)
