---
name: repo-intelligence
description:
  Repository analysis and contextualization including structure scanning,
  pattern detection, documentation quality review, and strategic planning
---

# Repository Intelligence Skill

This skill provides systematic repository analysis, pattern detection,
documentation review, and strategic insights to help understand and improve
codebases.

## Capabilities

1. **Repository Analysis** - Understand structure, tech stack, and patterns
2. **Documentation Review** - Assess and improve documentation quality
3. **Pattern Detection** - Identify frameworks, conventions, and architectural
   patterns
4. **Gap Analysis** - Find missing documentation, tests, or features
5. **Strategic Planning** - Propose prioritized next steps

## When to Use This Skill

Invoke this skill when:

- Starting work on an unfamiliar codebase
- Onboarding new team members
- Planning major features or refactors
- Conducting documentation audits
- Preparing for code review or release
- Understanding project conventions

## Repository Analysis Process

### Phase 1: Initial Discovery (Parallel Execution)

Run these operations simultaneously:

```bash
# In one message with multiple tool calls:
1. Read README.md
2. Read package.json/requirements.txt/Cargo.toml
3. Glob key directories (src/, tests/, docs/)
4. Read recent git commits
5. Check for ADRs and documentation
```

**Example:**

```bash
# These should all run in parallel
cat README.md
cat package.json
ls -R src/
git log --oneline -20
find . -name "ADR-*.md"
```

### Phase 2: Structure Analysis

Understand the organization:

1. **Directory Structure**
   - Identify layers (API, UI, DB, services)
   - Find test directories
   - Locate configuration files
   - Map documentation locations

2. **Tech Stack Detection**
   - Parse dependency files
   - Identify frameworks (React, Django, etc.)
   - Note language versions
   - Check tooling (build, test, lint)

3. **Pattern Recognition**
   - Component organization (by feature vs. by type)
   - Naming conventions
   - Import patterns
   - Error handling approaches

### Phase 3: Quality Assessment

Evaluate codebase health:

1. **Documentation Quality**
   - README completeness
   - API documentation
   - Code comments
   - Architecture diagrams
   - ADRs present

2. **Test Coverage**
   - Test directory structure
   - Test types (unit, integration, e2e)
   - Coverage metrics
   - Test quality

3. **Code Quality**
   - Linter configuration
   - Type checking setup
   - Formatting standards
   - Code complexity

4. **Maintenance Indicators**
   - TODO/FIXME count
   - Dependency staleness
   - Open issues patterns
   - Recent commit activity

### Phase 4: Strategic Insights

Generate actionable recommendations:

1. **Quick Wins** - Easy improvements with high impact
2. **Technical Debt** - Areas needing refactoring
3. **Missing Pieces** - Gaps in docs, tests, features
4. **Next Steps** - Prioritized action items

## Repository Summary Template

```markdown
# Repository Analysis: [Project Name]

## Overview

**Purpose:** [What the project does] **Tech Stack:** [Languages, frameworks,
major dependencies] **Architecture:** [Monolith/Microservices, layers, patterns]

## Structure
```

project/ src/ # [Description] api/ # [Purpose] components/ # [Purpose]
services/ # [Purpose] tests/ # [Coverage: X%] docs/ # [Completeness:
Good/Fair/Poor]

```

## Key Patterns

- **Organization:** [Feature-based / Type-based]
- **Naming:** [camelCase / snake_case / PascalCase]
- **State Management:** [Redux / Context / Zustand]
- **Error Handling:** [try/catch / .catch / Result types]
- **Testing:** [Jest / pytest / cargo test]

## Tech Stack Details

### Dependencies

**Production:**
- [Framework]: v[X.Y.Z] - [Purpose]
- [Library]: v[X.Y.Z] - [Purpose]

**Development:**
- [Tool]: [Purpose]
- [Tool]: [Purpose]

### Tooling

- **Build:** [webpack / vite / cargo / go build]
- **Test:** [jest / pytest / cargo test]
- **Lint:** [eslint / ruff / clippy]
- **Format:** [prettier / black / rustfmt]

## Quality Metrics

- **Test Coverage:** [X%]
- **TypeScript Coverage:** [X% typed / Y% any]
- **Documentation:** [Comprehensive / Adequate / Sparse]
- **Code Complexity:** [Low / Medium / High]
- **Dependency Health:** [X outdated / Y vulnerable]

## Documentation Status

### Present
- [x] README with setup instructions
- [x] API documentation
- [x] Architecture overview

### Missing
- [ ] Contributing guidelines
- [ ] Deployment runbook
- [ ] ADRs for major decisions

## Architectural Insights

### Strengths
- [Well-organized service layer]
- [Comprehensive test coverage]
- [Clear separation of concerns]

### Areas for Improvement
- [Missing error handling in X]
- [N+1 queries in Y]
- [High complexity in Z module]

## Recent Activity

- **Last Commit:** [Date]
- **Active Contributors:** [Count]
- **Recent Focus:** [Features / Bug fixes / Refactoring]

## Immediate Opportunities

### Quick Wins (1-2 hours)
1. [Add missing README section]
2. [Fix obvious type errors]
3. [Add linter rules]

### Technical Debt (1-2 days)
1. [Refactor authentication module]
2. [Add integration tests]
3. [Update dependencies]

### Strategic (1+ weeks)
1. [Implement feature X]
2. [Migrate to framework Y]
3. [Improve performance of Z]

## Next Steps

**Priority 1 (This Week):**
1. [Action item]
2. [Action item]

**Priority 2 (This Month):**
1. [Action item]
2. [Action item]

**Priority 3 (This Quarter):**
1. [Action item]
2. [Action item]
```

