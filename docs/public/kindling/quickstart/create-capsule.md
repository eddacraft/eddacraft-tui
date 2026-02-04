---
id: create-capsule
title: Create a Capsule
description: Creating your first Kindling capsule.
sidebar_position: 2
---

# Create a Capsule

Capsules are containers for related observations. Create one for each project,
feature, or context.

## Create a Capsule

```bash
kindling capsule create my-project
```

**Output:**

```
Created capsule: my-project
Location: ~/.kindling/capsules/my-project.db
```

## Naming Conventions

Good capsule names:

- `payment-integration` — feature-focused
- `auth-refactor` — task-focused
- `q1-2024-learnings` — time-boxed
- `debugging-session-001` — session-focused

Avoid:

- Spaces (use hyphens)
- Special characters
- Very long names

## Set as Default

Make a capsule the default for observations:

```bash
kindling capsule use my-project
```

Now `kindling observe` uses this capsule automatically:

```bash
kindling observe "Something I learned"
# Stored in: my-project
```

## List Capsules

```bash
kindling capsule list
```

**Output:**

```
CAPSULE              OBSERVATIONS  CREATED           LAST UPDATED
default              3             2024-01-10        2024-01-10
my-project           0             2024-01-15        2024-01-15
payment-integration  42            2024-01-12        2024-01-15  (active)
```

The `(active)` marker shows the current default.

## View Capsule Details

```bash
kindling capsule show payment-integration
```

**Output:**

```
Capsule: payment-integration

Location: ~/.kindling/capsules/payment-integration.db
Created: 2024-01-12T09:00:00Z
Last updated: 2024-01-15T16:30:00Z
Observations: 42

Top tags:
  api (15)
  stripe (12)
  error-handling (8)

Recent observations:
  [2024-01-15] Stripe webhook signature must use raw body
  [2024-01-15] Idempotency key required for payment intents
  [2024-01-14] Test mode keys start with sk_test_
```

## Archive a Capsule

When you're done with a project:

```bash
kindling capsule archive payment-integration
```

Archived capsules:

- Remain searchable
- Don't appear in active list
- Can be unarchived later

## Delete a Capsule

:::caution Deletion is permanent. :::

```bash
kindling capsule delete old-project
```

Confirm when prompted.

## Project-Local Capsules

Create a capsule in your project directory:

```bash
cd my-project
kindling capsule create --local .
```

This creates `.kindling/` in the project root. Useful for:

- Team-shared observations
- Version-controlled knowledge
- Project-specific context

---

**Next:** [Write observations →](/docs/kindling/quickstart/write-observations)
