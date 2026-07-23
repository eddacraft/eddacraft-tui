---
name: using-anvil
description: >-
  Use anvil to make AI-generated code safer through activation, checks, gates,
  watch mode, architecture boundaries, and CI validation. Use when the user
  asks about anvil setup, protection states, architecture drift, AI guardrails,
  anvil check/gate/watch/doctor, or anvil CI integration.
---

# Using anvil

Use anvil as a deterministic guardrail for AI-assisted development. It catches
architecture drift, anti-patterns, policy issues, and secrets before they reach
review.

## Companion skills

| Need                                                              | Skill                          |
| ----------------------------------------------------------------- | ------------------------------ |
| Setup, CLI, checks, gates, watch, CI, light config                | **this skill** (`using-anvil`) |
| In-session graph queries and per-write MCP validation             | `anvil-developer-functions`    |
| Design/author custom policy packs (Rego, PolicyInput, pack tests) | Current product documentation  |

Once anvil reports `protecting`, load **`anvil-developer-functions`** for the
graph-context tools and the `anvil_validate_write` per-write loop; this skill
owns installation, activation, and CLI checks — not the in-agent edit loop and
not custom policy-pack design.

## Mental model

Keep these terms distinct:

- **Check:** evaluates one concern, such as `secret-detection`, `import-boundaries`, `antipattern-scan`, `policy`, `lint`, `test`, `coverage`, `dependency`, or `command-safety`.
- **Finding:** a result emitted by a check, such as a boundary violation or explicit `any` usage.
- **Gate:** the workflow judgement over one or more checks; use it to decide whether work can advance.

Use `anvil check` for targeted analysis. Use `anvil gate` when the user needs pass/fail workflow judgement.

## First response workflow

When helping with anvil in a repo:

1. Check whether `anvil` is available with `anvil --version` or `anvil version`.
2. If setup state matters, run `anvil start --verify` or `anvil status --verify` before changing config.
3. Use `anvil doctor` for environment health and setup diagnostics.
4. Use `anvil check --all` to inspect current findings.
5. Use `anvil gate --profile dev` or a narrower gate when deciding whether work can proceed.

Do not claim anvil is protecting a repo until anvil reports the protection state.

## Activation

From the repository root, the normal activation path is:

```bash
anvil start
```

`anvil start` initializes when needed, baselines the repo, wires supported MCP entries, and ends in one protection state:

| State                    | Meaning                                               |
| ------------------------ | ----------------------------------------------------- |
| `protecting`             | MCP pre-write validation is live                      |
| `ready_restart_required` | Config is wired but the editor or daemon needs action |
| `watching`               | Save-time watch fallback is active                    |
| `needs_action`           | Follow the printed repair hint                        |
| `unsupported`            | Repo language profile is outside current scope        |
| `error`                  | Read diagnostic output and troubleshoot               |

An `unsupported` or persistent `error` state is a blocked outcome, not a prompt
to improvise. Report the state and its diagnostic output to the user and stop;
do not work around the protection layer with ad-hoc scripts or config edits.

For a read-only probe:

```bash
anvil start --verify
```

To activate and enter save-time fallback:

```bash
anvil start --watch
```

Watch mode is a fallback, not equivalent to MCP pre-write interception.

## Choosing commands

| Need                                | Command                          |
| ----------------------------------- | -------------------------------- |
| Verify setup without writing config | `anvil start --verify`           |
| Inspect environment health          | `anvil doctor`                   |
| Surface source findings             | `anvil check --all`              |
| Scan staged files                   | `anvil check --changed --staged` |
| Decide whether work can advance     | `anvil gate --profile dev`       |
| Run a CI gate                       | `anvil gate --profile ci`        |
| Continuous save-time validation     | `anvil watch --source`           |
| Machine-readable watch stream       | `anvil --json watch`             |

Prefer narrow commands when the user is investigating one issue. Prefer `gate` when the user asks whether the repo is safe to merge, commit, or proceed.

## Configuration

anvil's active project settings live in `.anvilrc`. `anvil init` creates it.
**YAML is the default**; **TOML and JSON are also supported** (same schema,
different encoding — use whichever the project already standardises on).

Example (YAML):

```yaml
schemaVersion: "1.0.0"
planningDir: plans
format: yaml
checks:
  - secret-detection
  - import-boundaries
  - antipattern-scan
```

Use canonical check names in docs, plans, and commands:

- `secret-detection`
- `import-boundaries`
- `antipattern-scan`
- `policy`
- `command-safety`

`.anvil/gate-config.json` is a planning surface for gate composition, but current gate runs are controlled by `.anvilrc#checks` and CLI flags such as `--only-checks` or `--skip-checks`.

Custom **Rego packs**, PolicyInput rules, pack manifests, and pack test design
are outside this skill. Use only current anvil product and repository
documentation; do not infer an authoring schema or command. Prefer fixing code
or architecture over broad suppressions. This skill only covers light
configuration and the architecture YAML sketch below.

## Architecture boundaries

Use `.anvil/architecture.yaml` for **import-boundary** rules (layers and
deps). For domain invariants that need custom Rego or pack admission, stop this
workflow and follow the current product's documented policy-authoring path.

A layer declares the files it owns and which layers it may depend on:

```yaml
schema_version: "0.1.0"
template: custom
layers:
  api-layer:
    patterns:
      - "src/api/**"
    depends_on:
      - service-layer
      - utils

  service-layer:
    patterns:
      - "src/services/**"
    depends_on:
      - repository-layer
      - utils

  repository-layer:
    patterns:
      - "src/repositories/**"
    depends_on:
      - utils

  utils:
    patterns:
      - "src/utils/**"
    depends_on: []
```

Validate architecture config before relying on it:

```bash
anvil architecture validate
```

If a boundary violation is intentional, prefer fixing the architecture or code. Suppress only with a clear explanation, such as `@anvil-ignore ARCH-001: Legacy pattern, will refactor in Q2`.

## CI and hooks

For pre-commit staged-file checks:

```bash
anvil check --changed --staged
```

For managed hook installation on supported Git versions, use anvil's hook installer when appropriate:

```bash
anvil hooks install --config
```

For CI, install anvil and run a CI profile gate:

```bash
anvil gate --profile ci
```

If beta access or licensed checks are required in CI, use `ANVIL_LICENSE` from the CI secret store. Do not hardcode or print the license value.

## Troubleshooting

If `anvil` is not found:

- Confirm the binary is on `PATH`.
- On macOS/Linux standalone installs, check `~/.eddacraft/bin`.
- If installed through Homebrew, run `brew link eddacraft/tap/anvil` or `brew upgrade eddacraft/tap/anvil`.

If `ready_restart_required` persists:

- Restart Cursor or Claude Code so MCP config is picked up.
- If the daemon is unreachable, run `anvil intercept start --foreground` in another terminal and retry verification.
- Run `anvil intercept status` to distinguish daemon-down, stale, and unenforced-worktree states.
- If a worktree is fenced, follow anvil's unblock guidance instead of deleting state blindly.

Use `ANVIL_LOG=warn` when diagnostic detail is needed, but avoid collecting or exposing secrets from logs.

## Guardrails

- Do not present planned session commands such as `anvil session start` as available public CLI features.
- Do not edit secret-bearing config or CI variables directly; use the user's secret manager or CI secret store.
- Do not treat anvil as a linter, formatter, test runner, or deployment tool. It complements those tools.
- Always report the exact command used and summarize findings without overstating protection status.
