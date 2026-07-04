# Headless Background Save-Time Driver Design (DSV-046)

| Upstream | Downstream |
| -------- | ---------- |
| [DSV-046](../modules/daemon-save-time-validation.aps.md), [ADR-061](../decisions/061-save-time-daemon-delta-validation.md), [ADR-082](../decisions/082-daemon-lifecycle-user-startup.md), [ADR-092](../decisions/092-mcp-optional-activation-spine.md), [ADR-094](../decisions/094-worktree-registration-ux.md), [RELEASE-PLAN](../../RELEASE-PLAN.md) | [ADR-101](../decisions/101-headless-save-time-driver.md), DSV-047..051 work items |

## Problem

ACTMO-006 and ADR-092 say `anvil start` should arm daemon-backed save-time
validation without requiring MCP or a visible `anvil watch` terminal. ACTMO-014
through ACTMO-020 delivered durable worktree registration and honest membership
status, but **nothing observes the filesystem unattended**:

- `anvil watch` is a **foreground** client: kernel `notify` →
  `watch_save_time.rs` → daemon `validate_paths` (`crates/anvil-cli/src/commands/watch.rs`,
  `watch_save_time.rs`).
- The intercept daemon certifies paths it receives; it does **not** own a
  background filesystem watcher (`crates/anvil-intercept/src/save_time.rs` reads
  anchor-guarded bytes only after a client names changed paths).
- `anvil start` therefore still closes with guidance to run `anvil watch` or
  `anvil intercept status` when the spine is live but no driver is attached
  (`crates/anvil-cli/src/commands/start.rs`).

The [RELEASE-PLAN](../../RELEASE-PLAN.md) `v0.9.0-beta` usefulness addendum names
this gap explicitly: registration UX (ACTMO) and unattended validation (DSV-046)
are two halves; shipping only the former delivers honest membership without
background findings.

## Boundary with ACTMO (no overlap)

Per [ADR-094](../decisions/094-worktree-registration-ux.md) decision 7 and the
[worktree-registration UX design](./2026-06-29-worktree-registration-ux-design.md)
§"Boundary with DSV-046":

| Owner | Concern |
| ----- | ------- |
| **ACTMO** | How a worktree enters/leaves the durable protected set; commands (`workspace register`), status membership axis, `register_on_start`, guided hooks |
| **DSV-046** | Who watches the filesystem unattended, driver lifecycle, resource limits, findings surfacing without a terminal, restart recovery |

**Shared seam:** `SessionRegistry::set_membership_hook` (ACTMO-014, merged).
The registry is the sole producer of `MembershipChange::{Registered,Unregistered,Reaped}`;
DSV-047's supervisor is the consumer that attaches/detaches one driver per
canonical worktree path.

## Architecture decision (ADR-101)

**Chosen: daemon-supervised detached CLI driver sidecars** — one headless
`anvil watch --save-time-driver` child per registered worktree, spawned by a
`SaveTimeDriverSupervisor` inside the intercept daemon.

### Why not in-daemon `notify`?

ADR-064 keeps `tree-sitter`/parser/`notify` out of the resident daemon. A
daemon-internal watcher would still need a `SymbolParser` feed for full
certification (the daemon reads bytes but enriches via injected parser from the
CLI process at daemon startup — see `save_time.rs` §Symbols feed). Duplicating
the kernel watch loop inside `anvil-intercept` would either:

- violate ADR-064 (parser + notify enter the daemon), or
- certify without symbols (permanent `Partial` verdicts — unacceptable).

Reusing the existing **`watch` + `watch_save_time` client path** preserves
ADR-061 parity (DSV-009), ADR-064 boundaries, and the frozen `validate_paths`
wire.

### Why daemon-spawned sidecars (not `anvil start` only)?

- Registration can happen without `start` (`workspace register`, `register_on_start`
  on daemon restart).
- The supervisor must re-attach drivers after daemon restart when persisted
  registrations reload (ACTMO-014).
- Single lifecycle owner aligned with the registry producer.

### Rejected alternatives

| Option | Why rejected |
| ------ | ------------ |
| In-daemon `notify` watcher | ADR-064 + parser injection boundary; high reversal cost |
| Honest copy downgrade ("daemon ready; run `anvil watch` when you want findings") | Fails the `v0.9.0-beta` minimum useful release shape |
| Foreground `anvil watch` spawned by `anvil start` in the same terminal | Does not satisfy "no visible watch terminal" |

## Driver contract

### Spawn shape

On `MembershipChange::Registered` (and on startup reconciliation for each
reloaded durable registration), the supervisor spawns **one detached child**:

```text
anvil watch --save-time-driver --worktree <canonical-root>
```

Properties:

- **Working directory:** worktree root.
- **Detached:** same launcher pattern as DLIFE-002 (`CREATE_NO_WINDOW` on
  Windows; stdout/stderr redirected to a per-worktree **crash-capture** file
  `{ANVIL_HOME}/runtime/save-time-drivers/<worktree-id>.spawn.log` — distinct
  from the findings log below, which the child owns).
- **No daemon lifecycle in the child:** the driver assumes the parent daemon is
  live; it does not offer or spawn a daemon (watch's `--no-daemon` equivalent
  is implicit in driver mode).
- **Routing:** `ANVIL_WATCH_DAEMON` unset → `DefaultOnWhenLive` (DSV-021); the
  child talks to the already-running daemon only.
- **Output:** plain/headless only — no TUI, no `[watching]` banners on stdout;
  findings append to the driver **findings log**
  (`<worktree-id>.log`, path handed down via `ANVIL_SAVE_TIME_DRIVER_LOG`);
  the **child owns that file end-to-end** — open, append, rotate/truncate at
  1 MiB — the supervisor never writes to it (single-writer rule: rotation under
  a supervisor-held redirect fd is the failure mode this avoids); optional
  one-line human summary on stderr when a batch produces new findings (same
  severity discipline as plain watch).
