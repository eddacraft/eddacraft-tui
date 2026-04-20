# Authoring `.anvil` Rules

This guide walks through adding a new anti-pattern to Anvil's detection
catalogue. All rules live as `.anvil` files under `patterns/` and are
compiled into `patterns/compiled/registry.json` by `scripts/compile-patterns`.

## Layout

```
patterns/
  <family>/
    definition.anvil      # family-level narrative (why these rules group)
    <RULE-ID>.anvil       # individual rule
```

Existing families:

- `guardrail-suppression` — disabling tools that were there to help
- `type-system-evasion` — escape hatches around the type system
- `error-visibility` — hiding failures that should surface
- `responsibility-laundering` — shifting blame or deferring review
- `deferred-debt` — recording work the author won't do now

Pick the family that best captures the *meta-issue* your rule exemplifies.
If none fit, a new family is allowed — propose it in an ADR first.

## Rule ID conventions

| Prefix | Meaning                                  |
| ------ | ---------------------------------------- |
| `AP-`  | Legacy numbering, still used where natural |
| `GS-`  | Guardrail-suppression family (new rules)  |
| `RL-`  | Responsibility-laundering                 |
| `DD-`  | Deferred-debt                             |
| `TE-`  | Type-system-evasion (reserved)            |
| `EV-`  | Error-visibility (reserved)               |

Pick the next free three-digit suffix within the prefix (`GS-002`, `RL-007`,
etc.). Rule IDs are globally unique across the registry.

## Rule file shape

```yaml
---
id: GS-002
type: rule
family: guardrail-suppression
title: Short descriptive title
version: 1

severity: warning          # error | warning | info
confidence: medium         # high | medium | low
spectrum_position: 3       # relative severity within the family (1 = mildest)

targets: [source]          # source | pr-description | commit-message | agent-output
file_extensions: [.ts, .tsx]
allowlist: ['**/*.test.ts', '**/__tests__/**']

detection:
  type: regex              # or ast
  pattern: 'your regex'
  flags: i                 # optional; `g` is always added

related: [AP-004]
enabled: true
opt_in: false
---

Narrative markdown body. Two paragraphs max:

1. **What to do instead** — concrete, short, action-focused. This is what
   becomes the `nudge` surfaced to the author / reviewer.
2. **Why it matters** — the reasoning, for humans and AI reviewers who
   navigate from the warning to the family definition.
```

### Fields

- `id` — globally unique rule ID (see prefix table above).
- `family` — must match an existing directory name under `patterns/`.
- `severity` / `confidence` — consumed by the scanner and gate runner.
- `spectrum_position` — integer ordering within the family. Lower numbers
  are "milder" violations; higher numbers are "stronger". Used by reviewers
  to reason about escalation.
- `targets` — which artifact kinds the rule scans. Most source-code rules
  are `[source]`; PR-description rules like the RL family use
  `[pr-description]` or combinations.
- `file_extensions` — scanner only invokes the rule for matching files.
  Omit for artifact targets like `pr-description`.
- `allowlist` — glob patterns where the rule is silent (e.g., test files
  for rules that are noisy in tests).
- `detection` — either a regex (`type: regex`, `pattern`, optional `flags`)
  or an AST query (`type: ast`, `ast_query`). Regex is far more common.
- `related` — other rule IDs worth reading alongside; shown to humans.
- `enabled` — `true` ships the rule; `false` keeps it in the registry
  but doesn't run it. Use this to stage rollout.
- `opt_in` — `true` requires `--include-opt-in`. Use for rules that are
  noisy in many codebases but valuable in specific ones.

### Body

The markdown body is the *rich definition* — what a reviewer should read
when they want to understand the rule beyond the one-line title. The first
short paragraph becomes the inline `nudge`. Keep it action-oriented:
"Use X instead of Y", not "this is bad".

## Compile and verify

```bash
# Compile .anvil sources into registry.json
pnpm --filter @eddacraft/anvil-core patterns:compile

# Run the core test suite (expects the new rule to appear)
pnpm --filter @eddacraft/anvil-core test

# Optional: typecheck
pnpm -r typecheck
```

The compiler validates:

- Each rule references an existing family definition.
- No two rules share an `id`.
- `targets` / `severity` / `confidence` / `detection.type` are valid enum values.
- Regex patterns parse.

A failing compile step points at the offending file and field; fix and rerun.

## Checklist before merge

- [ ] New `.anvil` file lives in the correct family directory.
- [ ] Rule ID uses the right prefix and is globally unique.
- [ ] `family`, `spectrum_position`, and `related` align with the family
      definition.
- [ ] `file_extensions` and `allowlist` match the scope (e.g., don't run
      a TS-only rule on `.html`).
- [ ] Registry recompiled and committed
      (`patterns/compiled/registry.json`).
- [ ] New assertion or snapshot in `patterns.test.ts` / `scanner.test.ts`
      if the rule introduces novel surface area.
- [ ] Public docs (`docs/public/anvil/overview.md`,
      `docs/public/anvil/concepts/gates.md`,
      `docs/public/anvil/operations/config.md`) list the new rule.

## Retiring a rule

Prefer `enabled: false` over deleting the file — keeps the rule ID
reserved and the narrative discoverable. Delete only when the rule is
outside Anvil's scope entirely (e.g., the HTML/CSS patterns retired in
ANVFMT-014 per decision D-002).