## Documentation Review Process

### Documentation Scan

Check for these artifacts:

1. **README.md**
   - [ ] Project description
   - [ ] Installation instructions
   - [ ] Usage examples
   - [ ] Contributing guidelines
   - [ ] License
   - [ ] Contact/support info

2. **API Documentation**
   - [ ] Endpoint documentation
   - [ ] Request/response examples
   - [ ] Authentication docs
   - [ ] Error codes explained

3. **Architecture Documentation**
   - [ ] System overview
   - [ ] Component diagrams
   - [ ] Data flow diagrams
   - [ ] Technology decisions (ADRs)

4. **Development Documentation**
   - [ ] Setup guide
   - [ ] Development workflow
   - [ ] Testing guide
   - [ ] Debugging tips

5. **Operational Documentation**
   - [ ] Deployment guide
   - [ ] Monitoring setup
   - [ ] Troubleshooting runbook
   - [ ] Disaster recovery

### Documentation Quality Assessment

For each document, evaluate:

**Clarity**

- Is it understandable?
- Are examples clear?
- Is jargon explained?

**Completeness**

- Are all sections filled in?
- Are edge cases covered?
- Is it up to date?

**Accuracy**

- Does it match the code?
- Are examples tested?
- Are versions correct?

**Maintainability**

- Is it easy to update?
- Is it properly structured?
- Are there clear ownership?

### Gap Identification

Common documentation gaps:

- Missing setup instructions
- No troubleshooting guide
- Outdated examples
- Missing architecture diagrams
- No ADRs for major decisions
- Sparse inline comments
- Missing API documentation
- No deployment guide

### Improvement Prioritization

**High Priority (Fix Immediately)**

- Incorrect information
- Missing critical setup steps
- Broken links/examples
- Security-related gaps

**Medium Priority (Fix This Week)**

- Incomplete sections
- Missing examples
- Outdated screenshots
- Poor organization

**Low Priority (Fix When Convenient)**

- Typos
- Style inconsistencies
- Minor improvements
- Nice-to-have additions

## Pattern Detection Reference

See `pattern-library.md` for detailed framework and library pattern catalogs.

### Common Framework Patterns

**React Patterns**

- Component organization
- State management approach
- Hook patterns
- Styling approach

**Node.js Patterns**

- Express middleware patterns
- Error handling
- Async patterns
- Database access patterns

**Python Patterns**

- Django apps organization
- View patterns
- ORM usage
- Async patterns

**Rust Patterns**

- Module organization
- Error handling (Result/Option)
- Ownership patterns
- Trait usage

## Analysis Scripts

The `scripts/` directory contains helper scripts:

- `analyze-structure.sh` - Directory tree analysis
- `detect-stack.sh` - Tech stack detection
- `find-todos.sh` - TODO/FIXME aggregation
- `check-docs.sh` - Documentation completeness checker

## Integration with Agents

This skill works with:

- **docs-writer agent** - Uses analysis to update documentation
- **planner agent** - Uses insights for strategic planning
- **architect agent** - Uses patterns for design decisions

The skill provides the "what is" analysis, agents use it for decision-making.

## Output Formats

### Quick Summary (5 minutes)

```markdown
# [Project Name] - Quick Analysis

**Stack:** [Tech stack] **Structure:** [Organization pattern] **Quality:** [Test
coverage, docs status] **Focus:** [Recent work]

**Immediate Actions:**

1. [Action]
2. [Action]
```

### Standard Analysis (20 minutes)

Full repository summary using template above.

### Deep Dive (1+ hour)

- Comprehensive pattern analysis
- Detailed quality metrics
- Security audit findings
- Performance profiling
- Strategic roadmap

## Best Practices

1. **Run analysis in parallel** - Maximize tool concurrency
2. **Look for patterns** - Don't just list files, understand conventions
3. **Be specific** - "Missing error handling in auth.ts:45" not "Needs better
   error handling"
4. **Prioritize findings** - Quick wins vs. strategic improvements
5. **Provide examples** - Show the pattern from existing code
6. **Be constructive** - Frame as opportunities, not just problems
7. **Update regularly** - Re-run analysis after major changes

## Common Analysis Patterns

### New Codebase

1. README + package file analysis
2. Directory structure mapping
3. Pattern detection in 3-5 sample files
4. Documentation completeness check
5. Quick win identification

### Documentation Audit

1. Scan all markdown files
2. Check completeness of each type
3. Verify accuracy against code
4. Identify gaps
5. Prioritize improvements

### Pre-Feature Planning

1. Find similar existing features
2. Extract patterns used
3. Identify conventions to follow
4. Map integration points
5. Plan minimal viable change

### Onboarding Support

1. Generate comprehensive summary
2. Create getting started guide
3. Highlight key patterns
4. Map common workflows
5. Provide quick reference

## Tips for Success

1. **Start broad, then narrow** - README → structure → specific patterns
2. **Trust the build tools** - Look at package.json scripts for workflow
3. **Follow the tests** - Test structure reveals intended architecture
4. **Read recent commits** - Shows current focus and conventions
5. **Check for ADRs** - Understand "why" behind decisions
6. **Look for .editorconfig** - Reveals style preferences
7. **Examine CI/CD** - Shows quality gates and deployment

---

**Skill Version:** 1.0 **Compatible With:** Claude Code, Claude.ai, Claude Agent
SDK **Last Updated:** 2025-11-08
