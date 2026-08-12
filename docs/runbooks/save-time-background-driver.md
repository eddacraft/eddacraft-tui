# Save-Time Background Driver — Operator Runbook

| Type    | Authority     | Owner | Status | Freshness                                                               |
| ------- | ------------- | ----- | ------ | ----------------------------------------------------------------------- |
| Runbook | Authoritative | DSV   | Live   | Filed 2026-07-06 for DSV-051 against ADR-101 and `anvil start --no-mcp` |

| Upstream                                                                                                                                                                                                                                                                                                                                                                                                              | Downstream                                                                                                                          |
| --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------- |
| [DSV](../../plans/archive/modules/daemon-save-time-validation.aps.md), [ADR-101](../../plans/decisions/101-headless-save-time-driver.md), [headless driver design](../../plans/specs/2026-07-04-headless-save-time-driver-design.md), [`anvil start`](../../crates/anvil-cli/src/commands/start.rs), [`anvil watch`](../../crates/anvil-cli/src/commands/watch.rs) `plans/decisions/101-headless-save-time-driver.md` | [Save-time validation guide](../public/anvil/guides/save-time-validation.md), [MCP-optional activation](anvil-no-mcp-activation.md) |

Use this runbook when `anvil start` has registered a worktree with the intercept
daemon and a supervised background save-time driver is expected to validate file
saves without a visible `anvil watch` terminal.

## Expected Posture

The background driver is one detached `anvil watch --save-time-driver` child per
durably registered worktree. The daemon owns supervision; the child owns file
watching and appends findings to a driver log.

Healthy posture:

- `anvil start --no-mcp` exits successfully without opening a foreground watch.
- `anvil intercept status` includes a non-zero `drivers:` active count.
- `anvil status --json` for the worktree reports `save_time_driver: "attached"`.
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

If `anvil start` says `save-time driver is not attached`, use
`anvil intercept status` first. The false branch means only that no attached
driver was proven; the richer status surface distinguishes absent, failed, and
attached worktrees.

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
worktree is a heartbeat refresh, not a request for another child; active driver
counts should stay one per distinct registered worktree.

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
PID records. The daemon does not auto-respawn a driver that dies while the
daemon is still running; killing a child directly should degrade the worktree to
`save_time_driver: "failed"` until the daemon is restarted or the worktree is
registered again.

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
