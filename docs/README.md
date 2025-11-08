# Anvil Documentation

**Last Updated:** 2025-10-23

Welcome to the Anvil documentation! This directory contains all project
documentation organised by category.

## 📖 Quick Navigation

- **[📑 Complete Documentation Index](INDEX.md)** - Comprehensive listing of all
  documents
- **[🗺️ Migration Guide](MIGRATION_GUIDE.md)** - Find documents in their new
  locations
- **[📁 Documentation Organisation Plan](DOC_ORGANISATION.md)** - How docs are
  structured

## 📂 Documentation Categories

### [status/](status/) - Current Status & Progress

What's done, what's in progress, what's next.

- [Current Status Overview](status/README.md)
- [CLI Integration Complete Report](status/cli-integration-complete.md)
- [Next Steps: Interface Migration](status/next-steps.md)

### [planning/](planning/) - Planning & Strategy

Strategic plans, implementation roadmaps, task tracking.

- [Planning Overview](planning/README.md)
- [Strategic Plan (Three Acts)](planning/strategic-plan.md)
- [Implementation Plan](planning/implementation-plan.md)
- [Active Tasks](planning/tasks.md)

### [guides/](guides/) - Development Guides

How-to guides for developers.

- [Development Guides Index](guides/README.md)
- [Adapter Development Guide](guides/adapters/README.md)
- [Git Worktree Workflow](guides/git-worktree-workflow.md)

### [prd/](prd/) - Product Requirements

Product specifications and requirements documents.

- [CLI-SpecKit Integration PRD](prd/cli-speckit-integration.md)

### [adr/](adr/) - Architecture Decision Records

Records of significant architectural decisions.

- [ADR Index](adr/README.md)
- [ADR-0001: Use Zod for APS Schema](adr/0001-use-zod-for-aps-schema-definition.md)

### [formats/](formats/) - Format Specifications

Templates and specifications for supported formats.

- [SpecKit Templates](formats/speckit-templates.md)
- [BMAD Templates](formats/bmad-templates.md)

### [archive/](archive/) - Historical Documents

Superseded plans, old TODOs, historical analysis.

- [Archive Index](archive/README.md)

## 🚀 Getting Started

### For New Contributors

1. Read [Project README](../README.md)
2. Review [Architecture](../ARCHITECTURE.md)
3. Check [Current Status](status/README.md)
4. Explore [Development Guides](guides/README.md)

### For Developers

1. See [CLAUDE.md](../CLAUDE.md) for AI workflow
2. Review [Adapter Development Guide](guides/adapters/README.md)
3. Check [Active Tasks](planning/tasks.md)

### For Project Managers

1. Read [Strategic Plan](planning/strategic-plan.md)
2. Check [Roadmap](../ROADMAP.md)
3. Review [Current Status](status/README.md)

## 📊 Documentation Status

- **Total documents:** 40+ organised files
- **Categories:** 7 main categories
- **Last reorganisation:** 2025-10-23
- **Coverage:** Strategic, technical, and operational documentation

## 🔍 Finding What You Need

1. **Start with [INDEX.md](INDEX.md)** - Complete documentation listing
2. **Browse by category** - Use directories above
3. **Search in IDE** - All docs are searchable
4. **Check package READMEs** - For package-specific info

## 📝 Contributing to Documentation

### Adding New Docs

1. Place in appropriate category directory
2. Follow naming conventions (kebab-case)
3. Include metadata header
4. Update [INDEX.md](INDEX.md)

### Updating Existing Docs

1. Update "Last Updated" date
2. Archive old versions if major changes
3. Update links if paths change

### Style Guide

- Use Markdown formatting
- Include code examples
- Link to related documents
- Keep language clear and concise

## 🗂️ Directory Structure

```
docs/
├── INDEX.md                 # Comprehensive index
├── README.md               # This file
├── MIGRATION_GUIDE.md      # File moves reference
├── DOC_ORGANISATION.md     # Organisation plan
│
├── status/                 # Current status
├── planning/               # Plans & tasks
├── guides/                 # How-to guides
├── prd/                    # Requirements
├── adr/                    # Decision records
├── formats/                # Format specs
└── archive/                # Historical docs
```

## 💡 Tips

- **Bookmark [INDEX.md](INDEX.md)** - Fastest way to find anything
- **Check README files** - Each directory has a README
- **Use search** - IDE search works across all docs
- **Follow links** - Documents cross-reference each other

---

**Questions?** See [INDEX.md](INDEX.md) or check package-specific documentation.
