---
id: team-flow
title: Team workflow
description:
  Roll anvil out with shared configuration, reviewable gates, and predictable
  recovery.
---

# Team workflow

**For:** teams adopting anvil in a shared repository

**Time:** 30–60 minutes for the first pilot

**Outcome:** a documented, reproducible gate that does not depend on one
person's machine

## 1. Pilot locally

Choose one representative project and run the [quickstart](../quickstart.md).
Record false positives, unsupported file types, and setup friction before
enforcing a gate.

## 2. Agree on the contract

Document:

- which gate profile runs;
- which checks are advisory or blocking;
- who owns configuration changes;
- how suppressions are reviewed;
- what command reproduces CI locally; and
- how to report a tool failure separately from a product finding.

## 3. Commit project configuration

Review generated configuration and baseline files like code. Do not commit
credentials, personal paths, daemon state, or caches.

## 4. Add local and CI layers

After the pilot is activated, each developer's daily path is bare `anvil` (turn
protection on without reinstalling). Keep `anvil start` for first activation and
configuration changes only.

Use [Git hooks](../operations/git-hooks.md) as fast local feedback and
[continuous integration](../integrations/github.md) as the shared authority.

## 5. Review results

A reviewer should be able to reproduce the gate from a clean checkout. Use
machine-readable output for automation and keep human guidance linked to one
canonical page.

## Rollout safety

Start advisory, measure, then tighten. If a language has parsing-only support,
do not claim the same rule depth as the primary language set.

## Next step

Configure [continuous integration](../integrations/github.md).
