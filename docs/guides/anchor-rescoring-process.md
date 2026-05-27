# Anchor Re-Scoring Process

| Type  | Authority     | Owner   | Status | Freshness                                        |
| ----- | ------------- | ------- | ------ | ------------------------------------------------ |
| Guide | Authoritative | LANGCOV | Live   | Metadata backfilled 2026-05-24 during DOCGOV-009 |

| Upstream                                                 | Downstream                                       |
| -------------------------------------------------------- | ------------------------------------------------ |
| `plans/specs/2026-04-08-language-and-coverage-design.md` | Language anchor modules and re-scoring snapshots |

> Process gate that runs before each language anchor module begins
> implementation. Required by the
> [2026-04-08 Language and Coverage Design](../../plans/specs/2026-04-08-language-and-coverage-design.md)
> council review (§16.5 #8, council finding C-006).

## Why this gate exists

The language-and-coverage spec ranks anchor candidates against **demand × blast
radius × strategic fit × pack-unlock potential** (spec §6). The original ranking
was calibrated against two surveyed user stacks (Anvil itself + User B); the
2026-04-19 refresh added User C and re-scored Python.

The fragility (council C-006) is that the scoring is data-thin. A new
early-access user with a stack the design did not anticipate could invalidate
the anchor sequence silently. Specifically: a third user with a Go-heavy stack
would change the Track 1 sequence between Rust and Python, and the design owes
that user a re-evaluation rather than defaulting to the existing order.

This process gate ensures the re-scoring happens at a defined trigger point with
defined evidence — not "whenever someone remembers".

## When this gate runs

Run the re-scoring **before** any of these events:

1. The first work item on a Track 1 anchor module starts execution (LANGTS,
   RSTLAN, PYLAN).
2. A new early-access user is onboarded with a stack that includes a language
   not currently in the anchor set or the tail wave.
3. An existing surveyed user's stack changes materially (e.g. User B adopts Go;
   User C confirms Django vs FastAPI).
4. Six months have elapsed since the last re-scoring, regardless of other
   triggers.

The gate is **mandatory** for trigger 1 (anchor work cannot start without it)
and **strongly recommended** for triggers 2–4.

## What gets re-scored

For each candidate language currently in the anchor set, the tail wave, or under
serious consideration, re-evaluate the four §6 criteria:

| Criterion        | Question to answer                                                                                                                                |
| ---------------- | ------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Demand**       | How many surveyed users have this language in their repo today? Anvil counts as one user. Confirmed = mix data, not speculation.                  |
| **Blast radius** | Has anything changed in the candidate's typical use that shifts low/medium/high/critical?                                                         |
| **Strategic**    | Does the candidate still unlock the positioning story it was scored for? Has another candidate emerged that unlocks the same story for less work? |
| **Pack unlock**  | Has the named pack list for this candidate changed (additions, retirements, new third-party demand)?                                              |

For each candidate, produce a row in the re-scoring snapshot table (see template
below). Compare to the previous snapshot. Flag any candidate whose composite
changed enough to alter sequence position.

## Sequence change rule

If the re-scoring would change the sequence within Track 1 (TS → Rust → Python),
the anchor work pauses pending an explicit decision recorded as:

- An update to spec §17 (amendments applied) listing the re-score finding.
- Any affected anchor module's Ready Checklist re-validated against the new
  sequence.

If the re-scoring confirms the existing sequence, record the snapshot and
proceed.

If the re-scoring changes Track 2 (tail wave) membership but not Track 1
sequence, update the
[`lang-tail-wave`](../../plans/modules/lang-tail-wave.aps.md) module's candidate
table and the `LANGTAIL-001` grammar maturity audit task — no sequence pause
needed.

If the re-scoring promotes a tail-wave language to T2/T3 candidacy, that becomes
a separate "promotion" decision recorded as an ADR — out of scope for this gate.

## Owner

The re-scoring gate has **no permanent named owner** at the time of this guide.
Each invocation of the gate names a session owner before it runs — the session
owner is responsible for collecting evidence, producing the snapshot, and
recording the result. Default escalation path: anchor module owner if named,
otherwise the planning council convener.

This is a known gap. The §17.3 reconciliation actions list naming a permanent
owner as outstanding work; this guide is the process the owner will inherit when
named.

## Snapshot template

Each gate invocation produces a snapshot stored at
`plans/decisions/anchor-rescore-YYYY-MM-DD.md` with the table below. Snapshots
accumulate; the most recent one is canonical.

```markdown
# Anchor Re-Scoring Snapshot — YYYY-MM-DD

**Triggered by:** [trigger 1/2/3/4 with detail] **Session owner:** [name]
**Surveyed user mix at time of snapshot:** [Anvil, User B, User C, ...]

## Candidates

| Candidate                   | Demand | Blast | Strategic | Pack unlock | Composite | Δ from prior |
| --------------------------- | ------ | ----- | --------- | ----------- | --------- | ------------ |
| TypeScript                  | ...    | ...   | ...       | ...         | ...       | ...          |
| Rust                        | ...    | ...   | ...       | ...         | ...       | ...          |
| Python                      | ...    | ...   | ...       | ...         | ...       | ...          |
| Dart                        | ...    | ...   | ...       | ...         | ...       | ...          |
| ...other tail languages...  |        |       |           |             |           |              |
| ...new-demand candidates... |        |       |           |             |           |              |

## Outcome

- [ ] Sequence unchanged — anchor work proceeds.
- [ ] Sequence changed within Track 1 — pause; record amendment in spec §17.
- [ ] Tail-wave membership changed — update LANGTAIL.
- [ ] Promotion candidacy surfaced — open ADR.

## Notes

[Free text on judgement calls, near-misses, and anything that should inform the
next snapshot.]
```

## Anti-patterns

These shortcuts defeat the gate:

- **Skipping the gate because "the data hasn't changed"** — that is itself the
  finding the snapshot should record. Skipping leaves no evidence the gate ran.
- **Pre-deciding the outcome and writing a snapshot to match** — the template's
  `Δ from prior` column makes this visible to reviewers.
- **Tail-wave membership churn without the LANGTAIL update** — the tail-wave
  grammar maturity audit (LANGTAIL-001) consumes the snapshot; silent churn
  invalidates that audit.
- **One person deciding alone for trigger 1** — at minimum, the snapshot is
  reviewed by the anchor module owner before the work item starts. The session
  owner can be the same person, but the review must be separate.

## References

- Spec:
  [2026-04-08 Language and Coverage Design](../../plans/specs/2026-04-08-language-and-coverage-design.md)
  §6 (criteria), §7.2 (anchor set), §16.5 #8 (council requirement), §17
  (amendments)
- ADRs: [ADR-027](../../plans/decisions/027-pack-architecture.md) (pack
  architecture — pack ROI is a strategic input to anchor scoring)
- APS modules: [lang-ts-audit](../../plans/modules/lang-ts-audit.aps.md),
  [lang-rust](../../plans/modules/lang-rust.aps.md),
  [lang-python](../../plans/modules/lang-python.aps.md),
  [lang-tail-wave](../../plans/modules/lang-tail-wave.aps.md)
