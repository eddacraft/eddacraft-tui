---
id: policy
title: Policy command reference
description:
  Pack lifecycle commands and the shipped evaluation surfaces. Policy is a gate
  check; anvil check will not run it.
owner: DOCDEF
upstream:
  - crates/anvil-cli/src/commands/policy/mod.rs
  - crates/anvil-cli/src/commands/policy/members.rs
  - crates/anvil-cli/src/commands/exception.rs
  - crates/anvil-policy-engine/src/pack/overlay.rs
verified_against: 0.9.7-beta
---

# Policy command reference

This is the command list for policy **packs**. For the model, read
[policy model](../concepts/policy-model.md). For the happy path, follow the
[policy tutorial](../tutorials/policies.md).

Policy is a **gate** check (`policy`). `anvil check` will not run it.

## Pack commands

| Command                                      | Purpose                                                 |
| -------------------------------------------- | ------------------------------------------------------- |
| `anvil policy install --list`                | List bundled packs that can be installed.               |
| `anvil policy show <pack>`                   | Preview a bundled pack without writing files.           |
| `anvil policy install <pack>`                | Install a bundled pack under `.anvil/policies/`.        |
| `anvil policy install <pack> --off <member>` | Install and disable named members in the overlay.       |
| `anvil policy members <pack>`                | List overlay-selected members of an installed pack.     |
| `anvil policy members <pack> --off/--on`     | Toggle members. The overlay survives `install --force`. |
| `anvil policy validate <path>`               | Check manifest, metadata, structure, and tests.         |
| `anvil policy test [path]`                   | Run the pack's included tests. Path is optional.        |
| `anvil gate --only-checks policy`            | Enforce installed packs as the `policy` gate check.     |

Bundled packs:

| Pack                     | Role                                                                                                                                   |
| ------------------------ | -------------------------------------------------------------------------------------------------------------------------------------- |
| `anvil-baseline`         | Advisory starter guardrails, not a compliance programme. See the [policy tutorial](../tutorials/policies.md) for known gaps.           |
| `anvil-control-examples` | Engineering-control templates. Not privacy, GDPR, or AI Act certification. `crypto-human-signoff` hard-stops MCP writes until a grant. |

## Exceptions

| Command                                               | Purpose                                      |
| ----------------------------------------------------- | -------------------------------------------- |
| `anvil exception grant --policy <id> --reason <text>` | Record a scoped, attributed exception.       |
| `anvil exception list`                                | List tracked exceptions and their verdicts.  |
| `anvil exception show <id>`                           | Show one exception in full.                  |
| `anvil exception revoke <id> --reason <text>`         | Revoke a grant and keep the audit trail.     |
| `anvil exception verify`                              | Verify scope, expiry, revocation, and owner. |

## Evaluation surfaces

These commands are shipped evaluation and regression surfaces. They are not a
pack-writing tutorial.

| Command                          | Purpose                                                                 |
| -------------------------------- | ----------------------------------------------------------------------- |
| `anvil policy eval`              | Evaluate a policy file against an input document.                       |
| `anvil policy eval-regression`   | Run eval suites and report regressions against a persisted eval record. |
| `anvil policy attack-regression` | Run a prompt-attack pack and report a fail-policy verdict.              |
| `anvil policy probe-trends`      | Show adversarial probe pass/fail trends by category.                    |

## Other shipped commands

| Command                | Purpose                                                                              |
| ---------------------- | ------------------------------------------------------------------------------------ |
| `anvil policy diff`    | Line-oriented diff of two policy files. Not pack-authoring help.                     |
| `anvil policy list`    | Compiled anti-pattern and architecture **rule catalogue**. Not pack discovery.       |
| `anvil policy explain` | Explains a rule or architecture id from that same catalogue. Not the authoring door. |

Look up those catalogue entries in the [compiled pattern catalogue](rules.md).
Discover packs with `anvil policy install --list`.

## Related definitions

- [Policy model](../concepts/policy-model.md)
- [Check catalogue: `policy`](checks.md#policy)
- [How anvil evaluates a project](../concepts/evaluation-model.md)
- [Introduction baseline](../concepts/baseline.md)
- [CLI command reference](cli.md)
