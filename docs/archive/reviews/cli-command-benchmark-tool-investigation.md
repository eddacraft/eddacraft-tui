# CLI Command Benchmark Tool Investigation

| Type  | Authority | Owner | Status | Freshness                                                                                                                    |
| ----- | --------- | ----- | ------ | ---------------------------------------------------------------------------------------------------------------------------- |
| Guide | Advisory  | RLB   | Draft  | Investigated 2026-07-07 against `crates/anvil-bench`, `crates/anvil-cli`, `scripts/bench/run.sh`, and `benchmarks/README.md` |

| Upstream                                                                                                                                                                                                                          | Downstream                                                                                                    |
| --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------- |
| `crates/anvil-bench/src/{spawn.rs,proc_sampler.rs,watch_resource_budget.rs,report.rs,main.rs}`, `crates/anvil-cli/src/main.rs`, `scripts/bench/run.sh`, `benchmarks/README.md`, `plans/modules/resource-load-benchmarking.aps.md` | Proposed APS item for per-command CLI benchmarking; future `anvil-bench` and benchmark-history schema changes |

## Question

How should Anvil add a benchmarking tool that can measure individual `anvil` CLI
commands, such as `anvil status --verify`, `anvil check --all`, or
`anvil gate --only-checks import-boundaries`, without duplicating the existing
benchmark infrastructure or producing misleading numbers?

## Summary Recommendation

Build the tool inside `crates/anvil-bench` as an **Anvil-specific process-level
command benchmark runner**, not as a generic shell benchmarker. The current
harness already has the hard parts this needs: binary resolution via
`ANVIL_BENCH_ANVIL_BIN`, process-group cleanup, process-tree CPU/RSS sampling,
structured reports, synthetic repo fixtures, and the `pnpm bench` artefact
contract.

Recommended v1 shape:

```bash
# Build the product binary once, then benchmark one CLI command end-to-end.
cargo build -p eddacraft-anvil --release --bin anvil
ANVIL_BENCH_ANVIL_BIN=target/release/anvil \
  cargo run -p anvil-bench --bin anvil-bench-command -- \
  --name status-verify --repeat 30 --warmup 5 --fixture small \
  -- status --verify
```

The tool should measure end-to-end CLI process cost: startup, argument parsing,
auth/feature-flag routing, command execution, output generation, and durable
side effects under an isolated benchmark state root. For internal hot-path
latency, continue using Criterion benches in the owning crates.

## Existing Surfaces To Reuse

### `crates/anvil-bench` is the right home

`anvil-bench` is already active and describes itself as the stress/benchmark
harness that generates configurable repositories, measures timing and memory,
and emits structured JSON reports. Its README also names `pnpm bench` as the
single routine benchmark entrypoint and points individual benchmark runs at
`cargo bench -p anvil-bench --bench ...`.

Relevant source:

- `crates/anvil-bench/README.md:1-18` — current harness purpose and modules.
- `crates/anvil-bench/README.md:30-67` — current single-command benchmark-suite
  entrypoint and per-bench invocation pattern.
- `crates/anvil-bench/src/report.rs:8-120` — reusable `Metric`,
  `ScenarioResult`, and `BenchReport` JSON structures.
- `crates/anvil-bench/src/main.rs:1-9` and `:26-103` — existing stress runner is
  simple scenario filtering and report writing; it can stay stable while a new
  binary handles command benchmarks.

### Existing process helpers already solve cleanup and binary selection

The process resource benches already drive real `anvil` subprocesses and measure
their process trees. The key helpers are reusable:

- `crates/anvil-bench/src/spawn.rs:31-50` resolves the product binary from
  `ANVIL_BENCH_ANVIL_BIN`, then debug/release targets.
- `crates/anvil-bench/src/spawn.rs:16-29` and `:76-138` put the subprocess in a
  process group, check liveness, and kill/reap the group on drop.
- `crates/anvil-bench/src/proc_sampler.rs:1-23` explains why measuring only the
  parent process is wrong for Anvil commands that spawn child processes.
- `crates/anvil-bench/src/proc_sampler.rs:215-260` starts a tree sampler and
  emits CPU/RSS samples over a measurement window.

### `watch_resource_budget` is the closest implementation template

`watch_resource_budget` already benchmarks a real command path, runs it inside a
synthetic repo, disables update hints, sets `ANVIL_DEV=1`, captures
whole-process CPU/RSS, and refuses to report a successful measurement if the
child exits early. It is specific to long-running `watch`, but the process
lifecycle is directly reusable for finite CLI commands.

Relevant source:

- `crates/anvil-bench/src/watch_resource_budget.rs:72-120` — generate fixture,
  spawn `anvil`, settle, sample process tree, check liveness, shut down, and
  evaluate.
- `crates/anvil-bench/src/watch_resource_budget.rs:123-150` — pins the command
  arguments and tests that the benchmark exercises the intended production path.

