---
id: tutorials
title: Tutorials
sidebar_position: 1
---

# Tutorials

Step-by-step guides for the public anvil workflow. If you are new, follow the
recommended path below; each feature tutorial also stands alone when you already
know which surface you need.

## Recommended Path

1. [Quickstart](/anvil/quickstart) — install, authenticate, run `anvil start`,
   and end in one literal protection state.
2. **`anvil tutorial`** (in your terminal) — runs the **ProtectionLoop** default
   path: protection-loop intro → fixture description → simulated check result →
   state vocabulary (`protecting`, `ready_restart_required`, `watching`,
   `needs_action`, `unsupported`, `error`) → `anvil start --verify`.
3. [Your First Save Caught](first-save-caught.md) — the same loop on your own
   repo: `anvil start`, a deliberately bad save, and the finding anvil raises.
4. [First Project](/anvil/first-project) — add architecture boundaries to a real
   repository.
5. [Architecture Boundaries](/anvil/tutorials/architecture) — tune the boundary
   file and run the import-boundary gate.
6. [CI Integration](/anvil/tutorials/ci) — make the same checks run on pull
   requests.
7. Add [Custom Policies](/anvil/tutorials/policies),
   [Drift Detection](/anvil/tutorials/drift), and
   [Suppressions](/anvil/tutorials/suppressions) when you need those specific
   workflows.

## Getting Started

| Tutorial                              | Description                                                       |
| ------------------------------------- | ----------------------------------------------------------------- |
| [Quickstart](/anvil/quickstart)       | Install, scan, and fix your first issue in under 5 minutes        |
| [First Project](/anvil/first-project) | Set up anvil with architecture boundaries in an existing codebase |

## Feature Tutorials

Use these when you are ready for a specific capability:

| #   | Tutorial                                                 | Description                                                           | Prerequisites        |
| --- | -------------------------------------------------------- | --------------------------------------------------------------------- | -------------------- |
| 1   | [Your First Save Caught](first-save-caught.md)           | Activate save-time protection and catch a deliberate mistake yourself | Install + auth       |
| 2   | [Architecture Boundaries](/anvil/tutorials/architecture) | Define layers and enforce module boundaries                           | `anvil init`         |
| 3   | [Custom Policies](/anvil/tutorials/policies)             | Write OPA/Rego rules to enforce your team's standards                 | OPA binary installed |
| 4   | [Drift Detection](/anvil/tutorials/drift)                | Capture snapshots and track architectural drift over time             | Architecture set up  |
| 5   | [CI Integration](/anvil/tutorials/ci)                    | Add anvil to GitHub Actions, GitLab CI, and git hooks                 | —                    |
| 6   | [Suppressions](/anvil/tutorials/suppressions)            | Suppress warnings for legacy code and intentional decisions           | —                    |

---

:::tip

Prefer a guided walk-through in your terminal? Run `anvil tutorial` — its
default path is **ProtectionLoop**, which walks through the activation states
and ends in a real `anvil start --verify` against your repo.

:::
