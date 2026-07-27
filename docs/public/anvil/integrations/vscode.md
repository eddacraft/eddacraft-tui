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

Guided activation in `anvil start` still defaults to Cursor and Claude Code. VS
Code is not on that guided path in the current public release.

Newer betas expand `anvil mcp install --client` so VS Code can be configured
explicitly when your binary lists it. Discover support first:

```text
anvil mcp install --help
```

If `vscode` appears among the client values, install once, restart VS Code when
asked, then verify protection separately:

```text
anvil start --verify
```

Success is `protecting`. Configuration alone is not enough. Prefer a dedicated
`--verify` pass on the MCP entry only when your help text documents that flag
for the client you installed.

## Optional language server

Newer betas can expose an advisory Language Server Protocol surface over the
resident graph. Check top-level help first:

```text
anvil --help
```

If an `lsp` command is listed, use its help for transport flags (commonly
stdio). It is experimental and advisory-only — not a full diagnostics product
and not a substitute for MCP pre-write validation.

## Verify results

Make a safe deliberate finding with the [ten-minute tutorial](../first-gate.md).
Success is visible in the watcher terminal, or as a blocked write when MCP
pre-write protection is active.

## Next step

Add [Git hooks](../operations/git-hooks.md) for editor-independent local checks,
or review [MCP integration](mcp.md) for other clients.
