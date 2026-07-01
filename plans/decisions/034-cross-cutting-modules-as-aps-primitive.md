# ADR-034: Cross-cutting modules as a first-class APS primitive

## Status

Accepted

## Date

2026-04-30

## Context

APS originally modelled work as a tree of bounded modules: each module owns a
domain surface, owns its tasks, and archives independently. The first
cross-cutting bundle in this repository — `launch-flow-readiness` (LAUNCH) —
needed to coordinate work across `RCLI`, `KERN`, `TUIDASH`, and `DRVR` without
owning any of those surfaces. LAUNCH solved the gap by carrying a local
"cross-cutting convention" section in its module body and inventing three
prose callout types (`Coordinates with:`, `Blocks on:`, `Superseded by:`) for
references into other modules.

That convention was deliberately ring-fenced ("do not copy this to a second
module before it has been tried in anger here"). LAUNCH is now 5/7 with five
items closed (LAUNCH-001, -003, -004, -005, -007), and the three audit findings
about cross-references that came up during those closes were each handled
correctly by the closer. The pattern works in anger.

A second cross-cutting module is now needed. The Planning Council session
plan-b00c16c7 (2026-04-30) decided to split observability into three: OBS
(domain ops module, deferred), TRACE (cross-cutting tracing foundation,
launch-blocker), and EXPORT (sink choice, deferred). TRACE is the second
cross-cutting bundle — it touches `anvil-intercept`, `anvil-cli`,
`anvil-api`, and the dashboard ops surface, and owns no surface itself.

Two paths were on the table:

1. Copy LAUNCH's convention block into TRACE verbatim and keep the trial
   running module-by-module.
2. Promote the convention to `aps-rules.md` so a single normative spec covers
   every cross-cutting module that follows.

LAUNCH's own ring-fence said the trigger to promote is "at the point a second
author is tempted to copy" — and that is where this ADR is being written.

## Decision

Promote the cross-cutting module convention to a first-class APS primitive in
`plans/aps-rules.md` under a new `## Cross-Cutting Modules` section. The
section reads as follows:

> A cross-cutting module coordinates work that touches multiple domains
> without owning a single product surface. Such modules MUST follow these
> rules:
>
> 1. **Owns its own work items** — every cross-cutting task is owned and
>    counted by the cross-cutting module, never by the surfaces it touches.
> 2. **Cross-references via prose callouts** — use `Coordinates with:`,
>    `Blocks on:`, `Supersedes:`, and `Superseded by:` in task bodies. Use
>    `Supersedes:` when the current task replaces an older item; use
>    `Superseded by:` when the current task is replaced by a newer item. No
>    typed relations, no separate dependency graph. (`Blocks on:` is
>    currently provisional — to be hardened once exercised in a completed
>    task.)
> 3. **Closer sweeps callouts on task completion** — whoever closes a task
>    with cross-ref callouts MUST read each one in the body and either
>    resolve it (reference is now correct), downgrade it (e.g. `Blocks on:`
>    → `Coordinates with:`), or document the rationale and **close the
>    callout in the same edit**. Documenting MUST NOT defer the callout
>    into the archive.
> 4. **Closer sweeps all open callouts at archive time** — when a
>    cross-cutting module is archived (via `git mv` to
>    `plans/archive/modules/`), the closer sweeps every remaining open
>    callout in the module body and resolves/downgrades/documents-and-closes
>    each. None may carry into archive unresolved.
>
> **Anti-drift hook:** Changes to this section update
> `plans/modules/launch-flow-readiness.aps.md` and
> `plans/modules/tracing-foundation.aps.md` headers in the same PR. New
> cross-cutting modules cite this section by anchor link.

### Precondition before adoption — RESOLVED 2026-04-30

LAUNCH-003 carried an open `Coordinates with: TUIDASH-009` callout in its
body whilst in `Complete` state. Per rule 3 above, that callout should have
been resolved at close time and was not. The promotion of this convention
to a first-class APS primitive was gated on resolving it — not as a TRACE
sub-task, but as proof that rule 3 is exercisable against a live artefact.

**Resolution (2026-04-30):** the callout was swept and closed. LAUNCH-003
shipped first, so the conditional "Superseded by: TUIDASH-009" branch did
not fire and is closed. The named `WatchStats` contract LAUNCH-003 produced
remains the inheritance TUIDASH-009 will consume when the dashboard surface
lands. Both the LAUNCH-003 task body and the TUIDASH-supersession risk
entry in the LAUNCH module Risks section were updated in the same edit.
This is the first real exercise of rule 3 against a live cross-reference;
the convention is now "tried in anger" rather than theoretical.

The shape of an acceptable resolution is preserved here for reference: a
callout must be **closed** at close time — resolved (the reference is now
correct), downgraded (to a less-binding callout type), or document-and-
closed-in-the-same-edit (rationale recorded inline and the callout marked
closed). **Document-and-defer is not an acceptable resolution**: a callout
that is documented but left open in the archive violates rule 3.

### `Blocks on:` provisional clause

LAUNCH originally introduced three callout types: `Coordinates with:`,
`Blocks on:`, and `Superseded by:`. The promoted vocabulary also accepts
`Supersedes:` for the inverse direction so newer tasks can say they replace an
older item without editing the older item first. In the LAUNCH trial only
supersession callouts and `Coordinates with:` saw use in completed tasks.
`Blocks on:` was declared but never exercised through a close.

The promoted spec retains `Blocks on:` because it captures an obvious
coordination shape (this work cannot land until the referenced item lands),
but flags it as **provisional** in the spec text. The clause hardens once a
cross-cutting task with an open `Blocks on:` callout reaches Complete and the
closer sweeps it under rule 3. At that point a follow-up edit removes the
"provisional" flag from the spec.

This is intentional. A spec written before its referent has been used at
least once through a real close cycle is overspecified, and the LAUNCH trial
is the evidence base — `Blocks on:` is not in that evidence base yet, and
the spec marks the gap honestly rather than claiming coverage it does not
have.

## Rationale

### Alternatives considered

| Option | Pros | Cons |
|--------|------|------|
| Promote to `aps-rules.md` now (chosen) | One spec covers every cross-cutting module; new authors cite the rule by anchor link rather than copy-pasting prose; LAUNCH's own trigger ("at the second copy") is honoured | Promotes a convention whose `Blocks on:` clause hasn't been exercised; risks codifying a shape we'd later refine |
| Copy the LAUNCH block into TRACE | Zero spec change; pattern stays bottom-up | Creates two divergent canonical statements (LAUNCH's and TRACE's); the LAUNCH ring-fence said this is the moment to promote, not to copy |
| Wait for a third cross-cutting module before promoting | Even more evidence | LAUNCH's own ring-fence already named this point as the promotion trigger; further delay is inertia |
| Promote to `aps-rules.md` and add a YAML frontmatter callout syntax with a lint | Machine-checkable cross-references survive renames | Speculative; LAUNCH's experience shows prose callouts plus a closer sweep are sufficient in practice; lint cost not yet justified |

### Why this option

LAUNCH's ring-fence explicitly named the second-author trigger. The TRACE
module is the second author. Promoting now, with a clear acknowledgement that
`Blocks on:` is provisional and an anti-drift hook that ties the two existing
cross-cutting modules' headers to changes in the spec section, gives the
right balance: enough rigor to stop divergence between LAUNCH and TRACE,
honest enough to mark its own gaps, lightweight enough to not over-engineer
ahead of evidence.

The closer-sweep rules are preserved verbatim from the LAUNCH trial — they
were the part of the convention that actually mattered when LAUNCH tasks
closed, and they survive directly into the spec.

## Consequences

- **Positive:** Cross-cutting modules now have a single normative reference;
  TRACE cites `plans/aps-rules.md#module-types-vertical-and-conductor` rather than
  re-declaring the convention; future cross-cutting modules do the same.
  LAUNCH's "do not copy" gate is honoured by promotion rather than copy.
- **Positive:** The closer sweep obligation is now part of the rules every
  agent reads on planning entry, not buried in one module's body.
- **Negative:** `Blocks on:` is codified before being exercised through a
  close. Mitigated by the explicit "provisional" flag and a follow-up edit
  contract.
- **Negative:** The anti-drift hook (header updates in same PR) is prose
  enforcement; nothing prevents a header from drifting silently. Acceptable
  for a two-module population. Revisit if a third cross-cutting module lands
  and headers diverge.
- **Risks:** A new author writes a cross-cutting module without reading the
  rule, declares its own callout vocabulary, and the divergence the ADR is
  meant to prevent happens anyway. Mitigation: the rule cites the two
  current modules by name, and any review of a new cross-cutting module
  should fail if it does not anchor-link the spec section.
- **Mitigations:** When LAUNCH archives, sweep its remaining callouts per
  rule 4, then update `aps-rules.md` to retire the LAUNCH header reference
  and (if appropriate) harden the `Blocks on:` clause based on whatever
  closes were observed in the meantime.

## References

- Related context: ADR-019 (feature flag telemetry alignment) introduced the
  domain-owned `anvil.flags.*` convention; cross-cutting modules ratify rather
  than design naming schemes
- Origin convention: `plans/modules/launch-flow-readiness.aps.md`
  (Cross-cutting convention section, lines 33–67)
- Spec home: `plans/aps-rules.md` (`## Cross-Cutting Modules` section added
  in this ADR)
- Second trial: `plans/modules/tracing-foundation.aps.md`
- Planning Council session: plan-b00c16c7 (2026-04-30)
