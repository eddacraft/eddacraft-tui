---
id: opencode
title: OpenCode Adapter
description: Integrating Kindling with OpenCode.
sidebar_position: 1
---

# OpenCode Adapter

Capture observations from OpenCode sessions.

## What is OpenCode?

OpenCode is a terminal-based AI coding assistant. The Kindling adapter captures observations from your OpenCode sessions automatically.

## Setup

### Install the Adapter

```bash
kindling adapter install opencode
```

### Configure OpenCode

Add to your OpenCode configuration:

```json
{
  "hooks": {
    "onSessionEnd": "kindling adapter opencode capture"
  }
}
```

### Set Target Capsule

```bash
kindling adapter opencode config --capsule my-project
```

## Automatic Capture

With the adapter configured, Kindling captures:

- **Decisions made** — when you accept solutions
- **Problems solved** — error resolutions
- **Context discovered** — API behaviours, gotchas

### Example Session

```
OpenCode: I've fixed the authentication issue by using the refresh token...
[Kindling: Captured as 'decision']

OpenCode: Note that the API rate limits at 100 req/min...
[Kindling: Captured as 'discovery']
```

## Manual Capture

During an OpenCode session:

```
/memory add "This API requires idempotency keys for POST requests"
```

This creates a Kindling observation immediately.

## Review Captures

After your session:

```bash
kindling recent --source opencode
```

```
[2024-01-15T10:30:00Z] Authentication uses refresh tokens for session extension
  Source: opencode
  Kind: decision

[2024-01-15T10:25:00Z] API rate limits at 100 requests per minute
  Source: opencode
  Kind: discovery
```

## Configuration

### Capture Settings

```json
{
  "opencode": {
    "capsule": "my-project",
    "capture": {
      "decisions": true,
      "discoveries": true,
      "errors": true,
      "codeBlocks": false
    },
    "minLength": 20
  }
}
```

| Option | Description | Default |
|--------|-------------|---------|
| `capsule` | Target capsule | Active capsule |
| `capture.decisions` | Capture decisions | `true` |
| `capture.discoveries` | Capture discoveries | `true` |
| `capture.errors` | Capture error resolutions | `true` |
| `capture.codeBlocks` | Capture code snippets | `false` |
| `minLength` | Minimum content length | `20` |

### Exclude Patterns

Don't capture certain content:

```json
{
  "opencode": {
    "exclude": [
      "password",
      "secret",
      "token"
    ]
  }
}
```

## Troubleshooting

### Observations Not Appearing

1. Check adapter is installed:
   ```bash
   kindling adapter list
   ```

2. Check configuration:
   ```bash
   kindling adapter opencode config --show
   ```

3. Run manually:
   ```bash
   kindling adapter opencode capture --debug
   ```

### Duplicate Observations

Enable deduplication:

```json
{
  "opencode": {
    "dedupe": true,
    "dedupeWindow": "1h"
  }
}
```

---

**Next:** [PocketFlow adapter →](/docs/kindling/adapters/pocketflow)