### The routine benchmark contract already has a place to plug in

`scripts/bench/run.sh` is the stable local benchmark entrypoint. It writes logs
under `benchmark-results/manual-<timestamp>/`, runs compile checks and routine
benchmarks, builds the release `anvil` binary before resource benches, and
passes that binary through `ANVIL_BENCH_ANVIL_BIN`.

Relevant source:

- `scripts/bench/run.sh:17-39` — user-facing `pnpm bench` help and switches.
- `scripts/bench/run.sh:67-89` — per-run artefact directory and log wrapper.
- `scripts/bench/run.sh:115-129` — build release binary, then run resource
  benches with `ANVIL_BENCH_ANVIL_BIN=target/release/anvil`.

`benchmarks/README.md` already defines the durable benchmark-history schema and
warns that hardware must match for comparisons. A new command benchmark section
should extend that schema rather than invent a new data store.

Relevant source:

- `benchmarks/README.md:15-43` — current schema.
- `benchmarks/README.md:46-72` — quiet-box and same-hardware comparability
  rules.
- `scripts/bench/to-history.py:26-45` and `:64-129` — log-name registry plus
  parsers for bencher, latency-gate, and resource-budget output.

## Proposed Tool Design

### Binary and module layout

Add a new binary and library module without changing the existing stress-runner
CLI:

```text
crates/anvil-bench/src/cli_command.rs          # config, runner, stats, report
crates/anvil-bench/src/bin/anvil-bench-command.rs
crates/anvil-bench/tests/cli_command.rs        # fake-binary integration tests
```

Using a separate binary avoids breaking the existing
`cargo run -p anvil-bench --release -- <scenario>` stress-runner contract.

### Command-line interface

Recommended v1 options:

```text
anvil-bench-command \
  --name <label> \
  --repeat <n> \
  --warmup <n> \
  --fixture <empty|small|default|path:...> \
  --timeout-ms <n> \
  --sample-interval-ms <n> \
  --output benchmark-results/manual-.../cli-command-<label>.json \
  -- <anvil args...>
```

Rules:

1. The executable is always the resolved `anvil` binary; the tail after `--` is
   **arguments to Anvil**, not an arbitrary shell command.
2. Run the subprocess directly with `Command::new(anvil_bin).args(args)`, not
   via `bash -c`, so quoting does not alter the measured command and the tool
   does not become a shell runner.
3. Default to an isolated temp `ANVIL_HOME` and temp project fixture. Require an
   explicit `--cwd path:...` or `--fixture path:...` when benchmarking a live
   checkout.
4. Default to benchmark-safe environment:
   - `ANVIL_DISABLE_UPDATE_HINT=1`
   - `ANVIL_USAGE_DISABLE=1`
   - `ANVIL_INTERCEPT_DISABLE_OBSERVATION=1`
   - `DO_NOT_TRACK=1`
   - temp `ANVIL_HOME`
   - optional `ANVIL_DEV=1` only when the benchmark profile declares it
5. Refuse to record raw argument values in JSON by default. Store the label,
   command family, exit code, duration/resource statistics, stdout/stderr byte
   counts, and a redacted argument shape. Add `--include-raw-argv` only if a
   future reviewed use case needs it.

### Metrics

For each measured iteration:

- exit code
- wall-clock duration in milliseconds
- stdout byte count
- stderr byte count
- CPU percentage where `100.0` is one core, when `/proc` sampling is available
- peak RSS MiB, when `/proc` sampling is available
- timeout / startup failure classification

Aggregate report:

- samples, failures, timeouts
- min, mean, median, p95, p99 for wall-clock duration
- max stdout/stderr bytes
- p95/max CPU and RSS when available
- fixture name/spec, binary path hash or version, commit, rustc version, host
  metadata, and benchmark-safe env switches

This should use a new report object rather than forcing everything into the
existing `ScenarioResult.metrics` list. `ScenarioResult` is fine for stress
scenarios, but per-command samples need structured per-iteration data so p95 and
failure counts remain auditable.

### Fixture profiles

Start with three deterministic profiles:

| Fixture   | Purpose                                                                                              |
| --------- | ---------------------------------------------------------------------------------------------------- |
| `empty`   | Startup, help, version, and read-only status surfaces.                                               |
| `small`   | A small generated repo for fast `check`, `gate`, `architecture validate`, and `drift` smoke timings. |
| `default` | Current `RepoSpec::default()` for realistic command-path measurements.                               |

Fixture generation should reuse `crates/anvil-bench/src/fixture.rs`. Commands
that intentionally mutate state should run only in temp fixtures unless the user
passes an explicit live path and acknowledgement flag.

### Integration into `pnpm bench`

Keep arbitrary ad-hoc command benchmarking manual by default. Add only a small,
stable curated set to `scripts/bench/run.sh`, for example:

