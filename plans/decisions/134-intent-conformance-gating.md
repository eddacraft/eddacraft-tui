# ADR-134: Deterministic Intent Conformance Gating

## Status

Accepted 2026-08-27 (operator)

## Date

2026-08-27

## Context

anvil can already describe what a change did through git changed paths and the
GV2 per-file `GraphDelta` stream. It does not yet compare that effect with what
the author declared the change would do. The CONF module proposes that
comparison as intent conformance: deterministic claim extraction followed by an
advisory gate or closeout verdict.

The product boundary needs a durable decision before implementation. anvil is
not a planning, requirements, retrieval, or task-management product. However, a
deterministic check that tests a declared claim against observed change evidence
is a validation surface at the point of creation, with a direct path to
enforcement. It belongs in the same product lane as other advisory-by-default
checks under ADR-002.

The decision must also preserve ADR-001. A useful first slice cannot require APS,
another plan format, Kindling, or a complete ILGOV ledger. It must work from git
and author-supplied claims while leaving richer intent sources available later.

Finally, GV2 evidence is per-file. A single `GraphDelta.file` cannot establish
which files comprise the whole change. Treating the deltas that happen to be
available as complete would allow missing evidence to look conformant. Coverage
and evidence therefore need separate, fail-honest authorities.

## Decision

### 1. Product lane and vocabulary

Deterministic intent conformance is an in-scope anvil validation surface. It
compares declared intent with observed change evidence and emits a gate or
closeout finding. It does not search, rank, summarise, or retrieve plans and
documentation.

Use **conformance**, not **drift**, for this capability. ADR-052 owns
architecture edge drift. The canonical terms are:

- **claim** — a deterministic assertion extracted from a declared intent
  source;
- **coverage set** — the complete, canonical set of changed files the
  evaluation is required to account for;
- **effect evidence** — claim-appropriate evidence for one member of the
  coverage set: Git classification for closed path/file-class claims, or a
  bound GV2 `GraphDelta` for graph-semantic claims;
- **conformance verdict** — the result, kept separate from complete, partial, or
  absent evidence strength.

No probabilistic or LLM-inferred claim may enter the enforcement path.

### 2. Intent-source tiers

All sources normalise into one future canonical conformance contract. The tiers
describe source richness, not increasing permission or automatic trust:

| Tier | Source | Product boundary |
| --- | --- | --- |
| 0 | Conventional Commit `type(scope)` and deterministic PR-body claim patterns | Universal and planless; Conventional Commits first |
| 1 | Session intent events supplied through the ILGOV/Kindling boundary | Optional wired-agent enrichment |
| 2 | Plan artefacts supplied through declared adapters, APS first | Optional plan enrichment; no plan retrieval engine in anvil |

Tier 0 is independently useful and does not wait for the full
`IntentLedgerRecord` rescope. CONF-002 must leave an explicit extension seam so
the later ILGOV canonical record can be co-designed without creating two
incompatible intent contracts. Tier 1 remains owned by ILGOV; Tier 2 parsing
remains owned by the adapter layer and external plan toolchains.

Conventional Commit extraction is the first executable Tier-0 slice
(CONF-003/004). Deterministic PR-body patterns remain Tier 0, but land later
under CONF-005 and do not enter the first slice.

### 3. Deterministic Git extraction

An evaluation selects either one commit `C` or a range `B..H`. Inputs are
resolved with
`git rev-parse --verify --end-of-options "<input>^{commit}"` and the full
object IDs are recorded. Option-shaped inputs are therefore revisions to reject
or resolve, never Git command options.

Every resolution, traversal, diff, and object read runs as
`git --no-replace-objects` from the canonical worktree under a closed,
documented environment. The evaluator sets `LC_ALL=C`, `TZ=UTC`,
`GIT_CONFIG_NOSYSTEM=1`, `GIT_CONFIG_GLOBAL=<empty-config>`, and
`GIT_OPTIONAL_LOCKS=0`; the global-config value names an evaluator-owned empty
file rather than a platform-specific null device. The Git executable is
resolved before constructing the child environment. No ambient `GIT_*`
variable is inherited: repository, worktree, common-directory, object-store,
alternate-object, namespace, index, replacement, shallow-file, external-diff,
and injected `GIT_CONFIG_COUNT`/`GIT_CONFIG_KEY_*`/
`GIT_CONFIG_VALUE_*` variables are absent. The three `GIT_*` values above are
the complete allowlist.

Commands pass the canonical worktree explicitly and pin every consulted local
configuration key on the command line. In particular, raw diffs use
`--no-ext-diff --no-textconv`, `-M50%`, and `-l1000`. The implementation must
maintain and test this environment and configuration allowlist.

