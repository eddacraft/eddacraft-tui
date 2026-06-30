---
id: overview
title: What anvil Does
description:
  anvil catches architecture drift and AI anti-patterns at save-time, before
  they reach code review.
sidebar_position: 1
---

# What anvil Does

anvil is a deterministic development automation platform that makes AI-generated
code changes safe for production.

## The Problem anvil Solves

AI coding assistants produce code that compiles and passes tests. But they also:

- **Drift from architecture** — introducing dependency edges that violate your
  intended boundaries
- **Introduce anti-patterns** — broad `eslint-disable`, explicit `any` types,
  empty catch blocks
- **Erode quality gradually** — each small compromise compounds over time

Code review _should_ catch these issues. But:

- Reviewers are overwhelmed by AI-generated volume
- Architectural violations are subtle and easy to miss
- By the time issues reach review, the cognitive load to fix them is high

## How anvil Works

anvil validates changes **before they land**—at the AI's pre-write point when
that path is wired, and at save-time as a fallback otherwise.

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│   Change    │ ──▶ │    Gate     │ ──▶ │  Evidence   │
│  (AI/Human) │     │ (Validate)  │     │  (Audit)    │
└─────────────┘     └─────────────┘     └─────────────┘
```

The important distinction is:

- **checks** evaluate one concern
- **findings** are the results emitted by checks
- **gates** are the workflow judgement over one or more checks

That is why `anvil check` and `anvil gate` both exist. `check` is best for
targeted analysis; `gate` is best when you need to know whether work can
advance.

### 1. `anvil start` — Activation

`anvil start` is the activation entrypoint. From a fresh install, the canonical
first minute is:

```bash
anvil start
```

The activator wires Cursor and Claude Code MCP entries, baselines the repo, and
ends in one literal `ProtectionState` (`protecting`, `ready_restart_required`,
`watching`, `needs_action`, `unsupported`, or `error`). When MCP is reachable,
`anvil_validate_write` runs in front of the AI's writes — backed by the local
Anvil daemon over owner-only IPC, with the embedded scanner as a
correctness-equivalent fallback. Pass `--verify` for a read-only probe that
prints the same diagnostic without writing config.

### 2. Watch Mode (save-time fallback)

When MCP cannot pre-write attach, watch mode is the **save-time fallback**. It
is honest fallback only, never claimed equivalent to MCP pre-write interception:

```bash
anvil start --watch    # activation + fallback handoff
anvil watch            # standalone watcher
```

### 3. Gate Validation

When files change, anvil runs quality gates:

- **Architecture boundaries** — catches new dependency edges crossing contexts
- **Anti-pattern detection** — 18 registry-driven rules across 5 families (15
  default, 3 opt-in), scanned in parallel by the Rust engine (tens of artifacts
  at once)
- **Policy evaluation** — custom rules via OPA/Rego
- **Secret detection** — pattern + entropy analysis

### 4. Immediate Feedback

Findings surface instantly in your terminal or editor—not in a PR comment hours
later.

```
  ⚠ [AP-003] Explicit any type usage
    src/utils/parser.ts:42
    Consider using a more specific type or generic.
