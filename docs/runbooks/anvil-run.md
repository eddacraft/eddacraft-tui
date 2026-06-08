# anvil-run(1)

| Type    | Authority     | Owner  | Status | Freshness                                                     |
| ------- | ------------- | ------ | ------ | ------------------------------------------------------------- |
| Runbook | Authoritative | @aneki | Live   | First filed 2026-05-18 as N4 doc-lane closure for v0.7.0-beta |

| Upstream                                                                                                                                                                                                  | Downstream                                                                                                                                                                                                                             |
| --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| [`plans/archive/modules/intercept-launcher.aps.md`](../../plans/archive/modules/intercept-launcher.aps.md) (INTL-001..-009), [ADR-038 §3](../../plans/decisions/038-hook-surface-and-noise-discipline.md) | [`crates/anvil-run/`](../../crates/anvil-run/), [`crates/anvil-run/shell/anvil-run.sh`](../../crates/anvil-run/shell/anvil-run.sh), [`crates/anvil-intercept-proto/src/session.rs`](../../crates/anvil-intercept-proto/src/session.rs) |

## NAME

**anvil-run** — wrap an agent process launch in an Anvil-managed session.

## SYNOPSIS

```
anvil-run --tool <NAME> [options] -- <command> [args...]
anvil-run hook register --tool <NAME> [--cwd <PATH>] [--pid <PID>]
anvil-run --help | --version
```

## DESCRIPTION

`anvil-run` is the **wrapped-launch ingress** for the Anvil Intercept Loop. It
resolves the launch context (cwd, repo root, worktree root, tmux pane), queries
the local daemon for reachability and worktree-fence status, registers a
session, spawns the wrapped command in a dedicated process group (Unix) or named
Job Object (Windows), heartbeats while the child runs, and unregisters on exit.
The launcher is single-shot per child process.

The launcher operates in two modes:

- **Wrap mode** (default): `anvil-run --tool <name> -- <cmd...>` wraps a child
  command in a controlled session. Used by the shell wrappers in
  [`shell/anvil-run.sh`](../../crates/anvil-run/shell/anvil-run.sh) to route
  `claude`, `codex`, and `aider` through the launcher.
- **Hook mode** (`anvil-run hook register`): registers a side-channel session
  for a calling agent (e.g. Claude Code `PreToolUse`). Enforcement for
  hook-registered sessions is capped at fence-only by the daemon per the trust
  model below.

The launcher writes its refusal banners and diagnostics to **stderr**; the
wrapped child controls **stdout** exclusively.

## OPTIONS

### Wrap mode

`--tool <NAME>` : Driver / tool identifier — e.g. `claude-code`, `codex`,
`aider`. Passed to the daemon at registration so per-driver policy applies.
Required in wrap mode.

`--agent-id <ID>` : Optional claimed agent id (driver-supplied). Opaque to the
launcher; the daemon's session registry decides what to do with it. Defaults to
a stable per-invocation token. **Advisory only** — see the trust model.

`--worktree <PATH>` : Override the worktree root the daemon should fence-check
against. Defaults to walking up from `--cwd` to the nearest git worktree.

`--cwd <PATH>` : Override the working directory the child is spawned in.
Defaults to the launcher's own cwd.

`--dry-run` : Print the resolved plan + the daemon decision and exit without
spawning. Useful for smoke tests and shell-wrapper debugging.

`-- <command> [args...]` : The wrapped command and its arguments. Everything
after `--` is forwarded verbatim to the child.

### Hook mode

`hook register --tool <NAME> [--cwd <PATH>] [--pid <PID>]` : Register the
calling process as a side-channel session. `--tool` is required; `--cwd`
defaults to the launcher's cwd; `--pid` defaults to the parent PID.
Hook-registered sessions are capped at fence-only enforcement by the daemon
(degraded mode `degraded:fence-cascade`).

## EXIT STATUS

| Code    | Constant                  | Meaning                                                                              |
| ------- | ------------------------- | ------------------------------------------------------------------------------------ |
| `0`     | —                         | Wrapped child exited 0.                                                              |
| `1..63` | —                         | Wrapped child exited with that status (forwarded verbatim).                          |
| `64`    | `EXIT_USAGE`              | Bad CLI input (missing `--tool`, missing `--`, etc).                                 |
| `69`    | `EXIT_DAEMON_UNAVAILABLE` | Daemon socket / pipe could not be reached, or the handshake failed.                  |
| `73`    | `EXIT_SPAWN_FAILED`       | Spawn / registration failed for a reason that is not "fenced" and not "daemon down". |
| `75`    | `EXIT_FENCED`             | Worktree is currently fenced. The launcher refuses to start the command.             |
| `78`    | `EXIT_BAD_CONFIG`         | Required configuration is missing — e.g. `--tool` resolves to no driver id.          |
| `128+N` | —                         | Wrapped child terminated by signal `N` (Unix convention).                            |

