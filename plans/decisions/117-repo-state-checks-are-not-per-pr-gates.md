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
  `release-plan`, `adr`, `aps`, `index-freshness`. Their scripts take **zero**
  references to the PR diff or base ref. They answer "is the repository tidy
  right now?"

That split is not yet a decision rule — "doesn't read the diff" describes four
surfaces whose failure modes differ sharply. The rule that matters is narrower:

> **Can this assertion begin failing with no commit at all?**

Applied surface-by-surface, only `release-plan` can. Applied to the assertions
inside it — which is the granularity the evidence actually has — the answer is
narrower still. `scripts/docs/check-release-plan.mjs` makes three:

| # | Assertion | Source | Fails with no commit? |
| --- | --- | --- | --- |
| 1 | exactly one `## Active window` section | `check-release-plan.mjs:47-56` | No — pure file content |
| 2 | no legacy `Next Release Window` / `Shipped` headers | `check-release-plan.mjs:59-68` | No — pure file content |
| 3 | the active window's version is not already a git tag | `check-release-plan.mjs:71-86` | **Yes — `git tag` alone flips it** |

Assertions 1 and 2 read `RELEASE-PLAN.md` and nothing else; they can only go red
because a commit edited that file, and the PR carrying that commit is the one
that fails. Assertion 3 shells out to `git tag --list` and compares against the
repository's tag state, which no commit carries.

`adr`, `aps` and `index-freshness` have the same shape as assertions 1 and 2:
each goes red only because some commit broke it — an ADR added without its log
row, a module edited past its recorded count, a doc added without regenerating
the index. Their weakness is only *inheritance* — once a bad commit lands on
`main`, every later PR inherits the red until someone repairs it. That is a real
cost, but it is an argument for repairing `main` quickly, not for removing the
gate that catches the mistake at source.

Wiring an assertion of the third kind into a required per-PR gate means
repository drift fails every contributor's build for something no contributor
did.

This is not hypothetical. On 2026-08-04 assertion 3 began failing every open PR
because `v0.9.2-beta` had been **tagged**. Cutting the tag made the assertion
true without anyone editing a file. `main` went red, and stayed red, until an
unrelated release-closeout task was performed. Earlier the same day the
`public-docs` surface failed every PR for a mirror-image reason: the changelog
named the next version before its generated reference was regenerated. Both are
the same underlying defect — a release step that mutates repository state
without carrying the bookkeeping that the checks assert.

The blast radius is widest where it is least appropriate. This monorepo hosts two
products that ship on **their own cadence** to their own public mirrors:

- `tools/starters/acknowledgements` → `eddacraft/acknowledgements-starter`
- `crates/eddacraft-tui` → `eddacraft/eddacraft-tui` + crates.io

Both release by tagging a commit that their release workflow requires to be
reachable from `origin/main`, so both must merge a version bump through the full
required-check set. The kit carries `README.md` and `CHANGELOG.md`, so a kit-only
change trips `markdownlint-required` (`.github/workflows/ci.yml:139-148`) and
runs the whole of `docs:check`. A patch release of a self-contained bash kit that
shares no code with anvil was therefore blocked by assertion 3 about anvil's
release plan.

The forcing function: kit `v1.1.0` shipped a freshness gate that a valid
CommonMark document could silently disable. The repair (`v1.1.1`) sat behind a
red assertion about a different product's release.

Two further facts constrain the decision:

- **Assertion 3 has no enforcement outside `docs:check` today.**
  `pnpm release-plan:check` is declared at `package.json:75` and referenced in
  prose (`README.md:351`, `RELEASE-PLAN.md:31`), but no script or workflow
  invokes it. `scripts/release/preflight.sh` runs a fixed gate list
  (`preflight.sh:335-347`) that does not include it, and
  `scripts/release/closeout.sh` never reads `RELEASE-PLAN.md` — the prune is a
  manual operator step in `docs/policies/release-cadence.md`. Removing assertion
  3 from the gate without wiring it elsewhere would not relocate the assertion;
  it would delete it.
- There is duplication worth removing: `aps` runs both as a `docs:check` surface
  and as the separately-required `APS Drift Check` job.

## Decision

**A merge gate may only fail for something a commit did — assessed per
assertion, not per surface.**

