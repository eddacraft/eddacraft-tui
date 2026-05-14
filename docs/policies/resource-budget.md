# Resource Budget — `anvil watch`

| Type  | Authority     | Owner                                                                                            | Status | Freshness                                                                                                 |
| ----- | ------------- | ------------------------------------------------------------------------------------------------ | ------ | --------------------------------------------------------------------------------------------------------- |
| Guide | Authoritative | ADOPT ([`plans/modules/adoption-friction.aps.md`](../../plans/modules/adoption-friction.aps.md)) | Draft  | Last reviewed 2026-05-14 against `main` for v0.7.0-beta sit-on; implementation state `Pre-implementation` |

| Upstream                                                      | Downstream                                                                                                             |
| ------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------- |
| `crates/anvil-bench` (`budget`, `fixture`, `measure` modules) | `.github/workflows/resource-budget.yml` (follow-up), `crates/anvil-bench/benches/watch_resource_budget.rs` (follow-up) |

Pinned ceiling for `anvil watch` on the reference benchmark fixture. Anvil's
adoption test is that senior users do not notice it on their battery or CPU
graph during sustained daily use; this policy is the hard line, enforced in CI.

## Ceiling (v1)

| Axis             | Ceiling | Rationale                                                                                                                                                                                                                                                   |
| ---------------- | ------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Steady-state CPU | 5%      | Below the per-core threshold most laptop battery dashboards aggregate into a visible bar. Tracked once the initial scan has settled — startup spikes are out of scope, persistent background draw is the failure mode senior users actually complain about. |
| Peak RSS         | 200 MiB | Comparable to a single VS Code window's resident set on a quiet repo. Two of these is roughly what users tolerate before they ask what is running.                                                                                                          |

These values live in source as `ResourceBudget::ANVIL_WATCH_V1` in
`crates/anvil-bench/src/budget.rs`. Bumping either field requires:

1. An entry in `plans/decisions/DECISION-LOG.md` recording the new value and why
   the previous ceiling is no longer reachable, and
2. A user-facing release note in the next candidate.

Silent drift defeats the point of the budget. The pinned test
`anvil_watch_v1_ceiling_is_pinned` makes any constant change a visible diff, but
a reviewer can change both the constant and the test in the same commit and the
test alone will go green. The DECISION-LOG + release note steps are the only
human gate against an intentional bump landing without review; treat this as a
process invariant, not an enforced one.

## Reference fixture

The bench scenario runs against the synthetic repository produced by
`crates/anvil-bench::fixture` with `LanguageMix::default()`. The fixture is
committed and seeded so the same SHA produces the same file tree on every run —
that is what makes the ceiling meaningful across machines and runners.

## Measurement protocol

`anvil watch` is started against the fixture and allowed to settle. The bench
scenario then:

1. Waits past the initial scan window (deterministic — owned by the scenario,
   not wall-clock guessed).
2. Samples CPU steady-state across the measurement window at **0.1%**
   resolution.
3. Samples peak RSS across the same window via the existing `MemoryGuard`
   primitive in `crates/anvil-bench/src/measure.rs` at **1 MiB** resolution.
4. Emits a `MeasurementSample` and feeds it to `anvil_bench::budget::evaluate`.

The precision floor matters because `evaluate` treats "exactly at the ceiling"
as a Pass. Sampling noisier than the precision floor would make that boundary
rule meaningless; sampling finer than the floor is allowed and harmless.

The bench scenario itself is the follow-up step on this APS item; this primitive
ships the comparison contract first so CI integration work has a stable target.

## CI assertion

The `.github/workflows/resource-budget.yml` job (added in the follow-up step)
runs the scenario, captures the JSON verdict, and fails the build when
`status != "pass"`. The verdict shape is:

```jsonc
{
  "schema_version": 1,
  "status": "pass" | "fail_cpu" | "fail_rss" | "fail_both",
  "budget": { "steady_state_cpu_pct": 5.0, "peak_rss_mib": 200.0 },
  "sample": { "steady_state_cpu_pct": 0.8, "peak_rss_mib": 142.3 },
  "cpu_over_pct": -4.2,   // negative = headroom
  "rss_over_mib": -57.7
}
```

`schema_version` (pinned in Rust as `BUDGET_VERDICT_SCHEMA_VERSION`) is bumped
whenever a field is added or renamed. CI scripts should read it before parsing —
an unknown version is itself a failure mode.

CI logs the full JSON on every run so headroom (negative values) is visible even
on green builds — slow drift is detectable before it becomes a failure.

## Out of scope (filed separately)

- Cold-start RAM footprint of `anvil start` (separate budget — file if/when
  there is a complaint).
- Watcher CPU during very large recursive scans (`audit` covers that surface).
- Daemon RAM during multi-agent bursts (covered by the intercept benchmarks).

## Cross-references

- `ResourceBudget::ANVIL_WATCH_V1` — `crates/anvil-bench/src/budget.rs`
- Fixture generator — `crates/anvil-bench/src/fixture.rs`
- Memory measurement — `crates/anvil-bench/src/measure.rs`
- APS — `plans/modules/adoption-friction.aps.md` (ADOPT-002)
