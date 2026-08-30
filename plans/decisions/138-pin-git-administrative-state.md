# ADR-138: Pin Git Administrative State During Conformance Evaluation

## Status

Accepted 2026-08-30 (operator; approved C-009 repair and verifier V-001)

## Date

2026-08-30

## Context

ADR-134 closes the Git child-process environment and rejects replacement refs,
legacy grafts, and shallow repositories before intent-conformance evaluation.
Its original three-value `GIT_*` allowlist was sufficient for observing the
repository's real administrative state, but it did not pin that state for later
revision, traversal, diff, and object reads. A concurrent write to
`$GIT_COMMON_DIR/shallow` or `$GIT_COMMON_DIR/info/grafts` could therefore
change the commit graph after admission.

Pinning alternate shallow and graft paths before admission would create the
opposite error: Git would stop observing pre-existing repository state, allowing
a shallow or grafted repository to appear complete. The contract needs two
explicit phases rather than one environment described too broadly.

This is a same-user process boundary, not an operating-system isolation
boundary. A hostile process running as the same operating-system user may be
able to inspect or modify evaluator-owned temporary state, Git objects, or the
selected executable. This decision prevents ordinary concurrent repository
administration from changing the admitted graph; it does not claim protection
from a determined same-user adversary.

## Decision

This ADR narrowly amends section 3 of ADR-134. All Git child processes start
from an empty environment, keep `LC_ALL=C` and `TZ=UTC`, use the previously
resolved absolute Git executable, pass `--no-replace-objects`, and retain the
pinned command-line configuration. Their permitted `GIT_*` variables depend
on the phase.

### Administration-observation phase

Commands that inspect the repository's real administrative state admit exactly
three `GIT_*` values:

| Variable | Value |
| --- | --- |
| `GIT_CONFIG_NOSYSTEM` | `1` |
| `GIT_CONFIG_GLOBAL` | evaluator-owned empty configuration file |
| `GIT_OPTIONAL_LOCKS` | `0` |

No `GIT_SHALLOW_FILE` or `GIT_GRAFT_FILE` override is present in this phase.
The evaluator checks the canonical common directory for
`info/grafts`, queries the real shallow state, and rejects any pre-existing
replacement ref, legacy graft, or shallow repository as reason-coded not
evaluated.

### Post-admission phase

Only after those checks pass, every resolution, traversal, diff, and object read
admits exactly five `GIT_*` values:

| Variable | Value |
| --- | --- |
| `GIT_CONFIG_NOSYSTEM` | `1` |
| `GIT_CONFIG_GLOBAL` | evaluator-owned empty configuration file |
| `GIT_OPTIONAL_LOCKS` | `0` |
| `GIT_SHALLOW_FILE` | run-owned, never-written shallow sentinel path |
| `GIT_GRAFT_FILE` | run-owned, never-written graft sentinel path |

The empty configuration and both sentinel paths share one evaluator-owned
temporary directory, created private to the run where the platform supports
that guarantee. The evaluator creates the empty configuration file but never
creates or writes either sentinel file. The paths therefore remain absent
during a normal run and direct Git away from later shallow or graft mutations
under the repository's common directory.

The implementation must test both halves of the contract: pre-existing
administrative state remains visible and is rejected, while a shallow or graft
file created in the repository after the observation check cannot change the
evaluated graph. Documentation must call these paths run-owned and
never-written, not operating-system immutable.

## Rationale

A phase-specific allowlist preserves fail-honest admission and then fixes the
time-of-check/time-of-use gap without copying the repository or adding Git
subprocesses to the evaluation hot path. It also makes the environment contract
testable as two closed sets rather than relying on an informal statement that
ambient `GIT_*` state is cleared.

### Alternatives Considered

| Option | Pros | Cons |
| --- | --- | --- |
| Observe real state, then use never-written sentinel paths | Preserves rejection of pre-existing state and pins later graph reads with no extra Git subprocess | Does not isolate the evaluator from a hostile same-user process |
| Set shallow/graft overrides before observation | One environment for every command | Hides the real repository state and can admit incomplete or grafted history |
| Observe once, then keep consulting repository paths | Simple and reflects later administration | Leaves a time-of-check/time-of-use gap and makes one run's graph mutable |
| Clone or snapshot the repository for each run | Stronger isolation from repository administration | Material I/O, latency, object-storage, and lifecycle cost disproportionate to an advisory check |

## Consequences

- **Positive:** Pre-existing replacement, graft, and shallow state remains
  visible and fail-honestly rejected.
- **Positive:** Ordinary concurrent shallow/graft administration after
  admission cannot redirect later Git graph reads.
- **Positive:** The three-value observation allowlist and five-value
  post-admission allowlist are explicit, reviewable, and testable.
- **Negative:** The evaluator owns temporary configuration and sentinel path
  lifecycle for each run.
- **Risk:** Same-user hostile code can attack resources outside this contract
  or may discover and alter run-owned paths.
- **Mitigation:** State the boundary honestly, keep the temporary directory
  private where supported, never write the sentinel files, retain
  replacement-disabled commands and exact object bindings, and treat any
  environment or administrative-state setup failure as not evaluated.

## References

- Related ADRs:
  [ADR-134](134-intent-conformance-gating.md),
  [ADR-002](002-warnings-over-blocks.md)
- APS modules:
  [CONF-011](../modules/intent-conformance.aps.md#conf-011-external-pr-declaration-check)
- Verification: Council C-009 repair and independent verifier V-001