Before resolving the requested revisions, the evaluator uses that same
replacement-disabled environment to inspect the repository. Any ref below
`refs/replace/`, or the presence of a legacy `info/grafts` file in the
canonical Git common directory, makes the run reason-coded not-evaluated.
Disabling replacement is mandatory even after that check so a concurrent
replacement cannot change traversal or object reads. The recorded object IDs
therefore name the commit, tree, message, and blobs that were actually
evaluated; replacement or graft semantics can never be evaluated under an
unreplaced identifier.

For a single commit, the evaluated base is its first parent, or
Git's empty-tree object for a root commit. For a range, `B` is exclusive and
`H` inclusive; the commits returned by `git rev-list B..H` are treated as a
set and sorted by full object-ID bytes before evaluation. The evaluator first
requires `git merge-base --is-ancestor B H` to succeed. A shallow repository,
missing object, ancestry failure, or non-commit input makes the run
not-evaluated. A root commit uses the repository-object-format empty tree,
computed as the tree object for zero bytes, never a hard-coded SHA-1 identifier.

Each commit is evaluated separately. Its Conventional Commit claim applies only
to that commit's footprint, never to the union of a range. A merge commit is
diffed against its **first parent**; commits reachable through other parents
remain separate range members when they are in `B..H`.

The footprint is the NUL-delimited raw output of:

```text
git --no-replace-objects diff-tree --no-commit-id --raw -z -r \
  --no-ext-diff --no-textconv -M50% -l1000 --no-abbrev \
  <first-parent-or-empty-tree> <commit>
```

Extraction has a versioned budget contract. The first implementation pins:

| Budget | Limit |
| --- | ---: |
| Commits in one evaluation | 10,000 |
| Raw records for one commit | 100,000 |
| Raw diff bytes for one commit | 64 MiB |
| Decoded UTF-8 message plus path bytes for one commit | 64 MiB |
| Possible rename sources or targets for one commit | 1,000 each |
| One Git subprocess | 30 seconds elapsed |
| Whole evaluation | 300 seconds elapsed |

Before rename detection, a bounded `--no-renames` raw preflight counts possible
delete/type-change sources and add/type-change targets. If either count exceeds
1,000, the commit is `not-evaluated:budget.rename-candidates`; exhaustive
rename detection is never attempted and Git's `-l1000` limit is a second
defence, not permission to reinterpret a skipped rename search as delete-plus-
add. `-l0` is forbidden.

The commit, record, raw-byte, decoded-byte, and rename-candidate ceilings are
deterministic semantic budgets: the same object set and budget-contract version
reach the same decision. Elapsed-time ceilings are operational fail-safes and
are not claimed to be reproducible under different machine load. A timeout
produces `not-evaluated:budget.git-timeout` or
`not-evaluated:budget.run-timeout`, records the stage, configured limit,
observed elapsed time, and counts/bytes processed to the last complete record,
and can never produce conformant.

Any budget overflow terminates evaluation with its specific
`not-evaluated:budget.*` reason. Diagnostics retain the configured limit,
observed commit/record/candidate/byte counts, command stage, and the digest of
captured raw output. A record is never dropped, truncated into a different
record, or reclassified to stay within budget: output from an over-budget or
timed-out stage is diagnostic only, and that commit contributes no conformance
verdict. Reports still count the affected commit and its not-evaluated reason.

The admitted status set is `A`, `D`, `M`, `R<score>`, and `T`.
Mode-only changes, type changes, and gitlink/submodule object changes are
coverage even when file content is unchanged. A rename contributes both its
old and new path; the other admitted statuses contribute their emitted path.
Any unmerged, combined, copied, unknown, or malformed raw record makes that
commit not-evaluated rather than being dropped. The bounded preflight and final
diff must name the same replacement-disabled endpoints; a mismatch makes the
commit not-evaluated.

Paths are parsed as bytes from `-z` output. They must be valid UTF-8 and remain
the exact repository-relative, forward-slash spelling emitted by Git: no lossy
decoding, Unicode normalisation, case folding, percent decoding, or separator
rewriting. Invalid UTF-8 or a non-relative/empty-segment traversal shape makes
the commit not-evaluated. Coverage is the raw-UTF-8-byte-sorted, de-duplicated
union of admitted old/new paths.

The claim source is the selected commit object's message, not the worktree or a
hosted rendering. Decode it as UTF-8, take the first LF-delimited line (removing
one terminal CR), and match:

```text
type(scope)!: description
type!: description
type: description
```

`type` is lowercase ASCII `[a-z][a-z0-9-]*`; `scope`, when present, is
non-empty and contains no parentheses or line break; the description begins
with a non-whitespace character. A non-matching header is malformed. A valid
header whose type/scope has no closed claim is unknown/not-applicable. Both
outcomes are not-evaluated, never conformant.

