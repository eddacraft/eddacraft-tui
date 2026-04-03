---
id: suppressions
title: Suppressions
sidebar_position: 6
---

# Suppressions

<!-- prettier-ignore-start -->
:::caution CLI commands planned
The `anvil suppress` CLI commands shown in this tutorial (e.g. `--all`,
`--list`, `--count`) are planned for a future release. Inline `@anvil-ignore`
comments and `.anvil/suppressions.json` file-level suppressions work today.
:::
<!-- prettier-ignore-end -->

Not every warning needs fixing right now. Anvil's suppression system lets you
acknowledge known issues with a mandatory explanation, so nothing is silently
ignored.

## When to Suppress

Suppressions are appropriate for:

- **Legacy code** -- violations that exist before Anvil was adopted
- **Intentional decisions** -- an architectural shortcut with a documented
  reason
- **Temporary exceptions** -- work planned for a future sprint

Suppressions are not a substitute for fixing issues. Track them and reduce the
count over time.

## Inline Suppression

Add a comment directly above the offending line:

```typescript
// @anvil-ignore AP-003 Legacy parser uses any, migration planned Q2
export function parse(input: any): Record<string, unknown> {
  // ...
}
```

The format is:

```
// @anvil-ignore <RULE-ID> <reason>
```

### Reason is Required

A bare `@anvil-ignore` without a reason triggers its own warning:

```
  [SUP-001] Suppression without reason
    src/utils/parser.ts:41
    @anvil-ignore must include an explanation
```

This ensures every suppression is documented. There are no silent exceptions.

### Multiple Rules

Suppress multiple rules on the same line by separating them with commas:

```typescript
// @anvil-ignore AP-003, AP-006 Legacy code, full rewrite in progress
```

## File-Level Suppression

For files where many lines trigger the same rule, add entries to
`.anvil/suppressions.json`:

```json
[
  {
    "pattern": "src/legacy/**",
    "checks": ["AP-003", "AP-006"],
    "reason": "Legacy code, migration planned Q2"
  },
  {
    "pattern": "src/generated/**",
    "checks": ["*"],
    "reason": "Auto-generated from protobuf definitions"
  }
]
```

| Field     | Description                                 |
| --------- | ------------------------------------------- |
| `pattern` | Glob matching the files to suppress         |
| `checks`  | Array of rule IDs, or `["*"]` for all rules |
| `reason`  | Required explanation                        |

## Bulk Suppression for Existing Codebases

When adopting Anvil in a large project, you may have hundreds of existing
violations. Add file-level suppressions to `.anvil/suppressions.json` grouped by
directory and rule, then work through them incrementally:

```json
[
  {
    "pattern": "src/**",
    "checks": ["AP-003", "AP-006"],
    "reason": "Baseline: pre-Anvil adoption"
  }
]
```

Run `anvil check --all` to confirm a clean baseline. New code is held to the
full standard from day one.

<!-- prettier-ignore-start -->
:::caution
Bulk suppression is a one-time onboarding tool. Avoid adding overly broad
patterns — they mask new violations alongside old ones. Narrow the patterns as
you fix issues.
:::
<!-- prettier-ignore-end -->

## Tracking Suppressions Over Time

Review suppressions periodically by searching your codebase for inline comments
and checking `.anvil/suppressions.json`:

```bash
# Find inline suppressions
grep -rn "@anvil-ignore" src/

# Count them
grep -rn "@anvil-ignore" src/ | wc -l
```

Track the count over time to ensure it trends downward. You can add a CI step
that fails if the count exceeds a threshold — for example, using a script that
counts `@anvil-ignore` occurrences and compares against a budget.

---

**Previous:** [CI Integration](/anvil/tutorials/ci) | **All tutorials:**
[Tutorials index](/anvil/tutorials)