```bash
run_logged_shell cli-command-status-verify \
  "ANVIL_BENCH_ANVIL_BIN=target/release/anvil cargo run -p anvil-bench --bin anvil-bench-command -- --name status-verify --repeat 20 --warmup 3 --fixture empty -- status --verify"
run_logged_shell cli-command-version \
  "ANVIL_BENCH_ANVIL_BIN=target/release/anvil cargo run -p anvil-bench --bin anvil-bench-command -- --name version --repeat 20 --warmup 3 --fixture empty -- version"
```

Then extend `benchmarks/README.md` and `scripts/bench/to-history.py` with a
`cli_commands` section keyed by label. Keep `scripts/bench/run.sh` as the single
source of routine surfaces.

## Ownership Recommendation

Create a new APS work item under `resource-load-benchmarking.aps.md`, not
`cli-command-truth.aps.md`.

Rationale:

- RLB already owns process-level CPU/RSS coverage, load harnesses, and benchmark
  SLOs for `watch`, intercept, MCP, and concurrent process paths.
- CLICT owns documentation/runtime command truth; it should not own benchmark
  tooling implementation.
- TCOV-026 owns the broader routine-benchmark contract alignment and should be
  linked as a coordination point if `scripts/bench/run.sh`,
  `scripts/bench/to-history.py`, or `benchmarks/README.md` are changed.

Suggested item shape, if the investigation is accepted:

```markdown
### RLB-009: Per-command CLI benchmark runner

- **Status:** Ready
- **Intent:** Add an Anvil-specific runner that measures individual finite
  `anvil` CLI commands end-to-end with repeat/warmup controls, isolated state,
  safe argv redaction, and JSON reports.
- **Expected Outcome:** Operators can run one command benchmark without editing
  Criterion benches or `scripts/bench/run.sh`; curated command benchmarks can be
  normalised into `benchmarks/history/`.
- **Files:** `crates/anvil-bench/src/cli_command.rs`,
  `crates/anvil-bench/src/bin/anvil-bench-command.rs`,
  `crates/anvil-bench/tests/cli_command.rs`, `crates/anvil-bench/README.md`,
  optional `scripts/bench/run.sh`, `scripts/bench/to-history.py`,
  `benchmarks/README.md`
- **Validation:** `cargo test -p anvil-bench cli_command`; smoke command against
  `status --verify`; `pnpm docs:check` if docs change.
- **Coordinates with:** TCOV-026 for routine bench/history-schema alignment.
```

## Implementation Slices

1. **Runner core** — parse CLI options, resolve `anvil`, create isolated temp
   state, run warmups, run measured repeats, collect wall-clock and exit/output
   metrics, write JSON.
2. **Linux resource sampling** — fold in `TreeSampler` for per-iteration CPU/RSS
   where `/proc` is available; keep non-Linux wall-clock-only rather than
   failing the whole tool.
3. **Fixture selection** — support `empty`, `small`, `default`, and explicit
   `path:` fixtures; document which commands are safe on each.
4. **Redaction and safety** — default to redacted argv shape, temp `ANVIL_HOME`,
   usage/observation opt-outs, no shell execution, timeout enforcement, and
   process-group cleanup.
5. **Routine-suite integration** — add one or two stable command benchmarks to
   `scripts/bench/run.sh`, extend history normalisation, and document the
   history schema.

## Risks and Guardrails

| Risk                                                   | Guardrail                                                                                                                                                     |
| ------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Numbers compare badly across machines                  | Reuse `benchmarks/README.md` same-hardware rule and include host metadata.                                                                                    |
| Benchmark pollutes usage analytics or user credentials | Default to temp `ANVIL_HOME` plus `ANVIL_USAGE_DISABLE=1`, `ANVIL_INTERCEPT_DISABLE_OBSERVATION=1`, and `DO_NOT_TRACK=1`.                                     |
| Raw command args leak secrets into reports             | Redact argv by default; avoid raw argv storage unless a reviewed flag is explicitly passed.                                                                   |
| Tool becomes arbitrary command execution               | Resolve and execute only the Anvil binary; tail args are Anvil args, not a shell command.                                                                     |
| Stateful commands mutate the developer repo            | Default to temp fixtures; require explicit live path and acknowledgement for live repo benchmarking.                                                          |
| Long-running commands skew finite-command stats        | Treat v1 as finite-command only; keep `watch`, daemon, MCP, and concurrent resources in existing RLB benches unless a future item adds duration-mode support. |

## Conclusion

The repo does not need a new benchmark subsystem. It needs a small, Anvil-only
command-runner binary inside `anvil-bench`, built on the existing process,
fixture, resource-sampling, and history-normalisation infrastructure. The first
implementation should optimise for trustworthy local investigation of one CLI
command at a time; only a curated subset should be wired into `pnpm bench` and
history once the JSON shape is stable.
