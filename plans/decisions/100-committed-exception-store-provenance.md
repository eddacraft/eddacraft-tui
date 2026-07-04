# ADR-100: Committed Exception-Store Provenance at the L4 Gate

## Status

Accepted (owner decision 2026-07-04)

## Date

2026-07-04

## Context

The L4 gate (the pre-push hook and `anvil l4-validate`) applies tracked policy
exceptions (ADR-073) when validating commits. Since EXCEPT-006 wired that
evaluation, the gate has read the exception store from the **live worktree**
(`ExceptionStore::load(repo_root)`), the same way `anvil/policy.yml` is
loaded.

The 2026-07-04 EXCEPT-006 council flagged this as a trust-model gap
(EXCEPT-010): an **uncommitted, never-pushed** grant written to
`anvil/exceptions/store.json` satisfies the author's own local pre-push gate,
leaves no trace in the pushed refs, and can be deleted after the push. CI sees
only committed grants, so local and CI evaluations of the same push can
disagree by design. Range validation compounds the ambiguity: one worktree
snapshot applies to every commit in the range regardless of what is checked
out when `l4-validate <base>..<head>` runs.

This contradicts two standing principles. ADR-073's thesis is that exceptions
are durable, PR-reviewable governance state — suppression authority that never
enters review is exactly what it exists to end. And the project's
determinism principle (same input, same output) is violated when a gate
verdict depends on uncommitted local state.

The local hook is advisory by construction (`--no-verify` bypasses it), so
this is not primarily a hardening question — it is a semantics question:
*what state is allowed to authorise suppression, everywhere the gate runs?*

## Decision

The L4 gate reads the exception store from the **tree of the pushed range's
tip commit**, not from the worktree:

- The pre-push hook resolves the store from `local_sha` (the commit being
  pushed) for each push ref.
- `anvil l4-validate <base>..<head>` resolves it from `<head>`.
- A grant therefore applies only if it is **committed somewhere in the
  history being published**. A grant commit at the tip covers earlier commits
  in the same push, so the natural workflow — push blocked → grant → push
  again — is unchanged.
- A tip with no store file evaluates as an empty store (no exceptions apply;
  findings stand — fail-safe). An unreadable or oversized store blob likewise
  applies no exceptions.
- The legacy local `.anvil/exceptions.json` fallback consequently never
  influences gate evaluation: it cannot exist in a tree. Legacy grants must
  be promoted via `anvil exception migrate` and committed to count.

The principle, stated once for future surfaces: **configuration may be
local; authority must be committed.** `anvil/policy.yml` (configuration:
which rules run, how branches route) deliberately keeps worktree loading and
is out of scope here; the exception store (authority: which findings are
suppressed) must be committed.

## Rationale

Tip-scoped loading is the only option that is simultaneously deterministic,
workflow-compatible, and closes the council's finding:

- **Local/CI parity.** Both surfaces evaluate the identical committed store.
  A forgotten `git add anvil/exceptions/store.json` blocks locally with an
  actionable message instead of surprising the author in CI.
- **Determinism.** The verdict is a pure function of the pushed ref —
  checkout state, stashes, and uncommitted edits stop mattering, and range
  validation always means "the head's store".
- **Closes the finding.** An effective grant is necessarily in the pushed
  history, i.e. visible in the PR diff a reviewer approves — the ADR-073
  review-is-the-approval model, now enforced rather than assumed.

### Alternatives Considered

| Option | Pros | Cons |
| ------ | ---- | ---- |
| Tip-of-pushed-range tree (chosen) | Deterministic; local/CI parity; grant-after-finding workflow intact; uncommitted-grant hole closed | Diverges from `policy.yml` loading (mitigated by the stated principle); legacy fallback drops out of gate evaluation |
| Live worktree (status quo) | Matches `policy.yml`; zero change | Verdict depends on uncommitted state; designed-in local/CI divergence; the council's zero-trace self-grant PoC stays live |
| Per-commit tree | Strictest per-commit auditability | Breaks the grant-after-finding workflow permanently (grant commits follow the code they cover; only history rewriting could satisfy it) |
| Tiered (worktree locally, tip in CI) | Preserves worktree convenience locally | Re-introduces local/CI divergence as the design; two semantics to document and test; convenience is marginal (`anvil exception verify` needs no push) |

## Consequences

- **Positive:** gate verdicts are reproducible from the pushed ref alone;
  suppression authority is always PR-visible. (Capsule create/verify still
  read the worktree store — aligning them with tip-scoped semantics is a
  follow-up, not delivered here.)
- **Negative:** operators must commit the store before a grant takes effect
  at the gate (the CLI's grant success message and the operator guide say
  so); unmigrated repos lose silent legacy-grant influence on gates — an
  intended, documented behaviour change.
- **Risks:** call sites that forget to thread the tip SHA would evaluate no
  exceptions.
- **Mitigations:** the tip field is explicit at both call sites and the
  fail-safe direction (no exceptions → findings stand) means a threading bug
  blocks rather than silently admits; regression tests pin the
  uncommitted-grant refusal and the grant-at-tip-covers-range workflow.

## References

- Related ADRs: ADR-073 (durable vs local anvil state), ADR-002 (warnings
  over blocks), ADR-098 (policy enforcement reset gate)
- APS modules: EXCEPT-010 (this decision), EXCEPT-006 (gate wiring),
  EXCEPT-004 (CLI), EXCEPT-009 (capsule inclusion)
- Council: 2026-07-04 EXCEPT-006 adversarial review (uncommitted self-grant
  PoC)