### 4. Claim-appropriate evidence and binding

Git classification is sufficient evidence for closed path and file-class
claims. A claim about symbols, imports, architecture, trust, or another graph
semantic requires GV2 evidence; Git cannot substitute for it.

Every evaluation records a unique run identifier and binds evidence to:

- stable repository and canonical worktree identity, with no current-directory
  fallback;
- evaluated base/head and per-commit object IDs;
- per-path status, old/new path, mode, type, and old/new blob or gitlink object
  IDs from the raw Git record;
- for GV2, the `GraphDelta` schema version, workspace-assurance generation,
  evaluated path, and the exact evaluated revision/blob or content digest.

A worktree-current delta is not evidence for an historical commit merely
because `GraphDelta.file` matches. If repository/worktree identity,
revision/blob, schema, generation, path, or run association is absent, stale, or
mismatched, graph-semantic evidence is unusable.

Every coverage member receives one evidence disposition:
`git-sufficient`, `gv2-bound`, `missing`, `stale`, `mismatched`, or
`unsupported`; a contributing policy/configuration change additionally uses
`policy-change`. A disposition states evidence availability or authority
change, not the conformance result.

### 5. Outcomes and monotonic aggregation

The verdict and evidence strength are separate fields:

- **conformant** — at least one applicable closed claim was evaluated; every
  claim-required coverage member has sufficient bound evidence; no violation
  was found;
- **non-conformant** — at least one violation was proven;
- **not-evaluated** — no applicable claim exists, the claim is malformed or
  unknown, or evidence is insufficient and no violation was proven.

Evidence strength is `complete`, `partial`, or `absent`. Aggregation is
monotonic: a proven violation remains non-conformant even when another coverage
member is missing or stale, yielding `non-conformant + partial`, never a
partial-evidence pass or not-evaluated escape. Conversely, incomplete evidence
cannot produce conformant.

Only conformant contributes to a conformance numerator. Non-conformant
contributes to the evaluated denominator only; not-evaluated contributes to
neither. Reports still count and reason-code every not-evaluated commit.

This is fail-honest, not fail-closed-by-default: ADR-002 still controls process
exit behaviour.

### 6. Conventional Commit claim semantics

Conventional Commit scope is free-form text, not inherently a repository path.
The evaluator must not invent path semantics from a label.

Direct path authority uses only this explicit scope grammar:

```text
path:<segment>(/<segment>)*
```

