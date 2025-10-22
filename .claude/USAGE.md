# Claude Projects Lite - Usage Instructions

## Overview

Claude Projects Lite is a lightweight starter kit for running projects natively
inside Claude Code. No servers, no orchestration frameworks, just markdown agent
files, commands, addons, and documentation templates.

## Quick Start

### First-Time Setup (New Repository)

When copying Claude Projects Lite into a new repository, run this prompt to
contextualize agents for your project:

```
Update the agents based on my project context
```

This will:

- Analyse your codebase structure, tech stack, and patterns
- Update agent instructions with project-specific conventions
- Customize examples to match your frameworks and libraries
- Add relevant file paths and pattern references
- Ensure agents follow your coding standards

### Regular Usage

1. Open Claude Code in your repo
2. Activate addons for your project type (see `.claude/addons/ACTIVATION.md`)
3. Run a slash command, for example:
   ```bash
   /feature "artist merch carousel"
   ```
   Claude will sequence: planner → architect → coder → reviewer → tester
4. Outputs appear step by step — you can stop, edit, or continue at any handoff

## Available Agents

- **planner** — Breaks a goal into steps, success criteria, and delegations
- **architect** — Defines interfaces, file changes, schema tweaks, and
  guardrails
- **product-manager** — Writes a crisp PRD with users, use cases, and acceptance
  criteria
- **coder** — Implements the smallest change needed, with diffs
- **tester** — Produces test plans and code from acceptance criteria
- **reviewer** — Performs pragmatic code review and decision (approve / request
  changes)
- **docs-writer** — Updates READMEs, ADRs, and usage notes
- **security-auditor** — Checks for auth, PII, dependency risk, and must-fix
  issues
- **data-modeler** (optional) — Schema/migration design
- **ui-ux-designer** (optional) — Flows, props, accessibility, and component
  breakdowns

## Slash Commands

Available in `.claude/commands/`:

- **/new-project** → Scaffold a repo with PRD, architecture, code stubs, docs,
  and tests
- **/feature** → Break down and implement a new feature end-to-end
- **/full-spec** → Generate a detailed PRD and contract-first API spec
- **/ship** → Review, audit, and document a change for release
- **/demo** → Build a minimal demo with UI flow

Additional commands available via addons:

- **/commit** → Create standardized Git commits (via git-workflow addon)
- **/create-pr** → Create pull requests (via git-workflow addon)
- **/changelog** → Generate changelogs (via git-workflow addon)
- **/prime-repo** → Prime Claude with repo context (via repository addon)
- **/prime-docs** → Scan and improve documentation (via repository addon)

## Doc Templates

Located in `.claude/docs-templates/`:

- **PRD.md** → Product Requirements Document for defining features
- **ADR.md** → Architecture Decision Record for documenting "why" decisions were
  made
- **Architecture-Design.md** → Technical implementation blueprint with
  interfaces and data models
- **Code-Review.md** → Structured checklist for code reviews
- **Data-Model.md** → Database schema and migration specifications
- **Implementation-Plan.md** → Step-by-step development plan
- **Runbook.md** → Operational guide for deployment and maintenance
- **Security-Audit.md** → Security review checklist and findings
- **Test-Plan.md** → Comprehensive test strategy and cases
- **UI-Design-Spec.md** → UI/UX specifications and component design
- **README-section.md** → Template for README sections

The docs-writer agent can fill these in automatically or you can draft manually.

## Philosophy

Claude Projects Lite is about lean sequencing, not heavy orchestration:

- Agents are plain markdown files with focused responsibilities
- Commands are reusable workflows that sequence agents
- Addons provide automatic quality gates without interrupting flow
- Templates ensure consistent documentation

You stay in control and Claude handles the busywork with appropriate quality
checks.

## Directory Structure

```
.claude/
  agents/                    # Development personas
    architect.md
    planner.md
    product-manager.md
    coder.md
    tester.md
    reviewer.md
    docs-writer.md
    security-auditor.md
    data-modeler.md         # optional
    ui-ux-designer.md       # optional

  commands/                 # Multi-agent workflows
    feature.md
    new-project.md
    ship.md
    demo.md
    full-spec.md

  addons/                   # Modular extensions
    hooks/                  # Quality gates & automation
      safety/
      node-typescript/
      python-modern/
      agent-quality/
      git-workflow/
    commands/               # Additional commands via addons
      git-workflow/         # Git workflow commands
      repository/           # Repo management commands

  docs-templates/           # Documentation templates
    PRD.md
    ADR.md
    Architecture-Design.md
    Code-Review.md
    Data-Model.md
    Implementation-Plan.md
    Runbook.md
    Security-Audit.md
    Test-Plan.md
    UI-Design-Spec.md
    README-section.md
```

- `/agents/` → Agent definitions with focused responsibilities
- `/commands/` → Slash commands that sequence agents together
- `/addons/` → Modular extensions for hooks and additional commands
- `/docs-templates/` → Structured templates for consistent documentation

## Agent Contextualization

### Why Contextualize Agents?

The default agents contain generic examples and patterns. After copying Claude
Projects Lite to your repository, contextualize them to:

1. **Match your tech stack** - Replace generic examples with your actual
   frameworks (Express vs Fastify, React vs Vue, etc.)
2. **Follow your conventions** - Update file paths, naming patterns, and code
   structure examples
3. **Use your tools** - Reference your actual npm scripts, build commands, and
   test runners
4. **Align with patterns** - Include examples from your existing codebase so
   agents naturally follow your style

### What Gets Updated?

When you run "Update the agents based on my project context", Claude will:

- **Detect your stack**: Analyze package.json, requirements.txt, go.mod, or
  similar
- **Find patterns**: Search for common patterns like controllers, services,
  components
- **Update examples**: Replace generic code snippets with project-specific ones
- **Add file paths**: Include actual directories like `src/api/`, `components/`,
  `tests/`
- **Customize commands**: Update build/test/lint commands to match your scripts
- **Set conventions**: Document naming patterns (camelCase vs snake_case, etc.)

### Example: Before vs After

**Before (Generic):**

```typescript
// Check for: Express vs Fastify vs Nest
// DI pattern: manual vs @Injectable
```

**After (Contextualized for Next.js + TypeScript):**

```typescript
// Your project uses Next.js 14 with App Router
// API routes in: app/api/
// Components in: components/
// Check existing patterns: components/shared/Button.tsx
```

### When to Re-contextualize

Run "Update the agents" again when you:

- Migrate to a new framework or major version
- Change your file structure significantly
- Add new tooling (linters, test frameworks, etc.)
- Onboard new team members (ensures fresh, accurate guidance)

### Manual Customization

You can also manually edit any agent in `.claude/agents/` to add:

- Team-specific coding standards
- Internal library documentation
- Common gotchas or anti-patterns to avoid
- Links to internal wikis or ADRs
