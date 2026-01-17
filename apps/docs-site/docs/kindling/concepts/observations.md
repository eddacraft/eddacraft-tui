---
id: observations
title: Observations
description: Understanding observation structure and types in Kindling.
sidebar_position: 2
---

# Observations

Observations are individual pieces of captured knowledge.

## Observation Structure

Every observation has:

```typescript
interface Observation {
  id: string; // Unique identifier
  content: string; // The actual knowledge
  kind: ObservationKind;
  tags: string[];
  source: Source;
  context?: Context;
  createdAt: Date;
  capsule: string;
}
```

## Content

The core knowledge being captured:

```bash
kindling observe "Stripe webhooks require signature verification using the raw request body"
```

### Content Guidelines

**Be specific:**

```bash
# ❌ Vague
kindling observe "API thing"

# ✓ Specific
kindling observe "API returns 429 after 100 requests per minute"
```

**Include reasoning:**

```bash
# ❌ Just the what
kindling observe "Use Redis"

# ✓ Include why
kindling observe "Use Redis for session storage - it handles our 10k concurrent users better than in-memory"
```

**Note exceptions:**

```bash
kindling observe "All API calls need auth EXCEPT /health and /metrics"
```

## Kinds

Observations have kinds that indicate their nature:

### Discovery

Something you learned:

```bash
kindling observe "The retry mechanism uses exponential backoff with jitter" --kind discovery
```

### Decision

A choice made with reasoning:

```bash
kindling observe "Chose PostgreSQL over MongoDB for ACID transactions" --kind decision
```

### Gotcha

A pitfall or warning:

```bash
kindling observe "Don't call the API during the maintenance window (2-3 AM UTC)" --kind gotcha
```

### Reference

A link or citation:

```bash
kindling observe "See RFC 6749 for OAuth 2.0 spec: https://tools.ietf.org/html/rfc6749" --kind reference
```

### Question

Something to investigate:

```bash
kindling observe "Why does the cache invalidation take 30 seconds?" --kind question
```

### Todo

A follow-up action:

```bash
kindling observe "Add rate limiting to the webhook endpoint" --kind todo
```

## Tags

Tags enable categorisation and filtering:

```bash
kindling observe "Use content-type: application/json" --tag api --tag headers
```

### Tag Conventions

Establish consistent tags:

| Tag         | Use for                 |
| ----------- | ----------------------- |
| `#api`      | API-related knowledge   |
| `#security` | Security considerations |
| `#perf`     | Performance insights    |
| `#bug`      | Bug findings            |
| `#config`   | Configuration details   |
| `#debug`    | Debugging techniques    |

### Inline Tags

Tags can be inline:

```bash
kindling observe "Rate limit is 100 req/min #api #limits"
```

## Source

Where the observation came from:

### Manual

Direct input:

```bash
kindling observe "..." --source manual
```

### Tool

From an integrated tool:

```typescript
{
  source: {
    type: "tool",
    name: "vscode",
    version: "1.85.0"
  }
}
```

### Import

From external data:

```typescript
{
  source: {
    type: "import",
    file: "notes.md",
    importedAt: "2024-01-15T10:00:00Z"
  }
}
```

## Context

Optional context about where the observation applies:

```bash
kindling observe "This function is O(n²)" \
  --context "src/utils/sort.ts:42"
```

### Context Types

```typescript
interface Context {
  file?: string; // File path
  line?: number; // Line number
  function?: string; // Function name
  commit?: string; // Git commit
  url?: string; // URL reference
}
```

## Provenance

Full origin tracking:

```typescript
{
  provenance: {
    actor: "alice@example.com",
    timestamp: "2024-01-15T10:30:00Z",
    source: "manual",
    environment: {
      os: "darwin",
      shell: "zsh",
      cwd: "/Users/alice/project"
    }
  }
}
```

Provenance enables:

- Attribution
- Timeline reconstruction
- Audit trails

## Relationships

Observations can reference each other:

```bash
kindling observe "Follow-up: see obs_abc123 for root cause" --related obs_abc123
```

---

**Next:** [Storage →](/docs/kindling/concepts/storage)
