---
id: kindling
title: Kindling (Capture)
description: The capture layer of the Edda Stack.
sidebar_position: 1
---

# Kindling (Capture Layer)

Kindling is the foundation of the Edda Stack—where observations begin.

## Role in the Stack

```
Edda Stack
│
├── Edda (Curated)      ← Verified knowledge
│       ▲
├── Ember (Candidate)   ← Promoted observations
│       ▲
└── Kindling (Capture)  ← Raw observations [YOU ARE HERE]
```

Kindling captures observations during development. It's optimised for:

- **Speed** — Capture quickly, don't interrupt flow
- **Structure** — Enough metadata for later use
- **Simplicity** — Works without the rest of the stack

## Current Status

**Available Now**

Kindling is fully functional:

- CLI for capture and search
- SQLite storage
- Adapter framework
- Export capabilities

[Full Kindling Documentation →](/kindling/overview)

## Quick Start

```bash
# Install
pnpm add -g @eddacraft/kindling
# or: npm install -g @eddacraft/kindling
# or: yarn global add @eddacraft/kindling
# or: bun add -g @eddacraft/kindling

# Initialise
kindling init

# Capture
kindling observe "API uses OAuth2 with PKCE"

# Search
kindling search "OAuth"

# Export
kindling export --format markdown
```

## Integration with Stack

### Promotion to Ember (Future)

When Ember is available:

```bash
# Flag observation for promotion
kindling promote obs_abc123

# Or via review interface
kindling review --session today
```

### Context for Edda (Future)

Kindling observations provide source data for Edda entries:

```
Edda Entry: "OAuth2 PKCE Flow"
Sources:
  - obs_abc123: "API uses OAuth2 with PKCE" (2024-01-15)
  - obs_def456: "PKCE prevents code interception" (2024-01-15)
```

## What to Capture

Kindling works best when you capture:

### Discoveries

```bash
kindling observe "The cache TTL is 5 minutes" --kind discovery
```

### Decisions

```bash
kindling observe "Using Redis for sessions due to TTL support" --kind decision
```

### Gotchas

```bash
kindling observe "Don't use sync writes in production" --kind gotcha
```

### References

```bash
kindling observe "See RFC 7231 for status codes" --kind reference
```

## Best Practices

### Capture During Flow

Don't wait—capture while context is fresh:

```bash
# Right after solving a problem
kindling observe "The bug was caused by race condition in auth"
```

### Be Specific

```bash
# ❌ Vague
kindling observe "API issue"

# ✓ Specific
kindling observe "API returns 500 when payload > 1MB"
```

### Tag Consistently

Establish conventions:

```bash
kindling observe "..." --tag api --tag auth
kindling observe "..." --tag bug --tag performance
```

---

**Next:** [Design Principles →](/edda-stack/design-principles)
