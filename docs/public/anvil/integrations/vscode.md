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

:::caution VS Code MCP install is not a v1 surface

The current Rust CLI supports automatic MCP install for **Cursor and Claude Code
only**. VS Code MCP install, Windsurf MCP install, Copilot CLI, Codex CLI, and
process auto-attach are not v1 surfaces. The VS Code extension below is
independent of the MCP launch shim — it provides in-editor diagnostics by
running the CLI, not through MCP. `anvil mcp-config` does **not** support a
`vscode` target either (LAUNCH-009.5 removed the previous `vscode` and
`windsurf` emitters); the `Target` enum is `claude-code | cursor` only. For VS
Code with manual MCP wiring, hand-write the configuration in your VS Code MCP
config file (VS Code 1.99+ uses `.vscode/mcp.json` with the `servers` key) using
the `command: "anvil"` + `args: ["mcp", "serve", "--stdio"]` shape from the
[MCP Integration](/anvil/integrations/mcp#manual-configuration) page.

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
