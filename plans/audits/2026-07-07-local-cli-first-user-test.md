# Local CLI first-user journey test — 2026-07-07

> **Purpose:** Physical command-path test of `welcome`, `start`, `status`,
> `watch`, and `check` on a local debug build; file follow-ups for first-user
> experience. **Conductor:** CIB-182.

## Environment

- Repo: `anvil-001` @ local `main`
- Binary: `target/debug/anvil` (`0.8.1-beta`), `cargo build -p eddacraft-anvil --bin anvil`
- Auth: `ANVIL_DEV=1` (local licence-gate override)
- Worktree: `$HOME/Projects/src/anvil-001` (large monorepo, 3000+ files)

## Methods

1. **Non-interactive pass** — `--no-tui`, piped output, `ANVIL_NO_PROMPT` in some runs
2. **PTY pass** — Python PTY harness (`/tmp/anvil-pty-runner.py`) with real stdin/stdout; auto-Enter on workflow picker

## Key findings (filed as GitHub issues)

| GH | Finding |
| --- | --- |
| [#3216](https://github.com/eddacraft/anvil-001/issues/3216) | `anvil status` false `Daemon: not running` when `intercept.pid` is multi-line (`start_time=`); contradicts `intercept status` and `Save-time: running` |
| [#3217](https://github.com/eddacraft/anvil-001/issues/3217) | First interactive `start` blocks on workflow picker with no discoverable skip (CIB-165 unticked default landed; still no hint / feels hung) |
| [#3218](https://github.com/eddacraft/anvil-001/issues/3218) | Non-interactive `start` over-directs to `intercept start --foreground` vs ADR-082 interactive auto-start path |
| [#3219](https://github.com/eddacraft/anvil-001/issues/3219) | `watch` ~30–45s warm-up on this repo with no completion signal |
| [#3220](https://github.com/eddacraft/anvil-001/issues/3220) | `ANVIL_HOME` wrong permissions → misleading `ready_restart_required` |
| [#3221](https://github.com/eddacraft/anvil-001/issues/3221) | Start verify recipe AKIA string not detected by `check` (false negative) |
| [#3222](https://github.com/eddacraft/anvil-001/issues/3222) | Bare `anvil status` launches blocking TUI in PTY/script sessions |

## Confirmed happy path (PTY)

Interactive `anvil start` (real TTY, no `ANVIL_NO_PROMPT`):

- ~0.8s to `state: protecting`
- `daemon: started the per-user save-time daemon`
- MCP `live_validation` for Cursor + Claude Code
- Does **not** require manual `anvil intercept start --foreground` when TTY + default home are correct

## Root cause note (#3216)

`read_daemon_summary()` (`status.rs`) parses entire PID file as `u32`; current format:

```
<pid>
start_time=<epoch>
```

Intercept crate parses line 1 correctly (`existing_pid_status`).

## Suggested fix order

1. #3216 (trust break)
2. #3217 / #3218 (first `start` path)
3. #3221 (verify recipe honesty)
4. #3220, #3219, #3222 (edge polish)

## Local artefacts

- `/tmp/anvil-tty-pass-v2/` — PTY pass logs from this local test session; not committed
- `/tmp/anvil-pty-runner.py` — reusable local harness from this test session; not committed
