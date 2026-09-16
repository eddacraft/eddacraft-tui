# ADR-148: Two-backlog audience split

## Status

Proposed

## Date

2026-09-16

## Context

`plans/modules/continuous-improvement-backlog.aps.md` is the project's single
standing intake. Measured at **`f2a8f1522`**, it holds **426 entries across
14,480 lines**: **342 terminal** (198 Released, 104 Merged, 38 Done, 2
Superseded) and **82 open** (40 Draft, 30 Proposed, 12 Ready). Figures are an
as-of measurement pinned to that commit, reproducible by command, not a
standing fact. Its Standing Module Policy states it may only be replaced "if a
future APS decision explicitly replaces the intake model", which is what this
ADR does.

Three distinct failures are attributable to that one file.

**Category collision.** A substantial share of the 82 open entries are
product-shaped —
CIB-333 (scanner reports the rule token's match start, not its column),
CIB-363 (`camelCase apiKey` bindings invisible to the API Key rule), CIB-380
(`state-boundary` offers a remediation that provably changes nothing) — and
others are workshop-shaped: CIB-403 (`.mjs` outside the pre-commit allowlist),
CIB-407 (CI path classifier over-triggers), CIB-048 (worktree `target/` dirs
fill the shared disk). Both kinds are selected by the same "what is `Ready`"
heuristic. The exact ratio is **not** asserted: establishing it per entry is
migration step 2. An earlier draft of this ADR quoted "~26 product vs ~32
workshop among 58 open" from a status normalisation that silently dropped
variant spellings such as `Draft — filed from ci-log triage`; the real open
count is 82. That error is recorded rather than quietly corrected, because it
is the same defect class the currency gate exists to catch. On 2026-09-16 a three-item wave (CIB-401/403/404)
spent three repair rounds on pre-commit glob coverage and shipped **no**
customer-visible change, while product defects sat `Proposed`.

**Hot-file contention.** The file is a shared multi-writer surface that feature
PRs are explicitly forbidden to edit, because concurrent writers collide on it.
This is not hypothetical: CIB-404 exists *because* a sibling PR conflicted on
this exact file, and `scripts/ci/pr-required-status.mjs` then reported the
conflicting PR as "not finished", costing a poller its full 52-minute budget.

**No currency mechanism.** CIB-401 remained `Ready` for thirteen days after
commit `dfbf5aefd` (EMBERRS-001) deleted the TypeScript Ember runtime and
dropped `better-sqlite3`, removing the crash it described. A worker was
dispatched on 2026-09-16 against a premise that no longer existed. Separately,
CIB-403's embedded tracked-file count drifted 77 → 87 → 88 inside a single PR,
and was miswritten a fourth time while correcting it.

A relevant pre-existing fact: the repository **already** operates a second,
undeclared intake. GitHub issues carry a full lifecycle taxonomy
(`priority:P0..P4`, `kind:*`, `readiness:*`, `tracked:needs-aps` /
`tracked:promoted-to-aps`, `area:*`), and Clawpatch files findings into it. The
decision below is therefore less "build a second backlog" than "declare the two
that exist and give each an owned scope".

## Decision

Split the intake into two queues partitioned by **audience**, with a shared
topology and a shared shedding rule.

**1. The discriminator is audience, not origin or module-fit.**

| Test | Queue |
| --- | --- |
| A user of anvil experiences it | PRODUCT |
| Only someone building anvil experiences it | CIB |

A product defect found internally is product work. Who raised it is irrelevant.

**2. Routing is applied at promotion, before an ID is minted.** The `ci-log`
flow is audience-blind by construction; triage is where the audience test is
applied. Promotion routes to a GitHub issue (user-facing) or to `CIB-NNN`
(workshop-only). This is load-bearing: `docs/guides/continuous-improvement-log.md`
step 3 and the CIB module's intake paragraph both currently instruct triage to
file `CIB-NNN` unconditionally, so without amending them every internally
discovered product defect keeps minting into CIB and the split is nominal.
`promote: CIB` in the `ci-log` follow-up vocabulary becomes
`promote: CIB` | `promote: PRODUCT` | `promote: <queue-undecided>`.

**3. Both queues use inbox + thin working set.**

- **PRODUCT** — inbox: GitHub issues. Working set:
  `plans/modules/product-backlog.aps.md`.
- **CIB** — intake unchanged (`ci-log` harvest → triage). Working set: the
  existing module, thinned.

The working set holds **only** `Ready` and `In Progress` entries.

**4. Identity.** An inbox item is its GitHub issue number. It earns an APS ID
(`PROD-NNN`) only on promotion into the working set.

**5. A completed entry leaves the working file**, to `plans/archive/`. This
applies to both queues.

**6. A deterministic advisory gate (`pnpm aps:currency`)** reads each `Ready`
entry's `Files` paths and warns when they no longer exist. Warn, never block;
baseline existing state and report new violations only.

**7. The shared multi-writer convention extends unchanged** to the product
module: feature PRs edit neither working file; intake, promotion, status
reconcile and count updates happen only on single-writer bookkeeping branches.

**8. No new labels.** The existing taxonomy is reused as-is.

Migration is sequenced: make the corpus figures command-derivable; archive the
342 terminal entries; classify the 82 open by audience; move the product-shaped
to issues with the `Ready` subset into the new module; leave the workshop-shaped
in CIB; amend the triage contract to route by audience; then add the gate.
Existing items are migrated, not grandfathered.

## Rationale

Audience wins over origin because origin leaves self-found product defects
(CIB-333, CIB-363) stranded in the workshop queue, which is the precise problem
being solved. It wins over module-fit because module-fit requires a soft
judgement per item, where audience is a single testable question.

The hybrid topology is chosen over a second full APS module because the
existing module's size *is* the contention problem; duplicating its shape would
duplicate the defect. It is chosen over issues-only because `dev-loop` selects
against APS truth and needs an ID to target — issues-only would remove half the
work from planning truth.

The shedding rule is the load-bearing part. CIB's difficulty is not its scope
but that it never sheds: 337 of 425 entries are history occupying the hot path.
A split without shedding produces two growing files instead of one.

The currency gate is advisory by deliberate choice, consistent with anvil's own
"warnings over blocks" and "new edges only" principles. A blocking gate on plan
prose would stop work for a documentation defect.

### Alternatives Considered

| Option | Pros | Cons |
| --- | --- | --- |
| **Audience split, hybrid topology, both shed (chosen)** | Testable boundary; captures self-found product defects; halves hot-file contention; keeps APS as planning truth | Requires migration; boundary is not self-enforcing |
| Split by origin (external vs self-harvested) | Clean provenance; trivial to apply | Leaves CIB-333/CIB-363 misfiled — the exact failure being fixed |
| Split by module-fit | Closest to the original phrasing | Soft per-item judgement; two people classify differently |
| Second full APS module mirroring CIB | Consistent with existing tooling | Recreates a 14,380-line hot file |
| GitHub issues only, no APS module | Zero new files; where customers already are | `dev-loop` loses an ID to target; APS stops being truth for half the work |
| Leave CIB alone; apply the rule to new items only | No migration risk | Product defects stay invisible in the workshop queue |
| Scheduled human currency sweep instead of a gate | Catches stale judgement a gate cannot | Unowned; CIB-401 shows the sweep is exactly what did not happen |

## Consequences

- **Positive:** product defects become visible as a queue rather than competing
  with workshop friction; the hottest file in the repo shrinks roughly 85%,
  reducing the collision class that produced CIB-404; staleness becomes
  detectable instead of discovered by a dispatched worker; the second intake
  that already existed becomes declared and owned.
- **Negative:** two queues to triage instead of one; a migration pass on the
  repo's most contended file; one new gate to maintain.
- **Risks:** the boundary erodes through miscategorisation, leaving one queue
  with extra ceremony — no gate enforces it, and this is stated plainly rather
  than mitigated away. If the triage-contract amendment (decision 2) is skipped
  or half-applied, the split is nominal and CIB keeps absorbing product work. The archive split may collide with concurrent
  bookkeeping. The gate detects stale premises, not entries that are merely no
  longer worth doing.
- **Mitigations:** audience is a single question answerable at review time and
  belongs in the PR checklist for intake changes; the archive split lands on a
  quiet bookkeeping branch with no siblings in flight (verified via
  `git worktree list` and open bookkeeping PRs); the gate's advisory output is
  read at triage, where judgement-level staleness is also assessed.

## References

- Related ADRs: ADR-053 (stored `N/M` counts not bumped in feature PRs)
- Spec: `plans/specs/2026-09-16-two-backlog-split.md`
- APS modules: CIB (amended), PROD (new)
- `plans/project-context.md#work-item-claim-issues`,
  `plans/project-context.md#keeping-plans-current`
- `docs/guides/continuous-improvement-log.md`
- Evidence: CIB-401 (stale premise, superseded by `dfbf5aefd`), CIB-403
  (drifting derived facts), CIB-404 (hot-file conflict misreported as
  "not finished")
