# Claude Projects Lite - Usage Instructions

## Overview

Claude Projects Lite is a lightweight starter kit for running projects natively
inside Claude Code. No servers, no orchestration frameworks, just markdown agent
files, commands, skills, and documentation templates.

## Quick Start

### First-Time Setup (New Repository)

When copying Claude Projects Lite into a new repository:

1. **Install Git Hooks (Recommended)**

   ```bash
   cp hooks/* .git/hooks/
   chmod +x .git/hooks/*
   ```

   This enables automatic formatting, linting, type checking, and testing. See
   `hooks/README.md` for details.

2. **Contextualise Agents** Run this prompt to update agents for your specific
   project:
   ```
   Update the agents based on my project context
   ```
   This will:
   - Analyse your codebase structure, tech stack, and patterns
   - Update agent instructions with project-specific conventions
   - Customise examples to match your frameworks and libraries
   - Add relevant file paths and pattern references
   - Ensure agents follow your coding standards

### Regular Usage

1. Open Claude Code in your repo
2. Run a slash command, for example:
   ```bash
   /feature "artist merch carousel"
   ```
   Claude will sequence: planner → architect → coder → reviewer → tester
3. Git hooks automatically handle code quality checks (format, lint, test)
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
- **test-reality-checker** — Detects tests that mock everything but test nothing
- **data-modeller** — Schema/migration design
- **ui-ux-designer** — Flows, props, accessibility, and component breakdowns

## Slash Commands

All commands are available in `.claude/commands/`:

### Core Workflows

- **/feature** → Break down and implement a new feature end-to-end
- **/new-project** → Scaffold a repo with PRD, architecture, code stubs, docs,
  and tests
- **/ship** → Review, audit, and document a change for release
- **/demo** → Build a minimal demo with UI flow
- **/full-spec** → Generate a detailed PRD and contract-first API spec
- **/project-review** → Comprehensive project analysis and recommendations
- **/security-review** → Security-focused codebase review

### Git & Documentation

- **/commit** → Create standardised conventional commits
- **/create-pr** → Create pull requests with title and body
- **/changelog** → Generate release changelogs
- **/prime-repo** → Analyse and load repository context
- **/prime-docs** → Scan and improve documentation

### Testing & Quality

- **/test-audit** → Audit test suite for circular mocking and false-passing
  tests

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

Claude Projects Lite combines persona-based agents with domain-specific skills
for maximum flexibility:

- **Agents** are plain markdown files defining "who" does the work
  (persona-based)
- **Skills** provide detailed "how-to" knowledge for specialised domains
  (capability-based)
- **Commands** sequence agents for multi-step workflows
- **Git Hooks** provide automatic quality gates
- **Templates** ensure consistent documentation

This hybrid approach gives you:

- **Scalability** - Skills use progressive disclosure to avoid context bloat
- **Composability** - Mix and match agents and skills for any task
- **Maintainability** - Clear separation between personas and procedures
- **Flexibility** - Agents maintain workflow context, skills provide deep
  expertise

You stay in control while Claude handles the busywork with appropriate quality
checks.

## Git Hooks

Git hooks in the `hooks/` directory provide automatic quality checks:

- **pre-commit** — Auto-formats code, runs linting, type checking, and tests
- **post-commit** — Shows commit summary and warns about committing to main
- **pre-push** — Runs security audits before pushing to remote
- **prepare-commit-msg** — Suggests conventional commit format
- **post-merge** — Checks for conflicts and reminds to update dependencies

### Installation

```bash
cp hooks/* .git/hooks/
chmod +x .git/hooks/*
```

The hooks auto-detect your project type (Node.js, Python, Rust, Go) and run
appropriate tools (Prettier, ESLint, Black, Ruff, etc.). See `hooks/README.md`
for customisation options.

## Directory Structure

