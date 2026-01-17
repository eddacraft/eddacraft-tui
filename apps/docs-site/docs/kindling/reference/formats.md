---
id: formats
title: Formats
description: Observation and export format specifications.
sidebar_position: 1
---

# Formats

Specification of Kindling's data formats.

## Observation Format

### JSON Structure

```json
{
  "id": "obs_abc123xyz789",
  "content": "The API rate limits at 100 requests per minute",
  "kind": "gotcha",
  "tags": ["api", "rate-limit", "performance"],
  "source": {
    "type": "manual",
    "actor": "alice@example.com"
  },
  "context": {
    "file": "src/api/client.ts",
    "line": 42
  },
  "provenance": {
    "timestamp": "2024-01-15T10:30:00.000Z",
    "environment": {
      "os": "darwin",
      "shell": "zsh"
    }
  },
  "capsule": "api-project",
  "createdAt": "2024-01-15T10:30:00.000Z",
  "updatedAt": null
}
```

### Field Descriptions

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `id` | string | Yes | Unique identifier (obs_*) |
| `content` | string | Yes | Observation content |
| `kind` | enum | Yes | Type of observation |
| `tags` | string[] | No | Categorisation tags |
| `source` | object | Yes | Origin information |
| `context` | object | No | Location context |
| `provenance` | object | Yes | Full origin metadata |
| `capsule` | string | Yes | Parent capsule |
| `createdAt` | ISO8601 | Yes | Creation timestamp |
| `updatedAt` | ISO8601 | No | Last update timestamp |

### Kind Values

```typescript
type ObservationKind =
  | 'discovery'   // Something learned
  | 'decision'    // Choice made
  | 'gotcha'      // Warning/pitfall
  | 'reference'   // Link/citation
  | 'question'    // Open question
  | 'todo';       // Follow-up action
```

### Source Types

```typescript
interface Source {
  type: 'manual' | 'tool' | 'import' | 'adapter';
  actor?: string;
  name?: string;
  version?: string;
  file?: string;
}
```

## Export Formats

### Markdown

```markdown
# api-project Observations

## 2024-01-15

### The API rate limits at 100 requests per minute

- **Kind:** gotcha
- **Tags:** api, rate-limit
- **Source:** manual

Exceeding the rate limit returns HTTP 429 Too Many Requests.
Use exponential backoff with jitter for retries.

---
```

### JSON Export

```json
{
  "capsule": "api-project",
  "exportedAt": "2024-01-15T16:00:00.000Z",
  "observations": [
    { ... },
    { ... }
  ],
  "meta": {
    "count": 42,
    "dateRange": {
      "from": "2024-01-10T00:00:00.000Z",
      "to": "2024-01-15T16:00:00.000Z"
    }
  }
}
```

### Context Format

For LLM consumption:

```
# Context from api-project (42 observations)

## Recent Discoveries

- The API rate limits at 100 requests per minute
- Authentication uses OAuth2 with PKCE flow
- Webhook payloads are signed with HMAC-SHA256

## Key Decisions

- Chose Redis for caching due to built-in TTL support
- Using exponential backoff for all retries

## Known Gotchas

- Rate limit doesn't reset on new requests
- Webhook signature uses raw body, not parsed JSON
```

### CSV Export

```csv
id,content,kind,tags,created_at
obs_abc123,"API rate limits at 100 req/min",gotcha,"api,rate-limit",2024-01-15T10:30:00Z
obs_def456,"OAuth2 with PKCE flow",discovery,"auth,oauth",2024-01-15T10:25:00Z
```

## Import Formats

### Kindling JSON

Native format:

```json
{
  "version": "1.0",
  "observations": [
    { ... }
  ]
}
```

### Plain Text

One observation per line:

```
The API uses OAuth2
Rate limit is 100 req/min
Webhook signature required
```

Import:
```bash
kindling import notes.txt --kind discovery
```

### Markdown Notes

```markdown
# API Notes

- The API uses OAuth2 #auth
- Rate limit is 100 req/min #api #limits
- Webhook signature required #security
```

Import:
```bash
kindling import notes.md --parse-tags
```

---

**Next:** [CLI reference →](/docs/kindling/reference/cli)
