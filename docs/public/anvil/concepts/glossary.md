---
id: glossary
title: Glossary
description:
  Plain-language definitions for the terms used in the anvil documentation.
owner: DOCSYNC
upstream:
  - crates/anvil-cli/src/activation/state.rs
  - crates/anvil-cli/src/commands/ensure.rs
verified_against: 0.9.0-beta
---

# Glossary

You can use anvil without memorising these terms. Return here when a command or
guide introduces an unfamiliar word.

| Term                     | Meaning                                                                                                                                        |
| ------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------- |
| **Baseline**             | Findings accepted when anvil was introduced. Not a check and not the `anvil-baseline` pack.                                                    |
| **Boundary**             | A declared structural dependency constraint. Prefer this word over "architecture" in ordinary use.                                             |
| **Check**                | One deterministic analysis, such as secret detection or architecture validation.                                                               |
| **Finding**              | A check result that names what was detected, where, and why it matters.                                                                        |
| **Gate**                 | A workflow decision made from one or more checks.                                                                                              |
| **Protection state**     | The final status reported by activation: protecting, restart required, watching, needs action, unsupported, or error.                          |
| **Daily ensure**         | Bare `anvil` (no subcommand): turn protection on for an already-activated project without reinstalling clients or hooks.                       |
| **Pre-write validation** | A supported AI client asks anvil to validate a proposed change before writing it.                                                              |
| **Save-time validation** | A local watcher validates a file after it is saved. It is a fallback, not the same guarantee as pre-write validation.                          |
| **Daemon**               | A per-user background process that serves local validation requests.                                                                           |
| **MCP**                  | Model Context Protocol, a standard connection used by supported AI clients to call local tools.                                                |
| **Policy**               | A project rule evaluated by a gate.                                                                                                            |
| **Suppression**          | A narrow, explained exception to a finding. Fixing the cause is preferred.                                                                     |
| **SARIF**                | A standard JSON format used by code-analysis tools and CI systems.                                                                             |
| **Witness**              | Local evidence that a protected workflow ran for a change.                                                                                     |
| **Review capsule**       | A portable bundle of governance evidence for a commit range.                                                                                   |
| **TUI**                  | Terminal user interface: an interactive screen drawn inside a terminal.                                                                        |
| **Audit**                | A broader exploratory report over findings (`anvil audit`). It is not a merge decision and not a substitute for `anvil check` or `anvil gate`. |
| **Config**               | The project file `.anvil.yaml` (also `.yml` / `.json` / `.toml`). Legacy `.anvilrc` is a read-only fallback.                                   |
| **Rule**                 | One compiled anti-pattern pattern. Rules belong to the `antipattern-scan` check; they are not themselves checks.                               |
| **Scan**                 | How evidence is gathered for a check. It is not a second product object and not a command you choose instead of `check` or `gate`.             |

## Next step

Return to [what anvil does](../overview.md) or
[install and get first value](../quickstart.md).

## Related definitions

- [How anvil evaluates a project](evaluation-model.md)
- [What anvil can do](../reference/what-anvil-can-do.md)
