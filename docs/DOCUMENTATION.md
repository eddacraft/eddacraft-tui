# Documentation Overview

**Last Updated:** 2025-10-23

> **Quick Start:** See [docs/INDEX.md](docs/INDEX.md) for comprehensive
> documentation index

## Documentation Structure

Anvil documentation is organized into clear categories:

### 📖 Strategic Documents (Root Level)

Core documents that define the project:

- **[README.md](README.md)** - Project overview, quickstart, contributing
- **[ARCHITECTURE.md](ARCHITECTURE.md)** - System design and architecture
- **[ROADMAP.md](ROADMAP.md)** - Project milestones and timeline
- **[CLAUDE.md](CLAUDE.md)** - Development guide for Claude Code

### 📂 Detailed Documentation (docs/)

#### Current Status & Progress

**Location:** [docs/status/](docs/status/)

What's done, what's in progress, what's next.

- [CLI Integration Complete](docs/status/cli-integration-complete.md) - Latest
  completion report
- [Next Steps](docs/status/next-steps.md) - Upcoming work

#### Planning & Strategy

**Location:** [docs/planning/](docs/planning/)

Strategic plans, implementation roadmaps, task tracking.

- [Strategic Plan](docs/planning/strategic-plan.md) - Three-act vision
- [Implementation Plan](docs/planning/implementation-plan.md) - Technical
  roadmap
- [Active Tasks](docs/planning/tasks.md) - Current TODO list

#### Development Guides

**Location:** [docs/guides/](docs/guides/)

How-to guides for developers.

- [Adapter Development](docs/guides/adapters/README.md) - Building format
  adapters
- [Git Worktree Workflow](docs/guides/git-worktree-workflow.md) - Branch
  management

#### Product Requirements

**Location:** [docs/prd/](docs/prd/)

Product specifications and requirements.

- [CLI-SpecKit Integration](docs/prd/cli-speckit-integration.md)

#### Architecture Decisions

**Location:** [docs/adr/](docs/adr/)

Records of significant architectural decisions.

- [ADR-0001: Use Zod for APS Schema](docs/adr/0001-use-zod-for-aps-schema-definition.md)

#### Format Specifications

**Location:** [docs/formats/](docs/formats/)

Templates and specs for supported formats.

- [SpecKit Templates](docs/formats/speckit-templates.md)
- [BMAD Templates](docs/formats/bmad-templates.md)

### 📦 Package Documentation

Each package has its own documentation:

- **Core:** [core/API.md](core/API.md), [core/EXAMPLES.md](core/EXAMPLES.md)
- **Adapters:** [packages/adapters/README.md](packages/adapters/README.md)

## Quick Links by Role

### New Contributors

1. [README.md](README.md) - Start here
2. [ARCHITECTURE.md](ARCHITECTURE.md) - Understand the system
3. [Development Guides](docs/guides/README.md) - Learn how to contribute

### Developers

1. [CLAUDE.md](CLAUDE.md) - AI development workflow
2. [Adapter Development](docs/guides/adapters/README.md) - Build adapters
3. [Active Tasks](docs/planning/tasks.md) - See what needs doing

### Project Managers

1. [Strategic Plan](docs/planning/strategic-plan.md) - Overall vision
2. [Roadmap](ROADMAP.md) - Timeline and milestones
3. [Current Status](docs/status/README.md) - Progress tracking

### Architects

1. [ARCHITECTURE.md](ARCHITECTURE.md) - System design
2. [ADR Index](docs/adr/README.md) - Decision records
3. [Implementation Plan](docs/planning/implementation-plan.md) - Technical
   details

## Recently Updated

- **2025-10-23:** Documentation reorganization completed
  - Created comprehensive index
  - Organized into clear categories
  - Added navigation guides

- **2025-10-23:** CLI-SpecKit Integration Complete
  - All tests passing (152/152)
  - Export command implemented
  - Full completion report published

## Navigation

- **📚 [Full Documentation Index](docs/INDEX.md)** - Complete listing of all
  docs
- **📁 [Documentation Migration Guide](docs/MIGRATION_GUIDE.md)** - What moved
  where
- **🗂️ [Archive](docs/archive/README.md)** - Historical documents

## Contributing to Documentation

### Adding New Documentation

1. Determine appropriate category (status/planning/guides/etc.)
2. Follow naming conventions (kebab-case, descriptive)
3. Include metadata header (date, status, related docs)
4. Update [docs/INDEX.md](docs/INDEX.md)
5. Link from related documents

### Updating Documentation

1. Update "Last Updated" date
2. If significantly changed, archive old version
3. Update links if path changes
4. Keep changelog current

### Documentation Style

- Use Markdown consistently
- Include code examples
- Add diagrams for complex concepts
- Link to related documents
- Keep language clear and concise

## Questions?

- Check [docs/INDEX.md](docs/INDEX.md) first
- Search documentation with IDE search
- See package-specific README files
- Review [ARCHITECTURE.md](ARCHITECTURE.md) for design context

---

**For AI Assistants:** See [CLAUDE.md](CLAUDE.md) for development instructions
and workflows.
