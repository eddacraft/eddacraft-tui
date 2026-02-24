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
comments and `.anvilrc` file-level suppressions work today.
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

For files where many lines trigger the same rule, suppress at the file level in
`.anvilrc`:

```json
{
  "suppressions": [
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
}
```

| Field     | Description                                 |
| --------- | ------------------------------------------- |
| `pattern` | Glob matching the files to suppress         |
| `checks`  | Array of rule IDs, or `["*"]` for all rules |
| `reason`  | Required explanation                        |

## Bulk Suppression for Existing Codebases

When adopting Anvil in a large project, you may have hundreds of existing
violations. Suppress them all at once and work through them incrementally:

```bash
anvil suppress --all --reason "Baseline: pre-Anvil adoption"
```

```
Suppressed 142 violations across 38 files.
Written to .anvilrc under "suppressions".

Run anvil check --all to verify a clean baseline.
```

This creates file-level suppressions grouped by rule. New code is held to the
full standard from day one.

<!-- prettier-ignore-start -->
:::caution
Bulk suppression is a one-time onboarding tool. Avoid running it
repeatedly -- it masks new violations alongside old ones.
:::
<!-- prettier-ignore-end -->

## Tracking Suppressions Over Time

List all active suppressions:

```bash
anvil suppress --list
```

```
Active suppressions:

  File-level (.anvilrc):
    src/legacy/**          AP-003, AP-006    "Legacy code, migration planned Q2"
    src/generated/**       *                 "Auto-generated from protobuf"

  Inline:
    src/utils/parser.ts:41     AP-003    "Legacy parser uses any, migration planned Q2"
    src/api/health.ts:12       ARCH-001  "Health endpoint needs direct DB access"

Total: 4 suppressions (2 file-level, 2 inline)
```

Track the count in CI to ensure it trends downward:

```bash
anvil suppress --count
```

```
Suppression count: 4
```

Add a threshold to your CI pipeline to prevent suppression creep:

```json
{
  "ci": {
    "maxSuppressions": 10
  }
}
```

If the count exceeds the threshold, the pipeline fails -- forcing the team to
resolve old suppressions before adding new ones.

---

**Previous:** [CI Integration](/anvil/tutorials/ci) | **All tutorials:**
[Tutorials index](/anvil/tutorials)