The 64 / 69 / 73 / 75 / 78 values are the BSD `sysexits.h` codes — common enough
that operators recognise them, and most real tools never exit with values in
that range. A wrapped tool exiting 69 is **indistinguishable** from "daemon
unavailable" looking at the exit code alone. Shell wrappers that must tell the
two apart should read the launcher's stderr banner; pure exit-code switching is
good enough for the common case.

## ENVIRONMENT

### Inputs

`ANVIL_RUN_BIN` : Override the `anvil-run` binary the shell wrappers in
`shell/anvil-run.sh` invoke. Must point to an executable file.

`ANVIL_RUN_DISABLE` : When set to `1`, `true`, `yes`, or `on`
(case-insensitive), the shell wrappers bypass `anvil-run` and exec the wrapped
command directly. **Any other value — including `0` and `false` — leaves
enforcement ON.** This bias is deliberate: explicit-enable defaults cannot
accidentally disable the loop.

`XDG_RUNTIME_DIR` / `HOME` / `LOCALAPPDATA` : Used (in that order,
platform-conditionally) to discover the daemon's IPC socket / named pipe
location. The launcher reuses `anvil_intercept::ipc` for discovery.

### Outputs (written on the child env before `exec`)

`ANVIL_TASK_ID` : Stable session id minted by the launcher and recorded with the
daemon at registration. Constant name: `ANVIL_TASK_ID_ENV`.

`ANVIL_AGENT_TAG` : Encoded `AgentTag` (driver_id, claimed_agent_id,
pid_starttime). Used by MLP-014's attribution-recovery walk if the child ever
needs to be re-identified by a descendant. Constant name: `ANVIL_AGENT_TAG_ENV`.

## TRUST MODEL

Env propagation is **advisory only**. Any same-UID process can spoof or unset
`ANVIL_TASK_ID` and `ANVIL_AGENT_TAG`. The daemon MUST:

1. Cross-check an env-supplied `AgentTag` against the `AgentTag` it issued for
   this pid lineage at registration. A tag that does not match the registration
   is treated as missing, not honoured.
2. Fall through to the process-tree walk on env miss (MLP-014). A walk that
   finds no registered ancestor downgrades to worktree-level fence per ADR-038
   noise-discipline (one terse line, then silent).
3. Treat the witness chain (ADR-037 §D-2) and `validate_at_l4` (ADR-037 §D-5) as
   the authentication backstop. Env propagation is correctness for the normal
   path, not a security boundary.

These daemon-side requirements are enforced in `crates/anvil-intercept` and
tested under `crates/anvil-intercept/tests/`. Neither `ANVIL_TASK_ID` nor
`ANVIL_AGENT_TAG` is a substitute for the witness chain as an audit record; the
chain (ADR-037 §D-2) is the only authenticated provenance.

See
[`plans/archive/modules/intercept-launcher.aps.md`](../../plans/archive/modules/intercept-launcher.aps.md)
for the contract.

## SECURITY CONSIDERATIONS

- **`ANVIL_RUN_DISABLE` is an honour-system control.** Any same-UID process (a
  sibling shell, a parent process, an attacker who already has user execution)
  can set it and bypass the loop. The witness chain and the server-side pre-push
  hook are the **actual** backstop; the launcher is correctness for the normal
  path, not a security perimeter.
- **PATH fallthrough is intentional.** If `anvil-run` is not on `$PATH` the
  shell wrappers exec the wrapped command directly rather than blocking. A
  misconfigured `$PATH` therefore silently drops enforcement. Operators who need
  hard enforcement should pin the full launcher path in `ANVIL_RUN_BIN`.
- **`anvil baseline --refresh --accept-suspicious` is operator-acked.** When a
  refresh would drop ≥75% of findings the binary refuses without the
  `--accept-suspicious` flag. The ack is meaningful — review the delta in
  findings before confirming, because an adversary who can introduce noise to
  inflate the baseline can later trip the suspicion heuristic and try to get the
  operator to ack a whitewashed state.

`<repo>/anvil/witness/active.ndjson` : The in-tree witness chain the daemon
consults during fence decisions. See
[witness-chain runbook](anvil-witness-chain.md).

`$XDG_RUNTIME_DIR/anvil/intercept.sock` (Linux)
`$HOME/.local/state/anvil/intercept.sock` (macOS fallback)
`\\.\pipe\anvil-intercept-<user>` (Windows) : Per-user daemon IPC endpoint. The
launcher rejects sockets that are not owned by the calling user.

`<install-prefix>/share/anvil-run/anvil-run.sh` : Shell integration. Source from
`.zshrc` / `.bashrc` to route `claude`, `codex`, and `aider` through the
launcher. Homebrew installs expose this at
`$(brew --prefix anvil)/share/anvil-run/anvil-run.sh`; curl installs at
`$HOME/.local/share/anvil/anvil-run.sh`.

## SHELL INTEGRATION

Source the shell-integration script to route common agent tools through the
launcher:

