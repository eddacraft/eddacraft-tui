# Anvil Documentation Index

**Last Updated:** 2025-10-23

## Quick Links

- 🏠 [Project README](../README.md) - Start here
- 🏗️ [Architecture](../ARCHITECTURE.md) - System design
- 🗺️ [Roadmap](../ROADMAP.md) - Project milestones
- 🤖 [Claude Code Instructions](../CLAUDE.md) - Development guide for AI

## Documentation Structure

### 📊 Status & Progress

Current state of the project, what's done, what's next.

- [**Current Status**](status/README.md) - Overview of project state
- [CLI Integration Complete](status/cli-integration-complete.md) - SpecKit
  integration completion report
- [Next Steps](status/next-steps.md) - Upcoming work: adapter interface
  migration
- [Changelog](status/changelog.md) - Project changes over time

### 📋 Planning

Strategic plans, implementation plans, and task tracking.

- [**Planning Overview**](planning/README.md)
- [Strategic Plan](planning/strategic-plan.md) - Three-act vision (from PLAN.md)
- [Implementation Plan](planning/implementation-plan.md) - Adapter
  implementation roadmap
- [Active Tasks](planning/tasks.md) - Current TODO list

### 📚 Guides

How-to guides for developers working on Anvil.

- [**Development Guides**](guides/README.md)
- [Adapter Development](guides/adapters/README.md) - Building format adapters
  - [Adapter Workflow Guide](guides/adapters/workflow-guide.md)
- [Git Worktree Workflow](guides/git-worktree-workflow.md) - Branch management

### 📄 Product Requirements (PRD)

Product specifications and requirements documents.

- [CLI-SpecKit Integration](prd/cli-speckit-integration.md) - CLI integration
  with SpecKit adapter

### 🏛️ Architecture Decision Records (ADR)

Records of significant architectural decisions.

- [ADR Index](adr/README.md)
- [ADR-0001: Use Zod for APS Schema](adr/0001-use-zod-for-aps-schema.md)

### 📐 Format Specifications

Templates and specifications for supported planning formats.

- [SpecKit Templates](formats/speckit-templates.md) - GitHub SpecKit format
- [BMAD Templates](formats/bmad-templates.md) - BMAD format (planned)

### 📦 Package Documentation

Package-specific API and usage documentation.

#### Core Package (@anvil/core)

- [Core README](../core/README.md)
- [API Documentation](../core/API.md)
- [Examples](../core/EXAMPLES.md)
- [Migration Guide](../core/MIGRATION.md)
- [Validation README](../core/src/validation/README.md)

#### Adapters Package (@anvil/adapters)

- [Adapters README](../packages/adapters/README.md)
- [Adapter Workflow Guide](../packages/adapters/ADAPTER_WORKFLOW_GUIDE.md)

### 🗄️ Archive

Historical documents, old plans, and superseded documentation.

- [Archive Index](archive/README.md)
- [Integration Gaps Analysis](archive/integration-gaps.md) - Historical analysis
- [RFC: Spec Tools Adoption](archive/rfc-spec-tools-adoption.md) - Initial RFC
- [Old Plans](archive/old-plans/) - Superseded planning documents
- [Old TODOs](archive/old-todo/) - Historical task lists

## Document Status Legend

- 🟢 **Active** - Current, maintained documentation
- 🟡 **Draft** - Work in progress
- 🔵 **Reference** - Stable reference material
- ⚪ **Archive** - Historical, superseded by newer docs

## Navigation Tips

### For New Contributors

1. Start with [README](../README.md)
2. Read [Architecture](../ARCHITECTURE.md)
3. Check [Current Status](status/README.md)
4. Review [Development Guides](guides/README.md)

### For Development

1. [CLAUDE.md](../CLAUDE.md) - AI development instructions
2. [Adapter Workflow](guides/adapters/workflow-guide.md) - Building adapters
3. [Git Workflow](guides/git-worktree-workflow.md) - Branch management

### For Project Planning

1. [Strategic Plan](planning/strategic-plan.md) - Overall vision
2. [Roadmap](../ROADMAP.md) - Milestones and timeline
3. [Active Tasks](planning/tasks.md) - Current work items
4. [Next Steps](status/next-steps.md) - Upcoming work

### For Understanding Decisions

1. [Architecture](../ARCHITECTURE.md) - System design rationale
2. [ADR Index](adr/README.md) - Decision records
3. [PRD Documents](prd/) - Product requirements

## Contributing to Documentation

### Adding New Documentation

1. Determine the appropriate category (status/planning/guides/etc.)
2. Follow naming conventions (kebab-case, descriptive)
3. Include metadata header (date, status, related docs)
4. Update this INDEX.md
5. Link from relevant documents

### Updating Existing Documentation

1. Update the "Last Updated" date
2. If significantly changed, move old version to archive/
3. Update links if file path changes
4. Keep CHANGELOG.md current

### Documentation Style Guide

- Use Markdown formatting consistently
- Include code examples where helpful
- Add diagrams for complex concepts
- Link to related documents
- Keep language clear and concise

## Documentation Maintenance

### Weekly Review

- Update [Current Status](status/README.md)
- Update [Active Tasks](planning/tasks.md)
- Archive completed work

### Monthly Review

- Update [Changelog](status/changelog.md)
- Review and archive old documentation
- Check for broken links
- Update roadmap progress

### Release Review

- Update all status documents
- Archive superseded plans
- Update API documentation
- Review examples for accuracy

---

**Questions or suggestions?** Open an issue or update this index directly.
