---
id: edda
title: Edda (Curated)
description: The curated knowledge layer of the Edda Stack.
sidebar_position: 3
---

# Edda (Curated Layer)

:::note Coming Soon
Edda is in the planning phase. This page describes the envisioned functionality.
:::

Edda is the crown of the stack—verified, reusable knowledge curated from team experience.

## Role in the Stack

```
Edda Stack
│
├── Edda (Curated)      ← Verified knowledge [PLANNED]
│       ▲
├── Ember (Candidate)   ← Promoted observations
│       ▲
└── Kindling (Capture)  ← Raw observations
```

## Vision

Edda will be the source of truth for team knowledge:

- **Verified** — Human-reviewed for accuracy
- **Structured** — Consistent format for discoverability
- **Connected** — Linked to source observations
- **Living** — Updated as knowledge evolves

## Envisioned Structure

### Knowledge Entries

```typescript
interface EddaEntry {
  id: string;
  title: string;
  content: string;
  category: string;
  tags: string[];
  sources: EmberEntry[];
  verifiedBy: string[];
  createdAt: Date;
  updatedAt: Date;
  confidence: 'high' | 'medium' | 'low';
}
```

### Example Entry

```yaml
title: Stripe Payment Processing
category: integrations/payments
confidence: high
verified_by:
  - alice@example.com
  - bob@example.com

content: |
  ## Amount Format
  Stripe uses cents (integers), not dollars.
  - $10.00 → 1000
  - $0.50 → 50

  ## Idempotency
  All POST requests require X-Idempotency-Key header.
  Use UUIDs. Stripe caches responses for 24 hours.

  ## Webhooks
  Always verify webhook signatures using raw request body.
  The parsed JSON body will have different signature.

sources:
  - ember_abc123: "Stripe uses cents not dollars"
  - ember_def456: "Webhook signature verification"
  - ember_ghi789: "Idempotency key requirements"
```

## Envisioned Features

### Search and Discovery

```bash
edda search "payment"
```

```
Results:

1. Stripe Payment Processing
   Category: integrations/payments
   Confidence: high
   Last updated: 2024-01-15

2. PayPal Integration Notes
   Category: integrations/payments
   Confidence: medium
   Last updated: 2024-01-10
```

### Context Surfacing

Edda will surface relevant knowledge in context:

```
You're editing: src/payments/stripe.ts

Relevant knowledge:
┌─────────────────────────────────────────┐
│ Stripe Payment Processing               │
│ - Amounts are in cents, not dollars     │
│ - Require X-Idempotency-Key header      │
│ [View full entry]                       │
└─────────────────────────────────────────┘
```

### Versioning

Knowledge evolves:

```bash
edda history entry_abc123
```

```
Version History:

v3 (current) - 2024-01-15
  Updated idempotency key TTL to 24 hours

v2 - 2024-01-10
  Added webhook signature verification

v1 - 2024-01-05
  Initial entry: amounts in cents
```

### Team Visibility

```bash
edda stats
```

```
Edda Knowledge Base

Entries: 142
Categories: 12
Contributors: 8

Top categories:
  - integrations (34 entries)
  - architecture (28 entries)
  - debugging (21 entries)

Recent additions:
  - Stripe Payment Processing (today)
  - Redis Cache Patterns (3 days ago)
```

## Integration Points

### MCP Server

AI assistants query Edda:

```json
{
  "tool": "edda_search",
  "arguments": {
    "query": "payment processing"
  }
}
```

### IDE Extension

Edda entries appear in editors as hover info and quick documentation.

### CI/CD

Validate code against Edda knowledge:

```yaml
- name: Check against knowledge base
  run: edda validate src/payments/
```

## The Name

**Edda** (Old Norse: [ˈedːɑ]) refers to the Prose Edda and Poetic Edda—medieval Icelandic texts that preserved Norse mythology and wisdom.

Like those texts, this Edda preserves knowledge that would otherwise be lost.

---

**Next:** [Design principles →](/docs/edda-stack/design-principles)
