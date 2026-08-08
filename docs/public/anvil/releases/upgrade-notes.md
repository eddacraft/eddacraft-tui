---
id: upgrade-notes
title: Upgrade anvil
description:
  Upgrade anvil safely and verify configuration, authentication, and protection
  afterwards.
---

# Upgrade anvil

**For:** users moving an existing beta installation to the current release

**Time:** 10–20 minutes

**Outcome:** one current binary, migrated configuration where needed, and a
verified protection state

## 1. Record the current installation

```text
anvil version
```

Note the version and installation method. Use one package manager or installer
per machine.

## 2. Upgrade through the owning method

### Built-in updater

```text
anvil update
```

When anvil was installed through Homebrew, Scoop, or WinGet, the updater offers
that manager's allowlisted upgrade command after explicit consent. Use `-y` only
in non-interactive scripts you trust. Direct installs keep the signed-artefact
path.

### Homebrew

```bash
brew upgrade eddacraft/tap/anvil
```

### WinGet

```powershell
winget upgrade --id eddacraft.anvil
```

### Scoop

```powershell
scoop update anvil
```

For standalone installations, return to the maintained
[quickstart](../quickstart.md) and use the installer for your platform.

## 3. Verify the binary

```text
anvil --version
```

Confirm only the intended installation appears:

macOS or Linux:

```bash
type -a anvil
```

Windows PowerShell:

```powershell
Get-Command anvil -All
```

Remove stale duplicates through the manager that installed them.

## 4. Migrate when requested

```text
anvil migrate --help
```

Run a schema or format migration only when the release output or command help
requires it. Review project-file changes before committing.

## 5. Verify authentication and protection

```text
anvil auth whoami
anvil start --verify
```

If authentication has expired, try `anvil auth refresh` and sign in again when
asked.

After upgrading, refresh diagnostics and re-verify any guided client you use:

```text
anvil doctor
anvil mcp install --client cursor --verify
anvil start --verify
```

When your binary lists additional MCP clients or a managed `skill` command, use
installed help to re-verify those assets after the upgrade.

## 6. Restart long-lived processes

A running daemon or editor/agent MCP process can keep the old anvil image after
the on-disk binary is replaced. Check the responding daemon against the current
CLI:

```text
anvil intercept status
```

When the daemon and CLI versions differ, run `anvil intercept stop`. The stop
command reports the daemon PID and can return before shutdown is complete on
Unix, so wait until that reported daemon process has exited before running
`anvil start`. Reload each open editor or agent to restart its MCP server as
well. anvil cannot enumerate every retained MCP process consistently across
supported platforms, so repeat this for each open client after an upgrade.

## 0.9.3-beta behaviour changes

These changes affect upgrades from 0.9.2-beta and earlier:

- **Standalone installs are recognised by their cargo-dist receipt.** Run
  `anvil version` and confirm the printed installation method, then use
  `anvil update --check`. Existing receipts under the legacy `anvil` app name
  remain supported.
- **Windows path normalisation can refresh code-scanning alerts once.** SARIF
  fingerprints for secret findings use the corrected path. Existing alerts may
  close and reappear after the first 0.9.3-beta upload; review the replacement
  alert rather than treating the churn as a new secret by itself.
- **Whole-file findings no longer use line zero.** Plain output omits a line
  number and JSON emits `"line": null`. Automation that required an integer for
  every finding must accept `null` for file-level evidence.
- **Workspace registration is verified before success is reported.** A failed
  durable registration now exits without claiming `Registered`. Scripts can
  confirm retained membership with `anvil workspace list --json`.
- **Coverage wording is more explicit.** `audit`, `gate`, and `check` name the
  file-type and check domains they actually evaluated. Treat that scope text as
  part of the result; a clean domain is not a claim about unscanned files.

## 0.9.1-beta behaviour changes

These shipped in `0.9.1-beta`. Confirm each surface with `anvil --help` on the
binary you actually installed:

- **Activation TUI by default.** On a real terminal, `anvil start` opens the
  consent-first interactive surface without `--tui`. Use `--no-tui` or
  `ANVIL_NO_TUI=1` for plain text. Read-only, `--json`, `--watch`, CI, and
  non-TTY sessions stay plain. `--tui` remains accepted as a no-op.
- **Warnings do not fail the gate by default.** Warning-severity anti-pattern
  findings no longer fail `anvil gate` unless you set `--fail-on-warnings` or
  `ANVIL_FAIL_ON_WARNINGS`. Broken ciphers / ECB and JWT `none` stay error
  severity and still block.
- **Multi-harness MCP on start.** Interactive `anvil start` offers every
  supported client in the consent list (unticked until you select one). List
  client ids with `anvil mcp install --help`. For scripted multi-client install
  on start, use `anvil start --mcp-client <id>` (repeatable) or
  `anvil start --all-mcp-clients` — those flags are on `anvil start`, not on
  `anvil mcp install`.
- **Bare `anvil` daily ensure.** After activation, run `anvil` with no
  subcommand to ensure the daemon and already-owned MCP entries without
  reinstall prompts. Use `anvil start` for first-time setup and reconfigure.
- **Disclosed opt-out telemetry.** After an eligible interactive first run shows
  its notice, anvil can send a narrow anonymous usage beacon at most once per 24
  hours. Inspect the gate and eligible payload with `anvil telemetry`. Disable
  sending with `anvil telemetry off`, `ANVIL_TELEMETRY=off`, or
  `DO_NOT_TRACK=1`; see [anonymous usage telemetry](../operations/telemetry.md)
  for the exact payload, timing, transient IP processing, and retention.

## 0.9.0-beta automation change

Authentication-required **action** commands exit **`3`** (`EXIT_AUTH_REQUIRED`)
so `&&` chains stop at an unauthenticated repo. Read-only `anvil status` still
exits **`0`** with an informational `authRequired` envelope under `--json`. Any
script that previously treated an unauthenticated action as success must now
handle authentication explicitly. See the
[CLI exit codes](../reference/cli.md#exit-codes).

Interactive activation choices also begin unselected. Pressing Enter without
choosing an item writes nothing.

## Roll back

Use the installation manager's supported version selection, then run
`anvil --version` and `anvil start --verify`. Restore project configuration only
from a reviewed version-control change; do not copy user credentials between
versions.

## Next step

Review the [public changelog](changelog.md) and repeat the
[beta testing brief](../beta-testing-guide.md).