```
.claude/
  agents/                    # Development personas
    planner.md              # Break down goals into steps
    architect.md            # Design system structure
    product-manager.md      # Create PRDs and requirements
    coder.md               # Implement features
    tester.md              # Write tests and test plans
    reviewer.md            # Code review and feedback
    docs-writer.md         # Documentation updates
    security-auditor.md    # Security checks
    test-reality-checker.md # Test quality audits
    data-modeller.md       # Database design
    ui-ux-designer.md      # UI/UX specifications

  commands/                 # Multi-agent workflows
    feature.md
    new-project.md
    ship.md
    demo.md
    full-spec.md
    project-review.md
    security-review.md
    commit.md
    create-pr.md
    changelog.md
    prime-repo.md
    prime-docs.md
    test-audit.md

  skills/                   # Domain-specific capabilities
    git-workflow/           # Git operations
      SKILL.md
      commit-patterns.md
      pr-templates.md
      scripts/
    code-review/            # Code review procedures
      SKILL.md
      checklists.md
      security-patterns.md
      performance-patterns.md
    repo-intelligence/      # Repository analysis
      SKILL.md
      pattern-library.md
      scripts/

  docs-templates/           # Documentation templates
    PRD.md                 # Product requirements
    ADR.md                 # Architecture decision records
    Architecture-Design.md # Technical implementation specs
    Code-Review.md        # Code review checklist
    Data-Model.md         # Database schema design
    Implementation-Plan.md # Step-by-step implementation
    Runbook.md            # Operational guides
    Security-Audit.md     # Security review checklist
    Test-Plan.md          # Test planning
    UI-Design-Spec.md     # UI/UX specifications
    README-section.md     # README sections

hooks/                     # Git hooks for quality checks
  pre-commit              # Format, lint, type check, test
  post-commit             # Commit summary
  pre-push                # Security audits
  prepare-commit-msg      # Conventional commit hints
  post-merge              # Merge reminders
  README.md               # Installation and customization guide
```

- `/agents/` → Agent definitions with focused responsibilities
- `/commands/` → Slash commands that sequence agents together
- `/skills/` → Domain-specific procedural knowledge and reference libraries
- `/docs-templates/` → Structured templates for consistent documentation

## Skills

Skills provide domain-specific procedural knowledge that agents can reference.
Unlike agents (which define "who"), skills define "how" - detailed procedures,
patterns, and reference materials.

### Available Skills

#### git-workflow

Comprehensive git operations following Conventional Commits:

- **SKILL.md** - Core git workflow procedures (commits, PRs, changelogs)
- **commit-patterns.md** - Conventional Commits reference with examples
- **pr-templates.md** - PR structure templates and best practices
- **scripts/analyze-diff.sh** - Git diff analysis helper

Use for: Creating commits, pull requests, and changelogs with proper formatting

#### code-review

Systematic code review with language-specific guidance:

- **SKILL.md** - Review methodology and comment frameworks
- **checklists.md** - Language-specific checklists (JS/TS, Python, Rust, Go,
  React, SQL)
- **security-patterns.md** - OWASP Top 10, injection, auth vulnerabilities
- **performance-patterns.md** - N+1 queries, algorithm complexity, memory leaks

Use for: Conducting thorough code reviews with security and performance focus

#### repo-intelligence

Repository analysis and pattern detection:

- **SKILL.md** - Repository analysis process and templates
- **pattern-library.md** - Framework patterns (React, Django, FastAPI, Rust, Go)
- **scripts/analyze-structure.sh** - Automated repository structure analysis

Use for: Understanding codebases, onboarding, documentation audits, strategic
planning

### How Agents Use Skills

Agents reference skills for detailed procedural knowledge:

**Example - Reviewer Agent:**

- Agent provides the persona: "I am a pragmatic reviewer"
- Skill provides the procedures: "Here's how to detect SQL injection"
- Result: Thorough review combining judgment with systematic checks

**Progressive Disclosure:**

- **SKILL.md** - Core guidance always available
- **Supplementary files** - Detailed reference loaded when needed
- **Scripts** - Executable helpers for automation

This keeps agent instructions lean while making deep expertise available on
demand.

## Agent Contextualization

### Why Contextualise Agents?

The default agents contain generic examples and patterns. After copying Claude
Projects Lite to your repository, contextualise them to:

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

- **Detect your stack**: Analyse package.json, requirements.txt, go.mod, or
  similar
- **Find patterns**: Search for common patterns like controllers, services,
  components
- **Update examples**: Replace generic code snippets with project-specific ones
- **Add file paths**: Include actual directories like `src/api/`, `components/`,
  `tests/`
- **Customise commands**: Update build/test/lint commands to match your scripts
- **Set conventions**: Document naming patterns (camelCase vs snake_case, etc.)

### Example: Before vs After

**Before (Generic):**

```typescript
// Check for: Express vs Fastify vs Nest
// DI pattern: manual vs @Injectable
```

**After (Contextualised for Next.js + TypeScript):**

```typescript
// Your project uses Next.js 14 with App Router
// API routes in: app/api/
// Components in: components/
// Check existing patterns: components/shared/Button.tsx
```

### When to Re-contextualise

Run "Update the agents based on my project context" again when you:

- Migrate to a new framework or major version
- Change your file structure significantly
- Add new tooling (linters, test frameworks, etc.)
- Switch languages or add new language support
- Onboard new team members (ensures fresh, accurate guidance)

### Manual Customisation

You can also manually edit any agent in `.claude/agents/` to add:

- Team-specific coding standards
- Internal library documentation
- Common gotchas or anti-patterns to avoid
- Links to internal wikis or ADRs
- Project-specific quality requirements
