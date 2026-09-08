# Council Review — PR #4440 durable wire gate canary wait

**Status:** Converged
**Tier:** full
**Target:** `origin/main...HEAD` on `cursor/fix-driver-recovery-registration-flake-1a8d`
(protected: `crates/anvil-intercept/src/ipc.rs`; `lib.rs` warm-up reviewed with
it)
**Date:** 2026-09-08
**PR:** https://github.com/eddacraft/anvil-001/pull/4440
**Contract:** GitHub issue #4439 (CIB-160 peer-exe faithfulness probe must wait
for canary exec before caching fail-closed; warm the once-per-process probe
before accept). The PR description is evidence of that small-fix residual.
JREL-004 rendezvous coordinator / physical identity (#4437, now on `main`) is
`out_of_scope` unless this PR regresses it. CIB-160's original portable
peer-exe work is Released/Shipped; this review does not reopen that item.
**Code head reviewed:** `6f3f283bc38c81d9ee66340abe98ad8113b362d0`. Later
commits on this branch are diagram freshness and this review only; protected
surfaces are unchanged after that SHA.
**Rebase:** this branch was rebased onto `origin/main`
(`68a11bb637bfce4b2238e15a01553bb7071ac781`, #4437 merged) with
`--force-with-lease`. Conflicts were freshness tables only; the canary wait,
warm-up, and inline-join fallback applied cleanly.

## Change under review

| File | Change | Production impact |
| --- | --- | --- |
| `ipc.rs` | `wait_for_execed_foreign_exe` polls `canonical_peer_exe` until the canary image differs or 250 ms expires; probe uses that wait; `canonical_peer_exe` shared by the gate | A pre-exec `/proc/<pid>/exe` read no longer permanently fail-closes durable `session.register` |
| `lib.rs` | `spawn_blocking` warm-up before accept; inline probe if the join fails | First register does not pay the exec wait on the 500 ms RPC budget; a cancelled/panicked warm-up still caches before listen |
| `ARCHITECTURE.md` | Membership invariant names the probe wait; freshness records CIB-160 after JREL-004 | Component authority matches the wire-gate fix |
| Three `docs/architecture/*` freshness tables | Downstream diagram-impact review recorded | Save/validation/fence diagrams unchanged |

Fail-closed behaviour is unchanged when the canary cannot be spawned, the pid
is never readable, or the image still equals this binary after the budget
(true aliasing, issue #3130). In-process `register_on_start` / `--persist`
remains the escape hatch. Non-anvil peers stay downgraded. Canary paths remain
absolute OS binaries (no `PATH`).

## Seats

| Role | Verdict | Summary |
| --- | --- | --- |
| general | approve | Poll-until-distinct plus pre-accept warm-up closes the false fail-closed. Remaining notes are a tautological assert, comment gaps on the inline fallback, and full-budget latency when every poll is `None`. |
| adversarial | approve | OnceLock is set before bind. Fail-closed on uncertainty, aliasing, and missing canary is intact. Concerns are the exceptional inline sleep on the executor, a test `panic` instead of skip on `None`, pre-existing per-call `current_exe`, and the 250 ms constant's empirical citation. |
| security | approve | Gate structure unchanged: faithfulness first, then same-exe. Live peers use `canonical_peer_exe`, not the waiting helper. Budget expiry still yields `faithful = false`. Canary spawn is still absolute binaries. |
| operations | approve (GO) | Flake root cause matches the code. Warm-up is off the RPC path. Residual 250 ms expiry is the same fail-closed as before, now after a wait. Inline fallback can stall a Tokio worker before accept; no success-path info log. |
| pragmatic | approve | Smallest fix that closes #4439. `canonical_peer_exe` is adjacent dedup, not drive-by. Freshness tables are diagram-impact stamps. No JREL-004 or CIB module edits. |
| **judge** | **Ship** | No in-contract critical or major findings. |

## Findings

No critical or major `must_fix` items. In-contract minors and nits are
advisory.

The operations seat labelled the inline-fallback sleep **medium**. The judge
keeps honest severity as **minor**: the contract requires the inline fallback;
it runs only when `spawn_blocking` join fails; it is before `accept`; the
sleep is bounded to 250 ms. That is not a production defect against #4439.

### In-contract minor (advisory)

- general / adversarial / security / operations: the `JoinError` fallback
  calls `warm_foreign_exe_faithfulness_probe` on the Tokio worker
  (`lib.rs` ~2656). That path may `std::thread::sleep` for up to
  `FOREIGN_EXE_EXEC_WAIT`. Acceptable before accept; `block_in_place` or a
  comment would make the bound explicit.
- general: if `canonical_peer_exe` is `None` on every poll,
  `wait_for_execed_foreign_exe` burns the full budget before returning
  `None` (`ipc.rs` ~5871). The probe keeps the canary alive; document the
  latency if an external reaper kills it.
- operations: warm-up does not emit an info-level result when the probe is
  faithful (`lib.rs` ~2649). Failures already `warn!`. A start-up line would
  help new runner images.

### In-contract nit (advisory)

- general: `wait_for_execed_foreign_exe_observes_canary_not_this_binary`
  `assert_ne!` is unreachable after the `#3130` skip (`ipc.rs` ~7740).
- adversarial / operations: 250 ms / 2 ms constants lack a measured p99
  citation (`ipc.rs` ~5788). Budget expiry still fail-closes, as contracted.
- operations: the zero-budget test bound is `poll / 2` (1 s), a correctness
  guard, not a latency sentinel (`ipc.rs` ~7759).

### Later / out of scope (not must-fix)

- Test `panic` when the canary pid is never readable (`ipc.rs` ~7729) rather
  than `[SKIP]` — test robustness, not the product fix (`no_contract`).
- Caching `current_exe()` in a `OnceLock` so probe-time and gate-time daemon
  paths cannot diverge after an in-place binary replace (`ipc.rs` ~5719) —
  pre-existing, fail-closed (`later_item`).
- Windows `SystemRoot` / `WINDIR` fallback candidates after the fixed
  `ping.exe` path (`ipc.rs` ~5914) — council #3582, unchanged (`out_of_scope`).
- JREL-004 rendezvous coordinator, live-endpoint advertisement, and physical
  candidate identity (#4437). This PR does not touch those paths; rebase
  conflicts were freshness tables only.
- Changing the once-per-process `OnceLock` lifetime, distinct `Stopping`
  status, CIB module edits.

## Evidence

- Full five-seat Council against issue #4439, PR #4440, CIB-160 probe
  comments, and `origin/main...HEAD` after rebase onto `68a11bb63`.
- Anvil developer MCP tools were unavailable; reviewers used ordinary file
  reads and `git diff`.
- `scripts/agent/guidance.sh --branch` reported `targeted` / `docs` because
  the latest commits are freshness stamps. Escalated to **full** because
  `ipc.rs` is on `.claude/hooks/council-protected-paths` and Josh required
  `/council full`.
- Local (author, post-rebase):
  `cargo test -p eddacraft-anvil-intercept --lib --no-fail-fast wait_for_execed_foreign_exe`
  — 2 passed;
  `cargo test -p eddacraft-anvil-intercept --lib --no-fail-fast dispatch_command_durable_claim`
  — 3 passed.
- Protected surfaces last moved at `6f3f283bc`. A maintainer applies
  `council:reviewed` on the live head that contains this review. Re-council
  if `ipc.rs` moves after the label.

## Decision

**Ship** the CIB-160 canary exec-wait on PR #4440. A maintainer applies
**`council:reviewed`** on the live head that contains this review and the
protected-path diff at `6f3f283bc`. Re-council if `ipc.rs` moves after the
label.
