---
id: vscode
title: Use Visual Studio Code
description:
  Use terminal, save-time, and optional MCP workflows for anvil from Visual
  Studio Code.
---

# Use Visual Studio Code

There is no public, independently verifiable Visual Studio Code extension
required by the current anvil workflow. Do not install an unverified VSIX or
assume inline diagnostics are available.

## Supported baseline workflow

1. Open the project in Visual Studio Code.
2. Open its integrated terminal.
3. Complete the [quickstart](../quickstart.md).
4. Run save-time validation:

```text
anvil watch
```

5. Keep the terminal visible while editing and saving supported files.

## Optional MCP configuration

VS Code is in the MCP install registry. To write anvil's documented entry into
the VS Code configuration shape:

```text
anvil mcp install --client vscode
anvil mcp install --client vscode --verify
```

Restart VS Code when the command asks, then run:

```text
anvil start --verify
```

Success is `protecting`. Configuration alone is not enough.

Guided activation in `anvil start` still defaults to Cursor and Claude Code. Use
the explicit `mcp install` command above for VS Code.

## Optional language server

Editors that speak Language Server Protocol can attach to anvil's advisory graph
surface:

```text
anvil lsp --stdio
```

This is an experimental, advisory-only frontend over the resident graph — it is
not a full diagnostics product and does not replace MCP pre-write validation.
Wire it through your editor's LSP client configuration only if you understand
that boundary.

## Verify results

Make a safe deliberate finding with the [ten-minute tutorial](../first-gate.md).
Success is visible in the watcher terminal, or as a blocked write when MCP
pre-write protection is active.

## Next step

Add [Git hooks](../operations/git-hooks.md) for editor-independent local checks,
or review [MCP integration](mcp.md) for other clients.
