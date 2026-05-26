# Resource Budget — `anvil watch`

| Type  | Authority     | Owner                                                                                                            | Status | Freshness                                                                                                                      |
| ----- | ------------- | ---------------------------------------------------------------------------------------------------------------- | ------ | ------------------------------------------------------------------------------------------------------------------------------ |
| Guide | Authoritative | ADOPT ([`plans/archive/modules/adoption-friction.aps.md`](../../plans/archive/modules/adoption-friction.aps.md)) | Live   | Last reviewed 2026-05-16 against `crates/anvil-bench/src/watch_resource_budget.rs` and `.github/workflows/resource-budget.yml` |

| Upstream                                                                    | Downstream                                                                                     |
| --------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------- |
| `crates/anvil-bench` (`budget`, `fixture`, `watch_resource_budget` modules) | `.github/workflows/resource-budget.yml`, `crates/anvil-bench/benches/watch_resource_budget.rs` |

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

## Reference Fixture

The bench scenario runs against the synthetic repository produced by
`crates/anvil-bench::fixture` with `RepoSpec::default()` and
`LanguageMix::default()`. The fixture is generated from a deterministic seed so
the same source revision produces the same file tree on every run — that is what
makes the ceiling meaningful across machines and runners.

## Measurement protocol

`cargo bench -p anvil-bench --bench watch_resource_budget` starts `anvil watch`
against the fixture with `ANVIL_DEV=1` for the local credential pre-check and
allows it to settle. The bench scenario then:

1. Waits past the initial scan window using the scenario-owned settle duration.
2. Samples CPU steady-state across the measurement window from `/proc` tick
   deltas.
3. Samples peak RSS across the same window from `/proc/<pid>/status`.
4. Emits a `MeasurementSample` and feeds it to `anvil_bench::budget::evaluate`.

`evaluate` treats "exactly at the ceiling" as a Pass. The sampler emits the raw
derived CPU and RSS values so CI logs show headroom without hiding slow drift
behind rounding.

Set `ANVIL_BENCH_ANVIL_BIN` to point at the binary under test. If it is unset,
the bench uses `target/debug/anvil` or `target/release/anvil` when present.

## CI assertion

The `.github/workflows/resource-budget.yml` job builds a release `anvil` binary,
runs the scenario, captures the JSON verdict, uploads it as an artifact, and
fails the build when `status != "pass"`. The verdict shape is:

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
- Watch budget sampler — `crates/anvil-bench/src/watch_resource_budget.rs`
- CI gate — `.github/workflows/resource-budget.yml`
- APS — `plans/archive/modules/adoption-friction.aps.md` (ADOPT-002)
