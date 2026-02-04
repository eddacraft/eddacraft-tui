---
id: validation
title: Validation
description: Validating APS documents for correctness.
sidebar_position: 2
---

# Validation

APS documents can be validated for structure, references, and consistency.

## Validation Levels

### Level 1: Syntax

Basic Markdown and frontmatter parsing.

```bash
anvil plan validate --level syntax plans/index.aps.md
```

Checks:

- Valid Markdown
- Valid YAML frontmatter
- Required fields present

### Level 2: Structure

Document structure validation.

```bash
anvil plan validate --level structure plans/index.aps.md
```

Checks:

- Heading hierarchy correct
- Task IDs match pattern
- Steps formatted correctly

### Level 3: References

Cross-reference validation.

```bash
anvil plan validate --level references plans/index.aps.md
```

Checks:

- All module files exist
- All dependencies resolved
- No dangling references

### Level 4: Semantic

Full semantic validation.

```bash
anvil plan validate --level semantic plans/index.aps.md
```

Checks:

- No circular dependencies
- All tasks have outcomes
- Validation commands present

### Full Validation

Run all levels (default):

```bash
anvil plan validate plans/index.aps.md
```

## Validation Rules

### Required Frontmatter

```yaml
---
format: aps # Required, must be "aps"
version: 1.0 # Required, semver format
---
```

### Task ID Format

Pattern: `{PREFIX}-{NNN}`

```markdown
✓ AUTH-001 ✓ PAY-123 ✓ CORE-001

✗ auth-001 # Must be uppercase ✗ AUTH-1 # Must be 3 digits ✗ AUTH_001 # Must use
hyphen
```

### Required Task Fields

```markdown
## Task: AUTH-001 — Title

**Outcome:** Required

**Validation:** Required
```

Optional:

- Status
- Dependencies
- Steps

### Step Format

```markdown
**Steps:**

1. [ ] Incomplete step
2. [x] Complete step
3. [ ] Another step
```

Rules:

- Numbered list
- Checkbox format
- Max 12 words recommended

## Error Messages

### Missing Module

```
Error: Module not found
  File: plans/modules/missing.aps.md
  Referenced in: plans/index.aps.md line 15

  Did you mean: plans/modules/auth.aps.md?
```

### Invalid Task ID

```
Error: Invalid task ID format
  Found: auth-001
  Expected: {PREFIX}-{NNN} (e.g., AUTH-001)
  Location: plans/modules/auth.aps.md line 23
```

### Missing Outcome

```
Error: Task missing outcome
  Task: AUTH-002
  Location: plans/modules/auth.aps.md line 45

  Add an **Outcome:** section to define success criteria.
```

### Circular Dependency

```
Error: Circular dependency detected
  PAY-001 → CART-002 → PAY-001

  Break the cycle by:
  1. Removing one dependency
  2. Extracting shared functionality
```

## CI Integration

### GitHub Actions

```yaml
- name: Validate Plans
  run: anvil plan validate --strict
```

`--strict` fails on warnings too.

### Pre-commit Hook

```bash
#!/bin/bash
# .git/hooks/pre-commit

if git diff --cached --name-only | grep -q "\.aps\.md$"; then
  anvil plan validate --cached
fi
```

## Programmatic Validation

```typescript
import { validatePlan } from '@eddacraft/anvil-aps';

const result = await validatePlan('plans/index.aps.md');

if (!result.valid) {
  for (const error of result.errors) {
    console.error(`${error.level}: ${error.message}`);
    console.error(`  Location: ${error.file}:${error.line}`);
  }
}
```

### Validation Result

```typescript
interface ValidationResult {
  valid: boolean;
  errors: ValidationError[];
  warnings: ValidationWarning[];
}

interface ValidationError {
  code: string;
  level: 'syntax' | 'structure' | 'reference' | 'semantic';
  message: string;
  file: string;
  line?: number;
  suggestion?: string;
}
```

---

**Next:** [Design rationale →](/docs/aps/design/rationale)
