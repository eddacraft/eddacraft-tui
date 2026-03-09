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
├── ARCHITECTURE.md         # System architecture overview
├── MONOREPO_STRUCTURE.md   # Repository layout
├── TESTING.md              # Test strategy and practices
├── architecture/           # Architecture deep-dives
├── guides/                 # Developer how-to guides
├── runbooks/               # Operational playbooks
├── specs/                  # Technical specifications
├── reviews/                # Point-in-time adversarial code reviews
├── strategy/               # Adoption candidates and competitor tracking
├── vision/                 # North star documents
├── public/                 # Public-facing documentation (off-limits here)
└── archive/                # Historical documents (read-only reference)
```

## Key Documents

### Architecture

- [ARCHITECTURE.md](ARCHITECTURE.md) — System architecture overview
- [architecture/edda-stack.md](architecture/edda-stack.md) — Edda stack design
- [architecture/rust-kernel-spec.md](architecture/rust-kernel-spec.md) — Rust
  kernel specification
- [architecture/rust-architecture-overview.md](architecture/rust-architecture-overview.md)
  — Rust architecture overview

### Development

- [TESTING.md](TESTING.md) — Test strategy and practices
- [guides/](guides/) — Developer how-to guides
- [runbooks/](runbooks/) — Operational playbooks

### Specifications

- [specs/](specs/) — Technical specifications (command safety, Edda contracts,
  enforcement hooks, authority/trust)

### Reviews

- [reviews/](reviews/) — Adversarial code reviews (point-in-time snapshots)

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

These are **read-only reference** — do not update archived documents.
