---
id: memory
title: Memory Commands
description: Slash commands for memory operations.
sidebar_position: 1
---

# Memory Commands

Kindling provides `/memory` commands for quick operations.

## Available Commands

### /memory add

Add an observation quickly:

```
/memory add "The API uses OAuth2 with PKCE"
```

With options:

```
/memory add "Rate limit is 100 req/min" --tag api --kind gotcha
```

### /memory search

Search observations:

```
/memory search rate limit
```

### /memory recent

Show recent observations:

```
/memory recent
/memory recent 10
```

### /memory export

Export current capsule:

```
/memory export
/memory export --format json
```

### /memory capsule

Capsule operations:

```
/memory capsule list
/memory capsule use my-project
/memory capsule create new-project
```

### /memory context

Generate context for current work:

```
/memory context
```

Returns relevant observations for the current session.

## Usage in Tools

### In Terminal

If your shell supports slash commands:

```bash
$ /memory add "Found the bug - race condition in auth"
Observation recorded.
```

### In Editors

VS Code command palette:
```
> Kindling: Add Memory
```

### In AI Assistants

During Claude/GPT sessions:

```
/memory add "This API requires snake_case parameters"
```

## Command Aliases

Configure shortcuts in `config.json`:

```json
{
  "aliases": {
    "/m": "/memory",
    "/ma": "/memory add",
    "/ms": "/memory search",
    "/mr": "/memory recent"
  }
}
```

Now use:
```
/ma "Quick observation"
/ms api
```

## Contextual Commands

Some commands infer context:

### /memory add (no content)

Opens an editor:

```
/memory add
# Opens $EDITOR for multi-line input
```

### /memory search (no query)

Interactive search:

```
/memory search
? Enter search query: _
```

## Integration

### Shell Function

Add to `.zshrc` or `.bashrc`:

```bash
mem() {
  kindling "$@"
}

# Usage: mem add "observation"
```

### Git Hook

Capture on commit:

```bash
# .git/hooks/post-commit
kindling observe "Committed: $(git log -1 --pretty=%B)" --tag commit
```

### Cron Job

Periodic reminder to capture:

```cron
0 17 * * * kindling observe --interactive --prompt "End of day observations?"
```

---

**Next:** [Formats reference →](/docs/kindling/reference/formats)
