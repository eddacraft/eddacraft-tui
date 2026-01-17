---
id: search-export
title: Search and Export
description: Finding and exporting observations from Kindling.
sidebar_position: 4
---

# Search and Export

Retrieve observations when you need them.

## Basic Search

```bash
kindling search "authentication"
```

**Output:**

```
Found 5 observations:

[2024-01-15T10:30:00Z] Authentication requires JWT in Authorization header
  Capsule: api-project
  Tags: auth, jwt
  Source: manual

[2024-01-14T14:22:00Z] OAuth2 tokens expire after 1 hour
  Capsule: api-project
  Tags: auth, oauth
  Source: manual

...
```

## Search Options

### By Capsule

```bash
kindling search "error" --capsule payment-integration
```

### By Tag

```bash
kindling search --tag security
```

### By Kind

```bash
kindling search --kind gotcha
```

### By Date

```bash
# Last 7 days
kindling search "api" --since 7d

# Date range
kindling search "api" --since 2024-01-01 --until 2024-01-15
```

### Combine Filters

```bash
kindling search "rate limit" \
  --capsule api-project \
  --tag api \
  --since 30d
```

## Search Syntax

### Exact Phrase

```bash
kindling search '"rate limit"'
```

### OR Search

```bash
kindling search "authentication OR authorization"
```

### Exclude Terms

```bash
kindling search "api -deprecated"
```

## View Single Observation

```bash
kindling show obs_abc123
```

**Output:**

```
Observation: obs_abc123

Content:
  The API rate limits at 100 requests per minute. Exceeding
  this returns 429 Too Many Requests. Use exponential backoff.

Kind: gotcha
Created: 2024-01-15T10:30:00Z
Capsule: api-project
Tags: api, rate-limit, performance
Source: manual
```

## Export

### Export to Markdown

```bash
kindling export --capsule api-project --format markdown > api-notes.md
```

**Output file:**

```markdown
# api-project Observations

## 2024-01-15

### Authentication requires JWT in Authorization header

Tags: auth, jwt

The Authorization header must contain `Bearer <token>`.
Requests without valid JWT return 401.

---

### API rate limits at 100 requests per minute

Tags: api, rate-limit

Exceeding returns 429 Too Many Requests. Use exponential backoff.

---
```

### Export to JSON

```bash
kindling export --capsule api-project --format json > api-notes.json
```

### Export Filtered Results

```bash
kindling search "security" --export markdown > security-notes.md
```

### Export for LLM Context

Generate context for AI prompts:

```bash
kindling export --capsule api-project --format context
```

**Output:**

```
# Context from api-project

The following observations were recorded during development:

- Authentication requires JWT in Authorization header
- OAuth2 tokens expire after 1 hour
- Rate limit is 100 requests per minute
- Error responses include request ID for debugging
```

## Recent Observations

View recent observations without search:

```bash
kindling recent
```

```bash
# Last 10 from specific capsule
kindling recent --capsule api-project -n 10
```

## Statistics

View capsule statistics:

```bash
kindling stats --capsule api-project
```

**Output:**

```
Capsule: api-project

Total observations: 42
Date range: 2024-01-10 to 2024-01-15

By kind:
  discovery: 25 (60%)
  decision: 10 (24%)
  gotcha: 5 (12%)
  reference: 2 (4%)

Top tags:
  api (18)
  auth (12)
  error-handling (8)
  performance (6)

Activity:
  Jan 10: ████████ 8
  Jan 11: ████ 4
  Jan 12: ██████████████ 14
  Jan 13: ██ 2
  Jan 14: ██████ 6
  Jan 15: ████████ 8
```

---

**Next:** [Understand capsules →](/docs/kindling/concepts/capsules)
