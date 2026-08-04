# ADR-117: Repo-state checks are not per-PR merge gates

## Status

Proposed

## Date

2026-08-04

## Context

`Docs Lint` is one of nine required status checks on `main`. It runs
`pnpm docs:check`, which executes nine surfaces. Those surfaces are not all the
same kind of thing:

- **Diff-judging surfaces** assess what the pull request changed — `metadata`,
  `tags`, `links`, `asbuilt-paths`, `public-docs`. A failure means *this PR* is
  wrong. Gating the PR on them is correct.
- **Repo-state surfaces** assert a property of the repository as a whole —
  `release-plan`, `adr`, `aps`, `index-freshness`.
  `scripts/docs/check-release-plan.mjs`, `scripts/docs/check-adr.mjs` and
  `scripts/docs/check-aps.mjs` take **zero** references to the PR diff or base
  ref; `scripts/docs/check-index-freshness.mjs` delegates to
  `docs-index.mjs --check`, which regenerates and compares the whole index.
  They answer "is the repository tidy right now?"

That is all nine surfaces, and the split alone is not yet a decision rule —
"doesn't read the diff" describes four surfaces whose failure modes differ
sharply. The rule that matters is narrower:

> **Can this check begin failing with no commit at all?**

Only `release-plan` can. It asserts the active window in `RELEASE-PLAN.md` is
not already a shipped tag, so **creating a git tag** makes it fail — no file
changed, no PR involved, nothing for any contributor to have done differently.

The other three can only break because a commit broke them: an ADR added
without its log row, a module edited past its recorded count, a doc added
without regenerating the index. In each case the offending PR is itself the one
that fails, which is a gate working correctly. Their weakness is only
*inheritance* — once a bad commit lands on `main`, every later PR inherits the
red until someone repairs it. That is a real cost, but it is an argument for
fixing `main` quickly, not for removing the gate that catches the mistake at
source.

Wiring a check of the first kind into a required per-PR gate means repository
drift fails every contributor's build for something no contributor did.

This is not hypothetical. On 2026-08-04 the `release-plan` surface began failing
every open PR because `v0.9.2-beta` had been **tagged**. The check asserts the
active window in `RELEASE-PLAN.md` is not already a shipped tag; cutting the tag
made that true without anyone editing a file. `main` went red, and stayed red,
until an unrelated release-closeout task was performed. Earlier the same day the
`public-docs` surface failed every PR for the mirror-image reason: the changelog
named the next version before its generated reference was regenerated.

The blast radius is widest where it is least appropriate. This monorepo hosts two
products that ship on **their own cadence** to their own public mirrors:

- `tools/starters/acknowledgements` → `eddacraft/acknowledgements-starter`
- `crates/eddacraft-tui` → `eddacraft/eddacraft-tui` + crates.io

Both release by tagging a commit that their release workflow requires to be
reachable from `origin/main`. So both must merge a version bump through the full
required-check set. A patch release of the acknowledgements kit — a self-
contained bash kit that shares no code with anvil — was blocked by the tidiness
of anvil's `RELEASE-PLAN.md`. Nothing about the kit depends on anvil's release;
the two are coupled only by passing through the same gate.

The forcing function: kit `v1.1.0` shipped a freshness gate that a valid
CommonMark document could silently disable. The repair (`v1.1.1`) sat behind a
red check about a different product's release plan.

There is also duplication worth removing: `aps` runs both as a `docs:check`
surface and as the separately-required `APS Drift Check` job.

## Decision

**A merge gate may only fail for something a commit did.**

1. **`release-plan` leaves the per-PR gate.** It is the one surface that can
   begin failing with no commit at all, because creating a tag makes it fail.
   It moves to a `Repo Health` workflow on push to `main` and on a schedule.
   Failure notifies the release owner; it does not block contributors.
   `Repo Health` is **not** added to the required-check set.

2. **`adr`, `aps` and `index-freshness` stay gating.** Each can only break
   because a commit broke it, and the PR carrying that commit is the one that
   fails — which is the gate doing its job. Moving them would let a broken ADR
   log or a stale index land unnoticed, trading a real defect for a smaller
   inconvenience.

3. **The release train keeps its hard stop.** `release-plan` remains blocking
   where it is meaningful: the release preflight/prepare path already runs
   `pnpm release-plan:check`, and that is the correct place for it to stop
   work, because that is the work it describes.

4. **Drop the duplicate `aps` execution.** `aps` currently runs both as a
   `docs:check` surface and as the separately-required `APS Drift Check` job.
   `APS Drift Check` remains the single required APS gate; the duplicate
   surface execution is removed. This is independent of the rest — it is
   redundancy, not classification.

