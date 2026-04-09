# Internal Engineering Documentation

This directory contains **internal engineering documentation** for Anvil
developers.

## Documentation Locations

| Location       | Purpose                              |
| -------------- | ------------------------------------ |
| `docs/` (here) | Internal engineering reference       |
| `plans/`       | **Source of truth** for all planning |
| `docs/public/` | Public-facing documentation          |

## Structure

```
docs/
├── architecture/           # Architecture deep-dives (incl. overview, monorepo structure)
│   ├── overview.md         # System architecture overview
│   └── monorepo-structure.md # Repository layout
├── guides/                 # Developer how-to guides (incl. testing, branching strategy)
├── runbooks/               # Operational playbooks
├── specs/                  # Technical specifications
├── reviews/                # Active review work that still needs follow-up
├── strategy/               # Adoption candidates and competitor tracking
├── vision/                 # North star documents
├── public/                 # Public-facing documentation (off-limits here)
└── archive/                # Historical documents (read-only reference)
    └── reviews/            # Dated review snapshots kept for reference
```

## Key Documents

### Architecture

- [architecture/overview.md](architecture/overview.md) — System architecture
  overview
- [architecture/monorepo-structure.md](architecture/monorepo-structure.md) —
  Repository layout
- [architecture/edda-stack.md](architecture/edda-stack.md) — Edda stack design
- [architecture/rust-kernel-spec.md](architecture/rust-kernel-spec.md) — Rust
  kernel specification
- [architecture/rust-architecture-overview.md](architecture/rust-architecture-overview.md)
  — Rust architecture overview

### Development

- [guides/testing.md](guides/testing.md) — Test strategy and practices
- [guides/branching-strategy.md](guides/branching-strategy.md) — Branching
  strategy
- [guides/](guides/) — Developer how-to guides
- [runbooks/](runbooks/) — Operational playbooks

### Specifications

- [specs/](specs/) — Technical specifications (command safety, Edda contracts,
  enforcement hooks, authority/trust)

### Reviews

- [reviews/](reviews/) — Active review notes that still need follow-up
- [archive/reviews/](archive/reviews/) — Historical adversarial reviews kept as
  dated snapshots

### Strategy

- [strategy/borrow-adopt-candidates.md](strategy/borrow-adopt-candidates.md) —
  Technology adoption candidates
- [strategy/competitor-tier2-tracking.md](strategy/competitor-tier2-tracking.md)
  — Competitor watchlist

## For Planning Documents

All planning is in the root `plans/` directory:

```
plans/
├── index.aps.md            # Master plan index
├── decisions/              # Architecture Decision Records
├── modules/                # Feature plans (.aps.md)
└── execution/              # Step-by-step execution plans
```

## For Public Documentation

User-facing documentation is in `docs/public/`:

- `docs/public/anvil/` — Anvil quickstart, guides, operations, tutorials
- `docs/public/aps/` — APS specification, schemas, examples
- `docs/public/edda-stack/` — Edda stack components and design
- `docs/public/kindling/` — Kindling adapters, concepts, quickstart
- `docs/public/beta/` — Beta quickstart

## Archive

The `archive/` directory contains historical documents preserved for reference:

- Pre-implementation EDDA architecture (superseded by `packages/edda-stack/`)
- Completed decision docs (Deno migration, Kindling integration, storage choice)
- Research notes (Ratatui diagrams)
- Superseded guides and specs
- Historical engineering logs and dated review snapshots

These are **read-only reference** — do not update archived documents.

Point-in-time review writeups should move to `archive/reviews/` once their
follow-up work is merged or superseded.
