# APS Examples

This directory contains example planning documents demonstrating different APS
patterns and use cases.

## Examples Overview

### 1. Feature Plan: User Authentication

**File:** `feature-auth.aps.md`

**Type:** Single-file plan

**Demonstrates:**

- Simple, self-contained feature plan
- Sequential task dependencies
- Clear task structure with Intent, Expected Outcome, Confidence
- Multi-scope tasks (AUTH + DB, AUTH + API)
- Module-level dependencies section
- Deferred work documented in Notes

**Use this as a template for:** Small to medium features with clear scope

---

### 2. System Plan: E-commerce Platform MVP

**Files:** `system-ecommerce/APS.md` + `modules/*.aps.md`

**Type:** Multi-file plan (index + 4 leaf specs)

**Demonstrates:**

- Large system broken into modules
- Index file with module metadata (Scope, Owner, Priority, Tags, Dependencies)
- Module dependency graph (auth → products → cart → payments)
- Open Questions section in index
- Decisions section with dates
- Different confidence levels across tasks
- Cross-module dependencies between tasks

**Modules:**

- `auth.aps.md` — Authentication (3 tasks, high confidence)
- `products.aps.md` — Product catalog (5 tasks, mixed confidence)
- `cart.aps.md` — Shopping cart (5 tasks, medium confidence with open questions)
- `payments.aps.md` — Stripe payments (6 tasks, includes low confidence task)

**Use this as a template for:** Large systems with multiple modules and team
ownership

---

### 3. Refactor Plan: API Error Handling

**File:** `refactor-error-handling.aps.md`

**Type:** Single-file plan

**Demonstrates:**

- Refactoring with uncertainty (low/medium/high confidence mix)
- Multi-scope tasks across many modules (API, AUTH, PROD, CART, PAY, INFRA,
  DOCS)
- Research/audit tasks (API-001 with low confidence)
- Design tasks before implementation
- Multiple parallel refactor tasks (API-005 through API-008)
- Open questions documenting unknowns
- Notes about rollout strategy and considerations

**Confidence distribution:**

- Low: 3 tasks (audit, monitoring integration, unknowns)
- Medium: 4 tasks (design, implementation with dependencies on external systems)
- High: 4 tasks (clear implementation with established patterns)

**Use this as a template for:** Technical debt, refactoring, infrastructure work
with unknowns

---

## Key Patterns Demonstrated

### Task Structure

All examples show proper task formatting:

```markdown
### SCOPE-001: Task title

**Intent:** What the task achieves **Expected Outcome:** Success criteria
**Confidence:** low|medium|high **Scopes:** SCOPE1, SCOPE2 **Tags:** tag1, tag2
**Dependencies:** SCOPE-XXX **Inputs:**

- Required input 1
```

### Confidence Levels

- **High:** Well-understood, clear path (most tasks in feature-auth)
- **Medium:** Generally clear with some unknowns (cart pricing, payment
  webhooks)
- **Low:** Uncertain approach, needs exploration (error audit, monitoring
  integration)

### Multi-Scope Tasks

Tasks that touch multiple areas use comma-separated scopes:

```markdown
**Scopes:** AUTH, DB, API
```

### Dependencies

- **Module dependencies:** In index file under each module
- **Task dependencies:** In task metadata with IDs

### Deferred Work

All examples document out-of-scope items in Notes:

- "OAuth integration deferred"
- "Image upload deferred to separate module"
- "Refunds deferred to phase 2"

---

## Using These Examples

### As Learning Material

1. Read `feature-auth.aps.md` first for basic structure
2. Explore `system-ecommerce/` for multi-module plans
3. Study `refactor-error-handling.aps.md` for uncertainty handling

### As Templates

1. Copy the example closest to your use case
2. Replace scope prefixes with your own
3. Update tasks to match your requirements
4. Adjust confidence based on your knowledge

### For Validation Testing

These examples can be used to test the APS validator:

- All should pass structural validation
- All should have valid task ID formats
- All should have resolvable links (in system-ecommerce)

---

## File Naming Conventions

- **Single-file plans:** `[type]-[name].aps.md` (e.g., `feature-auth.aps.md`)
- **Multi-file plans:** `[name]/APS.md` as index, `modules/*.aps.md` as leaf
  specs

---

## Next Steps

After exploring these examples:

1. Read the [APS Planning Spec](../docs/APS-Planning-Spec-v0.1.md) for full
   format details
2. Check [APS Conventions](../docs/APS-Conventions.md) for detailed rules
3. Use the template generator: `pnpm generate-templates`
