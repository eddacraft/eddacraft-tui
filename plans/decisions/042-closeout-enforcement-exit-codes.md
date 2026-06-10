# ADR-042: Closeout-Enforcement Checks Exit Non-Zero on Violation

## Status

Proposed

## Date

2026-05-12

## Context

ADR-002 ("Warnings Over Blocks") is one of Anvil's core philosophy ADRs. It
states verbatim:

> Anvil warnings do **not** block by default. Exit code 0 for warnings, non-zero
> only for errors (schema failures, crashes).
>
> CI integration offers opt-in `fail-on-warnings: true` for teams that want
> enforcement.

That decision governs how Anvil reports findings about **user code** — lint-like
signal that authors should see but not be blocked on. It deliberately frames
non-zero exit as reserved for schema failures and crashes so that adoption is
not gated on a "fix everything first" cliff.

DOCGOV-005 introduces `pnpm docs:check`, a documentation validation baseline
that fails CI on any violation (subject to the new-edges-only baseline from
ADR-003). On the surface this contradicts ADR-002: the new command will exit
non-zero on what could be characterised as documentation warnings.

Two earlier closeout-time integrity commands have already shipped with
hard-fail semantics — `pnpm adr:check` (DOCGOV-004) and `pnpm aps:drift` (the
APS module/index drift checker) — without a written rule explaining why they
are allowed to break the ADR-002 default. `docs:check` would be the third such
command, and `pnpm docs:index:check` (DOCGOV-007) is queued behind it. Without
a named carve-out, every new closeout check repeats the same implicit
argument, and the scope of ADR-002 keeps quietly drifting.

The decision below scopes ADR-002 to its original domain and names the
carve-out, rather than overriding ADR-002 or weakening the docs-check contract.

## Decision

**Closeout-enforcement checks** are a named category of validation that exits
non-zero on violation by design. The category is defined by three properties
together:

1. The check enforces a structural invariant of the repository's own planning
   or documentation artefacts (not user product code).
2. The check runs at closeout time — pre-commit, pre-PR, or CI — to keep those
   artefacts internally consistent across the lifetime of the repository.
3. The check is opt-out via the new-edges-only baseline from ADR-003, not via
   `--strict` flags. A violation against the baseline is a real regression of
   a property that was previously true.

### D-1: Current closeout-enforcement checks

The following commands are closeout-enforcement checks under this ADR and are
authorised to exit non-zero on violation:

- `pnpm adr:check` — ADR numbering and DECISION-LOG coverage integrity
  (DOCGOV-004).
- `pnpm aps:drift` — APS module / `plans/index.aps.md` consistency.
- `pnpm docs:check` — documentation metadata, tags, internal links, APS/index
  consistency, ADR integrity, generated-index freshness, as-built source
  references (DOCGOV-005 and queued sub-surfaces in DOCGOV-006 / DOCGOV-007).

Future commands joining this category MUST cite ADR-042 in the task or PR
that introduces them and MUST satisfy all three properties above.

### D-2: ADR-002 scope is runtime warnings on user code

ADR-002 continues to govern any Anvil output that classifies findings against
**user-authored product code, configuration, or infrastructure**. The default
"exit 0 for warnings, opt-in `fail-on-warnings: true`" contract is unchanged
for those surfaces. Adoption-friendliness, suppression syntax (ADR-004), and
new-edges-only baselining (ADR-003) all continue to apply there.

This ADR scopes ADR-002 rather than overriding it. The boundary is: ADR-002
applies when Anvil reports about *the user's work*; ADR-042 applies when a
closeout-enforcement check reports about *Anvil's own planning and
documentation invariants*.

### D-3: New-edges-only stays mandatory

A closeout-enforcement check that does not honour ADR-003's baselining
discipline is not eligible to claim the carve-out. `docs:check` ships with
`docs/governance/docs-check.baseline.json` capturing the current corpus state;
only net-new violations fail the check. Future closeout-enforcement checks
must ship a comparable baseline or justify in their introducing ADR why the
invariant is binary (no legacy state to baseline).

## Rationale

