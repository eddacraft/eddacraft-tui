---
id: first-gate
title: Your First Gate Moment
description: Experience Anvil catching an issue before it reaches review.
sidebar_position: 5
---

# Your First Gate Moment

This page walks through the experience of Anvil catching an issue in real-time.

## The Scenario

You're using an AI coding assistant to add a new endpoint. The AI generates
working code, but it violates your architecture.

## Step 1: Start Watch Mode

In a terminal:

```bash
anvil watch
```

You see:

```
Anvil Watch

Watching for changes...
Press Ctrl+C to stop.
```

## Step 2: AI Generates Code

You ask your AI assistant: "Add a DELETE endpoint for users"

The AI generates `src/api/handlers/delete-user.ts`:

```typescript
import { Request, Response } from 'express';
import { db } from '../../repositories/db'; // Direct DB access!

export async function deleteUser(req: Request, res: Response) {
  const { id } = req.params;

  try {
    await db.query('DELETE FROM users WHERE id = $1', [id]);
    res.status(204).send();
  } catch (e) {
    // AI left an empty catch block
    res.status(500).send();
  }
}
```

## Step 3: Save the File

The moment you save, Anvil responds:

```
Change detected: src/api/handlers/delete-user.ts

Checking architecture...
  ARCH-001: Boundary violation
    src/api/handlers/delete-user.ts:2
    imports from ../../repositories/db
    Rule: api-layer denies imports from src/repositories/**

    API handlers should use services, not repositories directly.

Checking anti-patterns...
  [AP-006] Empty catch block
    src/api/handlers/delete-user.ts:10:5

    Empty catch blocks hide errors. Log the error or re-throw.

1 error, 1 warning found.
Gate status: FAIL
```

## Step 4: Fix Before Commit

You now know _immediately_ that this code has issues—before you commit, before
you push, before a reviewer has to point it out.

Fix the architecture violation:

```typescript
import { Request, Response } from 'express';
import { UserService } from '../../services/user.service'; // Correct!

export async function deleteUser(req: Request, res: Response) {
  const { id } = req.params;

  try {
    await UserService.delete(id);
    res.status(204).send();
  } catch (error) {
    console.error('Failed to delete user:', error); // Proper handling
    res.status(500).json({ error: 'Failed to delete user' });
  }
}
```

Save again:

```
Change detected: src/api/handlers/delete-user.ts

Checking architecture... done
Checking anti-patterns... done

All gates passed.
```

## The Value

In traditional workflows:

1. AI generates code
2. You commit and push
3. CI runs (5 minutes)
4. Reviewer spots the issue (hours later)
5. You context-switch back to fix

With Anvil:

1. AI generates code
2. You save
3. Anvil catches it (milliseconds)
4. You fix while context is fresh

**Time saved:** Hours of review cycles, context-switching, and accumulated
technical debt.

## What Gates Provide

| Traditional               | With Anvil                  |
| ------------------------- | --------------------------- |
| Issues found in review    | Issues found at save        |
| Reviewer cognitive load   | Automated enforcement       |
| Inconsistent enforcement  | Deterministic rules         |
| "It passed tests" excuses | Architecture is tested too  |
| Invisible AI drift        | Visible, immediate feedback |

---

**Previous:** [First project](/anvil/first-project) | **Next:**
[Understand the concepts behind gates →](/anvil/concepts/gates)
