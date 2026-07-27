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

## Behaviour changes after 0.9.0-beta

These land with the next beta after 0.9.0-beta (draft on the maintained branch).
Confirm each surface with `anvil --help` on the binary you actually installed:

- **Activation TUI by default.** On a real terminal, `anvil start` opens the
  consent-first interactive surface without `--tui`. Use `--no-tui` or
  `ANVIL_NO_TUI=1` for plain text. Read-only, `--json`, `--watch`, CI, and
  non-TTY sessions stay plain. `--tui` remains accepted as a no-op.
- **Warnings do not fail the gate by default.** Warning-severity anti-pattern
  findings no longer fail `anvil gate` unless you set `--fail-on-warnings` or
  `ANVIL_FAIL_ON_WARNINGS`. Broken ciphers / ECB and JWT `none` stay error
  severity and still block.
- **More MCP clients via explicit install.** Guided activation still defaults to
  Cursor and Claude Code. Wider clients appear under `anvil mcp install --help`
  when your binary includes them.
- **Browser dashboard.** A loopback browser mode appears under
  `anvil dashboard --help` when your binary includes it; bare `anvil dashboard`
  remains the terminal picker.

## 0.9.0-beta automation change

Authentication-required action commands use a distinct non-zero exit code. Any
script that previously treated an unauthenticated action as success must now
handle authentication explicitly.

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
