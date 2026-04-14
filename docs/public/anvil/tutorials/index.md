---
id: tutorials
title: Tutorials
sidebar_position: 1
---

# Tutorials

Step-by-step guides for every major anvil feature. Each tutorial is
self-contained -- you can follow any of them independently.

## Getting Started

| Tutorial                              | Description                                                       |
| ------------------------------------- | ----------------------------------------------------------------- |
| [Quickstart](/anvil/quickstart)       | Install, scan, and fix your first issue in under 5 minutes        |
| [First Project](/anvil/first-project) | Set up anvil with architecture boundaries in an existing codebase |

## Feature Tutorials

We recommend following these in order — each builds on concepts from the
previous one:

| #   | Tutorial                                                 | Description                                                 | Prerequisites        |
| --- | -------------------------------------------------------- | ----------------------------------------------------------- | -------------------- |
| 1   | [Architecture Boundaries](/anvil/tutorials/architecture) | Define layers and enforce module boundaries                 | `anvil init`         |
| 2   | [Custom Policies](/anvil/tutorials/policies)             | Write OPA/Rego rules to enforce your team's standards       | OPA binary installed |
| 3   | [Drift Detection](/anvil/tutorials/drift)                | Capture snapshots and track architectural drift over time   | Architecture set up  |
| 4   | [CI Integration](/anvil/tutorials/ci)                    | Add anvil to GitHub Actions, GitLab CI, and git hooks       | —                    |
| 5   | [Suppressions](/anvil/tutorials/suppressions)            | Suppress warnings for legacy code and intentional decisions | —                    |

---

:::tip

Prefer a guided walk-through in your terminal? Run `anvil tutorial` for an
interactive version of these guides.

:::
