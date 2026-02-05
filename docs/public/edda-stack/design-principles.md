---
id: design-principles
title: Design Principles
description: Core principles guiding the Edda Stack design.
sidebar_position: 2
---

# Design Principles

The principles that guide Edda Stack design.

## 1. Provenance Matters

Every piece of knowledge traces to its origin.

### Why

- **Verification** — Know where knowledge came from
- **Attribution** — Credit contributors
- **Context** — Understand when/why it was captured
- **Updates** — Find related observations when knowledge changes

### How

```
Edda Entry: "Stripe uses cents"
├── Verified by: alice@example.com (2024-01-15)
├── Source: Ember entry ember_abc123
│   └── Source: Kindling observation obs_xyz789
│       ├── Captured: 2024-01-14T10:30:00Z
│       ├── Actor: bob@example.com
│       └── Context: src/payments/checkout.ts
```

## 2. Progressive Refinement

Raw → Candidate → Verified

### Why

- **Signal vs Noise** — Not everything deserves curation
- **Quality** — Each layer adds verification
- **Scalability** — Low friction capture, high quality output
- **Natural Selection** — Valuable knowledge surfaces naturally

### How

```
100 observations captured (Kindling)
    ↓ Promotion
 20 candidates reviewed (Ember)
    ↓ Verification
  5 entries curated (Edda)
```

## 3. Human in the Loop

AI assists; humans verify.

### Why

- **Accuracy** — AI can be wrong
- **Nuance** — Context matters
- **Trust** — Team trusts human-verified knowledge
- **Learning** — Humans learn through review

### How

AI suggests:

- "This observation might be worth promoting"
- "This seems similar to existing entry X"
- "Consider adding to category Y"

Humans decide:

- Approve/reject promotions
- Verify accuracy
- Edit and improve

## 4. Mechanical First

Exact retrieval before semantic search.

### Why

- **Predictability** — Find what you stored
- **Debuggability** — Understand why results appear
- **Speed** — Simple search is fast
- **Foundation** — Semantic optional, not required

### How

```bash
# Mechanical: exact match
kindling search "OAuth2 PKCE"
→ Returns observations containing "OAuth2 PKCE"

# Semantic (future): understanding
kindling search --semantic "authentication flow for SPAs"
→ Returns observations about OAuth2, PKCE, token handling...
```

## 5. Local-First

Data lives on your machine by default.

### Why

- **Privacy** — Your observations stay yours
- **Speed** — No network latency
- **Reliability** — Works offline
- **Control** — You own your data

### How

```
~/.kindling/
├── capsules/
│   └── my-project.db    # Your data, your machine
└── config.json
```

Sync and sharing are opt-in, not default.

## 6. Minimal Friction

Capture should be instant.

### Why

- **Timing** — Context is freshest immediately
- **Adoption** — Tools that interrupt get abandoned
- **Volume** — More capture = more knowledge

### How

```bash
# One command, done
kindling observe "The API rate limits at 100/min"

# Or even faster
/m "The API rate limits at 100/min"
```

## 7. Composable

Each layer works independently.

### Why

- **Adoption** — Start with what you need
- **Flexibility** — Mix with other tools
- **Testing** — Verify each layer
- **Evolution** — Upgrade independently

### How

Kindling works without Ember or Edda:

```bash
kindling observe "..."
kindling search "..."
kindling export
```

Add Ember later. Add Edda later. Or don't.

## 8. Team Scale

Individual → Team → Organisation

### Why

- **Solo value** — Useful for one person
- **Team value** — Multiplied when shared
- **Org value** — Compound knowledge asset

### How

```
Developer captures → Kindling (personal)
    ↓
Team promotes → Ember (team-visible)
    ↓
Org verifies → Edda (org-wide)
```

---

**Back to:** [Edda Stack Overview →](/docs/edda-stack/overview)
