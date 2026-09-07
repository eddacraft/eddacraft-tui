# Council Review — PR #4437 JREL-004 rendezvous coordinator

**Status:** Converged
**Tier:** full
**Target:** `origin/main...HEAD` on `fix/jrel-004-rendezvous-coordinator-anchor`
(protected: `crates/anvil-intercept/src/ipc.rs`; `ensure.rs` reviewed with it)
**Date:** 2026-09-08
**PR:** https://github.com/eddacraft/anvil-001/pull/4437
**Contract:** GitHub issue #4432 (state-home rendezvous coordinator, physical
candidate identity, owner-only live-endpoint advertise). Spec:
[`plans/specs/2026-09-07-jrel-004-rendezvous-coordinator-anchor.md`](../specs/2026-09-07-jrel-004-rendezvous-coordinator-anchor.md)
(Josh chose option B). #4433 / #4435 JREL-003 residuals are `out_of_scope`
unless this PR regresses them.
**Code head reviewed:** `b7e5effbde8b6b64a6c2cfa9d5ce342e66b26e12`. Later
commits on this branch are this review only; protected surfaces are unchanged
after that SHA.

## Change under review

| File | Change | Production impact |
| --- | --- | --- |
| `ensure.rs` | Coordinator is physical `$HOME/.local/state/anvil`, not `min(this process's candidates)`; winner publishes `intercept.rendezvous-endpoint`; conflict count uses `lstat` `(dev, ino)` | Disagreeing XDG shells serialise on one lock and reuse one daemon |
| `ipc.rs` | `push_unique_dir` unique by physical parent; connect set appends advertised parent; owner-only record read | Status/stop/later shells find a daemon bound at another runtime dir |
| `daemon_identity.rs` | Hermetic concurrent ensure: two explicit `XDG_RUNTIME_DIR` values, shared `HOME` | Real-binary proof of #4432 acceptance test 1 |
| `ARCHITECTURE.md` | Overclaim corrected: disagreeing shells converge once they share the coordinator **and** the advertisement | Component authority matches option B |
| Three `docs/architecture/*` freshness tables | Diagram-impact review recorded | Save/validation/fence diagrams unchanged |

Windows named pipe: unchanged. Isolated `ANVIL_HOME`: exclusive single-candidate
set and its own coordinator. Doctor still `HeldByCaller`. Coordinator still
taken before the per-install start lock.

## Seats

| Role | Verdict | Summary |
| --- | --- | --- |
| general | approve | Option B is implemented and tested. Remaining gaps are advisory coverage of the HOME-match path in the in-process concurrent unit test, an ensure-level XDG-onto-state-home assertion, and a re-probe on the Reused publish path. |
| adversarial | request_changes | Four majors: lossy `Path::display()` round-trip, re-probe under both locks, 100 ms lock-contention sleep, and unmatched-HOME suffix fallback. Judge downgrades all four (see findings). |
| security | approve | Record is `O_NOFOLLOW` / `0600` / euid / `0o022` refused. Advertised paths go through `SocketProbe::for_sibling` and `validate_socket_path_for_client`. Isolated home never reads the default-scope record. Unbounded same-uid read is defence-in-depth only. |
| operations | approve (GO) | Degrade-to-per-install-lock still warns. Publish failure is logged. Doctor `HeldByCaller` ordering intact. Binary test is hermetic (both XDG values set, so `/run/user/<uid>` is not implicit). |
| pragmatic | approve | Scope matches #4432 option B. No drive-by production code. Doctor `--fix` republish, JREL-011 timeout, and JREL-005 typed status stay later/out of scope. |
| **judge** | **Ship** | No in-contract critical or major `must_fix`. Adversarial majors keep honest severity below the blocker bar after contract scoring. |

## Findings

No critical findings. No in-contract major `must_fix` items.

The adversarial seat returned four `major` findings. The judge keeps them as
reported concerns and scores them as **not** `must_fix` for this review: they
are not in-contract production defects against #4432 / option B, or they are
test-seam / later-item issues.

### Adversarial majors — judge disposition

- adversarial: `write_live_endpoint_record` serialises with `socket.display()`
  (`ensure.rs` ~885), which is lossy for non-UTF-8 path bytes, so a later shell
  could probe a mangled path and spawn a second daemon.
  **Judge:** `in_contract` **minor**. HOME / XDG-derived anvil dirs are UTF-8 in
  every supported fixture and in the rest of this crate's path logging. The
  contract does not require `OsStr` wire format. A binary-safe record would be a
  later hardening, not a #4432 blocker.
- adversarial: `publish_observed_live_endpoint` re-probes under the coordinator
  and start locks (`ensure.rs` ~808 / ~832), claimed as up to ~6 s of
  `PROBE_TIMEOUT` hold.
  **Judge:** `in_contract` **minor** / adjacent to **JREL-011**. Dead sockets
  fail connect immediately; the 2 s budget applies only when a listener accepts
  and does not answer. JREL-011 already owns bounded coordinator wait. Not a
  duplicate-daemon defect.
