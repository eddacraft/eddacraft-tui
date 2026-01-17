---
id: overview
title: Edda Stack Overview
description: The layered architecture for governed development memory.
sidebar_position: 1
---

# Edda Stack

The Edda Stack is EddaCraft's architecture for capturing, promoting, and
curating development knowledge.

## The Vision

Development generates valuable knowledge:

- Decisions made and why
- Problems solved and how
- Patterns discovered and when to use them

But this knowledge is lost. Chat histories clear. Memory fades. The same lessons
get relearned.

The Edda Stack preserves and promotes knowledge systematically.

## The Layers

```
┌─────────────────────────────────────────────────┐
│                     Edda                         │
│            Curated, verified knowledge           │
│                  (Coming Soon)                   │
└─────────────────────┬───────────────────────────┘
                      │ Promotion
┌─────────────────────▼───────────────────────────┐
│                    Ember                         │
│         Candidate knowledge awaiting review      │
│                  (Coming Soon)                   │
└─────────────────────┬───────────────────────────┘
                      │ Capture
┌─────────────────────▼───────────────────────────┐
│                   Kindling                       │
│           Raw observations from sessions         │
│                  (Available Now)                 │
└─────────────────────────────────────────────────┘
```

### Kindling (Capture)

**Status: Available Now**

Raw observations captured during development:

```bash
kindling observe "The API requires idempotency keys for POST requests"
```

- Fast capture, minimal friction
- Structured storage
- Local-first, portable

[Kindling Documentation →](/docs/kindling/overview)

### Ember (Candidate)

**Status: Planned**

Observations flagged as potentially valuable:

```
Observation: "API requires idempotency keys"
→ Promoted to Ember (high utility, verified accurate)
→ Tagged for team review
```

Ember will provide:

- Automatic promotion suggestions
- Human review workflow
- Team sharing
- Deduplication

### Edda (Curated)

**Status: Planned**

Verified, reusable knowledge:

```
Entry: "Idempotency Keys"
- All POST requests to /payments require X-Idempotency-Key header
- Keys should be UUIDs, unique per logical operation
- Server returns cached response for duplicate keys
- Source: API documentation + team experience
```

Edda will provide:

- Verified accuracy
- Team-wide visibility
- Search and discovery
- Integration with development tools

## The Flow

### 1. Capture (Kindling)

During a session, observations are captured:

```bash
kindling observe "Stripe uses cents not dollars for amounts"
kindling observe "Webhook signature must use raw body"
kindling observe "Test card: 4242424242424242"
```

### 2. Promote (Ember)

After the session, valuable observations are promoted:

```
Review session observations:

[✓] "Stripe uses cents not dollars" → Promote to Ember
[✓] "Webhook signature uses raw body" → Promote to Ember
[ ] "Test card: 4242..." → Keep in Kindling (reference only)
```

### 3. Curate (Edda)

Team reviews and curates:

```
New Ember entry: "Stripe uses cents not dollars"

Review options:
- Verify and promote to Edda
- Edit and improve
- Merge with existing entry
- Reject (inaccurate or duplicate)
```

### 4. Apply

Curated knowledge is surfaced:

```
You're editing: src/payments/checkout.ts

Relevant Edda entries:
- "Stripe amounts are in cents, not dollars"
- "Always verify webhook signatures"
```

## What Exists Today

| Component    | Status    | Description                     |
| ------------ | --------- | ------------------------------- |
| **Kindling** | Available | Observation capture and storage |
| **Ember**    | Planned   | Promotion and review workflow   |
| **Edda**     | Planned   | Curated knowledge base          |

Kindling is fully functional today. Start capturing observations and benefit
from the stack as it grows.

## Why This Architecture?

### Progressive Refinement

Raw captures → Candidates → Verified knowledge

Not everything deserves curation. The stack filters naturally.

### Human in the Loop

AI can suggest, but humans verify. Critical for accuracy.

### Provenance Matters

Every piece of knowledge traces to its origin. Who observed it, when, how it was
verified.

### Team Scale

Individual observations → Team candidates → Organisation knowledge

---

**Next:** [Component details →](/docs/edda-stack/components/kindling)