Each segment is non-empty UTF-8 and is neither `.` nor `..`; `\`, NUL,
wildcards, leading/trailing `/`, and empty segments are forbidden. The prefix
keeps its exact Unicode and case spelling; there is no decoding or
normalisation. A coverage path matches when it equals the prefix or begins with
`<prefix>/`. Both old and new rename paths must match. The `path:` marker is
required, so a free-form scope such as `api`, `auth`, or `docs` never gains
path authority merely because a same-named tree entry exists.

Other scope tokens become path-enforceable only through the versioned
`intent_conformance.scope_mappings` section of the canonical anvil
configuration (ADR-120). Mapping values are sorted, de-duplicated path prefixes
that obey the grammar above without the `path:` marker; no globs or regexes.
The mapping schema version and source digest are recorded with the run.

Configuration authority is loaded from the **evaluated base commit's tree** by
the canonical config discovery/parser: the first parent for a single commit,
`B` for `B..H`, and empty for a root commit. Worktree or head configuration
cannot authorise the range being evaluated. A change to any config source that
contributes claim types or scope mappings receives a `policy-change` evidence
disposition and is evaluated only under base authority; its new values take
effect on a later evaluation whose base contains them.

An unmapped free-form label remains weak provenance. It may be reported as the
author's declared scope but its path conformance is not-evaluated. Name
similarity, ownership guesses, graph neighbourhoods, or LLM interpretation must
not turn it into a path constraint.

Commit types are likewise interpreted only through a closed, documented claim
table whose version is recorded with the run. A closed type claim, such as a
documentation-only or test-only class, may be evaluated against the complete
changed-file set using Git classification.
Unknown types and types with no closed effect claim remain provenance and do
not imply conformance constraints.

### 7. Advisory posture and evidence

Conformance findings follow ADR-002: warnings exit 0 by default and enforcement
requires an explicit operator opt-in through ADR-002's enforcement posture.
Running or attaching an evaluation at closeout is timing only: it creates no
special closeout class, carve-out, severity, or non-zero exit authority. Any
enforceable mode must preserve the same coverage and evidence semantics; it may
promote a non-conformant, required-but-not-evaluated, or
partial/absent-evidence condition according to declared policy, but it may not
relabel one as conformant.

Verdicts carry the evaluated claim, source tier and provenance, coverage set,
per-file evidence disposition, uncovered files, and the reason for any
incomplete evidence.

### 8. Capsule minimisation and privacy gate

Capsule attachment is later work owned jointly by CONF-006 (correlation and
attachment) and CONF-009 (evidence grading/minimisation). Both remain Proposed.
Before any capsule schema extension, a privacy review must approve a closed
payload allowlist.

The maximum proposed payload is: normalised claims; source kind plus stable
source reference and digest (never raw commit/PR body); outcome and evidence
strength; reason codes; evaluated object IDs and mapping/claim-table versions;
and workspace-relative coverage paths with dispositions. It excludes source
snippets, absolute paths, `GraphDelta.errors`, and all `GraphDelta` baseline
sets such as `previously_imported`, `previously_public`,
`previously_privileged`, `previously_boundary`, and
`previously_reexported_privileged`. Existing capsule privacy, canonicalisation,
and integrity rules remain in force.

### 9. Wave boundary

This ADR clears the product-lane decision, pins deterministic Git coverage and
claim-appropriate evidence requirements, and creates the Tier-0 sequencing
boundary that avoids waiting on full ILGOV. Existing `GraphDelta.file` is a
per-file input, but it is not sufficient until the implementation proves the
revision/schema/generation binding above.

It does **not** define the runtime contract or change GV2 schemas. CONF-002..004
remain Proposed until their own Ready promotion establishes the Rust contract,
claim table, deterministic scope mapping contract, tests, and dogfood evidence.
CONF-005..009 remain Proposed for later waves. The CONF module itself remains
Proposed until CONF-002..004 are separately promoted; acceptance of this ADR is
not that promotion.

## Rationale

Intent conformance strengthens deterministic control without turning anvil into
a planning product. The pinned Git extractor and claim-appropriate,
revision-bound evidence prevent a partial or unrelated graph stream from
laundering absence into a pass. The planless Tier-0 slice provides immediate
value and preserves a clean seam for richer sources, while the closed
interpretation rules keep free-form commit metadata from gaining invented
enforcement meaning.

### Alternatives Considered

| Option | Pros | Cons |
| --- | --- | --- |
| Pinned Git coverage plus claim-appropriate, revision-bound evidence; planless Tier 0 (chosen) | Deterministic; fail-honest; useful without plans; reuses shipped substrate | Requires explicit extraction, binding, and per-file dispositions |
| Use available `GraphDelta` files as the coverage set | Smallest implementation | Missing deltas silently disappear and can produce false conformance |
| Require ILGOV or APS before evaluation | Richer intent | Violates planless-first and delays the universal slice |
| Infer scope labels through graph or LLM matching | More scopes appear evaluable | Non-deterministic or heuristic; invents enforcement meaning |
| Keep all conformance outside anvil | Avoids a new surface | Discards a deterministic pre-merge control that fits the scope guard |
| Build plan/document search in anvil | Convenient intent discovery | Crosses into planning/retrieval product scope and is unnecessary for evaluation |

## Consequences

- **Positive:** anvil gains a clearly bounded claim-versus-effect product lane;
  Tier 0 works without planning infrastructure; incomplete or unrelated graph
  evidence cannot pass silently; proven violations remain visible.
- **Negative:** some familiar commit scopes will remain weak provenance until a
  deterministic mapping is declared; verdicts need more than a boolean.
- **Risks:** callers may collapse evidence strength into outcome, or treat
  arbitrary scope labels as paths; a policy change may self-authorise; future
  Tier-2/capsule work may drift into retrieval or over-collect evidence.
- **Mitigations:** monotonic outcome aggregation, per-member dispositions,
  explicit `path:` grammar, base-tree configuration authority, the capsule
  privacy gate, bounded extraction with reason-coded overflow, replacement-
  disabled and sanitised Git operations, option-safe revision parsing, and the
  Wave boundary above.

## References

- Related ADRs: [ADR-001](001-planless-first.md) (planless first),
  [ADR-002](002-warnings-over-blocks.md) (advisory default),
  [ADR-003](003-new-edges-only.md) (baseline posture),
  [ADR-052](052-automated-drift-snapshots.md) (drift vocabulary),
  [ADR-072](072-git-native-governance-substrate.md) (git substrate), and
  [ADR-074](074-review-capsule-v0-format.md) (evidence attachment)
- APS module: [CONF](../modules/intent-conformance.aps.md), especially CONF-001
- Programme:
  [Graph Trust Surfaces](../specs/2026-07-28-graph-trust-surfaces.md)
- Product boundary:
  [anvil Scope Guard](../../docs/vision/anvil-scope-guard.md)
- GV2 evidence:
  [`GraphDelta`](../../crates/anvil-graph-cache/src/incremental.rs)
