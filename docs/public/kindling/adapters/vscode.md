---
id: vscode
title: VS Code Adapter
description:
  Capture file saves and editor activity into kindling from VS Code, Cursor, or
  Windsurf.
sidebar_position: 4
---

# VS Code Adapter

The VS Code adapter integrates kindling with VS Code, Cursor, and Windsurf. It
opens a session capsule when the extension activates, records `file_diff`
observations when you save files, and closes the capsule with a short summary
when the extension deactivates.

All data stays local. The extension talks to the kindling daemon through the
`@eddacraft/kindling` thin client (auto-spawns `kindling serve` on first use).

```bash
npm install @eddacraft/kindling-adapter-vscode
```

Ensure the `kindling` binary is on your `PATH`, or set `KINDLING_BIN` (or the
extension's `KINDLING_BINARY` setting) to point at it.

## Installation

Build from the [kindling repository](https://github.com/eddacraft/kindling/tree/main/packages/kindling-adapter-vscode),
package with `vsce`, then install the resulting `.vsix` from your editor's
Extensions view (**Install from VSIX**). The same flow works in Cursor and
Windsurf.

## Commands

Open the Command Palette and run:

| Command                 | Title                   | Description                                                  |
| ----------------------- | ----------------------- | ------------------------------------------------------------ |
| `kindling.search`       | Kindling: Search Memory | Search observations in the current workspace scope           |
| `kindling.logSelection` | Kindling: Log Selection | Append the current editor selection as a message observation |
| `kindling.status`       | Kindling: Status        | Show daemon health and active session info                   |

## What is captured

- **File saves** — path recorded as a `file_diff` observation
- **Selection logging** — manual capture of selected text as a `message`
  observation
- **Terminal commands** — optional via `EditorSessionManager.onTerminalCommand`
  (not wired by default in the stock extension)

## Programmatic usage

```typescript
import { EditorSessionManager } from '@eddacraft/kindling-adapter-vscode';
import { Kindling } from '@eddacraft/kindling';

const kindling = new Kindling({ projectRoot: '/path/to/repo' });
const manager = new EditorSessionManager(kindling);

await manager.onSessionStart({
  sessionId: 'editor-session-1',
  intent: 'Refactor auth module',
  repoId: '/path/to/repo',
});

await manager.onFileSave({
  sessionId: 'editor-session-1',
  filePath: '/path/to/repo/src/auth.ts',
  repoId: '/path/to/repo',
});

await manager.onSessionEnd('editor-session-1', {
  summaryContent: 'Updated auth middleware',
  summaryConfidence: 0.9,
});
```

## Next

- [Integrations matrix →](/kindling/reference/integrations)
- [Custom Integrations →](/kindling/adapters/custom)
