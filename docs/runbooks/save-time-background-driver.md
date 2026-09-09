# Save-Time Background Driver — Operator Runbook

| Type    | Authority     | Owner | Status | Freshness                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            |
| ------- | ------------- | ----- | ------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Runbook | Authoritative | DSV   | Live   | Last reviewed 2026-09-09 for JREL-005 typed driver readiness, additive status evidence, failure exit behaviour, and public daily-command recovery copy. Prior review 2026-09-07 for JREL-003/004 residuals: gated duplicate `session.register` restores a dead child (heartbeat does not fork), placeholder stop, leftover adopt, atomic `<stem>.ready` write, and absent-driver as a debug line rather than a stderr warning. Prior review 2026-09-07 for JREL-003 (council round two): refresh-driven recovery, the lifetime-evidence respawn bound, the generation-bound marker / `save_time_driver_evidence` contract, the bounded stop, and the driver-failure keys in `anvil workspace register` JSON. Filed 2026-07-06 for DSV-051 against ADR-101 and `anvil start --no-mcp` |

| Upstream                                                                                                                                                                                                                                                                                                                                                                                                              | Downstream                                                                                                                          |
| --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------- |
| [DSV](../../plans/archive/modules/daemon-save-time-validation.aps.md), [ADR-101](../../plans/decisions/101-headless-save-time-driver.md), [headless driver design](../../plans/specs/2026-07-04-headless-save-time-driver-design.md), [`anvil start`](../../crates/anvil-cli/src/commands/start.rs), [`anvil watch`](../../crates/anvil-cli/src/commands/watch.rs) `plans/decisions/101-headless-save-time-driver.md` | [Save-time validation guide](../public/anvil/guides/save-time-validation.md), [MCP-optional activation](anvil-no-mcp-activation.md) |

Use this runbook when `anvil start` has registered a worktree with the intercept
daemon and a supervised background save-time driver is expected to validate file
saves without a visible `anvil watch` terminal.

For the cross-owner distinction between caller-buffer `scan_buffer` and
post-save `validate_paths`, see the central
[save-to-validation sequence](../architecture/save-to-validation.md). This
runbook keeps the operational start, status, recovery, and opt-out procedure.

## Expected Posture

The background driver is one detached `anvil watch --save-time-driver` child per
durably registered worktree. The daemon owns supervision; the child owns file
watching and appends findings to a driver log.

Healthy posture:

- `anvil start --no-mcp` exits successfully without opening a foreground watch.
- `anvil intercept status` includes a non-zero `drivers:` active count.
- `anvil status --json` for the worktree reports aggregate
  `readiness.state: "ready"`, `save_time_driver: "attached"`,
  `save_time_driver_readiness: "ready"`, and either `watches-installed` or
  `fresh-activity` as `save_time_driver_evidence`.
- Saving a file with a planted antipattern-family finding appends the finding to
  the worktree's driver log.

## Start Or Verify

From the repository root:

```bash
anvil start --no-mcp
```

`--no-mcp` skips editor MCP configuration only. It still lets activation start
or reuse the intercept daemon, register the worktree, install hook coverage when
allowed, and attach the save-time driver. See
[MCP-optional activation](anvil-no-mcp-activation.md) for the MCP opt-out
contract.

Inspect driver state:

```bash
anvil intercept status
anvil status --json
```

The daily bare command, `anvil start`, and `anvil status` now print an overall
`readiness` plus component states. Their JSON surfaces expose the same typed
projection. A selected but unavailable daemon, failed worktree registration, or
absent/failed save-time driver returns a non-zero exit and one
component-specific `next:` action. Explicit opt-outs remain `disabled` and do
not create a failure. Use `anvil intercept status` for the full daemon
inventory. Its registered-worktree lines and `anvil status --json` distinguish
`starting`, `ready`, `disabled`, `degraded`, and `failed`; attachment without
readiness evidence is not `ready`. For a scripted mutating ensure receipt, use
bare `anvil --json`; the existing `anvil start --json` contract remains a
read-only activation probe.

## Logs And Artefacts

Driver artefacts live under the save-time driver runtime directory:

- `ANVIL_HOME` set: `$ANVIL_HOME/runtime/save-time-drivers/`
- Linux/macOS with `XDG_RUNTIME_DIR`:
  `$XDG_RUNTIME_DIR/anvil/save-time-drivers/`
