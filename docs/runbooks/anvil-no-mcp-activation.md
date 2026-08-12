# MCP-Optional Activation — Operator Runbook

| Type    | Authority     | Owner | Status | Freshness                                                        |
| ------- | ------------- | ----- | ------ | ---------------------------------------------------------------- |
| Runbook | Authoritative | ACTMO | Live   | Filed 2026-06-26 for ACTMO-009 against ADR-092 and `anvil start` |

| Upstream                                                                                                                                                                                                                                                 | Downstream                                                                                                                                                                |
| -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| [ACTMO](../../plans/modules/activation-mcp-optional.aps.md), [ADR-092](../../plans/decisions/092-mcp-optional-activation-spine.md), [`anvil start`](../../crates/anvil-cli/src/commands/start.rs) `plans/decisions/092-mcp-optional-activation-spine.md` | [Activation as-built](../architecture/activation-as-built.md), [Wow start guide](../public/anvil/guides/wow-start-demo.md), [Hook coexistence](anvil-hook-coexistence.md) |

Use this runbook when an organisation blocks editor MCP integration, has not yet
approved AI-client MCP wiring (Claude Code, Cursor, Codex, VS Code, and the rest
of the install registry — see `anvil mcp install --help`), or wants to evaluate
Anvil without touching editor MCP config. The supported path is
`anvil start --no-mcp` or `ANVIL_NO_MCP=1 anvil start`; this skips MCP config
writes but still runs the activation spine.

## Expected Posture

`--no-mcp` disables only the MCP install step. It does not disable:

- config adoption and baseline creation;
- daemon ensure when the session can safely start or reuse the per-user daemon;
- worktree registration with the intercept daemon;
- Anvil-managed `pre-commit` and `pre-push` hook installation when a Git repo is
  present;
- the activation diagnostic and rule-mode summary.

The honest success state without MCP is usually `watching`, not `protecting`.
`watching` means daemon-backed activation or save-time fallback is active, but
pre-write MCP attachment is not in evidence. `protecting` still requires live
pre-write validation evidence.

## Procedure

1. From the repository root, run:

   ```bash
   anvil start --no-mcp
   ```

   For scripts or fleet wrappers:

   ```bash
   ANVIL_NO_MCP=1 anvil start
   ```

   > **`ANVIL_NO_MCP` is presence-based** (matching `ANVIL_NO_DAEMON`): any
   > non-empty value enables the opt-out, so `ANVIL_NO_MCP=0` and
   > `ANVIL_NO_MCP=false` still **skip** MCP install. To keep MCP install on,
   > leave the variable unset (or set it to the empty string) rather than `0`.

2. Confirm the output includes:

   ```text
   install: skipped — MCP config installation disabled (`--no-mcp` / `ANVIL_NO_MCP`)
   ```

3. Confirm the activation state. Accept `watching` when the daemon-backed spine
   is armed. Do not require `protecting` unless the team has approved MCP.

4. Inspect daemon registration when the next-step line asks for it:

   ```bash
   anvil intercept status
   ```

   A healthy daemon-backed spine shows the daemon as running and lists the
   registered worktree/session. If the daemon is not running, use the diagnostic
   repair hint from `anvil start` or rerun in an interactive terminal without
   `--no-daemon`.

5. Confirm hook coverage:

   ```bash
   anvil hooks status
   ```

   If the repo uses Husky, Lefthook, or another hook manager, follow the
   [hook coexistence runbook](anvil-hook-coexistence.md) for the
   manager-specific hand-off.

## When MCP Is Later Approved

Drop `--no-mcp` / `ANVIL_NO_MCP` and rerun:

```bash
anvil start
```

Anvil will install or refresh MCP entries for selected/supported clients when
safe (interactive start offers the full registry unticked; scripted installs use
`--mcp-client` / `--all-mcp-clients` or `anvil mcp install --client <id>` — see
`anvil mcp install --help`). Claude Code installs also merge `mcp__anvil__*`
into `.claude/settings.json` `permissions.allow` so Anvil MCP tools do not
prompt on every write. Existing Claude allow/deny rules are preserved.

Restart the named AI client after MCP install, then run:

```bash
anvil start --verify
```

`protecting` is expected only after the editor has attached and live validation
evidence is observed.

## Stopping the Daemon

To release the daemon and its PID lock (for example before an upgrade, or when
`anvil start` reports the daemon is already running):

```bash
anvil intercept stop
```

`anvil intercept status` prints the daemon PID and the `anvil intercept stop`
recovery command, so an operator who only has a terminal can find and stop a
headless daemon.

> **Windows uses forced termination.** On Linux/macOS the daemon receives a
> signal and shuts down in an orderly way (it unbinds the IPC listener and
> removes its PID file). On Windows `anvil intercept stop` calls
> `TerminateProcess`, which is an immediate, ungraceful kill: any in-flight
> save-time fence state is discarded and the PID file is cleaned up by the stop
> command afterwards. Re-run `anvil start` straight away to re-arm daemon-backed
> save-time validation.

## Troubleshooting

| Symptom                         | Meaning                                                                | Action                                                                              |
| ------------------------------- | ---------------------------------------------------------------------- | ----------------------------------------------------------------------------------- |
| `install: skipped` appears      | MCP install was intentionally disabled                                 | Continue with daemon/hook verification                                              |
| `state: watching`               | Daemon-backed or save-time fallback is active; MCP pre-write is absent | Accept for no-MCP rollout, or approve MCP for `protecting`                          |
| `state: ready_restart_required` | MCP is wired but not attached, or daemon evidence needs repair         | Restart the editor, then run `anvil start --verify`; check `anvil intercept status` |
| `state: needs_action`           | The spine is not armed yet                                             | Follow the printed `next:` line                                                     |
| `state: error`                  | Config or install failure blocked activation                           | Read `last_error`; do not overwrite editor config manually                          |

## Source References

- `crates/anvil-cli/src/commands/start.rs` — `--no-mcp` flag, `ANVIL_NO_MCP`
  handling, and first-run output.
- `crates/anvil-cli/src/activation/orchestrator/mod.rs` — ordered activation
  spine, MCP install policy, daemon registration, and hook install hand-off.
- `crates/anvil-cli/src/activation/orchestrator/install.rs` — MCP config
  installer and Claude Code allow-list merge.
- `crates/anvil-cli/src/activation/diagnostic.rs` — activation state mapping for
  MCP-optional daemon-backed runs.
- [ADR-092](../../plans/decisions/092-mcp-optional-activation-spine.md) —
  product decision for the MCP-optional spine.
- [ACTMO module](../../plans/modules/activation-mcp-optional.aps.md) — active
  implementation and validation tracking.
- [Activation as-built](../architecture/activation-as-built.md) — current
  source-backed lifecycle and state mapping.
- [Wow start guide](../public/anvil/guides/wow-start-demo.md) — public-facing
  first-run behaviour.
