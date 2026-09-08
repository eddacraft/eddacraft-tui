---
name: code-review
description: Review a pull request, branch, commit range, staged change, or working-tree diff against repository standards and the governing specification, reporting evidence-backed findings by severity. Use for code review, quality assessment, pre-merge review, or "review since X". For session-managed multi-reviewer review use council; for verifying a change against acceptance criteria use verify-loop.
---

# Code review

Review the actual change along two independent axes:

- **Standards and implementation quality:** does the change follow repository
  rules and avoid correctness, security, performance, maintainability, and test
  defects?
- **Specification:** does it implement the requested behaviour completely and
  only within the authorised scope?

A change can pass either axis while failing the other. Do not merge or rerank
the axes in a way that hides that distinction.

This is a single-pass review. For session-managed multi-reviewer review with
persisted findings, use `council`; for verifying a change against acceptance
criteria, use `verify-loop`.

## 1. Pin the review range

Resolve the fixed point in this order:

1. the commit, branch, tag, or base named by the user;
2. the pull request's recorded base branch;
3. the repository's upstream default branch for a branch review;
4. staged or working-tree changes when that is what the user asked to review.

Use a merge-base comparison for branch work:

```console
git rev-parse --verify <fixed-point>
git merge-base <fixed-point> HEAD
git diff <fixed-point>...HEAD
git log <fixed-point>..HEAD --oneline
```

For staged or working-tree review, use `git diff --cached` or `git diff`
respectively. Stop on an invalid ref. Report an empty diff rather than inventing
a review target.

## 2. Establish governing truth

Read the repository's agent/contributor instructions file (e.g. `AGENTS.md`)
and any contributor, coding, testing, security, or architecture rules that
govern the changed paths. Find the originating
specification from the pull request or issue, a user-supplied path, commit
references, APS plans, ADRs, or nearby design documents.

If no specification exists, say so and run only the standards axis. Do not treat
the absence of a spec as approval for inferred scope.

## 3. Review the two axes independently

If the runtime exposes native delegation and the change is large enough to
benefit, dispatch two read-only reviewers concurrently with the same pinned diff
but separate contexts. Otherwise review the axes sequentially and keep separate
notes. Never require delegation that the active runtime does not provide.

### Standards and implementation quality

Repository rules override generic guidance. Check each changed behaviour for:

- correctness, boundary conditions, error paths, concurrency, and resource
  cleanup;
- input trust, authentication and authorisation, secret handling, injection,
  data exposure, and dependency risk;
- algorithmic cost, repeated I/O, N+1 access, leaks, and hot-path regressions;
- clear ownership, names, cohesion, coupling, failure isolation, and needless
  abstraction;
- meaningful tests of behaviour and failure modes, not merely line execution;
- compatibility, migrations, operational impact, and documentation where the
  repository requires them.

Use these Fowler-style smells only as labelled judgement calls. Suppress a smell
when repository standards intentionally endorse the pattern, and skip matters
that deterministic tooling already settles:

- **Mysterious Name:** a name hides purpose.
- **Duplicated Code:** the same logic shape appears in multiple places.
- **Feature Envy:** code reaches into another component's data more than its own.
- **Data Clumps:** the same fields or parameters repeatedly travel together.
- **Primitive Obsession:** a primitive stands in for a domain concept.
- **Repeated Switches:** equivalent type dispatch recurs.
- **Shotgun Surgery:** one logical change requires scattered edits.
- **Divergent Change:** one module changes for unrelated reasons.
- **Speculative Generality:** abstractions serve no current requirement.
- **Message Chains:** callers navigate through another component's internals.
- **Middle Man:** a layer delegates without adding a useful boundary.
- **Refused Bequest:** inheritance supplies behaviour the implementation rejects.

### Specification

Map every finding to the governing requirement. Check for:

- missing or partial requirements;
- behaviour that contradicts the specification;
- scope added without authorisation;
- apparently implemented requirements whose runtime behaviour is wrong;
- acceptance criteria claimed without evidence.

Quote or cite the relevant requirement location without copying large source
passages.

## 4. Validate suspected findings

Read enough surrounding code to trace the behaviour. Run the narrowest safe,
repository-documented check that can confirm or refute a suspected defect. Do
not mutate the implementation during a review-only request. Distinguish product
defects from environment or tooling failures.

Report only actionable findings. A finding needs:

- severity: `critical`, `major`, `minor`, or `nit`;
- file and line (or the narrowest available location);
- the concrete failure or risk;
- evidence and the violated rule or requirement;
- the smallest credible correction.

Severity reflects impact, not confidence:

- `critical`: exploitable security, data loss, or unsafe release blocker;
- `major`: incorrect behaviour, material regression, or missing requirement;
- `minor`: bounded maintainability, resilience, or test weakness;
- `nit`: optional clarity improvement with no behavioural consequence.

If evidence is incomplete, label the item as a question or residual risk rather
than asserting a defect.

## 5. Report

Lead with findings ordered by severity within each axis:

```markdown
## Standards and implementation quality

- [major] `path/file.ts:42` — concrete issue, evidence, and correction.

## Specification

- No findings. | [major] `path/file.ts:87` — missing requirement and citation.

## Validation and residual risk

- Commands run and their results.
- Unreviewed or unverifiable areas.
```

If there are no findings, say so explicitly and still report the reviewed range,
validation performed, and residual risk. Do not add praise or generic checklist
noise in place of findings.