- adversarial: `disjoint_runtime_sets_contend_on_state_home_lock` uses
  `recv_timeout(100ms)` (`ensure.rs` ~2289), which can pass if the waiter is not
  yet scheduled.
  **Judge:** `in_contract` **minor** (test-coverage). Same pattern as the
  existing CIB-382 reversed-order tests. One-daemon proof is carried by
  `concurrent_cross_environment_cold_starts_converge_on_one_daemon` and the
  real-binary `concurrent_ensures_from_disjoint_runtime_dirs_start_exactly_one_daemon`.
- adversarial: when `HOME` is set but matches no candidate and two parents end
  with `.local/state/anvil`, selection falls back to `parents[0]`
  (`ensure.rs` ~800).
  **Judge:** `in_contract` **nit**. Production connect sets are built from the
  same `HOME`, so the lexical `$HOME/.local/state/anvil` match hits. The suffix
  / first-parent chain is a documented in-process test seam (`HOME` unset or
  elsewhere). Isolated `ANVIL_HOME` correctly uses that prefix as `parents[0]`.

### In-contract minor / nit (advisory)

- general: the in-process concurrent ensure uses the unique-suffix coordinator
  branch because process `HOME` is not the synthetic fixture home.
  `ensure.rs` ~2218. The real-binary test and
  `coordinator_prefers_home_state_over_runtime_dir_suffix` cover the primary
  branch.
- general: spec §5.3 XDG-canonicalises-onto-state-home is proven at candidate
  construction (`ipc.rs`
  `connect_candidates_dedupe_xdg_that_canonicalizes_onto_state_home`) and at
  ensure for ancestor-aliased sockets
  (`ancestor_alias_of_state_home_is_one_endpoint_not_a_conflict`). No extra
  ensure-level test wires the resolver output into `ensure_in`.
- general / operations: `EndpointLiveness::One` discards the winning path, so
  Reused republish re-probes. `ensure.rs` ~463 / ~832.
- security: `read_live_endpoint_socket` has no size cap (`ipc.rs` ~805). Same-uid
  trust boundary; 4 KiB cap would be defence-in-depth.
- security / operations: `pid_beside_socket` writes `0` when the PID file lags
  (`ensure.rs` ~840). Readers use only the socket line today.
- adversarial / general: the real-binary acceptance test does not assert the
  advertisement file contents (`daemon_identity.rs`
  `concurrent_ensures_from_disjoint_runtime_dirs_start_exactly_one_daemon`).
  One-daemon + both-shell status is the contracted proof.
- pragmatic: step-1 Reused (pre-lock) does not publish (`ensure.rs` ~429).
  Contracted path is concurrent **ensure**, which publishes after spawn and
  again under the lock on step-4 Reused. Manual `intercept start --foreground`
  (the daemon process itself) without a prior ensure is outside #4432.
- general: ARCHITECTURE.md states ensure reuse of the advertised socket and does
  not spell out that connect/status/stop also read the record outside the lock
  (`ARCHITECTURE.md` ~203). `ipc.rs` ~670 does that work.

### Later / out of scope (not must-fix)

- JREL-011: bounded wait / typed outcome when the coordinator cannot be
  acquired (today: warn and degrade to the per-install lock).
- JREL-005: typed same-scope conflict on `intercept status`.
- Doctor `--fix` relocating to this-process canonical without rewriting the
  record (spec non-goal; next ensure republishes).
- Option A (default canonical bind becomes physical state-home).
- Option C (shared lock only, no advertisement).
- #4433 / #4435 heartbeat, driver-map stall, leftover child, Windows terminate,
  silent-listener conflict tally — not touched; silent-listener Answered-only
  counting is preserved in `live_endpoints`.
- Making different `HOME` values for the same UID share a daemon.

One pragmatic finding cited CIB / JSIMP module edits. Those files are **not**
in `origin/main...HEAD` for this PR (seven files only). Discarded as
out-of-tree.

## Evidence

- Full five-seat Council against issue #4432, option B spec, ADR-036, ADR-060,
  and `origin/main...HEAD` at `b7e5effbd`.
- Anvil developer MCP tools were unavailable; reviewers used ordinary file
  reads and `git diff`.
- `scripts/agent/guidance.sh --branch` reported `targeted` / `council-reviewer`.
  Escalated to **full** because `ipc.rs` is on
  `.claude/hooks/council-protected-paths` and Josh required `/council full`.
- CI on `b7e5effbde8b6b64a6c2cfa9d5ce342e66b26e12`: Test, Clippy,
  Clippy (windows-msvc), Check, Format, Docs corpus, anvil watch ↔ language
  servers passed. **Protected surfaces reviewed** failed, as expected, until a
  maintainer applies `council:reviewed` on the live head that contains this
  review.

## Decision

**Ship** the JREL-004 option B coordinator on PR #4437. A maintainer applies
**`council:reviewed`** on the live head that contains this review and the
protected-path diff at `b7e5effbd`. Re-council if `ipc.rs` or `ensure.rs` move
after the label.