1. **Assertion 3 leaves the per-PR gate; assertions 1 and 2 stay.**
   `check-release-plan.mjs` gains a `--shipped-window-check` flag, default off.
   `docs:check` invokes the script without it, so `Docs Lint` continues to
   enforce that `RELEASE-PLAN.md` has exactly one active window and carries no
   shipped or legacy headers — both of which only a commit can break. The
   already-tagged comparison runs only when the flag is passed.

2. **Assertion 3 gets a real enforcement point, which it does not have today.**
   `pnpm release-plan:check` passes `--shipped-window-check` and is wired into:
   - `scripts/release/preflight.sh`, as a `run_gate` entry alongside the
     existing gates, so a release cannot be prepared against a plan that names
     an already-shipped version; and
   - a `Repo Health` workflow on push to `main` and on a schedule, which is
     where post-tag drift surfaces. Failure notifies the release owner; it does
     not block contributors. `Repo Health` is **not** added to the required-check
     set.

   Closeout remains the step that prunes the window. Wiring the assertion into
   `closeout.sh` so the prune is verified rather than remembered is the better
   fix and is left to a follow-up work item; `Repo Health` is the safety net
   until then.

3. **`adr`, `aps` and `index-freshness` stay gating, unchanged.** Each can only
   break because a commit broke it, and the PR carrying that commit is the one
   that fails — which is the gate doing its job. Moving them would let a broken
   ADR log or a stale index land unnoticed, trading a real defect for a smaller
   inconvenience.

4. **Drop the duplicate `aps` execution.** `aps` currently runs both as a
   `docs:check` surface and as the separately-required `APS Drift Check` job.
   `APS Drift Check` remains the single required APS gate; the duplicate surface
   execution is removed. This is independent of the rest — it is redundancy, not
   classification.

No sub-product path carve-out is created. The acknowledgements kit and
`eddacraft-tui` were blocked by assertion 3 specifically; with assertion 3 off
the gate, every remaining required check that a kit-only change runs is one that
judges that change. Declaring sub-product paths with a divergent required-check
set would be new machinery with no remaining motivating case, and its own
under-gating failure mode.

This ADR decides the classification and the principle. The mechanical wiring
(flag name, `Repo Health` job definition, `preflight.sh` gate placement) is
implementation detail owned by the executing work item.

## Rationale

A required status check answers one question: *is it safe to merge this change?*
A contributor can only answer that for things their commit did. When a check can
go red without any commit, it is asking a question the PR author has no standing
to answer and no means to fix — so it belongs somewhere that notifies its owner
instead.

The draft history of this ADR is the argument for the granularity. The first
draft grouped all four non-diff-reading surfaces together and moved three of
them. Review pointed out that the fourth, `index-freshness`, was unclassified,
and classifying it exposed that "doesn't read the diff" is the wrong rule:
`adr`, `aps` and `index-freshness` all go red only because some commit made them
go red. That narrowed the decision to `release-plan` alone. Applying the same
question one level further down narrows it again: two of `release-plan`'s three
assertions are also pure content checks that only a commit can break, and there
is no reason to lose them. The rule is about assertions; surfaces are just how
assertions happen to be packaged into scripts.

Deciding at assertion granularity also keeps the change small in the place where
size is risk. The ADR's own hazard is a required check silently dropped from the
ruleset during a split. Keeping `release-plan` in `docs:check` — with one
assertion behind a default-off flag — means the required-context list is
untouched and there is nothing to diff or restore.

The discovery that `release-plan:check` has no caller changes the shape of clause
2 from "relocate the hard stop" to "build the hard stop." An earlier draft
asserted that the release preflight path already ran it; it does not. Removing
the assertion from `docs:check` without that wiring would silently delete the
only thing enforcing it, which is the opposite of the intent.

### Alternatives Considered

