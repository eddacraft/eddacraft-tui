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
├── assessments/            # Migration and technology assessments
├── guides/                 # Internal development guides + runbooks
├── planning/               # Planning analyses
├── plans/                  # Dated planning documents
├── public/                 # Public-facing documentation (anvil, aps, edda-stack, kindling)
├── research/               # Research notes
├── specifications/         # Technical specifications
├── specs/                  # Detailed specifications (edda)
├── vision/                 # Vision documents
└── archive/                # Historical documents (read-only reference)
```

## Key Documents

### Architecture

- [ARCHITECTURE.md](ARCHITECTURE.md) - System architecture overview
- [architecture/edda-stack.md](architecture/edda-stack.md) - Edda stack design

### Development

- [TESTING.md](TESTING.md) - Test strategy and practices
- [guides/](guides/) - Development guides
- [guides/runbooks/README.md](guides/runbooks/README.md) - Operations runbooks

### Specifications

- [specifications/](specifications/) - Technical specifications

## For Planning Documents

All planning is now in the root `plans/` directory:

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
- `docs/public/start-here/` — Onboarding and glossary

## Archive

The `archive/` directory contains historical documents preserved for reference:

- Past planning documents
- Superseded designs
- Historical status reports

These are **read-only reference** - do not update archived documents.