- **Stop:** on `Unregistered` / `Reaped`, supervisor sends graceful terminate,
  waits bounded, then force-kills; clears PID record.
- **Daemon shutdown:** supervisor stops all children before exit.

### PID registry

Per worktree JSON record at
`{ANVIL_HOME}/runtime/save-time-drivers/<worktree-id>.json`:

- `pid`, `pid_starttime` (Windows parity with session lineage anti-spoof pattern)
- `worktree` (canonical path)
- `log_path`
- `started_at`

Startup reconciliation: for each durable registration without a live PID (or
stale PID), spawn a fresh driver. Orphan records from a crash are overwritten.

### Status / assurance derivation

Add to `WorktreeStatusV1` (wire addition, ACTMO-017 soft-dep closed):

```text
save_time_driver: attached | absent | failed
```

Derivation table (membership × driver × MCP), extending ADR-094 decision 6:

| Membership | Driver | MCP live | User-facing assurance label |
| ---------- | ------ | -------- | --------------------------- |
| registered | attached | yes | `protecting` |
| registered | attached | no | `watching` (save-time active, MCP optional) |
| registered | absent | * | `watching` (membership only — honest downgrade) |
| unregistered | * | * | `unregistered` |

`anvil status` plain text and `--json` expose `save_time_driver` per worktree.
`anvil intercept stop` reports driver count alongside registration count.

### Findings without a terminal

Cut-line minimum:

- Driver log file (human-readable finding lines, rotated/truncated at 1 MiB).
- `anvil status` shows `save_time_driver: attached` and points to the log path
  when the driver is active.
- Post-cut follow-up (not blocking): DPO-001 `gate.evaluated` Kindling rows.

### Opt-outs

| Control | Effect |
| ------- | ------ |
| `ANVIL_NO_SAVE_TIME_DRIVER=1` (or non-empty) | Supervisor does not spawn drivers; registration still works |
| `ANVIL_WATCH_DAEMON=0` | Driver child uses subprocess fallback (scoped check) — supervisor still spawns but status reports `failed` if daemon routing required; document as unsupported combo for cut-line |
| `--no-daemon` on `anvil start` | No daemon → no supervisor → no drivers (existing behaviour) |

### Resource limits

- One driver per distinct registered worktree, capped by the registration cap
  (default 64, ACTMO-014).
- Each driver holds one kernel `notify` watch set — same inotify budget as manual
  `anvil watch` (document in runbook; link CIB capacity guidance).
- RLB process-tree budgets apply to driver children (existing nightly harness).

## Implementation split (Ready work items)

| ID | Title | Cut-line | Depends on |
| -- | ----- | -------- | ---------- |
| DSV-047 | Daemon `SaveTimeDriverSupervisor` (membership hook consumer, spawn/stop/reap, PID registry, startup reconciliation) | yes | DSV-046, ACTMO-014, DSV-048 (spawn argv contract) |
| DSV-048 | CLI `anvil watch --save-time-driver` headless mode | yes | DSV-007 |
| DSV-049 | `save_time_driver` wire field + status/activation derivation | yes | DSV-047, ACTMO-017 |
| DSV-050 | `anvil start` / activation copy — honest armed posture without `anvil watch` next-step | yes | DSV-049, ACTMO-006 |
| DSV-051 | Runbook + E2E regression matrix (Linux + Windows) | yes | DSV-047..050 |

Recommended wave:

1. **DSV-048** — define and test the driver entrypoint in isolation.
2. **DSV-047** — supervisor spawns the entrypoint; integration tests with fake launcher.
3. **DSV-049** — status surfaces driver truth.
4. **DSV-050** — activation copy alignment.
5. **DSV-051** — docs + cross-platform E2E.

## Validation matrix (DSV-051 / cut criteria)

- `anvil start --no-mcp` from a worktree: no visible watch terminal; driver PID
  recorded; planted save produces a finding in the driver log within one debounce
  window.
- `anvil workspace register <other>`: second driver spawned; status lists both
  with `save_time_driver: attached`.
- Duplicate register = heartbeat (no duplicate driver).
- Daemon restart: persisted registrations reload; drivers re-attached.
- `anvil intercept stop`: drivers terminated; guidance mentions worktrees losing
  drivers.
- Windows named-pipe parity (DSV-010/011 transport reused by driver child).
- Opt-outs: `ANVIL_NO_SAVE_TIME_DRIVER`, `--no-daemon`.

## Planning council notes (direction validate)

| Lens | Finding | Disposition |
| ---- | ------- | ----------- |
| Security | Same-uid trust floor unchanged; driver only watches registered paths the registry admitted | Accepted — no new trust boundary |
| Adversarial | Orphan PIDs after crash, duplicate spawns on flaky heartbeat | PID registry + startup reconciliation in DSV-047 |
| Operations | inotify exhaustion with N worktrees | Document; cap = registration cap; link capacity.rs guidance |
| Pragmatic | Sidecar not in-daemon notify — reuses DSV-007 client | Accepted — fastest path to useful release |
| Kernel | ADR-031 gate must stay green | Driver uses same hot path as foreground watch; no new daemon work on save |

**Decision:** `proceed` — promote DSV-047..051 to Ready; mark DSV-046 design Done.

## References

- [daemon-save-time-validation](../modules/daemon-save-time-validation.aps.md)
- [activation-mcp-optional](../modules/activation-mcp-optional.aps.md) — ACTMO-017 soft-dep
- [RELEASE-PLAN](../../RELEASE-PLAN.md) — usefulness addendum cut criteria