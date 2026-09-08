---
name: using-anvil
description: >-
  Use anvil to make AI-generated code safer through activation, checks, gates,
  watch mode, architecture boundaries, and CI validation. Use when the user
  asks about anvil setup, protection states, architecture drift, AI guardrails,
  anvil check/gate/watch/doctor, or anvil CI integration.
---

# Using anvil

Deterministic guardrail for AI-assisted development: architecture drift,
anti-patterns, policy, and secrets, before review.

In-session graph queries and `anvil_validate_write` → `anvil-developer-functions`.
Custom Rego / PolicyInput packs → current product docs, not this skill. Once
anvil reports `protecting`, load the companion for the edit loop.

## Terms

- **Check:** one concern (`secret-detection`, `import-boundaries`,
  `antipattern-scan`, `policy`, `lint`, `test`, `coverage`, `dependency`,
  `command-safety`).
- **Finding:** a check result.
- **Gate:** pass/fail over checks. `anvil check` analyses; `anvil gate` decides
  whether work can advance. Warning anti-patterns do not block a gate by
  default; errors do. `--fail-on-warnings` / `ANVIL_FAIL_ON_WARNINGS` only when
  the project chose that posture.

## First response

1. `anvil --version` or `anvil version`.
2. Setup matters → `anvil start --verify` or `anvil status --verify`.
3. Health → `anvil doctor`.
4. Findings → `anvil check --all`.
5. Advance or merge → `anvil gate --profile dev` (CI: `--profile ci`).

Do not claim the repo is protected until anvil reports that state.

## Activation

From the repo root: `anvil start` (init, baseline, MCP wiring).

| State                    | Meaning                                        |
| ------------------------ | ---------------------------------------------- |
| `protecting`             | MCP pre-write validation is live               |
| `ready_restart_required` | Restart the editor or daemon                   |
| `watching`               | Save-time watch fallback — not MCP intercept   |
| `needs_action`           | Follow the printed repair hint                 |
| `unsupported` / `error`  | Blocked — report diagnostics; do not improvise |

Read-only probe: `anvil start --verify`. Watch fallback: `anvil start --watch`.

## Commands

| Need         | Command                          |
| ------------ | -------------------------------- |
| Verify setup | `anvil start --verify`           |
| Health       | `anvil doctor`                   |
| Findings     | `anvil check --all`              |
| Staged files | `anvil check --changed --staged` |
| Dev gate     | `anvil gate --profile dev`       |
| CI gate      | `anvil gate --profile ci`        |
| Watch        | `anvil watch --source`           |
| JSON watch   | `anvil --json watch`             |

Narrow `check` when investigating one issue; `gate` when asking whether work
can proceed.

## Config

Reads `.anvilrc` or `.anvil.yaml` / `.yml` / `.json` / `.toml`. `anvil init`
creates one format; `anvil config show` is the effective config. Keep a single
project format unless installed help documents precedence.

Canonical check names: `secret-detection`, `import-boundaries`,
`antipattern-scan`, `policy`, `command-safety`.

`.anvil/gate-config.json` is planning only. Live gates use `.anvilrc#checks`
and `--only-checks` / `--skip-checks`.

Generated files: recognised generated names/directories, or `linguist-generated`
in `.gitattributes`. Extra globs under `antipattern.exclude` apply only to
anti-pattern scanning; secret detection still inspects generated files.

## Architecture

`.anvil/architecture.yaml` holds import-boundary layers and deps. Schema and
examples: [references/architecture.yaml.md](references/architecture.yaml.md).
Validate with `anvil architecture validate`. Prefer fixing code or architecture
over suppressions; if needed: `@anvil-ignore ARCH-001: <reason>`.

## CI

Staged: `anvil check --changed --staged`. Hooks: `anvil hooks install --config`.
CI: `anvil gate --profile ci`. Put `ANVIL_LICENSE` in the CI secret store — never
hardcode or print it.

## Repair

Not on `PATH` or managed skills broken: `anvil doctor`, then its
`anvil skill install` guidance — do not copy managed skill files by hand.
`ready_restart_required`: restart the editor. Daemon down: `anvil intercept status`,
then `anvil intercept start --foreground` if needed. Fenced worktree: follow anvil's
unblock guidance. `ANVIL_LOG=warn` for detail; do not dump secrets from logs.

## Guardrails

- Do not invent public commands such as `anvil session start`.
- Do not edit secret-bearing config or CI variables; use the secret store.
- anvil is not a linter, formatter, test runner, or deployer.
- Report the exact command; do not overstate protection.
