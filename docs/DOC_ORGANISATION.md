# Documentation Organisation Plan

**Date:** 2025-10-23 **Purpose:** Consolidate and organise documentation for
better discoverability

## Current State Analysis

### Documentation Scattered Across:

- Root directory (8 files)
- docs/ directory (5 files + subdirectories)
- docs/adr/ (1 file)
- docs/formats/ (2 files)
- docs/prd/ (1 file)
- docs/archive/ (8+ files)
- core/ (3 files)
- packages/adapters/ (2 files)
- .claude/ (40+ files)

### Issues:

1. **Status documents scattered**: CLI_INTEGRATION_STATUS.md,
   CLI_INTEGRATION_COMPLETE.md, wip-planning-todo.md at different levels
2. **Duplicate planning docs**: TODO.md vs wip-planning-todo.md
3. **Archive not well organised**: Mix of old plans and RFCs
4. **No clear document index**

## Proposed Organisation Structure

```
anvil-001/
├── README.md                          # Main entry point
├── ARCHITECTURE.md                    # System architecture
├── CLAUDE.md                          # Claude Code instructions
├── ROADMAP.md                         # High-level roadmap
│
├── docs/
│   ├── INDEX.md                       # Documentation index
│   │
│   ├── status/                        # Project status & progress
│   │   ├── README.md                  # Current status overview
│   │   ├── cli-integration-complete.md
│   │   ├── next-steps.md
│   │   └── changelog.md               # Project changelog
│   │
│   ├── guides/                        # Implementation guides
│   │   ├── adapters/
│   │   │   ├── README.md              # Adapter development guide
│   │   │   └── workflow-guide.md
│   │   ├── git-worktree-workflow.md
│   │   └── development-setup.md
│   │
│   ├── planning/                      # Planning documents
│   │   ├── README.md                  # Planning overview
│   │   ├── strategic-plan.md          # From PLAN.md
│   │   ├── implementation-plan.md     # From ADAPTER_IMPLEMENTATION_PLAN.md
│   │   └── tasks.md                   # Active TODO list
│   │
│   ├── prd/                           # Product requirements
│   │   ├── README.md
│   │   └── cli-speckit-integration.md
│   │
│   ├── adr/                           # Architecture decision records
│   │   ├── README.md
│   │   └── 0001-use-zod-for-aps-schema.md
│   │
│   ├── formats/                       # Format specifications
│   │   ├── README.md
│   │   ├── speckit-templates.md
│   │   └── bmad-templates.md
│   │
│   └── archive/                       # Historical documents
│       ├── README.md                  # Archive index
│       ├── integration-gaps.md
│       ├── rfc-spec-tools-adoption.md
│       └── old-plans/
│           └── ...
│
├── core/                              # Core package docs
│   ├── README.md
│   ├── API.md
│   ├── EXAMPLES.md
│   └── MIGRATION.md
│
└── packages/adapters/                 # Adapter package docs
    ├── README.md
    └── WORKFLOW_GUIDE.md
```

## Migration Actions

### Phase 1: Create New Structure (Immediate)

1. Create docs/INDEX.md with comprehensive index
2. Create docs/status/README.md
3. Create docs/guides/README.md
4. Create docs/planning/README.md
5. Create docs/archive/README.md

### Phase 2: Move/Consolidate Files

1. **Status documents** → docs/status/
   - CLI_INTEGRATION_COMPLETE.md → cli-integration-complete.md
   - CLI_INTEGRATION_STATUS.md → (merge into cli-integration-complete.md)
   - NEXT_STEPS.md → next-steps.md
   - wip-planning-todo.md → (merge into tasks.md)

2. **Planning documents** → docs/planning/
   - PLAN.md → strategic-plan.md
   - TODO.md → tasks.md
   - ADAPTER_IMPLEMENTATION_PLAN.md → implementation-plan.md

3. **Guides** → docs/guides/
   - GIT_WORKTREE_WORKFLOW.md → git-worktree-workflow.md
   - ADAPTER_WORKFLOW_GUIDE.md → guides/adapters/workflow-guide.md

4. **Archive**
   - Archive content lives under docs/archive/
   - Move legacy INTEGRATION_GAPS.md → docs/archive/integration-gaps.md
   - Move legacy RFC_SPEC_TOOLS_ADOPTION.md →
     docs/archive/rfc-spec-tools-adoption.md

### Phase 3: Create Indexes

1. Update README.md with clear documentation links
2. Create comprehensive docs/INDEX.md
3. Add navigation to all major docs

### Phase 4: Update References

1. Update CLAUDE.md to reference new paths
2. Update package README files
3. Update any hardcoded paths in code

## Document Categories

### Strategic (Root Level)

- **README.md** - Project overview, quick start, links to all docs
- **ARCHITECTURE.md** - System design, component overview
- **ROADMAP.md** - Milestones, phases, timeline
- **CLAUDE.md** - Development instructions for Claude Code

### Operational (docs/)

- **status/** - Current state, progress, what's next
- **planning/** - Strategic plans, implementation plans, task lists
- **guides/** - How-to guides for developers
- **prd/** - Product requirements documents
- **adr/** - Architecture decision records
- **formats/** - Format specifications and templates
- **archive/** - Historical documents

### Technical (package-level)

- **core/API.md** - Core API documentation
- **packages/\*/README.md** - Package-specific documentation

## Naming Conventions

### File Names

- Use kebab-case: `cli-integration-complete.md`
- Be descriptive: `adapter-workflow-guide.md` not `guide.md`
- Date-based for snapshots: `status-2025-10-23.md`

### Directory Names

- Plural for collections: `guides/`, `formats/`, `prd/`
- Singular for categories: `status/`, `planning/`, `archive/`

### Headers

- Title case: `# CLI Integration Complete`
- Include metadata at top:

  ```markdown
  # Document Title

  **Date:** 2025-10-23 **Status:** Active/Archive/Draft **Related:** [Links to
  related docs]
  ```

## Benefits

1. **Discoverability** - Clear structure, comprehensive index
2. **Maintenance** - Easy to update, no duplicates
3. **Navigation** - Logical grouping, clear hierarchy
4. **Onboarding** - New developers can find what they need
5. **History** - Archive preserves context without cluttering active docs

## Implementation Timeline

- **Day 1** (Today): Create new structure and indexes
- **Day 2**: Move and consolidate files
- **Day 3**: Update references and links
- **Day 4**: Final review and cleanup

## Notes

- Test fixtures (cli/src/**tests**/fixtures/) are NOT documentation - leave as
  is
- .claude/ directory is Claude Code configuration - leave as is
- Package-specific docs stay with packages
- Keep both old and new paths temporarily with redirects in README files
