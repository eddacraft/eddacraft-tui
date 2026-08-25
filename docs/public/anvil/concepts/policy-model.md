---
id: policy-model
title: Policy model
description:
  The seven surfaces that change policy behaviour, how they relate, and how
  policy packs are discovered, installed, validated, tested, and enforced.
  Policy is a gate check, not part of anvil check.
owner: DOCDEF
upstream:
  - crates/anvil-cli/src/commands/policy/mod.rs
  - crates/anvil-cli/src/commands/policy/install.rs
  - crates/anvil-cli/src/commands/policy/starter_packs/anvil-baseline
  - crates/anvil-cli/src/commands/policy/starter_packs/anvil-control-examples
  - crates/anvil-policy-engine/src/pack/overlay.rs
  - crates/anvil-cli/src/commands/exception.rs
  - crates/anvil-cli/src/commands/check_catalog.rs
  - plans/decisions/108-policy-authoring-lint-and-agent-guidance.md
  - plans/decisions/129-policy-surface-inventory-and-precedence.md
  - plans/decisions/131-registry-override-explicit-only.md
verified_against: 0.9.7-beta
---

# Policy model

**For:** teams who need to know every place policy behaviour can change

**Time:** 10 minutes

**Outcome:** know the seven shipped surfaces, that they are complementary layers
rather than a single stack, and how packs are installed and enforced

## What you are looking at

A **policy** is a project rule evaluated by a **gate**. It is not a planless
`anvil check` engine. `anvil check` will not run `policy`, even if `policy`
appears in top-level `checks:`.

Policy is shipped as **packs**. The installer writes them under
`.anvil/policies/`. Begin with the bundled `anvil-baseline` pack rather than
writing a pack from scratch.

Packs are one of seven shipped surfaces. The others change what anvil flags or
how it acts on a flag, but they are not policy packs. Precedence between them is
complementary layers, not a single winner-takes-all stack.

This page is the model. The happy path is the
[policy tutorial](../tutorials/policies.md). The command list is the
[policy command reference](../reference/policy.md).

## Surfaces

| Surface                          | Where                                                | Role                                                                                   |
| -------------------------------- | ---------------------------------------------------- | -------------------------------------------------------------------------------------- |
| Rule modes                       | `.anvil.yaml` `enforcement.rules`                    | Stored per-rule off/warn/enforce for four named rules. Not a live evaluator control    |
| Code-policy packs                | `.anvil/policies/`                                   | Project rules evaluated as the `policy` gate check                                     |
| Architecture definition          | `architecture` section or `.anvil/architecture.yaml` | Layer and import constraints. Not a pack. See [architecture boundaries](boundaries.md) |
| Acceptance policy                | `anvil/policy.*`                                     | Whether a commit or push is accepted                                                   |
| Intercept-rule registration      | `.anvil.yaml` `enforcement.intercept-rules`          | Which save-time intercept rules run                                                    |
| Enforcement posture              | `.anvil.yaml` `enforcement.mode`                     | How strictly a block-worthy finding is acted on                                        |
| Anti-pattern registry resolution | `ANVIL_REGISTRY_PATH` (or API path)                  | Scanner catalogue. Default is the binary's embedded pack                               |

The anti-pattern catalogue is the pack baked into the binary. Setting
`ANVIL_REGISTRY_PATH` replaces it unsigned. A cloned
`patterns/compiled/registry.json` does not.

`.anvil.yaml` stands for the canonical project config (`.anvil.yaml` / `.yml` /
`.json` / `.toml`). Discovery is yaml-first. Three names that are not synonyms:
`anvil/policy.*` is commit-acceptance policy; `.anvil/policies/` is pack files;
`anvil policy` is the pack CLI (and, for `list`/`explain`, the compiled rule
catalogue). Packs do not accept or reject a push.

None of these is deprecated. They answer different questions:

- **Catalogue** — what exists to fire (packs, architecture, intercept
  registration, anti-pattern registry).
- **Posture** — how strictly a finding is acted on (`enforcement.mode`; rule
  modes for four named rules only).
- **Lifecycle** — when it fires (save-time intercept / MCP, `anvil check`,
  `anvil gate`, push acceptance).

A pack finding and an architecture finding are both findings. Neither cancels
the other. `enforcement.mode` does not unload packs. `enforcement.rules` does
not override pack outcomes. Commit-acceptance `on_block` is not an
enforcement-mode synonym.

The overlap rules are: different questions do not merge; `enforcement.mode` is
action-time posture, not a catalogue switch; `enforcement.rules` covers four
named rules only and is a stored mode, not a live evaluator control;
`anvil/policy.*` `on_block` is a commit-acceptance verb. If `enforcement.mode`
is unset, MCP pre-write is stricter than save-time intercept. Per-key catalogue
for config fields is the [config reference](../reference/config.md).

## Pack lifecycle

| Step           | Command                           | Meaning                                                               |
| -------------- | --------------------------------- | --------------------------------------------------------------------- |
| Discover packs | `anvil policy install --list`     | What can be installed. Not `anvil policy list`.                       |
| Inspect a pack | `anvil policy show <pack>`        | What a pack contains, without writing files.                          |
| Install        | `anvil policy install <pack>`     | Writes under `.anvil/policies/`. `--off <member>` writes the overlay. |
| Members        | `anvil policy members <pack>`     | List overlay state; `--off` / `--on` toggle members.                  |
| Validate       | `anvil policy validate <path>`    | Manifest and pack well-formedness.                                    |
| Test           | `anvil policy test [path]`        | Pack tests. Path is optional.                                         |
| Enforce        | `anvil gate --only-checks policy` | Policy is a **gate** check. `anvil check` will not run it.            |
| Exceptions     | `anvil exception`                 | Recorded exceptions to a policy finding.                              |

Two bundled packs ship in the binary:

| Pack                     | Role                                                                                                                                                                |
| ------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `anvil-baseline`         | Advisory starter guardrails over the working-tree diff: large change sets, and secrets or CI paths. Findings are warnings and never fail the gate.                  |
| `anvil-control-examples` | Engineering-control templates for custom policy authoring. Four members; `crypto-human-signoff` is a hard stop on MCP pre-write until a human records an exception. |

Neither pack is a compliance programme. They do not claim OWASP, SOC 2, ISO,
GDPR, or AI Act coverage. Broad framework packs are not shipped.

`anvil-control-examples` members can be selected independently. Selection lives
in `.anvil/policies/<pack>.overlay.yaml`, beside the pack directory so
`anvil policy install --force` cannot clobber it. Gate and MCP pre-write both
honour the overlay. Default is all members on.

`crypto-human-signoff` emits a blocking-intent finding. On MCP pre-write, with
the default interrupt posture, that vetoes the write until
`anvil exception grant --policy crypto-human-signoff` records a human. The other
three members emit warnings and never veto. The policy **gate** check still
reports those findings; it does not apply exception grants.

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
- [Architecture boundaries](boundaries.md)
- [Config reference](../reference/config.md)
- [How anvil evaluates a project](evaluation-model.md)
- [Introduction baseline](baseline.md)
- [Policy tutorial](../tutorials/policies.md)
- [Review capsules](review-capsules.md)
