# Internal Engineering Documentation

This directory contains **internal engineering documentation** for Anvil
developers.

## Documentation Locations

| Location               | Purpose                              |
| ---------------------- | ------------------------------------ |
| `docs/` (here)         | Internal engineering reference       |
| `plans/`               | **Source of truth** for all planning |
| `apps/docs-site/docs/` | Public-facing documentation          |

## Structure

```
docs/
├── ARCHITECTURE.md         # System architecture overview
├── MONOREPO_STRUCTURE.md   # Repository layout
├── TESTING.md              # Test strategy and practices
├── architecture/           # Architecture deep-dives
├── guides/                 # Internal development guides
├── specifications/         # Technical specifications
└── archive/                # Historical documents (read-only reference)
```

## Key Documents

### Architecture

- [ARCHITECTURE.md](ARCHITECTURE.md) - System architecture overview
- [architecture/edda-stack.md](architecture/edda-stack.md) - Edda stack design

### Development

- [TESTING.md](TESTING.md) - Test strategy and practices
- [guides/](guides/) - Development guides

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

User-facing documentation is in `apps/docs-site/docs/`:

- Quickstart guides
- User guides
- Troubleshooting
- API references

## Archive

The `archive/` directory contains historical documents preserved for reference:

- Past planning documents
- Superseded designs
- Historical status reports

These are **read-only reference** - do not update archived documents.