| Option | Pros | Cons |
| --- | --- | --- |
| **Chosen: split `release-plan` by assertion — the tag comparison leaves the gate behind a flag, the two content assertions stay** | Removes exactly the failure no contributor can cause or fix, and nothing more; keeps every assertion that catches a real mistake at source; touches one script and the ruleset not at all; makes a sub-product carve-out unnecessary | Needs the enforcement wiring in clause 2 to be built, not merely pointed at |
| Move the whole `release-plan` surface to `Repo Health` | Simpler taxonomy; one fewer moving part in `docs:check` | Discards assertions 1 and 2, which fail only when a commit breaks them; and — since `release-plan:check` has no caller — would delete assertion 3 rather than relocate it unless the same wiring is built anyway |
| Keep everything gating; fix drift faster | No tooling change; strongest tidiness pressure | Already demonstrably failing — drift is caused by *tagging* and by release prep, so the pressure lands on uninvolved contributors, not the owner |
| Make assertion 3 tolerate a shipped active window | One-line change | Removes the assertion's whole point: the window is supposed to be pruned on closeout, and a plan naming a shipped tag is genuinely wrong |
| Move all four non-diff-reading surfaces (`release-plan`, `adr`, `aps`, `index-freshness`) to `Repo Health` | Tidier taxonomy; one rule for "doesn't read the diff" | Over-broad — the other three only fail when a commit breaks them, so removing them lets a broken ADR log or stale index land unnoticed. Reviewing this alternative is what narrowed the decision to `release-plan`, and then to assertion 3 |
| Declare sub-product paths (`tools/starters/acknowledgements/**`, `crates/eddacraft-tui/**`) with their own required-check set | Directly unblocks the kit and the TUI | Treats a general defect as a special case, leaves every other contributor blocked by tag drift, and adds a divergent required-check set whose own drift under-gates silently. Unnecessary once assertion 3 is off the gate |
| Generate the active-window heading and check freshness, as `index-freshness` does | Single source of truth; familiar pattern | Reproduces the same defect with more machinery — a new tag still makes generated output diverge from the committed file with no commit involved |
| Run repo-state surfaces on a PR only when the PR touches the files they assert about | More general than a sub-product path list | Leaves a hole: a PR editing `RELEASE-PLAN.md` just after a tag still fails for a tag it did not cut. Larger `detect-changes` surface for a smaller gain than the chosen option |
| Automate the prune: have the release path open the closeout PR | Attacks the root cause; would also have prevented the `public-docs` incident the same day | Does not on its own make the gate correct — the drift window shrinks but never closes. Complementary, not a substitute; captured as the clause 2 follow-up |
| Move the sub-products to their own repositories | Total decoupling; no shared gate at all | Loses the subtree-mirror model deliberately chosen in ATTRIB-011/TUIR; large migration for a CI problem |

## Consequences

- **Positive:** Post-tag release-plan drift stops blocking unrelated work.
  Mirrored sub-products can cut patch releases on their own cadence, which is
  the point of publishing them separately, with no bespoke path carve-out.
  `Docs Lint` failures become actionable by the PR author, because every
  remaining assertion fails only when a commit broke it. The already-tagged
  assertion gains a release-path gate and a scheduled check, neither of which it
  has today. One duplicate check execution disappears. The required-context list
  on the ruleset is unchanged.
- **Negative:** Post-tag drift is no longer forced to the top of someone's
  attention by a red PR. It will be visible in `Repo Health` and on `main`, but
  visibility is weaker pressure than a block. `check-release-plan.mjs` gains a
  mode flag, so a caller that forgets it silently checks less.
- **Risks:** `RELEASE-PLAN.md` staleness could persist unnoticed, since the very
  situation this ADR addresses arose from a closeout step not being performed —
  and closeout remains manual until the clause 2 follow-up lands. A `Repo Health`
  workflow that nobody watches is not a notification path.
- **Mitigations:** `preflight.sh` refuses to prepare a release against a stale
  plan, which is the point at which staleness actually costs something.
  `Repo Health` failure on `main` is the notification path and must be wired to
  reach the release owner, not merely recorded — the implementing PR states how.
  Fixture-level coverage asserts that `docs:check` skips assertion 3 and that
  `pnpm release-plan:check` runs it, so the flag's default cannot silently
  invert.

## References

- Related ADRs: ADR-053 (advisory APS counts — same principle: report, don't
  block, when the assertion is not about the change under review)
- APS modules: ATTRIB-026 (the kit repair whose release surfaced this),
  ATTRIB-011 (acknowledgements mirror), TUIR (eddacraft-tui mirror + release
  flow)
- Code: `scripts/docs/docs-check.mjs` (surface table),
  `scripts/docs/check-release-plan.mjs` (the three assertions),
  `scripts/release/preflight.sh` (gate list, no release-plan gate today),
  `scripts/release/closeout.sh` (does not touch `RELEASE-PLAN.md`),
  `.github/workflows/ci.yml` (`docs-lint` job, `detect-changes` outputs)
- Policy: `docs/policies/release-cadence.md`,
  `docs/runbooks/acknowledgements-starter-release.md`,
  `docs/runbooks/eddacraft-tui-release.md`
