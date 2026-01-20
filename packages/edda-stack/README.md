# Edda Stack

> **Status:** Draft — Planning complete for v2.0

The Kindling · Ember · Edda Stack — a three-layer architecture that governs how
activity becomes memory.

## Philosophy

Not all activity deserves to be remembered. Not all memory should be automatic.
Not all meaning is obvious in the moment.

The stack exists to move carefully from signal to sense to record.

```
Kindling observes — captures without judgement
Ember reflects — meaning without authority
Edda remembers — memory with restraint
```

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                        Edda Stack                           │
│                   (Memory Architecture)                     │
└─────────────────────────────────────────────────────────────┘

                    ┌──────────────┐
                    │    Edda      │  ← Canonical memory (high-trust)
                    │  (Ledger)    │     Git-backed, versioned, auditable
                    └──────┬───────┘
                           │ Promotion (human decision)
                    ┌──────▼───────┐
                    │    Ember     │  ← Candidate memory (medium-trust)
                    │   (Queue)    │     SQLite, ephemeral, decays
                    └──────┬───────┘
                           │ Aggregation + Evaluation
                    ┌──────▼───────┐
                    │   Kindling   │  ← Observations (facts only)
                    │  (Camera)    │     Write-only, read-only query
                    └──────────────┘
```

## Layers

### Kindling — Capture without judgement

Kindling is the sensory layer. It observes what happens and records it
faithfully, without interpretation.

**Kindling's job:** "What occurred?"

- Captures: actions, tool usage, agent behaviour, communications, errors
- **Crucially does not decide:** what matters, what is reusable, what to keep
- Deliberately naïve — sees everything, understands nothing
- This makes it trustworthy

**Technical character:**

- Write-only emission (11 observation kinds)
- Read-only bounded queries (4 scopes)
- SQLite local storage
- No AI, no inference

### Ember — Meaning without authority

Ember is the interpretive layer. It sits between raw observation and durable
memory, looking for candidate meaning.

**Ember's job:** "Might this matter later?"

- Evaluates patterns: repetition, deviation, resolution, escalation
- Proposes memory, does not create it
- Outputs are suggestions, not facts
- Has taste, but no power

**Technical character:**

- Write-heavy, ephemeral by default
- Allowed to be wrong (probabilistic thinking)
- May use AI but must not depend on it
- Breaking Ember data is acceptable

### Edda — Memory with restraint

Edda is the canonical memory layer. It stores only what has crossed a deliberate
threshold.

**Edda's job:** "What do we know to be true enough to keep?"

- Curated, stable, referable, slow to change
- Not a log, not a transcript, not a database of everything
- An institutional memory: decisions, lessons, patterns, constraints, truths
- Forgets aggressively by default (feature, not bug)

**Technical character:**

- Low-volume, high-trust
- Versioned, append-biased, auditable
- AI-assisted, not AI-authored
- Breaking Edda is unacceptable

## Structure

```
packages/edda-stack/
├── contracts/           # Shared type definitions
│   ├── identifiers.ts   # ID formats (ObservationId, ProposalId, MemoryId)
│   ├── temporal.ts      # Timestamp conventions (ISO8601)
│   ├── confidence.ts    # Confidence scales
│   ├── provenance-link.ts # Cross-layer references
│   ├── type-mappings.ts # Proposal → Memory conversion
│   └── events.ts        # Layer communication events
├── ports/               # Interface definitions
│   ├── kindling.port.ts
│   ├── ember.port.ts
│   └── edda.port.ts
├── ember/               # Candidate memory system
│   ├── candidate-service.ts
│   ├── proposal-store.ts
│   ├── aggregator-service.ts
│   ├── evaluator-service.ts
│   ├── decay-service.ts
│   └── rules/
├── edda/                # Canonical memory system
│   ├── memory-service.ts
│   ├── memory-store.ts
│   ├── promotion-service.ts
│   ├── provenance-service.ts
│   ├── evolution-service.ts
│   └── migration/
└── testing/             # Test utilities and fixtures
    ├── mocks/
    ├── fixtures/
    └── validators/
```

## Key Constraints

### Each layer is intentionally limited

- Kindling cannot judge
- Ember cannot decide
- Edda cannot speculate

### Meaning emerges only through their separation

Most systems collapse these concerns:

- logs become memory
- memory becomes noisy
- noise becomes institutional truth

This stack resists that failure mode.

### Promotion must be deliberate

- AI proposes, humans dispose
- Attribution is mandatory
- Provenance links are preserved

## Integration with Kindling

Kindling (the external package) provides the observation layer. This package
integrates with it via `@eddacraft/anvil-kindling-integration`.

See: `packages/kindling-integration/` for observation contracts and query API.

## Plans

Detailed implementation plans are in `plans/modules/`:

- `kindling-integration.aps.md` — Observation layer (19 tasks)
- `ember.aps.md` — Candidate memory (14 tasks)
- `edda.aps.md` — Canonical memory (19 tasks)
- `edda-stack-integration.aps.md` — Cross-cutting concerns (16 tasks)

## Mental Models

```
Ember is a queue — you empty it intentionally
Edda is a ledger — you never casually rewrite it

If you can't explain why something is in Edda, it doesn't belong.

Truth flows one way: Kindling → Ember → Edda
AI never writes back to Kindling
```

## One Sentence Each

- **Kindling:** Probabilistic, write-only observation system optimised for facts
- **Ember:** Probabilistic, decaying candidate-memory system optimised for
  pattern detection
- **Edda:** Deterministic, versioned knowledge store optimised for institutional
  truth
