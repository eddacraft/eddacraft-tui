# Documentation Migration Guide

**Date:** 2025-10-23

## What Changed

Documentation has been reorganized into a more consistent structure. This guide
helps you find documents in their new locations.

## File Moves

### Status & Progress Documents

| Old Location                       | New Location                                            |
| ---------------------------------- | ------------------------------------------------------- |
| `docs/CLI_INTEGRATION_COMPLETE.md` | `docs/status/cli-integration-complete.md`               |
| `docs/NEXT_STEPS.md`               | `docs/status/next-steps.md`                             |
| `docs/CLI_INTEGRATION_STATUS.md`   | `docs/archive/cli-integration-status-old.md` (archived) |
| `wip-planning-todo.md`             | Will merge into `docs/planning/tasks.md`                |

### Planning Documents

| Old Location                          | New Location                                              |
| ------------------------------------- | --------------------------------------------------------- |
| `PLAN.md`                             | `docs/planning/strategic-plan.md` (copied, original kept) |
| `TODO.md`                             | `docs/planning/tasks.md` (copied, original kept)          |
| `docs/ADAPTER_IMPLEMENTATION_PLAN.md` | `docs/planning/implementation-plan.md` (copied)           |

### Guide Documents

| Old Location                                  | New Location                             |
| --------------------------------------------- | ---------------------------------------- |
| `docs/GIT_WORKTREE_WORKFLOW.md`               | `docs/guides/git-worktree-workflow.md`   |
| `packages/adapters/ADAPTER_WORKFLOW_GUIDE.md` | `docs/guides/adapters/workflow-guide.md` |

### Archive Documents

| Old Location                         | New Location                              |
| ------------------------------------ | ----------------------------------------- |
| `archive/INTEGRATION_GAPS.md`        | `docs/archive/integration-gaps.md`        |
| `archive/RFC_SPEC_TOOLS_ADOPTION.md` | `docs/archive/rfc-spec-tools-adoption.md` |

## Unchanged Locations

These documents remain in their original locations:

### Root Level (Strategic)

- `README.md` - Project overview
- `ARCHITECTURE.md` - System architecture
- `ROADMAP.md` - Project roadmap
- `CLAUDE.md` - Claude Code instructions
- `PLAN.md` - Original strategic plan (also copied to docs/planning/)
- `TODO.md` - Original TODO (also copied to docs/planning/)

### Package Documentation

- `core/API.md`
- `core/EXAMPLES.md`
- `core/MIGRATION.md`
- `packages/adapters/README.md`

### Test Fixtures

All test fixtures remain in their original locations (they're functional, not
documentation).

## New Structure

```
docs/
├── INDEX.md                     # Comprehensive documentation index
├── status/                      # Current status & progress
│   ├── README.md
│   ├── cli-integration-complete.md
│   └── next-steps.md
├── planning/                    # Strategic & implementation plans
│   ├── README.md
│   ├── strategic-plan.md
│   ├── implementation-plan.md
│   └── tasks.md
├── guides/                      # How-to guides
│   ├── README.md
│   ├── adapters/
│   │   ├── README.md
│   │   └── workflow-guide.md
│   └── git-worktree-workflow.md
├── prd/                         # Product requirements
│   └── cli-speckit-integration.md
├── adr/                         # Architecture decisions
│   ├── README.md
│   └── 0001-use-zod-for-aps-schema.md
├── formats/                     # Format specifications
│   ├── speckit-templates.md
│   └── bmad-templates.md
└── archive/                     # Historical documents
    ├── README.md
    └── ...
```

## Finding Documents

### Use the Index

Start at [docs/INDEX.md](INDEX.md) - it has links to everything.

### Browse by Category

- **Current status?** → [docs/status/](status/README.md)
- **Planning info?** → [docs/planning/](planning/README.md)
- **How to build something?** → [docs/guides/](guides/README.md)
- **Product requirements?** → [docs/prd/](prd/)
- **Architecture decisions?** → [docs/adr/](adr/README.md)
- **Format specs?** → [docs/formats/](formats/)
- **Old documents?** → [docs/archive/](archive/README.md)

### Search in IDE

All documentation is still searchable - just use your IDE's search function.

## Backward Compatibility

### Old Links

For the next few weeks, the old root-level files (PLAN.md, TODO.md) will remain
so existing links don't break. Eventually these will be replaced with symlinks
or redirects.

### In Code

No code changes needed - all file paths in code are correct.

### In CLAUDE.md

CLAUDE.md has been updated to reference new paths.

## Benefits of New Structure

1. **Better Organization** - Documents grouped by purpose
2. **Easier Navigation** - Clear hierarchy and indexes
3. **Less Clutter** - Root directory focused on strategic docs
4. **Better Onboarding** - New developers can find what they need
5. **Maintainability** - Easier to keep documentation current

## Questions?

See [docs/INDEX.md](INDEX.md) for the full documentation index.
