---
name: dev-loop-module
description: >-
  Module topology for the canonical agentic loop. A module owns integrated
  acceptance and close verification; publication may be one PR, a stack, or
  per-item according to policy. Supports calibration and bounded parallel waves.
---

# Module delivery topology

Load `agentic-loop` first. This skill does not define a separate lifecycle; it
adds module-specific planning, calibration, execution topology, and integrated
close verification.

## Invocation

```text
/dev-loop-module complete DASH
/dev-loop-module resume DASH
```

## Resolve the module

Resolve the governing module contract, children, dependency DAG, cross-child
invariants, integration tests, risk/reasoning, and publication policy.

Publication topology is separate from module scope:

- `one-pr` — tightly coupled coherent module;
- `stack` — ordered changes with useful review boundaries;
- `per-item` — independently landable children;
- `auto` — choose from dependency/size/risk/repository policy.

Do not assume a module means one PR.

## Design / planning challenge

Run `council design` when module reasoning is frontier, risk is high/critical,
decomposition is materially ambiguous, migration/trust boundaries exist, or
policy requires it. Challenge acceptance completeness, child boundaries,
dependencies, interfaces/invariants, test strategy, and landing topology before
burning implementation tokens.

## Calibration slice

For a non-trivial module, select an early representative child/slice before
broad drain. Implement it using the active native lead by default, run focused
evidence and fresh verification, and optionally assurance-Council it when risk
warrants.

If the calibration attempt is defect-dense or reveals plan/decomposition flaws,
stop the module drain and return `REWORK`/`REPLAN`. Do not produce the remaining
children with a known-bad execution pattern.

If quality is healthy, continue with lighter per-child focused evidence and the
configured verification sampling policy.

## Execution

Saying `complete <MODULE>` is the Ready grant for that module. Children are
`plan-ready`'d as they become unblocked. Child Ready is a mechanical plan-file
update, not a publication event and not a second human sitting. Do not open a Ready PR.
`drain` still must not promote items to Ready.

After each child, continue immediately to the next unblocked child in the same
run. Child completion is not a terminal outcome. Continuation follows `agentic-loop`.

Serial execution is the safe default, not an invariant. Bounded parallel waves
are allowed only when dependencies and write ownership prove independence.

Gating follows the selected publication topology, not the child count:

- `one-pr` — isolate **once** for the module and gate once at close.
  Do not isolate per child.
  No verify-loop, no council, no CI, no push per child.
  Do not push or open the PR until module close.
  Re-run `plan-ready` for children as they become unblocked.
- `stack` / `per-item` — each published boundary is a real candidate and carries
  its own evidence, verification, and merge gates.

- Each parallel writer gets its own worktree/write surface and child lease.
- Default maximum writers remains two unless repository policy chooses another
  finite limit.
- Workers are optional; the lead may implement serial/coherent children directly.
- Child completion messages never advance APS by themselves.
- Integrate child commits in dependency order and run focused integration checks
  after each wave.

## Partial landing

Stop the run at a real block. Land what already has focused evidence, and only
when the module/feature-flag strategy explicitly identifies that subset as
independently useful and safe to ship. Otherwise park a branch/draft PR and keep
the module blocked.

- The blocked child stays `Blocked`.
- Do not skip ahead to later slices after the block, even independent ones. The
  block is a signal about the module, not one child.
- If the first child is the block, return `blocked` with nothing landed.

## Module close

Close the module **once**. Run one blind integrated `verify-loop` against:

- the module contract;
- the union of the slices, not each slice again;
- cross-child invariants;
- the integrated candidate;
- required integration validation.

Reuse the focused evidence the children already produced.
Do not invent a second full workspace suite at close.
Run the integration validation the module contract actually requires.

Council is not automatic. Run `council assurance` only when risk, a material
dispute, defect signal, release boundary, or policy requires broader review.

Council finding handling follows `agentic-loop`: small bounded misses may
`REPAIR`; structural/defect-dense attempts `REWORK`; bad intent/decomposition
`REPLAN`.

Convergence is bounded: one repair pass, then delta-verify the changed surface.
No second council pack. No third verify. A further full pass requires material
redesign, a scope change, a critical finding, a new material verifier concern,
or explicit policy or user request.

## Publish and land

Publish according to the selected topology. Every candidate that is actually
landed must satisfy the canonical revision-bound verification and merge gates
from `agentic-loop`: CI green, no merge conflicts, no conflict markers, no open
blocking review threads, the verified head equal to the PR head, and the
`land-branch` pre-merge probe.

Process the review sources the repository configures.
Do not hard-code any one bot as the done bar, and do not wait for a second round
from one that is first-push only.

Completed children become `Merged` only after integration ancestry is proven.

## Outcomes

`review-ready`, `integrated`, `blocked`, `needs-plan-update`, `claim-conflict`,
`awaiting-merge-authority`, `repair-budget-exhausted`.