5. **Mirrored sub-products are treated as independent for gating purposes.**
   `tools/starters/acknowledgements/**` and `crates/eddacraft-tui/**` are
   declared sub-product paths. A change confined to a sub-product path is
   gated on that sub-product's own checks (for the kit: `Kit Self-Tests`,
   ShellCheck, its version/changelog consistency check) plus repository-wide
   *diff-judging* checks — not on anvil's release bookkeeping.

This ADR decides the classification and the principle. The mechanical wiring
(job names, `detect-changes` outputs, ruleset edits) is implementation detail
owned by the executing work item.

## Rationale

A required status check answers one question: *is it safe to merge this change?*
A contributor can only answer that for things their commit did. When a check
can go red without any commit, it is asking a question the PR author has no
standing to answer and no means to fix — so it belongs somewhere that notifies
its owner instead.

The first draft of this ADR grouped all four non-diff-reading surfaces together
and moved three of them. Review pointed out that the fourth, `index-freshness`,
was unclassified, and classifying it exposed that "doesn't read the diff" is
the wrong rule: `adr`, `aps` and `index-freshness` all go red only because some
commit made them go red, and catching that at source is exactly what a gate is
for. Only `release-plan` fails on an event — a tag — that no commit carries.
Narrowing to that one surface is the change this ADR actually needs.

The sub-product clause follows from the same reasoning applied to ownership. Two
products in this repo have independent release cadences and independent public
consumers. Gating their releases on anvil's internal bookkeeping is coupling with
no corresponding dependency, and it produces exactly the failure observed: a
security-relevant patch to a published artifact waiting on an unrelated
document.

### Alternatives Considered

| Option | Pros | Cons |
| --- | --- | --- |
| **Chosen: only `release-plan` leaves the gate; the rest stay** | Removes exactly the failure no contributor can cause or fix; keeps every gate that catches a real mistake at source; smallest change that solves the problem | Release-plan drift is no longer forced to attention by a red PR |
| Keep everything gating; fix drift faster | No tooling change; strongest tidiness pressure | Already demonstrably failing — drift is caused by *tagging* and by release prep, so the pressure lands on uninvolved contributors, not the owner |
| Make `release-plan` tolerate a shipped active window | One-line change | Removes the assertion's whole point: the window is supposed to be pruned on closeout, and a plan naming a shipped tag is genuinely wrong |
| Move all four non-diff-reading surfaces (`release-plan`, `adr`, `aps`, `index-freshness`) to Repo Health | Tidier taxonomy; one rule for "doesn't read the diff" | Over-broad — the other three only fail when a commit breaks them, so removing them lets a broken ADR log or stale index land unnoticed. Reviewing this alternative is what narrowed the decision to `release-plan` alone |
| Carve out only sub-product paths, leave the gate otherwise intact | Smallest diff; directly unblocks the kit and the TUI | Leaves every other contributor blocked by release-plan drift; treats a general defect as a special case |
| Move the sub-products to their own repositories | Total decoupling; no shared gate at all | Loses the subtree-mirror model deliberately chosen in ATTRIB-011/TUIR; large migration for a CI problem |

## Consequences

- **Positive:** A release-plan drift stops blocking unrelated work.
  Mirrored sub-products can cut patch releases on their own cadence, which is
  the point of publishing them separately. `Docs Lint` failures become
  actionable by the PR author, because every remaining surface fails only when
  a commit broke it. One duplicate check execution disappears.
- **Negative:** Release-plan drift is no longer forced to the top of someone's
  attention by a red PR. It will be visible in `Repo Health` and on `main`, but
  visibility is weaker pressure than a block.
- **Risks:** `RELEASE-PLAN.md` staleness could persist unnoticed, since the very
  situation this ADR addresses arose from a closeout step not being performed.
  A required check accidentally left off the ruleset during the split would
  silently weaken the gate.
- **Mitigations:** `Repo Health` failure on `main` is the notification path, and
  the release preflight retains `release-plan:check` as a hard stop — so the
  assertion still blocks the work it is actually about. The ruleset's required
  contexts are enumerated in this repo and should be diffed as part of the
  implementing PR, with the before/after list recorded in that PR.

## References

- Related ADRs: ADR-053 (advisory APS counts — same principle: report, don't
  block, when the assertion is not about the change under review)
- APS modules: ATTRIB-026 (the kit repair whose release surfaced this),
  ATTRIB-011 (acknowledgements mirror), TUIR (eddacraft-tui mirror + release
  flow)
- Code: `scripts/docs/docs-check.mjs` (surface table),
  `scripts/docs/check-release-plan.mjs`, `scripts/docs/check-adr.mjs`,
  `scripts/docs/check-aps.mjs` (the three repo-state surfaces),
  `.github/workflows/ci.yml` (`docs-lint` job, `detect-changes` outputs)
- Policy: `docs/policies/release-cadence.md`,
  `docs/runbooks/acknowledgements-starter-release.md`,
  `docs/runbooks/eddacraft-tui-release.md`
