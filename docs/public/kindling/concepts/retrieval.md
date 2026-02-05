---
id: retrieval
title: Retrieval
description: Philosophy and methods for retrieving observations.
sidebar_position: 4
---

# Retrieval

How to get knowledge back out of Kindling.

## Retrieval Philosophy

Kindling follows a **mechanical-first** approach:

1. **Exact search** — find what you explicitly stored
2. **Filtered search** — narrow by tags, dates, kinds
3. **Full-text search** — find by content keywords
4. **AI-assisted** — optional semantic search (future)

This is intentional. You should find exactly what you stored, not what an
algorithm thinks you want.

## Search Methods

### Exact Match

```bash
kindling search "rate limit"
```

Returns observations containing "rate limit".

### Tag Filter

```bash
kindling search --tag api --tag security
```

Returns observations with both tags.

### Kind Filter

```bash
kindling search --kind gotcha
```

Returns all gotchas.

### Date Range

```bash
kindling search --since 2024-01-01 --until 2024-01-31
```

### Combined

```bash
kindling search "authentication" \
  --capsule auth-project \
  --tag security \
  --kind gotcha \
  --since 30d
```

## Search Syntax

### Boolean Operators

```bash
# AND (implicit)
kindling search "rate limit"

# OR
kindling search "rate OR limit"

# NOT
kindling search "api -deprecated"
```

### Exact Phrase

```bash
kindling search '"Content-Type header"'
```

### Wildcard

```bash
kindling search "auth*"  # auth, authentication, authorise
```

## Result Ranking

Results are ordered by:

1. **Recency** — newer observations first
2. **Match quality** — exact matches over partial
3. **Relevance** — title matches over body

Override with:

```bash
kindling search "api" --sort created-asc
kindling search "api" --sort relevance
```

## Retrieval Use Cases

### During Development

Quick lookup:

```bash
kindling search "error code 422"
```

### For Documentation

Export for README:

```bash
kindling search --tag api-docs --export markdown
```

### For AI Context

Generate LLM context:

```bash
kindling export --capsule current-project --format context
```

Output:

```
Relevant context from development:
- The API uses OAuth2 with PKCE flow
- Rate limit is 100 requests per minute
- Errors include request ID in X-Request-ID header
```

### For Code Review

Find related decisions:

```bash
kindling search --kind decision --since 7d
```

## Proactive Retrieval

### Watch Mode (Future)

Kindling watches your work and surfaces relevant observations:

```
You're editing: src/auth/login.ts

Related observations:
- "JWT tokens expire after 1 hour"
- "Always validate token signature server-side"
```

### MCP Integration

AI assistants can query Kindling:

```json
{
  "tool": "kindling_search",
  "arguments": {
    "query": "authentication",
    "capsule": "current-project"
  }
}
```

## When Retrieval Fails

If you can't find something:

### Check Capsule

```bash
# List all capsules
kindling capsule list --all

# Search across capsules
kindling search "query" --all-capsules
```

### Broaden Search

```bash
# Remove filters
kindling search "auth"  # instead of "authentication"

# Check similar terms
kindling search "login OR auth OR session"
```

### Check Archived

```bash
kindling search "query" --include-archived
```

### List Recent

Sometimes browsing is easier:

```bash
kindling recent -n 50
```

---

**Next:** [Custom Adapters →](/docs/kindling/adapters/custom)
