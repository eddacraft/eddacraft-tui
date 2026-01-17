---
id: capsules
title: Capsules
description: Understanding Kindling capsules as containers for observations.
sidebar_position: 1
---

# Capsules

Capsules are the primary organisational unit in Kindling.

## What is a Capsule?

A capsule is a container for related observations. Think of it as a folder for knowledge about a specific context:

```
Capsule: payment-integration
├── Observation: "Stripe uses cents, not dollars"
├── Observation: "Webhook signature verification required"
├── Observation: "Test cards: 4242..."
└── Observation: "Idempotency keys prevent duplicates"
```

## Why Capsules?

### Bounded Context

Observations make sense within their context. "Use the new API" means different things in different projects.

Capsules provide that context.

### Search Scope

Searching within a capsule returns relevant results:

```bash
kindling search "API" --capsule payment-integration
# Returns payment-related API knowledge

kindling search "API" --capsule auth-system
# Returns auth-related API knowledge
```

### Portability

Each capsule is a standalone SQLite database. You can:
- Back up individual capsules
- Share capsules with teammates
- Archive completed projects
- Move capsules between machines

## Capsule Lifecycle

### 1. Create

When starting new work:

```bash
kindling capsule create feature-auth
```

### 2. Use

Set as active:

```bash
kindling capsule use feature-auth
```

### 3. Populate

Record observations during work:

```bash
kindling observe "JWT tokens stored in httpOnly cookies"
```

### 4. Query

Retrieve knowledge:

```bash
kindling search "cookie"
```

### 5. Archive

When work completes:

```bash
kindling capsule archive feature-auth
```

### 6. Export (Optional)

Extract for documentation:

```bash
kindling export --capsule feature-auth --format markdown
```

## Capsule Types

### Global Capsules

Stored in `~/.kindling/capsules/`:

```bash
kindling capsule create my-capsule
```

Available everywhere on your machine.

### Project-Local Capsules

Stored in project directory:

```bash
kindling capsule create --local .
```

Creates `.kindling/` in current directory. Benefits:
- Version controlled
- Team shared
- Project-specific

### Temporary Capsules

For throwaway exploration:

```bash
kindling capsule create --temp
```

Automatically deleted after 24 hours.

## Capsule Metadata

Each capsule stores metadata:

```json
{
  "name": "payment-integration",
  "created": "2024-01-10T09:00:00Z",
  "lastUpdated": "2024-01-15T16:30:00Z",
  "observationCount": 42,
  "status": "active",
  "tags": ["project:shop", "team:payments"]
}
```

### Custom Metadata

Add project-specific metadata:

```bash
kindling capsule set payment-integration \
  --meta project=shop \
  --meta team=payments
```

## Best Practices

### One Capsule Per Context

```bash
# ✓ Good: specific context
kindling capsule create api-v2-migration
kindling capsule create security-audit-2024

# ✗ Avoid: too broad
kindling capsule create work
kindling capsule create notes
```

### Descriptive Names

```bash
# ✓ Good: descriptive
kindling capsule create stripe-integration
kindling capsule create auth-refresh-tokens

# ✗ Avoid: cryptic
kindling capsule create proj1
kindling capsule create temp
```

### Archive When Done

Don't delete—archive. You might need it later:

```bash
kindling capsule archive completed-feature
```

### Review Periodically

List capsules and clean up:

```bash
kindling capsule list --all
```

---

**Next:** [Observations →](/docs/kindling/concepts/observations)
