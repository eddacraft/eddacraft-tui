---
id: policy-model
title: Policy packs
description:
  How policy packs are discovered, installed, validated, tested, and enforced.
  Policy is a gate check, not part of anvil check.
owner: DOCDEF
upstream:
  - crates/anvil-cli/src/commands/policy/mod.rs
  - crates/anvil-cli/src/commands/policy/install.rs
  - crates/anvil-cli/src/commands/policy/starter_packs/anvil-baseline
  - crates/anvil-cli/src/commands/exception.rs
  - crates/anvil-cli/src/commands/check_catalog.rs
  - plans/decisions/108-policy-authoring-lint-and-agent-guidance.md
verified_against: 0.9.6-beta
---

# Policy packs

**For:** teams adding project-specific policy to a gate

**Time:** 8 minutes

**Outcome:** know that policy is a gate check, how packs are installed, and
where authoring lives

## What you are looking at

A **policy** is a project rule evaluated by a **gate**. It is not a planless
`anvil check` engine. `anvil check` will not run `policy`, even if `policy`
appears in top-level `checks:`.

Policy is shipped as **packs**. The installer writes them under
`.anvil/policies/`. Begin with the bundled `anvil-baseline` pack rather than
writing a pack from scratch.

This page is the model. The happy path is the
[policy tutorial](../tutorials/policies.md). The command list is the
[policy command reference](../reference/policy.md).

## Pack lifecycle

| Step           | Command                           | Meaning                                                    |
| -------------- | --------------------------------- | ---------------------------------------------------------- |
| Discover packs | `anvil policy install --list`     | What can be installed. Not `anvil policy list`.            |
| Inspect a pack | `anvil policy show <pack>`        | What a pack contains, without writing files.               |
| Install        | `anvil policy install <pack>`     | Writes under `.anvil/policies/`.                           |
| Validate       | `anvil policy validate <path>`    | Manifest and pack well-formedness.                         |
| Test           | `anvil policy test [path]`        | Pack tests. Path is optional.                              |
| Enforce        | `anvil gate --only-checks policy` | Policy is a **gate** check. `anvil check` will not run it. |
| Exceptions     | `anvil exception`                 | Recorded exceptions to a policy finding.                   |

The shipped starter pack is `anvil-baseline`. It is advisory starter guardrails
over the working-tree diff: it flags large change sets, and it flags changes to
secrets or CI configuration for review. Success of
`anvil policy install anvil-baseline` reports files created under
`.anvil/policies/anvil-baseline/`.

Public docs name the pack and the install path. They do not reproduce pack
source as an authoring tutorial.

## Exceptions

`anvil exception` records a scoped, attributed exception to a policy finding.
Fixing the cause is preferred.

Typical lifecycle:

```text
anvil exception grant --policy <id> --reason "..."
anvil exception list
anvil exception show <id>
anvil exception revoke <id> --reason "..."
anvil exception verify
```

An exception is not a compiled anti-pattern suppression, and it is not the
[introduction baseline](baseline.md).

## How to author a pack

This site does not publish a pack-writing workshop.

To author or extend a pack, install the `authoring-anvil-policy` skill and use
the CLI or MCP-routed guidance it points at. Those commands exist so the
authoring corpus stays version-matched to the binary rather than copied here.

## What these commands are not

- `anvil policy list` and `anvil policy explain` render the compiled
  anti-pattern and architecture **rule catalogue**. They are not pack discovery
  and not the authoring door. Discover packs with `anvil policy install --list`.
  Look up rules in the [compiled pattern catalogue](../reference/rules.md).
- `anvil policy diff` compares two policy files line by line. It is not
  pack-authoring help.

## Related definitions

- [Policy command reference](../reference/policy.md)
- [Check catalogue: `policy`](../reference/checks.md#policy)
- [How anvil evaluates a project](evaluation-model.md)
- [Introduction baseline](baseline.md)
- [Policy tutorial](../tutorials/policies.md)
- [Review capsules](review-capsules.md)
