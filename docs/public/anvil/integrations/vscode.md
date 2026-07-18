---
id: vscode
title: Use Visual Studio Code
description:
  Use the supported terminal and save-time anvil workflows from Visual Studio
  Code.
---

# Use Visual Studio Code

There is no public, independently verifiable Visual Studio Code extension
required by the current anvil workflow. Do not install an unverified VSIX or
assume inline diagnostics are available.

## Supported workflow

1. Open the project in Visual Studio Code.
2. Open its integrated terminal.
3. Complete the [quickstart](../quickstart.md).
4. Run save-time validation:

```text
anvil watch
```

5. Keep the terminal visible while editing and saving supported files.

## AI-client distinction

Visual Studio Code is an editor. Cursor and Claude Code are the guided MCP
clients currently named by the support reference. An editor supporting MCP does
not automatically mean anvil has configured or verified it.

## Verify results

Make a safe deliberate finding with the [ten-minute tutorial](../first-gate.md).
Success is visible in the watcher terminal.

## Next step

Add [Git hooks](../operations/git-hooks.md) for editor-independent local checks.
