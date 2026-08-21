---
id: troubleshooting
title: Troubleshooting
description:
  Diagnose installation, authentication, activation, watcher, and gate problems
  safely.
owner: DOCSYNC
upstream:
  - crates/anvil-cli/src/commands/doctor.rs
  - crates/anvil-cli/src/commands/version.rs
  - crates/anvil-cli/src/commands/status.rs
  - crates/anvil-cli/src/commands/gate.rs
verified_against: 0.9.7-beta
---

# Troubleshooting

Start with:

```text
anvil version
anvil doctor
anvil status
```

Record the exact command, exit code, and first useful error line.

## Wrong install method or upgrade advice

`anvil version` prints the install method and upgrade guidance. Prefer it over
`anvil --version` when diagnosing install ownership.

- Official installer installs on Windows and macOS should not report a plain
  `cargo install` method or suggest a Rust rebuild you do not need (`0.9.4-beta`
  and later). If they still do, reinstall with the
  [quickstart](../quickstart.md) method for your platform, open a new terminal,
  and re-check.
- On Windows, official installs should point at the PowerShell installer line,
  not a Unix `curl | sh` pipe.
- When a package manager owns the binary, upgrade or remove it only through that
  manager. See [upgrade notes](../releases/upgrade-notes.md).

## anvil is not found

### macOS or Linux

```bash
command -v anvil
echo "$PATH"
```

Open a new terminal after installation. If a package manager owns the binary,
upgrade or remove it through that manager.

### Windows PowerShell

```powershell
Get-Command anvil -All
$env:Path -split ';'
```

Open a new PowerShell session after installation.

## Authentication fails

```text
anvil auth whoami
anvil auth refresh
```

If refresh fails, run `anvil auth login` again. In CI, confirm the
`ANVIL_LICENSE` secret is available to the event; do not print it.

Action commands such as `anvil start`, bare `anvil`, `anvil gate`, and
`anvil check` exit **`3`** when authentication is required so scripted `&&`
chains stop cleanly. Read-only `anvil status` exits **`0`** and reports
`authRequired` under `--json`. See
[CLI exit codes](../reference/cli.md#exit-codes).

## Daily ensure fails or says not activated

Bare `anvil` (no subcommand) is the day-to-day on-switch after the project has
been activated once. It ensures the local daemon and already-configured MCP
entries; it does not install clients you previously skipped.

```text
anvil
anvil --json
```

If the project was never activated, the command names `anvil start` or
`anvil welcome`. Run `anvil start` to activate, then bare `anvil` on later days.
For a read-only diagnosis without changing configuration, use
`anvil start --verify`.

## Activation needs a restart

Fully quit the named client, reopen it, then run:

```text
anvil start --verify
```

Use `anvil start --why` when the state remains `ready_restart_required`.

## Activation reports watching

The local daemon recognises the project, but this state alone does not prove
that a save-time driver is attached or that pre-write protection is active. Run
`anvil watch` for a visible save-time loop. To diagnose pre-write support, check
the [client support](../reference/support.md), then inspect:

```text
anvil mcp --help
anvil intercept status
```

## Gate or check appears hung on a large repo

From `0.9.7-beta`, `anvil gate` and `anvil check` no longer stall while
excluding files marked `linguist-generated` in `.gitattributes`. Repos that
never set that attribute skip the lookup. If a run still looks frozen, the hub
"Review gate decision" path now updates the loading line as each check starts;
cancel with Ctrl+C if you need to stop.

See [configuration](config.md) for how generated-file exclusion works.

## The watcher shows no saves

- Wait for the ready message.
- Confirm the file extension is supported.
- Save inside the project root.
- Retry with `anvil watch --no-tui`.
- Run a named-file check to separate watcher routing from analysis.

## Local and CI results differ

Compare:

1. anvil version;
2. commit SHA;
3. gate profile and flags;
4. project configuration;
5. operating system and environment variables; and
6. authentication availability.

Use JSON or SARIF for automation; do not parse terminal decoration.

## Safe reset

Preview removal before deleting state:

```text
anvil uninstall --dry-run
```

Avoid deleting configuration or baselines manually when the command can explain
its scope.

## Report a bug

Open a [public issue](https://github.com/eddacraft/anvil/issues/new/choose) with
version, platform, command, exit code, protection state, and a minimal
reproduction. Remove credentials, private paths, and source.