```bash
. "$(brew --prefix anvil)/share/anvil-run/anvil-run.sh"  # Homebrew
. "$HOME/.local/share/anvil/anvil-run.sh"                # curl installer
```

The script defines `claude()`, `codex()`, `aider()`, and a generic
`anvil-wrap <tool> <cmd> [args...]` function. Each wrapper:

1. Honours `ANVIL_RUN_DISABLE` (see ENVIRONMENT above).
2. Resolves the `anvil-run` binary on each call via `command -v`, so
   `brew upgrade anvil` picks up the new path without re-sourcing.
3. Falls through to direct `command "$@"` execution if `anvil-run` is not on
   `$PATH`. Losing Anvil's session enforcement is preferred over blocking the
   user — the daemon's witness chain and pre-push hook are the authentication
   backstop, not the launcher.

zsh and bash are supported in v1. Fish has a separate file (out of scope for
INTL-006 v1; see INTL-008 follow-ups).

## EXAMPLES

Wrap a Claude Code session:

```bash
anvil-run --tool claude-code -- claude code
```

Smoke-test the launch plan without spawning:

```bash
anvil-run --tool codex --dry-run -- codex --help
```

Register a side-channel session for a Claude Code `PreToolUse` hook:

```bash
anvil-run hook register --tool claude-code --cwd "$PWD" --pid "$PPID"
```

Disable the loop for one terminal:

```bash
ANVIL_RUN_DISABLE=1 claude code
```

Verify the launcher refuses a fenced worktree (expect exit 75):

```bash
anvil-run --tool claude-code -- echo hello
echo $?  # 75
```

## DIAGNOSTICS

**`anvil-run: daemon unavailable (exit 69)`** : The daemon socket / pipe was not
reachable. Run `anvil intercept start --foreground` and retry, or check
`anvil doctor` for the daemon-status diagnostic.

**`anvil-run: worktree fenced (exit 75)`** : The worktree's fence state is
active. Clear it with `anvil intercept unblock --worktree <path>` once the
underlying incident is resolved.

If the launcher is killed with `SIGTERM`, a terminal hangup, or parent-process
exit, the daemon may keep the session registered until the heartbeat-based reap
TTL expires. The default TTL is about 30 seconds; during that window,
`anvil intercept status` can still show the old session and a fresh launch in
the same worktree can observe a transient `Fenced` state. This is the expected
safety behaviour. If the fence blocks operator recovery, clear it with
`anvil intercept unblock --worktree <path>` after confirming the prior launch is
gone.

**`anvil-run: spawn failed: command not found (exit 73)`** : The wrapped command
does not resolve on `$PATH`. Confirm the binary is installed and reachable;
`$PATH` from the parent shell is inherited unmodified.

**`anvil-run: usage: --tool is required (exit 64)`** : You invoked the binary
directly without `--tool`. Use the shell wrappers, or pass `--tool <name>`
explicitly.

## BUGS AND LIMITATIONS

- **Foreground tty handoff** is not yet wired for fully interactive launches
  (e.g. a wrapped REPL with arrow-key history). Deferred to PR #1529.
- **Blocked-launch shell quoting** for unblock-instruction commands with spaces
  or special characters is best-effort in v1. Deferred under the same follow-up.
- **Fish shell** is unsupported. Source `shell/anvil-run.sh` only from zsh or
  bash.
- **macOS interrupt path** is fence-first this release (carry-forward from
  `v0.6.0-beta`): the launcher does not run the full SIGINT → SIGTERM → SIGKILL
  sequence on macOS. See details in
  [`docs/archive/runbooks/v0.6.0-beta-release-runbook.md`](../archive/runbooks/v0.6.0-beta-release-runbook.md).

## SEE ALSO

- [Witness chain operator runbook](anvil-witness-chain.md) — what the daemon's
  fence check authenticates against.
- [Hook coexistence operator runbook](anvil-hook-coexistence.md) — how Anvil
  hooks coexist with lefthook, husky, and pre-commit-framework.
- [Air-gap operation runbook](anvil-air-gapped.md) — the no-network guarantee
  `anvil-run` participates in.
- [`plans/archive/modules/intercept-launcher.aps.md`](../../plans/archive/modules/intercept-launcher.aps.md)
  — the full INTL module contract.
- [ADR-038 §3](../../plans/decisions/038-hook-surface-and-noise-discipline.md) —
  noise-discipline rules the launcher's refusal banner conforms to.

## PROVENANCE

- Filed 2026-05-18 as the N4 doc-lane closure for `v0.7.0-beta` (Wave 4
  release-gate evidence; see [`RELEASE-PLAN.md`](../../RELEASE-PLAN.md)).
- Implementation shipped via PR #1528 (merged 2026-05-14 at `5d38e546`) covering
  INTL-001..-009 with 49 unit + 3 shell-integration tests.
- Trust-model anchor: ADR-038 (Hook Surface and Noise Discipline) and MLP-014
  (attribution-recovery walk).
