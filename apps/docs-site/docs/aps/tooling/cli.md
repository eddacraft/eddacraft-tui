---
id: cli
title: CLI Usage
description: Using the Anvil CLI for APS operations.
sidebar_position: 1
---

# CLI Usage

The Anvil CLI provides commands for working with APS plans.

## Plan Commands

### Create Plan

Generate a new plan structure:

```bash
anvil plan create
```

Interactive prompts:
```
? Project name: My Project
? Initial module: core
? First task ID: CORE-001
? First task title: Initial setup

Created:
  plans/index.aps.md
  plans/modules/core.aps.md
```

### Validate Plan

Check plan structure and syntax:

```bash
anvil plan validate plans/index.aps.md
```

Output:
```
Validating plans/index.aps.md...
  ✓ Format valid
  ✓ All modules found
  ✓ All dependencies resolved
  ✓ No circular dependencies

Plan is valid.
```

### Validate with Errors

```bash
anvil plan validate plans/index.aps.md
```

```
Validating plans/index.aps.md...
  ✗ Module not found: modules/missing.aps.md
  ✗ Unresolved dependency: AUTH-099
  ⚠ Task AUTH-002 has no steps

2 errors, 1 warning
```

### Show Plan

Display plan structure:

```bash
anvil plan show
```

```
ShopCraft E-Commerce
├── auth (4 tasks)
│   ├── AUTH-001 [complete] User registration
│   ├── AUTH-002 [complete] User login
│   ├── AUTH-003 [complete] Auth middleware
│   └── AUTH-004 [pending] Password reset
├── products (3 tasks)
│   └── ...
├── cart (3 tasks)
│   └── ...
└── payments (2 tasks)
    └── ...

Total: 12 tasks (7 complete, 3 in_progress, 2 pending)
```

### Hash Plan

Compute content hash:

```bash
anvil plan hash plans/index.aps.md
```

```
sha256:a1b2c3d4e5f6789...
```

### Update Hashes

Recompute all hashes:

```bash
anvil plan update-hashes
```

```
Updated:
  plans/index.aps.md: sha256:abc...
  plans/modules/auth.aps.md: sha256:def...
  plans/modules/products.aps.md: sha256:ghi...
```

## Task Commands

### List Tasks

```bash
anvil task list
```

```
ID        STATUS       TITLE
AUTH-001  complete     User registration
AUTH-002  complete     User login
AUTH-003  complete     Auth middleware
CART-001  in_progress  Add to cart
PAY-001   pending      Stripe integration
```

With filters:

```bash
anvil task list --status pending
anvil task list --module auth
anvil task list --assignee @me
```

### Show Task

```bash
anvil task show AUTH-001
```

```
Task: AUTH-001 — User registration

Status: complete
Module: auth

Outcome:
  New users can create accounts with email and password.
  Passwords are hashed. Email uniqueness enforced.

Validation:
  pnpm test src/auth/register.test.ts

Steps:
  1. [x] POST /auth/register endpoint
  2. [x] Email format validation
  3. [x] Password hashing (bcrypt)
  4. [x] Duplicate email rejection

Dependencies: none
Dependents: AUTH-002, AUTH-003
```

### Update Status

```bash
anvil task status AUTH-001 complete
```

```
Updated AUTH-001: pending → complete
```

### Check Step

Mark step as complete:

```bash
anvil task check AUTH-001 3
```

```
AUTH-001 step 3: [x] Password hashing (bcrypt)
```

## Dependency Commands

### Show Dependencies

```bash
anvil deps show PAY-001
```

```
PAY-001 depends on:
  └── AUTH-003 [complete]
      └── AUTH-002 [complete]
          └── AUTH-001 [complete]
  └── CART-002 [pending]
      └── CART-001 [in_progress]
          └── AUTH-002 [complete]
          └── PROD-002 [complete]
```

### Check for Cycles

```bash
anvil deps check
```

```
No circular dependencies found.
```

Or:

```
Circular dependency detected:
  PAY-001 → CART-002 → PAY-001
```

## Export/Import

### Export to JSON

```bash
anvil plan export --format json > plan.json
```

### Import from JSON

```bash
anvil plan import plan.json
```

### Export to External Format

```bash
anvil plan export --format speckit > plan.speckit.md
```

## Integration with Anvil Run

Plans integrate with validation:

```bash
# Run validation for a specific task
anvil run --task AUTH-001

# Run validation for a module
anvil run --module auth

# Check plan-code alignment
anvil run --plan plans/index.aps.md
```

---

**Next:** [Validation tooling →](/docs/aps/tooling/validation)
