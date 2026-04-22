# Authoring `.anvil` Rules

This guide walks through adding a new anti-pattern to Anvil's detection
catalogue. All rules live as `.anvil` files under `patterns/` and are compiled
into `patterns/compiled/registry.json` by `scripts/compile-patterns`.

`registry.json` is the **contract both scan engines consume**: the Rust scanner
in `crates/anvil-checks` (authoritative, per [ADR-026]) and the TypeScript
scanner in `packages/anvil/core/src/antipattern` (kept for the VSCode extension
and MCP server until the napi-rs migration lands). Authoring a rule means
editing the `.anvil` source and recompiling the registry — neither engine
carries its own hand-written pattern table.

[ADR-026]: ../../plans/decisions/026-rust-scanner-authoritative.md

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

Pick the family that best captures the _meta-issue_ your rule exemplifies. If
none fit, a new family is allowed — propose it in an ADR first.

## Rule ID conventions

| Prefix | Meaning                                    |
| ------ | ------------------------------------------ |
| `AP-`  | Legacy numbering, still used where natural |
| `GS-`  | Guardrail-suppression family (new rules)   |
| `RL-`  | Responsibility-laundering                  |
| `DD-`  | Deferred-debt                              |
| `TE-`  | Type-system-evasion (reserved)             |
| `EV-`  | Error-visibility (reserved)                |

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
spectrum_position: 3       # relative severity within the family (1 = most severe)

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
- `spectrum_position` — integer ordering within the family, where `1` is the
  most severe violation and higher numbers are milder variants. Matches the
  schema contract in `packages/anvil/core/src/antipattern/types.ts`
  (`1 = most severe`). Used by reviewers to reason about escalation.
- `targets` — which artifact kinds the rule scans. Most source-code rules are
  `[source]`; PR-description rules like the RL family use `[pr-description]` or
  combinations.
- `file_extensions` — scanner only invokes the rule for matching files. Omit for
  artifact targets like `pr-description`.
- `allowlist` — glob patterns where the rule is silent (e.g., test files for
  rules that are noisy in tests).
- `detection` — either a regex (`type: regex`, `pattern`, optional `flags`) or
  an AST query (`type: ast`, `ast_query`). Regex is far more common.
- `related` — other rule IDs worth reading alongside; shown to humans.
- `enabled` — `true` ships the rule; `false` keeps it in the registry but
  doesn't run it. Use this to stage rollout.
- `opt_in` — `true` requires `--include-opt-in`. Use for rules that are noisy in
  many codebases but valuable in specific ones.

### Body

The markdown body is the _rich definition_ — what a reviewer should read when
they want to understand the rule beyond the one-line title. The first short
paragraph becomes the inline `nudge`. Keep it action-oriented: "Use X instead of
Y", not "this is bad".

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
- `targets` / `severity` / `confidence` / `detection.type` are valid enum
  values.
- Regex patterns parse.

A failing compile step points at the offending file and field; fix and rerun.

### Engine compatibility

The Rust scanner uses the `regex` crate (RE2-style: no backtracking, no
lookaround). The TypeScript scanner uses V8's PCRE-ish engine, which supports
lookaround. Patterns that rely on lookaround compile fine for the TS engine but
not under `regex`. The Rust scanner handles this in two steps:

1. `flags: "i"` on the registry entry is honoured by the Rust loader via an
   inline `(?i)` prefix, so `RL-*` and `DD-004` match case-insensitively in both
   engines (SPG-001).
2. Lookaround-bearing rules (DD-001, DD-002, DD-003, GS-001, RL-001, RL-005) are
   translated into a Rust-side post-filter paired with a base regex in
   `crates/anvil-checks/src/antipattern/scanner.rs::prepare_pcre_rewrite`
   (SPG-003).

Any other rule whose pattern fails to compile in the Rust scanner is surfaced by
`anvil doctor` as a `registry-patterns-compile` warning (SPG-002) — the
silent-drop path is observable. The parity harness (`pnpm test:scanner-parity`)
covers every enabled rule with at least one positive and one negative fixture;
see `tests/scanner-parity/README.md` for the fixture format and the current
engine-handling map. When possible, rewrite to avoid lookaround; when not
possible, add a new arm to `prepare_pcre_rewrite` and a pair of fixtures, and
run both suites to confirm parity.

### Registry integrity

The compiled registry (`patterns/compiled/registry.json`) is a trust boundary.
The Rust loader resolves it in this order (see
`crates/anvil-checks/src/antipattern/registry_loader.rs::resolve_registry_path`):

1. `LoadRegistryOptions.registry_path` (explicit override, tests only).
2. The `ANVIL_REGISTRY_PATH` environment variable.
3. Upward walk from the current working directory.
4. Upward walk from the executable's directory (installed binaries).

**`ANVIL_REGISTRY_PATH` is a trust boundary.** If it is set, the scanner will
load whatever JSON lives at that path with no integrity check on the payload. A
poisoned registry — for example one with every rule's `enabled` flipped to
`false`, or with detection patterns rewritten to match nothing — silently
disables scanning without any other signal that something is wrong.

Treat the env var accordingly:

- **CI jobs should rely on the in-tree registry compiled at the checked-out
  SHA.** Do not set `ANVIL_REGISTRY_PATH` to a path outside the repo.
- **Never let external input (a PR body, a webhook payload, an untrusted build
  argument) flow into `ANVIL_REGISTRY_PATH`.** It is intended for local
  development overrides and test fixtures only.
- **Scanner self-check.** Running `anvil doctor` prints the
  `registry-patterns-compile` check; if a poisoned registry has also broken the
  regex shape, that check will flag the affected rules. It does not yet verify
  the registry's byte hash — a `--expect-registry-hash` CLI flag is a possible
  follow-up (ADR material, not part of this module). Compile checks detect
  _shape_ failures only. A registry that is syntactically valid but semantically
  poisoned — every rule's `enabled` flipped to `false`, every pattern rewritten
  to `.*` so nothing meaningful surfaces — will not be caught by this check. Pin
  the registry's hash in CI if that matters.
- **Symlinks.** `resolve_registry_path` does not canonicalise the env-var value.
  If `ANVIL_REGISTRY_PATH` points into a writable directory whose contents can
  be swapped by another process (e.g. a shared `/tmp`), that same write surface
  owns the scanner catalogue for the process's lifetime. Use absolute paths
  inside the repo or a trusted config root.

## Checklist before merge

- [ ] New `.anvil` file lives in the correct family directory.
- [ ] Rule ID uses the right prefix and is globally unique.
- [ ] `family`, `spectrum_position`, and `related` align with the family
      definition.
- [ ] `file_extensions` and `allowlist` match the scope (e.g., don't run a
      TS-only rule on `.html`).
- [ ] Registry recompiled and committed (`patterns/compiled/registry.json`).
- [ ] New assertion or snapshot in `patterns.test.ts` / `scanner.test.ts` if the
      rule introduces novel surface area.
- [ ] Public docs (`docs/public/anvil/overview.md`,
      `docs/public/anvil/concepts/gates.md`,
      `docs/public/anvil/operations/config.md`) list the new rule.

## Retiring a rule

Prefer `enabled: false` over deleting the file — keeps the rule ID reserved and
the narrative discoverable. Delete only when the rule is outside Anvil's scope
entirely (e.g., the HTML/CSS patterns retired in ANVFMT-014 per decision D-002).
