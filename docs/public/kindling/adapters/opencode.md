---
id: opencode
title: OpenCode Adapter
description: Capture observations from OpenCode development sessions.
sidebar_position: 2
---

# OpenCode Adapter

The OpenCode adapter captures observations from [OpenCode](https://opencode.ai)
development sessions. It maps OpenCode events to Kindling observations, manages
session capsule lifecycles, and includes content filtering with automatic secret
redaction.

It is a TypeScript package over the `@eddacraft/kindling` thin client (daemon-backed):

```bash
npm install @eddacraft/kindling-adapter-opencode @eddacraft/kindling
# or: pnpm add @eddacraft/kindling-adapter-opencode @eddacraft/kindling
```

## Session Management

The `SessionManager` handles the full session lifecycle: opening capsules on
session start, mapping events to observations, and closing capsules on session
end. All methods are **async** — they persist through the daemon client.

```typescript
import { SessionManager } from '@eddacraft/kindling-adapter-opencode';
import { Kindling } from '@eddacraft/kindling';

const kindling = new Kindling();
const manager = new SessionManager(kindling);

// Start a session — opens a capsule
const context = await manager.onSessionStart({
  sessionId: 'session-1',
  intent: 'Fix authentication bug',
  repoId: '/home/user/my-project',
});

// Process events as they arrive
await manager.onEvent({
  type: 'command',
  sessionId: 'session-1',
  timestamp: Date.now(),
  command: 'npm test',
  exitCode: 1,
  stderr: 'Auth token expired',
});

// End the session — closes the capsule
await manager.onSessionEnd('session-1', {
  summaryContent: 'Fixed JWT expiration handling',
});
```

If a session already has an open capsule (e.g. from a previous run),
`onSessionStart` resumes it instead of creating a new one.

## Event Types

The adapter maps these OpenCode event types to Kindling observation kinds:

| OpenCode Event  | Observation Kind | Description                    |
| --------------- | ---------------- | ------------------------------ |
| `tool_call`     | `tool_call`      | AI tool invocations            |
| `command`       | `command`        | Shell commands with exit codes |
| `file_change`   | `file_diff`      | File modifications with diffs  |
| `error`         | `error`          | Errors with stack traces       |
| `message`       | `message`        | User/assistant messages        |
| `session_start` | _(skipped)_      | Handled by session lifecycle   |
| `session_end`   | _(skipped)_      | Handled by session lifecycle   |

Each mapped observation includes provenance metadata extracted from the original
event (tool name, exit codes, file paths, etc.).

## Event Mapping

You can map events individually or in batch:

```typescript
import { mapEvent, mapEvents } from '@eddacraft/kindling-adapter-opencode';

// Single event
const result = mapEvent({
  type: 'tool_call',
  sessionId: 'session-1',
  timestamp: Date.now(),
  toolName: 'read_file',
  args: { path: 'src/auth.ts' },
  result: '...',
});

if (result.observation) {
  // Use the mapped observation
}

// Batch — skips lifecycle events, warns on errors
const observations = mapEvents(events);
```

## Content Filtering

The adapter includes safety filters to prevent accidental secret capture and
reduce noise.

### Secret Detection

Content is scanned for common secret patterns (API keys, AWS credentials, bearer
tokens, basic auth) and can be automatically masked:

```typescript
import {
  containsSecrets,
  maskSecrets,
  filterContent,
} from '@eddacraft/kindling-adapter-opencode';

// Check for secrets
containsSecrets('api_key=sk-abc123xyz'); // true

// Mask secrets in content
maskSecrets('token: sk-abc123xyz');
// → 'token=[REDACTED]'

// Apply all filters at once
filterContent(content, {
  maxLength: 50000,
  maskSecrets: true,
  showTruncationNotice: true,
});
```

### Path Exclusion

Sensitive file paths are excluded from capture:

```typescript
import { isExcludedPath } from '@eddacraft/kindling-adapter-opencode';

isExcludedPath('node_modules/foo/bar.js'); // true
isExcludedPath('.env'); // true
isExcludedPath('src/auth.ts'); // false
```

Excluded patterns: `node_modules`, `.git/`, `.env`, `.pem`, `.key`,
`credentials`, `secrets`.

### Truncation

Large content is automatically truncated to 50KB with a notice:

```typescript
import { truncateContent } from '@eddacraft/kindling-adapter-opencode';

truncateContent(largeString, { maxLength: 50000 });
// → '...content...\n\n[Truncated 12345 characters]'
```

## Memory Commands

The adapter provides CLI-style commands for interacting with Kindling memory
from within OpenCode sessions:

- **status** — Show current session and capsule state
- **search** — Query stored observations
- **pin** — Mark important observations as non-evictable

`forget` and `export` memory commands are currently stubs in the adapter — use
`kindling forget` and `kindling export` from the CLI for those operations.

## Next

- [PocketFlow adapter →](/kindling/adapters/pocketflow)
- [VS Code adapter →](/kindling/adapters/vscode)