### Alternatives Considered

| Option | Pros | Cons |
|--------|------|------|
| Name the carve-out (chosen) | Resolves the apparent ADR-002 contradiction once; future closeout checks cite this ADR rather than re-arguing the point; preserves ADR-002's runtime contract intact | Adds a fourth core-philosophy ADR; authors must remember to cite ADR-042 when adding closeout checks |
| Make `docs:check` warn-only with `--strict` opt-in | Literal ADR-002 compliance; zero new ADR needed | Reproduces the failure mode ADR-002 was written to prevent in the *opposite* direction — closeout checks that nobody enables in CI; defeats the point of DOCGOV-005 |
| Reframe metadata violations as schema errors to inherit ADR-002's "schema failures and crashes" clause | No new ADR required | Stretches "schema failure" past breaking point — internal-link breakage, tag-catalogue divergence, and as-built source drift are not schema failures in any normal reading; would license arbitrary checks to claim the same exemption with no boundary |
| Override ADR-002 wholesale | Simplifies the rule surface | Throws away a deliberately-designed adoption strategy for user code; ADR-002's domain is healthy and unchanged |

### Why this option

ADR-002 was written with a specific audience and failure mode in mind:
developers facing a wall of warnings on legacy code, choosing to disable
checks rather than triage them. That dynamic does not apply to documentation
or planning invariants in this repository. The corpus is small, the
maintainers are the authors, and the new-edges-only baseline already absorbs
the legacy state without requiring a flag.

Naming the carve-out, rather than letting it accumulate by implication
through `adr:check` and `aps:drift`, makes the boundary inspectable. Any
future closeout-enforcement check is forced to cite ADR-042 and demonstrate
it meets the three properties — that is a cheap design review the repository
can run mechanically.

## Consequences

- **Positive:** `pnpm docs:check` (and the queued `docs:index:check`) can
  exit non-zero on regression without contradicting Anvil's core philosophy.
  The contract is explicit rather than implicit.
- **Positive:** ADR-002's domain is sharpened. "Warnings over blocks" is now
  visibly about user-code warnings, not about every command Anvil ships.
- **Positive:** Existing `adr:check` and `aps:drift` hard-fail behaviour is
  retrospectively legitimised by a written rule, removing a latent
  inconsistency.
- **Negative:** A fourth core-philosophy ADR raises the surface area authors
  must read before adding new commands. Mitigated by the explicit
  three-property test and the requirement to cite ADR-042.
- **Risks:** A future contributor adds a user-code check that claims the
  closeout-enforcement carve-out to avoid implementing the warn-then-opt-in
  pattern. Mitigation: D-1 requires citation in the introducing ADR, and the
  three-property test in the Decision section excludes anything reporting on
  user product code.
- **Risks:** Baseline drift — closeout-enforcement checks accumulate large
  baselines that never shrink, and the "new edges only" framing becomes
  cover for unbounded legacy debt. Mitigation: DOCGOV-008 is already
  scheduled to shrink the `docs:check` baseline; future closeout checks
  should ship a comparable shrink task.
- **Mitigations:** When a fourth closeout-enforcement command is proposed,
  revisit whether the three-property test is still discriminating enough or
  whether a sharper definition is needed.

## References

- Related ADRs:
  - [ADR-002](002-warnings-over-blocks.md) — runtime warning contract that this
    ADR scopes rather than overrides
  - [ADR-003](003-new-edges-only.md) — baseline-then-warn-on-new-violations
    discipline that closeout-enforcement checks must inherit
  - [ADR-004](004-suppression-syntax.md) — suppression syntax for ADR-002's
    runtime domain (does not apply to closeout-enforcement checks)
- APS modules:
  - [DOCGOV](../archive/modules/documentation-governance.aps.md) — DOCGOV-004 (ADR
    integrity, first closeout-enforcement check), DOCGOV-005 (this ADR's
    triggering task), DOCGOV-006 / DOCGOV-007 (queued sub-surfaces)
- Execution plan: [DOCGOV-005.steps.md](../archive/execution/DOCGOV-005.steps.md)
