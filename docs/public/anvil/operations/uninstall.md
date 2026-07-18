---
id: uninstall
title: Uninstall and clean up
description:
  Preview and remove project or user-level anvil state without deleting
  unrelated files.
---

# Uninstall and clean up

**For:** users removing anvil from one project or from a machine

**Time:** 5–10 minutes

**Outcome:** selected anvil-managed state is removed with a reviewable scope

## Preview first

From the project root:

```text
anvil uninstall --dry-run
```

Read every path before continuing.

## Remove project state

```text
anvil uninstall
```

The default scope covers the current project's anvil state and managed hooks. It
does not remove the installed binary or global credentials.

## Remove user-level state too

Preview:

```text
anvil uninstall --global --dry-run
```

Then, if the scope is correct:

```text
anvil uninstall --global
```

Use `--keep-mcp` or `--keep-daemon` only when you understand why that state must
remain.

## Remove the binary

After cleaning state, remove the binary with the method that installed it:

```bash
brew uninstall eddacraft/tap/anvil
```

```powershell
winget uninstall eddacraft.anvil
```

```powershell
scoop uninstall anvil
```

For the standalone installer, use the path reported by `Get-Command anvil -All`
on Windows or `command -v anvil` on macOS/Linux, then remove only that binary.

## Verify

Open a new terminal and run:

```text
anvil --version
```

A command-not-found result confirms the binary is no longer on PATH. Also run
`anvil hooks status` before removing the binary if you need evidence that
project hooks were cleaned up.
