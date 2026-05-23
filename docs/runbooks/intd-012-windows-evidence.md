# INTD-012 — Windows Confidence Evidence

**Date:** 2026-05-06 **Branch:** `a2/wave1-windows-confidence` **Owner:** A2
Wave 1 **Status:** Honest-scope artefact for INTD-012 (Windows CI Matrix)

> **Status update (2026-05-08):** the `cross-compile` trigger gap this document
> was written against has been closed. PR #1325 (`ed957ce1`) widened the gate to
> fire on pushes to `main` **and** `dev`, and on PRs targeting either branch —
> see
> [`.github/workflows/rust.yml#L442-L445`](../../.github/workflows/rust.yml#L442-L445)
> for the current `if:` and
> [`docs/archive/runbooks/v0.6.0-beta-release-runbook.md`](../archive/runbooks/v0.6.0-beta-release-runbook.md)
> §5 for the operator-facing scope description. The TL;DR, "Workflow trigger
> reality" section, and "Items deliberately deferred" item below are preserved
> as the original 2026-05-06 evidence record but no longer reflect the current
> workflow gate. Feature branches without a PR to `main`/`dev` still skip
> cross-compile; that's the residual gap.

This runbook records the evidence that the four intercept crates
(`anvil-intercept`, `anvil-intercept-proto`, `anvil-intercept-rules`,
`anvil-intercept-win32`) build and pass tests on `x86_64-pc-windows-msvc`
without a separate Windows job needing to be added — the workspace matrix in
`.github/workflows/rust.yml` already covers them when the matrix runs. It also
flags the gap in _when_ that matrix runs.

## TL;DR

- The `Cross (x86_64-pc-windows-msvc)` job in `.github/workflows/rust.yml`
  builds **all four intercept crates** (verified by name in the build output)
  and runs `cargo test --workspace --target x86_64-pc-windows-msvc`.
- The most recent runs on `main` are green; intercept-crate test counts are
  stable run-over-run.
- **Gap:** the `cross-compile` job is gated by
  `if: (push to main) || (pull_request to main)`. PRs targeting `dev` and pushes
  to `dev` skip the Windows matrix entirely. Drift on Windows is therefore only
  caught at the dev → main release-sync, not on every change. This is a
  deliberate cost/coverage trade-off recorded here; expanding the trigger is
  **out of scope** for this PR per the A2 Wave 1 hard rules ("do not add a
  separate Windows CI matrix entry").
- One Windows-only gate has been added in this branch:
  `crates/anvil-intercept/src/ipc.rs::ipc::tests::named_pipe_scan_buffer_envelope_parity_with_embedded`,
  which mirrors the Linux UDS parity test
  (`crates/anvil-cli/src/mcp/validation.rs::mcp::validation::tests::local_daemon_client_returns_scan_buffer_diagnostics_with_embedded_parity`)
  for the named-pipe transport. It will fail closed if a future change silently
  desyncs the Windows daemon-backed `scan_buffer` envelope from the embedded
  `EnforcementPipeline` path.

## Workflow trigger reality

`.github/workflows/rust.yml` lines 354–384 (cross-compile job):

```yaml
cross-compile:
  name: Cross (${{ matrix.target }})
  if: >-
    (github.event_name == 'push' && github.ref == 'refs/heads/main')
    || (github.event_name == 'pull_request' && github.base_ref == 'main')
  ...
  matrix:
    include:
      - target: x86_64-pc-windows-msvc
        os: windows-latest
        can_test: true
      - target: aarch64-pc-windows-msvc
        os: windows-latest
        can_test: false
```

The `Test`, `Clippy`, `Format`, `Check`, `cargo-deny`, `Hakari verify`, and
`Acknowledgements freshness` jobs run on every push and PR. The `Cross (...)`
job — the only one that builds for Windows — runs **only** on push to `main` or
PRs targeting `main`.

## Most recent successful Windows runs (push to `main`)

| Run                                                                            | Date       | Commit    | Title                                                             | Cross (x86_64-pc-windows-msvc) |
| ------------------------------------------------------------------------------ | ---------- | --------- | ----------------------------------------------------------------- | ------------------------------ |
| [25375091014](https://github.com/eddacraft/anvil-001/actions/runs/25375091014) | 2026-05-05 | `a08ba84` | fix(docs): use next() in upstream middleware to fall through      | success                        |
| [25327005734](https://github.com/eddacraft/anvil-001/actions/runs/25327005734) | 2026-05-04 | `21925c5` | hotfix: docs-shell empty-body proxy fix + dev sync (#1264)        | success                        |
| [25285311593](https://github.com/eddacraft/anvil-001/actions/runs/25285311593) | 2026-05-03 | `1d686c3` | test(tui): update snapshots for v0.5.1-beta                       | success                        |
| [25223175008](https://github.com/eddacraft/anvil-001/actions/runs/25223175008) | 2026-05-01 | `7b3c0bc` | feat(plans): add SKOBS module for skill discovery + observability | success                        |

Each run shows the four intercept crates compiled by name in the build log:

```
Compiling eddacraft-anvil-intercept-rules v0.5.1-beta (...)
Compiling eddacraft-anvil-intercept-proto v0.5.1-beta (...)
Compiling eddacraft-anvil-intercept-win32 v0.5.1-beta (...)
Compiling eddacraft-anvil-intercept       v0.5.1-beta (...)
```

…and the corresponding test binaries run on Windows:

```
Running unittests src\lib.rs (target\x86_64-pc-windows-msvc\debug\deps\anvil_intercept-<hash>.exe)
Running unittests src\main.rs (target\x86_64-pc-windows-msvc\debug\deps\anvil_intercept-<hash>.exe)
Running unittests src\lib.rs (target\x86_64-pc-windows-msvc\debug\deps\anvil_intercept_proto-<hash>.exe)
Running unittests src\lib.rs (target\x86_64-pc-windows-msvc\debug\deps\anvil_intercept_rules-<hash>.exe)
Running unittests src\lib.rs (target\x86_64-pc-windows-msvc\debug\deps\anvil_intercept_win32-<hash>.exe)
Doc-tests anvil_intercept
Doc-tests anvil_intercept_proto
Doc-tests anvil_intercept_rules
Doc-tests anvil_intercept_win32
```

### Intercept-crate test counts on `windows-latest`

Reading the `running N tests` markers and matching test name prefixes in the run
logs:

| Crate                   | Test names                                                                                                                | Tests run on Windows (run 25375091014, 2026-05-05) | Tests run on Windows (run 25327005734, 2026-05-04) |
| ----------------------- | ------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------- | -------------------------------------------------- |
| `anvil-intercept` (lib) | `enforcement::tests::*`, `fence::tests::*`, `ipc::tests::*` (Windows variants), `midedit::tests::*`, `registry::tests::*` | 58                                                 | 58                                                 |
| `anvil-intercept` (bin) | (no inline tests — the binary is `main.rs`)                                                                               | 0                                                  | 0                                                  |
| `anvil-intercept-proto` | proto serde + envelope shape                                                                                              | 10                                                 | 10                                                 |
| `anvil-intercept-rules` | rule registry, fixtures                                                                                                   | 34                                                 | 34                                                 |
| `anvil-intercept-win32` | owner-only DACL build, no GA grant, current-process liveness/creation-time, named-pipe creation                           | 4                                                  | 4                                                  |

Linux baseline for comparison (this worktree, 2026-05-06):

```
$ cargo test -p eddacraft-anvil-intercept --lib
test result: ok. 91 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.10s
```

The 91 → 58 gap on Windows is by design: 33 tests are gated `#[cfg(unix)]`
(socket-dir permission ladder, `unix_perms::*`, `run_foreground_*` PID-file
flow, fence default-path tests for XDG/`HOME`). The Windows-side equivalents
either live in `anvil-intercept-win32` (DACL, owner-only pipe creation) or are
exercised through the existing named-pipe smoke test
(`ipc::tests::named_pipe_scan_buffer_smoke_uses_injected_service`). The new test
added in this branch raises the Windows lib count to 59.

## `dev` push runs — the failures since 2026-05-05 are **not** Windows or test failures

| Run                                                                            | Date       | Commit    | Outcome           | Notes                                                                                                                                                                       |
| ------------------------------------------------------------------------------ | ---------- | --------- | ----------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| [25438137556](https://github.com/eddacraft/anvil-001/actions/runs/25438137556) | 2026-05-06 | `fe23357` | `Test` job failed | All 2836 tests passed (`Summary [47.246s] 2836 tests run: 2836 passed, 1 skipped`). Failure is **`llvm-cov` profile-data merge corruption** in the post-test coverage step. |
| [25436566831](https://github.com/eddacraft/anvil-001/actions/runs/25436566831) | 2026-05-06 | `e72483a` | `Test` job failed | Same `llvm-cov` corruption pattern (`invalid instrumentation profile data (file header is corrupt) ... no profile can be merged`).                                          |
| [25435539200](https://github.com/eddacraft/anvil-001/actions/runs/25435539200) | 2026-05-06 | `1fe86ed` | `Test` job failed | Same.                                                                                                                                                                       |
| [25428776104](https://github.com/eddacraft/anvil-001/actions/runs/25428776104) | 2026-05-06 | `e743e9d` | `Test` job failed | Same.                                                                                                                                                                       |
| [25421558557](https://github.com/eddacraft/anvil-001/actions/runs/25421558557) | 2026-05-06 | `9325a91` | `Test` job failed | Same.                                                                                                                                                                       |
| [25420836729](https://github.com/eddacraft/anvil-001/actions/runs/25420836729) | 2026-05-06 | `c89e3a6` | All jobs green    | LAUNCH-010 follow-up; first all-green dev push that day.                                                                                                                    |

Failure root cause across all five 2026-05-06 dev failures (excerpt from the
`Test` job log of run 25438137556):

```
2026-05-06T13:28:41.9678436Z      Summary [47.246s] 2836 tests run: 2836 passed, 1 skipped
2026-05-06T13:30:20.0551188Z warning: target/llvm-cov-target/anvil-001-4100-1374896049293381330_0.profraw: invalid instrumentation profile data (file header is corrupt)
2026-05-06T13:30:20.0575503Z error: no profile can be merged
2026-05-06T13:30:20.0577779Z error: failed to merge profile data: process didn't exit successfully:
                                  llvm-profdata merge -sparse -f anvil-001-profraw-list -o anvil-001.profdata (exit status: 1)
2026-05-06T13:30:20.0793805Z ##[error]Process completed with exit code 1.
```

**Conclusion:** the dev-push failures since 2026-05-05 are flaky
`cargo llvm-cov`/coverage tooling, not Rust test failures and not
intercept-related. Tracking that flake is **out of scope for this PR**; INTD-012
is concerned with Windows confidence, and Windows is green on every release-path
run that has executed since `v0.5.1-beta`.

## Windows-only `#[cfg(...)]` gates and what runs vs. what doesn't

The purpose of this section is to make the "`--workspace` already covers it"
claim verifiable line-by-line.

`rg "#\[cfg(windows)\]" crates/anvil-intercept-win32 crates/anvil-intercept/src/ipc.rs`
and
`rg "#\[cfg(target_os = \"windows\")\]" crates/anvil-cli/src/mcp/validation.rs`:

| Location                                              | Gate                                                                                                                          | Exercised by `cargo test --workspace` on `windows-latest`?                                                                                                                                             |
| ----------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `crates/anvil-intercept-win32/src/lib.rs:7`           | `#![cfg(windows)]` (whole crate)                                                                                              | Yes — 4 unit tests + 1 `#[tokio::test]` named-pipe creation test (`creates_owner_only_pipe_server`).                                                                                                   |
| `crates/anvil-intercept/src/ipc.rs:263`               | `#[cfg(windows)] pub fn resolve_pipe_name`                                                                                    | Yes — `ipc::tests::resolve_pipe_name_uses_user_suffix`.                                                                                                                                                |
| `crates/anvil-intercept/src/ipc.rs:430..437`          | `#[cfg(windows)]` `IpcListener` named-pipe fields                                                                             | Yes — exercised via `bind`/`bind_with_scan_buffer_service`.                                                                                                                                            |
| `crates/anvil-intercept/src/ipc.rs:643`               | `#[cfg(windows)] impl<D> IpcListener<D>` (bind, serve)                                                                        | Yes — `ipc::tests::named_pipe_scan_buffer_smoke_uses_injected_service` exercises the full path on Windows.                                                                                             |
| `crates/anvil-intercept/src/ipc.rs:2195..2202`        | `#[cfg(windows)]` two unit tests (`resolve_pipe_name_uses_user_suffix`, `named_pipe_scan_buffer_smoke_uses_injected_service`) | Yes.                                                                                                                                                                                                   |
| **NEW** `crates/anvil-intercept/src/ipc.rs` (this PR) | `#[cfg(target_os = "windows")]` `named_pipe_scan_buffer_envelope_parity_with_embedded`                                        | Yes — see "Windows-only assertion gate" below.                                                                                                                                                         |
| `crates/anvil-cli/src/mcp/validation.rs:115..124`     | `#[cfg(unix)]`-only `LocalDaemonValidationClient::with_socket_path`                                                           | **No** — Windows path is `DaemonValidationOutcome::Unavailable` stub (validated by Linux-only parity test at line 517). The new test sits in `anvil-intercept` so it does **not** modify this surface. |

### Windows-only assertion gate added in this PR

`crates/anvil-intercept/src/ipc.rs::ipc::tests::named_pipe_scan_buffer_envelope_parity_with_embedded`:

- Builds an `EnforcementPipeline` from `default_rule_registry()` (the same
  registry the embedded validator uses via `EnforcementPipeline::default()`).
- Computes the expected diagnostics directly via
  `pipeline.diagnostics_for_proposed_changes_with_limit(...)` — this is the
  canonical embedded path for the same input.
- Spins up a `tokio::net::windows::named_pipe` server through
  `IpcListener::bind_with_scan_buffer_service` against a per-PID pipe name with
  the same pipeline backing the `ScanBufferService`.
- Connects with `ClientOptions::new().open(...)`, sends a JSON-RPC 2.0
  `scan_buffer` request carrying the secret-fixture content, reads one framed
  line back, and decodes it as `JsonRpcResponse<ScanBufferResult>`.
- Asserts the daemon-returned `result.diagnostics` are byte-for-byte equal (via
  `serde_json::to_value` round-trip) to the embedded pipeline's diagnostics.
  Mode and rule_id are also asserted.
- The test is gated `#[cfg(target_os = "windows")]` so it compiles only on
  Windows targets and is correctly skipped on Linux/macOS. The crate is in
  `--workspace`, so `cargo test --workspace --target x86_64-pc-windows-msvc`
  (already configured in `rust.yml`) picks it up automatically when the
  cross-compile job runs.

If a future change desyncs the Windows daemon-backed `scan_buffer` envelope from
the embedded path — for example, a new field added on one side but not the
other, or a serialisation difference — the test will fail closed at the next
release-path Windows run.

## Items deliberately deferred

- **Adding a Windows job that runs on every dev push.** The cost is a
  windows-latest runner per PR and per dev push, which the maintainers have
  decided is not worth it pre-launch. Re-evaluate after the A2 release if
  Windows regressions slip past dev.
- **Named-pipe Job Object termination parity.** Owned by INTD-006 (Wave 2). The
  Windows pipe surface assertion in this PR is read-only; it does not exercise
  process termination. INTD-006 will add the Job-Object-vs-`TerminateProcess`
  confidence in its own worktree.
- **Windows-side `LocalDaemonValidationClient` with named-pipe transport.**
  Today `crates/anvil-cli/src/mcp/validation.rs::request_daemon_diagnostics` is
  `#[cfg(unix)]`; the Windows path returns `Unavailable`. The embedded-parity
  assertion in this PR is positioned **inside** `anvil-intercept`'s own surface
  (the `IpcListener` + JSON-RPC `scan_buffer` boundary), so we get a
  Windows-only fail-closed gate without touching daemon error semantics in
  `validation.rs`. The Windows daemon client itself remains a separate ticket if
  and when the embedded fallback is removed.
