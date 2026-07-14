---
id: vscode
title: VS Code Integration
description: Using the anvil VS Code extension for in-editor feedback.
sidebar_position: 2
---

# VS Code Integration

The anvil VS Code extension provides real-time feedback directly in your editor.

## Installation

:::info Closed beta

The VS Code extension is not yet published to the VS Code Marketplace. During
the closed beta, install it from the `.vsix` file provided with your beta
access. If you don't have the file, contact us at
[eddacraft.ai](https://eddacraft.ai) or check the
[GitHub releases](https://github.com/eddacraft/anvil/releases) for your version.

:::

:::caution MCP configuration is separate from the extension

The extension below provides in-editor diagnostics by running the CLI. It is
independent of the MCP launch shim. For project-scoped MCP setup, run
`anvil mcp install --client vscode --scope project`; Anvil writes
`.vscode/mcp.json` using VS Code's `servers` shape. Global setup delegates to
the vendor profile command (`code --add-mcp`) instead of guessing a profile
file. A successful configuration write still does not prove a live MCP
handshake; reload VS Code and verify the server from its MCP UI.

:::

### From VSIX File

```bash
code --install-extension anvil-vscode-0.1.0.vsix
```

Or in VS Code: **Extensions** (Ctrl+Shift+X / Cmd+Shift+X) → **⋯** menu →
**Install from VSIX…** → select the file.

## Features

### Inline Diagnostics

Issues appear as squiggly underlines in your code:

- **Red** — errors (gate failures)
- **Yellow** — warnings (anti-patterns)
- **Blue** — info (suggestions)

Hover for details and quick fixes.

### Problems Panel

All issues appear in the Problems panel (Ctrl+Shift+M / Cmd+Shift+M):

```
PROBLEMS (2)
  src/api/users.ts
    ⚠ [AP-003] Explicit 'any' type - line 42
  src/services/auth.ts
    ⚠ [AP-006] Empty catch block - line 87
```

### Quick Fixes

Some issues have automatic fixes:

1. Click the lightbulb (or Ctrl+.)
2. Select the fix

Available quick fixes:

- Add type annotation (AP-003)
- Add error logging (AP-006)
- Add suppression with placeholder

### Status Bar

The status bar shows the current anvil status:

- **✓ Anvil** — all clear
- **⚠ Anvil (2)** — 2 warnings
- **✗ Anvil (1)** — 1 error

Click for quick actions.

### Code Actions

Right-click context menu:

- **Anvil: Suppress this issue**
- **Anvil: Run check on file**
- **Anvil: Show Output**

## Configuration

### Extension Settings

Open Settings (Ctrl+, / Cmd+,) and search for "Anvil":

| Setting                | Description              | Default    |
| ---------------------- | ------------------------ | ---------- |
| `anvil.enable`         | Enable/disable extension | `true`     |
| `anvil.configPath`     | Path to config file      | `.anvilrc` |
| `anvil.validateOnSave` | Run on save              | `true`     |
| `anvil.validateOnType` | Run while typing         | `false`    |
| `anvil.debounceMs`     | Delay before validation  | `300`      |

### Workspace Settings

Configure per-workspace in `.vscode/settings.json`:

```json
{
  "anvil.enable": true,
  "anvil.validateOnSave": true,
  "anvil.debounceMs": 500
}
```

## Commands

Access via Command Palette (Ctrl+Shift+P / Cmd+Shift+P):

| Command              | Description                   |
| -------------------- | ----------------------------- |
| `Anvil: Run`         | Run Anvil on current file     |
| `Anvil: Run All`     | Run Anvil on entire workspace |
| `Anvil: Toggle`      | Enable/disable Anvil          |
| `Anvil: Show Output` | View Anvil output channel     |
| `Anvil: Clear Cache` | Clear validation cache        |

## Keyboard Shortcuts

Default shortcuts:

| Shortcut       | Command                   |
| -------------- | ------------------------- |
| `Ctrl+Shift+A` | Run Anvil on current file |
| `Ctrl+Alt+A`   | Toggle Anvil              |

Customise in Keyboard Shortcuts.

## Output Channel

View detailed logs:

1. View → Output (or Ctrl+Shift+U)
2. Select "Anvil" from dropdown

Useful for debugging configuration issues.

## Integration with Other Extensions

### ESLint

anvil complements ESLint—they run independently:

- ESLint shows style/semantic issues
- anvil shows architecture/pattern issues

Both appear in Problems panel.

### Formatters

anvil doesn't interfere with formatter extensions.

### GitLens

GitLens and Anvil can be used side by side: GitLens explains who last changed a
line, while Anvil explains whether the current change violates a rule.

## Troubleshooting

### Extension Not Loading

1. Check Output channel for errors
2. Verify `.anvilrc` exists
3. Run `Anvil: Show Output`

### Diagnostics Not Appearing

1. Ensure `anvil.enable` is `true`
2. Check file is in workspace
3. Verify file type is supported

### Performance Issues

1. Increase `debounceMs`
2. Disable `validateOnType`
3. Add large directories to ignore list

---

**Next:** [MCP integration →](/anvil/integrations/mcp)
