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

1. [Quickstart](/anvil/quickstart) — install, authenticate, initialise, and run a
   first scan.
2. [First Project](/anvil/first-project) — add architecture boundaries to a real
   repository.
3. [Architecture Boundaries](/anvil/tutorials/architecture) — tune the boundary
   file and run the import-boundary gate.
4. [CI Integration](/anvil/tutorials/ci) — make the same checks run on pull
   requests.
5. Add [Custom Policies](/anvil/tutorials/policies),
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