- Windows: `%LOCALAPPDATA%\anvil\save-time-drivers\`
- fallback: `~/.local/state/anvil/save-time-drivers/`

Each registered worktree uses a stable stem based on the worktree leaf name and
the canonical path hash:

- `<stem>.pid` — driver PID and PID start-time discriminator.
- `<stem>.log` — findings rendered by the driver child.
- `<stem>.spawn.log` — child stdout/stderr crash capture; not used for findings.
- `<stem>.ready` — readiness marker written by the child: `watching` once the
  initial scan completed and the file watches are installed, then `activity`
  each time a save batch obtained a daemon verdict. The daemon reads it on every
  status probe and removes it before each spawn, so it never carries a previous
  child's evidence.

The marker holds two lines — the state above, then the PID of the child that
wrote it:

```text
watching
48213
```

The daemon trusts the state only when that PID is the child it currently
supervises. A marker left by a previous generation still exiting, or by a manual
`anvil watch --save-time-driver` run over the same paths, therefore claims
nothing beyond a live PID. The PID line is additive: a marker without it is
simply not read as evidence.

`anvil intercept status --json` reports the marker as
`save_time_driver_evidence` beside `save_time_driver: "attached"`: `spawned`
(live PID only, initial scan still running), `watches-installed`, or
`fresh-activity` (a verdict within the last 60 seconds). The key is omitted
unless the driver is attached.

The findings log is capped by the driver. A growing `<stem>.log` with recent
findings is expected; growing `<stem>.spawn.log` usually means the child failed
before it could attach.

## Register More Worktrees

Register another worktree:

```bash
anvil workspace register /path/to/other-worktree
anvil intercept status
```

Expected result: a second save-time driver attaches. Re-registering the same
worktree is a verified duplicate `session.register`, not a request for another
child; active driver counts should stay one per distinct registered worktree.

Small fixture worktrees are recommended for multi-driver smoke tests on shared
Linux runners because every driver holds one kernel watch set. If the host is
near its inotify limit, use the capacity guidance from
[`anvil doctor`](cli-surface.md) and the resource budgeting notes in
[Cargo Target Eviction](cargo-target-eviction.md) before increasing concurrent
worktree coverage.

## Restart And Stop Recovery

Restart the daemon:

```bash
anvil intercept stop
anvil start --no-mcp
anvil intercept status
```

Expected result: durable registrations reload and drivers reattach with fresh
PID records.

The daemon never respawns a driver on its own. Killing a child directly degrades
the worktree to `save_time_driver: "failed"`; the daily command restores it
without touching the durable registration:

```bash
anvil            # or: anvil start / anvil workspace register <worktree>
anvil intercept status --json
```

Re-running any of these re-registers the existing membership
(`worktree: registration refreshed`) through the gated `session.register` verb,
and that refresh is what makes the daemon spawn exactly one replacement child. A
heartbeat alone does not fork a child. The command waits up to one second for
the daemon to report the driver attached, so its output reflects the restored
driver rather than the dead one. The wait ends as soon as the answer cannot
improve: an attached driver returns immediately, an absent driver (opt-out or no
supervisor) returns after a short grace, and a failed snapshot keeps polling
until the budget so a healthy respawn is not reported as failed.

A refreshed membership is never reported as coverage by itself. When the daemon
still reports `failed`, both bare `anvil` and `anvil workspace register` name
the failed save-time driver for that worktree and point at
`anvil intercept status`. Under `--json`, `anvil workspace register` adds
`"save_time_driver": "failed"` to its document, and `--all` lists every such
worktree under `"save_time_driver_failed"`; both keys are present only when a
driver actually failed.

Respawns are bounded. After three failed generations in a row — a refused spawn,
or a child that was never proven to have run for 60 seconds after its spawn —
the daemon refuses further respawns until 60 seconds have passed since the last
failure, and the worktree stays `failed` in the meantime. Inspect
`<stem>.spawn.log` for the crash capture, then re-run the daily command once the
cause is fixed. A child's lifetime is measured from evidence that it ran — the
daemon's own liveness probes, or the fact that this generation wrote its
`<stem>.ready` marker — not from converting that marker's wall-clock
modification time into a monotonic instant, and not from the moment its death
was noticed. A child that crashes immediately still counts as a failed
generation however long the gap between `anvil` runs. A child proven to have
outlived the 60-second window resets the count.

Stopping a driver waits (briefly, escalating to a hard kill) for the child to
exit before the daemon accepts a replacement for that worktree, so an unregister
immediately followed by a re-register never leaves two children on one worktree.

Stopping the daemon terminates supervised drivers. When registered worktrees are
known, `anvil intercept stop` warns that those worktrees lose protection and
must be re-registered or restarted.

## Opt-Outs

Use opt-outs when debugging resource pressure, reproducing foreground watch
behaviour, or running an environment where detached children are not allowed.

```bash
ANVIL_NO_SAVE_TIME_DRIVER=1 anvil start --no-mcp
anvil start --no-daemon
ANVIL_NO_DAEMON=1 anvil start
```

`ANVIL_NO_SAVE_TIME_DRIVER` disables driver supervision for the daemon lifetime
but still allows the worktree to register. Status should show the worktree as
registered with `save_time_driver: "absent"`.

`--no-daemon` and `ANVIL_NO_DAEMON` suppress daemon auto-start. They are broader
than the driver opt-out: with no daemon, there is no supervised background
driver to attach.

## Windows Notes

Windows uses the same driver contract over the named-pipe transport, but the
manual smoke still needs a real Windows session for detached-process and console
window observations. Use
[`plans/execution/DSV-051.windows.actions.md`](../../plans/execution/DSV-051.windows.actions.md)
for the operator checklist.

The Windows daemon runs parser-less at this cut-line. Plant an
antipattern-family finding for verification and expect partial coverage; do not
require `Certified` coverage on the Windows leg.

## Escalation Checklist

Collect these before opening an incident or follow-up issue:

- `anvil intercept status --json`
- `anvil status --json` from the affected worktree
- the relevant `<stem>.pid`, `<stem>.log`, and `<stem>.spawn.log`
- `anvil start --no-mcp` output after a daemon restart
- on Windows, a `tasklist` snapshot showing daemon and driver processes

Do not delete driver artefacts while the daemon is running unless you are
already inside an incident response; prefer `anvil intercept stop` so the daemon
flushes state and terminates children deliberately.
