# ADR-052: Automated Drift Capture — Edge-Delta Event Ledger

## Status

Proposed

## Date

2026-05-27

## Context

`anvil drift snapshot`/`report` already exist (`crates/anvil-cli/src/commands/drift.rs`),
writing a whole-state `DriftSnapshot` to `.anvil/snapshots/`. The drift *trend*
the product needs — to serve the success criterion **"new cross-boundary edges
per sprint decreases by 30% within 8 weeks"** and the planned
`anvil insights --drift` sparkline (INSIGHTS-003) — requires a populated,
team-comparable, local-only **time-series**. Capture is currently manual, so the
series is empty for almost every repo and there is no data to measure against.

A planning council (`plan-0e9c300c`: architect, delivery lead, devil's advocate)
investigated how to populate it and surfaced three findings that reframe the
decision away from the original "scheduled CI workflow → weekly snapshot PR":

1. **The consumer's spec names a different source.** INSIGHTS-003
   (`plans/modules/usage-insights.aps.md`) says its data "is derived from
   **baseline diff entries**" (`crates/anvil-baseline/*`) — *not*
   `.anvil/snapshots/`. The original ADR silently re-pointed it at whole-state
   snapshots.
2. **Sampling loses signal.** A weekly whole-state snapshot measures `main` HEAD
   once per week, so an edge added and removed within the same week nets to zero
   — the metric goes blind to intra-week churn. The actual drift signal is the
   **edge delta** (`BaselineDiff.added`/`removed`), which is lossless if captured
   as an event.
3. **The snapshot record can't support a valid trend.** `DriftSnapshot` carries
   no `anvil_version` and no `rules_sha`, so a week-over-week delta conflates
   "code drifted" with "the rule set / org-policy ref changed" — not a
   controlled measurement.

Anvil is local-only (no telemetry); `main` is trunk-protected (PR + checks); the
metric is team/sprint-level, so the canonical series must be shared and
comparable, not divergent per-developer-machine.

## Decision

Capture drift as an **append-only edge-delta event ledger**, not periodic
whole-state snapshots:

- **Data model:** a new in-tree NDJSON ledger `anvil/drift/edges.ndjson`
  (`merge=union`, like the witness/CI-log), local-only. One record per change
  that alters cross-boundary edges, shaped
  `{ ts, commit_sha, anvil_version, rules_sha, added: [<entry>], removed: [<entry>] }`,
  where each `<entry>` is an `anvil_baseline::BaselineDiffEntry`
  (`rule_id`, `file_path`, `fingerprint`) — the delta is exactly
  `BaselineDiff.added`/`removed`. Richer cross-boundary attribution
  (`from_layer`/`to_layer`) is available from the drift snapshot's `violations`
  (`SnapshotViolation`) if a future consumer needs it; the exact recorded field
  set is pinned at implementation (see Open Implementation Questions).
  Whole-state `.anvil/snapshots/` (the existing `anvil drift snapshot`) remains
  supported for point-in-time comparison but is **not** the trend source.
- **Capture is event-driven on merge to `main`, not a wall-clock timer.** The
  canonical record is appended when new cross-boundary edges land on `main`,
  riding the same PR that introduced them — no separate scheduled workflow, no
  separate auto-merge PR, no trunk bypass. Preferred write actor (pinned at
  implementation): the existing required CI check on the PR computes the
  `BaselineDiff` against `main`'s recorded state and, when non-empty, appends the
  record to the PR branch; the local `anvil baseline --refresh` path appends the
  same record as the offline fallback. Manual `anvil drift snapshot` and any
  opportunistic local capture are **supplements**, never the canonical series.
- **Consumers:** `anvil drift report` and INSIGHTS-003 read
  `anvil/drift/edges.ndjson`, bucket records by `ts` into weeks, and report
  "insufficient data" honestly when fewer than two weeks of records exist.

This resolves two of the council's correctness gaps by construction: because the
ledger holds *every* add/remove event, INSIGHTS-003 can compute **both** net and
gross ("peak") new-edges-per-week (no forced peak-vs-net choice); and because
each record carries `anvil_version` + `rules_sha`, consumers can segment or annotate
the series when a rule-set change — not code — moved the number.

## Rationale

The trigger question was downstream of a more fundamental one: *what is captured*.
Capturing the edge **delta** as an event is lossless, matches INSIGHTS-003's
declared source (`baseline diff entries`), and is intrinsically event-aligned to
when edges are actually introduced — so it neither misses intra-week churn nor
depends on a wall-clock cadence. Writing it on merge-to-`main` via the PR that
caused the change keeps the series team-shared and trunk-safe without a scheduled
workflow, a recurring auto-merge PR, or a ruleset bypass.

### Alternatives Considered

| Option | Pros | Cons | Verdict |
|--------|------|------|---------|
| **Edge-delta event ledger on merge (chosen)** | Lossless; matches consumer spec; event-driven (no intra-week blindness); supports net + peak; carries determinism fields; rides existing PR, no scheduler/extra-PR/bypass | New on-disk format + writer to version; series advances only when edges change (needs explicit zero-fill for quiet weeks); write-actor on merge needs pinning (CI-appends-to-PR has a fork-PR caveat) | **Chosen** |
| Scheduled CI workflow → weekly auto-merge PR (original proposal) | Shared canonical series; weekly cadence = trend granularity | Samples whole state weekly → blind to intra-week add/remove; recurring PR to triage forever; must build/obtain `anvil` weekly; "auto-merge, no bypass" is in tension (auto-merge needs a reviewer or a scoped bypass); pruning was CI-shell-only, skipped on manual runs | Rejected — sampling loses signal; high ops tail |
| Lazy opportunistic capture on `anvil insights`/`drift` run | Zero infra; planless; lives inside the consuming command | Series density tracks user engagement, not calendar → gaps-then-bursts read as false deterioration; silent no-data for non-interactive users | Rejected as canonical; viable only as a local supplement |
| CI artefact → orphan/data branch (not `main`) | No PR-noise; main tree stays clean; bot pushes direct with no main bypass | Off-main data ref hurts discoverability/onboarding; needs its own force-push protection or history can be silently rewritten; CLI must fetch a ref (breaks local-only-at-command-time) | Rejected — integrity + discoverability cost |
| Release/tag-time capture | Sprint-ish cadence; reuses release build | Too coarse on typical cadences (monthly release → 2 points in 8 weeks); quiet gaps unsampled | Rejected as primary; useful only as supplementary release markers |
| Reuse the witness chain as the series | Hardened append/verify/timestamp/share already shipped | Records hook events, not boundary counts; per-commit (not per-sprint) cadence; per-machine + uncommitted by default; pollutes a tamper-evidence artefact with analytics and turns a metrics-schema change into a hash-chained-line change | Rejected |
| Intercept-daemon timer / post-commit hook | Local, no CI | Per-developer-machine series diverge (can't aggregate to a team metric); daemon only runs when up; post-commit snapshots land as uncommitted working-tree churn, violating atomic commits | Rejected (fatal for a team metric) |

## Consequences

- **Positive:** the drift success criterion becomes measurable on a lossless,
  event-aligned series; INSIGHTS-003 + `anvil drift report` get the source their
  spec names; net *and* peak are both derivable; rule-set vs code changes are
  distinguishable; no scheduled workflow, no recurring auto-merge PR, no trunk
  bypass; stays local-only and team-shared (in-tree, union-merge).
- **Negative:** a new on-disk ledger format + writer to design, version, and
  document; the "append on merge" write actor still needs pinning (CI-appends-to-
  PR has a fork-PR caveat; the local `anvil baseline --refresh` fallback relies on that
  path being used); a zero-fill rule is needed so quiet weeks render honestly
  rather than as gaps.
- **Risks:** if no write actor is wired, the ledger is empty and INSIGHTS-003
  permanently reports "insufficient data" (the same silent-no-data failure the
  council flagged for every option); a non-deterministic scan would corrupt
  deltas.
- **Mitigations:** ship the write actor *before* (or with) INSIGHTS-003 so the
  feature never ships against an empty source; record `anvil_version` + `rules_sha`
  so non-determinism is detectable; distinguish "insufficient data yet" from
  "capture pipeline silently stopped" in the consumer's output.

## Open Implementation Questions

Carried from the council; settled during implementation, not in this ADR:

1. **Write actor on merge:** CI check appends the delta to the PR branch
   (fork-PR caveat) vs. a local `anvil baseline --refresh` / pre-push-to-`main`
   append vs. a post-merge follow-up. Decide before the implementing PR.
2. **Zero-fill semantics:** how INSIGHTS-003 renders weeks with no edge change
   (explicit `0`, not a gap) without implying missing data.
3. **Ledger retention/rollover:** append-only NDJSON growth; reuse the witness
   manifest rollover pattern, or leave unbounded (records are small).
4. **Ledger schema versioning** (a `schema_version`/format pin on the file).

## References

- Planning council: `plan-0e9c300c` (architect / delivery lead / devil's advocate)
- Drift machinery: `crates/anvil-cli/src/commands/drift.rs`; the actual signal:
  `crates/anvil-baseline/src/diff.rs` (`BaselineDiff`)
- APS: INSIGHTS-003 (consumer; spec names "baseline diff entries" as its source);
  a new INSIGHTS item — filed with the implementation once this ADR is accepted —
  tracks the ledger + write actor; archived DRIFT module shipped
  `anvil drift snapshot`/`report`
- Success criterion: "new cross-boundary edges per sprint decreases by 30% within
  8 weeks" (`plans/index.aps.md`)
