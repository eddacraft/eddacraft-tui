---
id: tutorials
title: Tutorials
sidebar_position: 1
---

# Tutorials

anvil has two beta paths to first value, and the tutorials are organised around
them. Pick the path that matches what you want first — daily protection, or a
look at what anvil finds — then deepen with the feature tutorials. Every page
also stands alone.

Prefer a guided walk-through in your terminal? Run **`anvil tutorial`** — the
interactive sibling of this set. Its default **ProtectionLoop** path tells the
same story as [Your First Save Caught](first-save-caught.md): the protection
loop, a deliberate escape hatch, the finding, the state vocabulary, and a real
`anvil start --verify` against your repo.

Platform notes are called out where shell syntax differs. The `anvil` CLI
subcommands themselves are the same from macOS Terminal or iTerm, Linux shells,
and Windows PowerShell; path creation, environment variables, and cleanup use
platform-specific examples.

## The daily-value path — `anvil start`

Daemon-backed save-time protection that validates files as you (or your AI
agent) save them.

1. [Quickstart](../quickstart.md) — install, authenticate, and take a path.
2. [Your First Save Caught](first-save-caught.md) — the flagship walk: activate
   protection, make a deliberately bad save, read the finding, check your
   posture.
3. [First Project](../first-project.md) — set anvil up properly in an existing
   codebase.

## The discovery path — `anvil welcome`

See what anvil finds in your own repo before wiring protection.

1. [Quickstart](../quickstart.md) — install and authenticate.
2. [Analyse a Rust Project](rust-project.md) — discovery on a real Rust repo:
   the advisory rule catalogue and the language-profile claim. (For
   TypeScript/JavaScript repos, the Quickstart's `anvil check --all` step is the
   same walk.)
3. When you are done exploring, the daily-value path above is the handoff.

## Feature tutorials

Use these when you are ready for a specific capability:

| Tutorial                                            | Description                                                              | Prerequisites             |
| --------------------------------------------------- | ------------------------------------------------------------------------ | ------------------------- |
| [Developer Acceleration](developer-acceleration.md) | Wire your AI coding agent, validate its edits, and feed it graph context | Install + auth            |
| [Your First Save Caught](first-save-caught.md)      | Activate save-time protection and catch a deliberate mistake yourself    | Install + auth            |
| [Analyse a Rust Project](rust-project.md)           | Discovery scan, the advisory Rust rules, and the language profile        | A Rust repo               |
| [Architecture Boundaries](architecture.md)          | Define layers and enforce module boundaries                              | `anvil init`              |
| [Drift Detection](drift.md)                         | Capture snapshots and track architectural drift over time                | Architecture set up       |
| [Custom Policies](policies.md)                      | Write OPA/Rego rules to enforce your team's standards                    | `anvil init` + OPA binary |

Looking for CI setup or suppressions? Those live with the surfaces that own
them: the [GitHub integration guide](../integrations/github.md) covers
pipelines, SARIF, and branch protection, and the
[insights guide](../guides/insights.md) covers suppression health.
