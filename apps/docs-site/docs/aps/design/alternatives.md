---
id: alternatives
title: Alternatives Considered
description: Other approaches we evaluated before designing APS.
sidebar_position: 2
---

# Alternatives Considered

Before creating APS, we evaluated several existing formats and approaches.

## Existing Formats

### GitHub Issues / Jira

**What they offer:**
- Task tracking
- Assignments
- Comments
- Workflow states

**Why we didn't use them:**
- Not deterministic (mutable state)
- No hash stability
- API-dependent
- Don't define success criteria structurally
- Outcomes buried in prose

**APS difference:**
APS is a *specification*, not a tracker. It defines *what* success looks like. Trackers manage *who* and *when*.

### RFC Documents

**What they offer:**
- Structured proposals
- Problem/solution format
- Historical record

**Why we didn't use them:**
- Too heavyweight for tasks
- Design-focused, not execution-focused
- No validation hooks
- Not modular

**APS difference:**
APS is for *execution*, not design. RFCs inform the plan; they don't replace it.

### Cucumber / Gherkin

**What they offer:**
- Behaviour specification
- Given/When/Then structure
- Executable specs

**Why we didn't use them:**
- Focused on test scenarios
- Verbose for simple tasks
- Tied to testing tools
- Doesn't capture architecture

**APS difference:**
APS defines tasks and outcomes. Gherkin defines test cases. They're complementary—APS might reference Gherkin specs in validation.

### OpenAPI / AsyncAPI

**What they offer:**
- API specification
- Contract definition
- Code generation

**Why we didn't use them:**
- API-specific
- Implementation-focused
- Doesn't capture "why"
- Not for general tasks

**APS difference:**
APS is domain-agnostic. An API task might validate against OpenAPI, but the plan is broader.

### Architecture Decision Records (ADRs)

**What they offer:**
- Decision documentation
- Context and consequences
- Historical reasoning

**Why we didn't use them:**
- Decision-focused, not task-focused
- No execution validation
- One decision per file

**APS difference:**
ADRs inform *why* we're building something. APS defines *what* we're building. Use both.

## Custom Formats

### Pure YAML

We considered:

```yaml
plan:
  modules:
    - id: auth
      tasks:
        - id: AUTH-001
          outcome: Users can log in
          validation: pnpm test
```

**Pros:**
- Machine-parseable
- Strict structure
- Easy validation

**Cons:**
- Verbose for prose
- Poor diff experience
- Hard to read
- No inline documentation

**Decision:** Markdown with YAML frontmatter gives best of both.

### Pure JSON

We considered:

```json
{
  "modules": [{
    "id": "auth",
    "tasks": [...]
  }]
}
```

**Pros:**
- Universal parsing
- Strict types
- Language-agnostic

**Cons:**
- Not human-writable
- Terrible for prose
- No comments
- Merge conflicts

**Decision:** JSON is output format, not authoring format.

### Custom DSL

We considered a domain-specific language:

```
module auth
  task AUTH-001 "Login"
    outcome "Users can log in"
    validate `pnpm test`
    depends AUTH-000
```

**Pros:**
- Concise
- Purpose-built
- Precise semantics

**Cons:**
- Learning curve
- Tooling investment
- No existing editor support
- Renders as code on GitHub

**Decision:** Familiar Markdown > custom DSL.

## Architectural Approaches

### Monorepo of Specs

One giant specification file.

**Rejected because:**
- Merge conflicts
- No modularity
- Harder to navigate
- Ownership unclear

### Database-Backed

Store plans in a database with UI.

**Rejected because:**
- Not version controlled
- Not reviewable in PRs
- Tooling dependency
- Harder to audit

### Wiki-Style

Plans in a wiki system.

**Rejected because:**
- Not in repo
- Versioning unclear
- No CI integration
- Ownership model weak

## What We Kept

From our evaluation, we kept:

| From | What |
|------|------|
| Issues | Task status tracking concept |
| RFCs | Structured documentation approach |
| Gherkin | Declarative success criteria |
| ADRs | Companion documentation pattern |
| OpenAPI | Schema validation approach |

## Lessons Learned

1. **Human-readable matters** — developers will edit these
2. **Reviewable matters** — PR workflow is essential
3. **Deterministic matters** — automation requires stability
4. **Simple matters** — adoption beats features

---

**Back to:** [APS Overview →](/docs/aps/overview)
