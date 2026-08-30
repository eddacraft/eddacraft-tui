# ADR-139: Bound Conformance Range Evidence and Report Materialisation

## Status

Accepted 2026-08-30 (operator; CONF-011 verifier repair V-002)

## Date

2026-08-30

## Context

ADR-134 bounded commit selection and each commit's Git extraction, but did not
place an independent ceiling on the evidence retained across a range. A range
could therefore contain many individually admissible commits whose combined
records and bytes were impractical to aggregate. CONF-011 also publishes both
the deterministic range view and per-commit audit envelopes. That deliberate
duplication makes commit binding inspectable, but amplifies an unbounded range
during evaluation and serialisation.

The caller-owned 300-second limit originally covered Git identity and
extraction. CPU evaluation and report materialisation could continue after the
deadline, and direct streaming could expose a partial document before an output
limit was discovered. A production advisory check must fail honestly and emit
one complete small report under either condition.

## Decision

This ADR amends only ADR-134's resource-budget contract. It does not change
claim semantics, evidence binding, advisory posture, or the aggregate and
per-commit audit shapes.

### Whole-range evidence envelope

In addition to every per-commit ceiling, one evaluation admits at most:

| Budget | Limit |
| --- | ---: |
| Raw Git records across the selected range | 100,000 |
| Raw Git diff bytes across the selected range | 64 MiB |
| Decoded Git path bytes across the selected range | 64 MiB |

The limits equal the corresponding maximum already admitted for one commit, so
a formerly admissible maximal commit remains admissible while multiple commits
share one envelope. The extractor accumulates these counters before range
evaluation. Crossing a ceiling drops the accumulated publishable evidence and
returns one top-level, reason-coded `not-evaluated` result at the
`range-aggregation` stage. The diagnostic retains the exact selected-commit
cardinality and the complete cumulative counters observed at the crossing
commit. A failed per-commit extraction contributes diagnostic counters only; it
does not consume this publishable-evidence envelope or replace its more specific
failure reason.

The stable reasons are `budget.evaluation-records`,
`budget.evaluation-raw-bytes`, and `budget.evaluation-decoded-bytes`.

### Whole-run deadline and atomic reports

The five-minute monotonic deadline starts before the bounded declaration input
is read and spans repository identity, Git extraction, evaluation, and report
materialisation. A deadline crossed before or after CPU evaluation or report
materialisation yields `budget.run-timeout`, never a semantic verdict.

Plain, JSON, and SARIF output is materialised atomically into a capped buffer of
128 MiB before one stdout write. If the cap is crossed, the partial buffer is
discarded and replaced by one small reason-coded `not-evaluated` report with
`budget.report-bytes` at the `report-materialisation` stage. The same atomic
fallback applies when materialisation crosses the deadline. Blocking while the
already-bounded complete document is written to stdout is outside the
evaluative deadline because it is controlled by the caller's output consumer.

Selection cardinality remains honest. A fully resolved range can publish its
exact count on aggregate-budget, evaluation-timeout, or report-materialisation
failure. A truncated revision-list overflow cannot prove the total selected
population and therefore keeps `notEvaluatedCommitCount` unknown.

## Rationale

Sharing the existing one-commit maxima across the range is easy to explain,
keeps deterministic extraction semantics, and bounds the deep-copy and
aggregate/per-commit amplification without shrinking the largest previously
admissible single commit. Atomic materialisation preserves the one-document CLI
contract and makes overflow fail closed instead of producing invalid JSON or
SARIF.

### Alternatives Considered

| Option | Pros | Cons |
| --- | --- | --- |
| Whole-range envelope plus atomic report cap | Deterministic, fail-honest, bounded, preserves current audit schema | Large admissible reports can still approach 128 MiB |
| Remove aggregate or per-commit evidence | Smaller output | Breaks the accepted audit contract and loses either range or commit-local inspection |
| Keep only per-commit limits | No new counters | Multiplication by range length remains effectively unbounded |
| Stream output directly | Lower peak output buffer | Can expose partial JSON/SARIF before detecting overflow or timeout |

## Consequences

- **Positive:** Evaluation memory and report size no longer scale to the product
  of the commit-count and per-commit maxima.
- **Positive:** Timeout and output overflow always produce one complete advisory
  report with honest cardinality.
- **Positive:** The aggregate and per-commit evidence contract remains intact.
- **Negative:** A valid range whose combined evidence exceeds the shared
  envelope is not evaluated and must be narrowed.
- **Risk:** A report near the 128 MiB ceiling is still expensive.
- **Mitigation:** The range envelope bounds source evidence to 64 MiB, report
  materialisation is atomic, and the five-minute deadline covers construction.

## References

- Related ADRs:
  [ADR-134](134-intent-conformance-gating.md),
  [ADR-138](138-pin-git-administrative-state.md)
- APS module:
  [CONF-011](../modules/intent-conformance.aps.md#conf-011-external-pr-declaration-check)
- Verification: independent verifier V-002
