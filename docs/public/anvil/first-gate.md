---
id: first-gate
title: Your First Gate Moment
description: Experience anvil catching an issue before it reaches review.
sidebar_position: 5
---

# Your First Gate Moment

This page walks through the experience of anvil catching an issue in real-time.

:::tip Run `anvil start` first

For the install-to-protection flow on a fresh repo, run `anvil start` — it wires
Cursor / Claude Code MCP entries, baselines the repo, and ends in a literal
protection state. The scenario below assumes you're already past that step and
have watch mode (the save-time fallback) running.

:::

## Before You Start

This example assumes you have already run `anvil init`, created
`.anvil/architecture.yaml`, and started watch mode. If you have not done that
yet, follow [First Project](/anvil/first-project) first.

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

The moment you save, anvil responds:

```
Change detected: src/api/handlers/delete-user.ts

Checking import-boundaries...
  ARCH-001: Boundary violation
    src/api/handlers/delete-user.ts:2
    imports from ../../repositories/db
    Rule: api-layer denies imports from src/repositories/**

    API handlers should use services, not repositories directly.

Checking antipattern-scan...
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

Checking import-boundaries... done
Checking antipattern-scan... done

All gates passed.
```

## The Value

In traditional workflows:

1. AI generates code
2. You commit and push
3. CI runs (5 minutes)
4. Reviewer spots the issue (hours later)
5. You context-switch back to fix

With anvil:

1. AI generates code
2. You save
3. anvil catches it (milliseconds)
4. You fix while context is fresh

**Time saved:** Hours of review cycles, context-switching, and accumulated
technical debt.

## What Gates Provide

| Traditional               | With anvil                  |
| ------------------------- | --------------------------- |
| Issues found in review    | Issues found at save        |
| Reviewer cognitive load   | Automated enforcement       |
| Inconsistent enforcement  | Deterministic rules         |
| "It passed tests" excuses | Architecture is tested too  |
| Invisible AI drift        | Visible, immediate feedback |

---

**Previous:** [First project](/anvil/first-project) | **Next:**
[Understand the concepts behind gates →](/anvil/concepts/gates)