```

### 5. Evidence Trail

Every validation run produces evidence: which checks ran, what findings were
emitted, what passed, what failed, and when.

## Key Features

| Feature                      | Description                                                                                                                                                   |
| ---------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **`anvil start` Activation** | Wires Cursor / Claude Code MCP entries, baselines, and prints one literal protection state                                                                    |
| **Daemon-backed MCP**        | `anvil_validate_write` runs pre-write through the local daemon; embedded fallback                                                                             |
| **Architecture Safety**      | Detects dependency violations using import analysis                                                                                                           |
| **Anti-Pattern Library**     | 18 registry-driven rules (15 default, 3 opt-in)                                                                                                               |
| **Parallel Scan Engine**     | Rust scanner runs tens of artifacts concurrently                                                                                                              |
| **Watch Mode (fallback)**    | Save-time fallback when MCP pre-write cannot attach                                                                                                           |
| **Repo language profile**    | TS / JS / Rust / Python fully supported; Go, Java, Kotlin, C#, C/C++, Dart, Zig & WebAssembly-text parsed; SQL partial; out-of-scope languages named honestly |
| **Suppression System**       | Allow exceptions with mandatory explanations                                                                                                                  |
| **GitHub Integration**       | PR checks and inline comments                                                                                                                                 |

## Anti-Patterns Detected

Anvil ships 18 rules organised into five **families**, each representing a
shared meta-issue:

- **guardrail-suppression** — disabling tools that were there to help (AP-001,
  AP-002, AP-004, AP-005, GS-001)
- **type-system-evasion** — escape hatches around the type system (AP-003)
- **error-visibility** — hiding failures that should surface (AP-006, AP-007)
- **responsibility-laundering** — shifting blame or deferring review
  (RL-001..RL-006)
- **deferred-debt** — recording work the author won't do now (DD-001..DD-004)

### Default patterns (enabled out of the box)

| ID     | Family                    | Pattern                                  | Severity |
| ------ | ------------------------- | ---------------------------------------- | -------- |
| AP-001 | guardrail-suppression     | Broad `eslint-disable`                   | warning  |
| AP-003 | type-system-evasion       | Explicit `any` type                      | warning  |
| AP-004 | guardrail-suppression     | `@ts-ignore` directive                   | warning  |
| AP-006 | error-visibility          | Empty catch block                        | warning  |
| GS-001 | guardrail-suppression     | Non-null assertion overrides nullability | warning  |
| RL-001 | responsibility-laundering | Unverified "pre-existing" claim          | warning  |
| RL-002 | responsibility-laundering | Phantom follow-up tracking               | warning  |
| RL-003 | responsibility-laundering | Blanket unrelated dismissal              | error    |
| RL-004 | responsibility-laundering | Unverified "not touched" claim           | warning  |
| RL-005 | responsibility-laundering | Deferred without artifact                | warning  |
| RL-006 | responsibility-laundering | Reply disguised as fix                   | info     |
| DD-001 | deferred-debt             | TODO/FIXME without tracking reference    | warning  |
| DD-002 | deferred-debt             | HACK comment without tracking reference  | warning  |
| DD-003 | deferred-debt             | Temporary code without expiry            | info     |
| DD-004 | deferred-debt             | Completion claim with outstanding TODOs  | warning  |

### Opt-in patterns

Enable these in your `.anvilrc` when relevant to your project.

| ID     | Family                | Pattern                         | Severity |
| ------ | --------------------- | ------------------------------- | -------- |
| AP-002 | guardrail-suppression | Rule-specific `eslint-disable`  | info     |
| AP-005 | guardrail-suppression | `@ts-expect-error` directive    | info     |
| AP-007 | error-visibility      | Console statement in production | info     |

> HTML and CSS anti-patterns (formerly AP-008..AP-013) were retired because
> dedicated linters — HTMLHint, Stylelint — cover that territory better. See
> `docs/vision/anvil-scope-guard.md` for the scope guardrail.

## What anvil Doesn't Do

anvil is focused. It doesn't:

- **Run your tests** — use your existing test runner
- **Format your code** — use your existing formatter and linter
- **Deploy your code** — use your existing CI/CD
- **Replace code review** — it augments review, not replaces it

anvil catches _structural_ and _architectural_ issues that other tools miss.

For the full explanation of checks, findings, and gates, see
[Understand gates](/anvil/concepts/gates).

---

**Ready to start?** [Go to the quickstart →](/anvil/quickstart)

anvil is currently in beta — the latest tagged release is `v0.8.1-beta`. See the
[beta testing guide](/anvil/beta-testing-guide) for what to expect. If you're
upgrading from `0.6.x`, read the
[0.7.0-beta upgrade notes](releases/upgrade-notes.md#upgrading-to-070-beta)
first, then apply the later patch notes through `0.7.2-beta`.

### v0.7.x-beta operator surfaces

`v0.7.0-beta` is the **daemon-working** cut; `v0.7.1-beta` is the activation
honesty patch on top. Every protection layer (hooks, witness chain, baseline
adoption, L4 policy, wrapped agent launch) defines a single typed
`ProtectionClaim` rendered on `anvil status --json`, `anvil doctor --json`, the
`anvil_validate_write` MCP-tool response when the daemon is reachable, and the
TypeScript driver-client. Windows MCP protection-claim parity landed in
`v0.7.1-beta`. The runbooks below are the source of truth — public docs
deep-link rather than duplicate them:

- [`anvil-run`](https://github.com/eddacraft/anvil-001/blob/main/docs/runbooks/anvil-run.md)
  — wrapped-launch ingress for `claude`, `codex`, `aider`, and similar agents.
- [`anvil l4-validate`](https://github.com/eddacraft/anvil-001/blob/main/docs/runbooks/v0.7.0-beta-release-runbook.md)
  — dedicated L4-policy validator for CI consumers.
- Protection-claim contract — the `ProtectionClaim` shape and the four surfaces
  it renders on.
- `--new-identity` — fork opt-out on `anvil start` / `anvil baseline`.
- [`--acknowledge-cascade`](https://github.com/eddacraft/anvil-001/blob/main/docs/runbooks/v0.7.0-beta-release-runbook.md)
  — clear a rate-limited fence cascade after the incident is resolved.
- [Witness chain](https://github.com/eddacraft/anvil-001/blob/main/docs/runbooks/anvil-witness-chain.md)
  and
  [hook coexistence](https://github.com/eddacraft/anvil-001/blob/main/docs/runbooks/anvil-hook-coexistence.md)
  — in-tree `anvil/witness/` content and integration under lefthook / husky /
  pre-commit-framework.
