---
id: ember
title: Ember (Candidate)
description: The candidate layer of the Edda Stack.
sidebar_position: 2
---

# Ember (Candidate Layer)

:::note Coming Soon Ember is in active development. This page describes the
planned functionality. :::

Ember is the middle layer—where observations are reviewed and refined before
becoming verified knowledge.

## Role in the Stack

```
Edda Stack
│
├── Edda (Curated)      ← Verified knowledge
│       ▲
├── Ember (Candidate)   ← Promoted observations [PLANNED]
│       ▲
└── Kindling (Capture)  ← Raw observations
```

## Planned Functionality

### Promotion from Kindling

Observations move from Kindling to Ember:

```bash
# Manual promotion
ember promote obs_abc123

# Bulk review
ember review --capsule my-project
```

### Review Interface

```
┌─────────────────────────────────────────────────┐
│ Ember Review: my-project                        │
├─────────────────────────────────────────────────┤
│                                                 │
│ Pending Review (5)                              │
│                                                 │
│ [ ] "API uses OAuth2 with PKCE flow"           │
│     Captured: 2024-01-15 | Source: manual      │
│     [Promote] [Edit] [Dismiss]                 │
│                                                 │
│ [ ] "Rate limit is 100 req/min"                │
│     Captured: 2024-01-15 | Source: manual      │
│     [Promote] [Edit] [Dismiss]                 │
│                                                 │
└─────────────────────────────────────────────────┘
```

### Automatic Suggestions

Ember will suggest promotions based on:

- **Frequency** — Observations referenced multiple times
- **Quality** — Well-structured, tagged observations
- **Recency** — Recent observations for active projects
- **Signals** — Star ratings, explicit flags

### Team Sharing

```bash
# Share candidate with team
ember share entry_abc123 --team platform

# Request review
ember review-request entry_abc123 --reviewer @alice
```

### Deduplication

```
Potential duplicate detected:

Existing: "API rate limit is 100 requests per minute"
New: "Rate limit: 100 req/min"

Actions:
- [Merge] Combine into one entry
- [Keep Both] Different contexts
- [Replace] Use new version
```

## Planned API

```typescript
interface EmberEntry {
  id: string;
  content: string;
  status: 'pending' | 'approved' | 'rejected';
  sources: KindlingObservation[];
  reviewers: string[];
  promotedAt?: Date;
  editedContent?: string;
}

// Promote from Kindling
await ember.promote(observationId);

// Review
await ember.approve(entryId);
await ember.reject(entryId, { reason: 'Duplicate of...' });

// Edit during review
await ember.edit(entryId, { content: 'Improved version...' });
```

## Timeline

Ember development is planned to begin after Kindling stabilisation.

Expected capabilities:

- Phase 1: Manual promotion and review
- Phase 2: Automatic suggestions
- Phase 3: Team workflows
- Phase 4: AI-assisted review

---

**Next:** [Edda (Curated Layer) →](/docs/edda-stack/components/edda)
