# Development Guides

| Type   | Authority | Owner | Status | Freshness                                                                                                                                                                                                                      |
| ------ | --------- | ----- | ------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| README | Advisory  | AICON | Live   | Last reviewed 2026-08-23 for DOCRB-008 and DOCRB-009 against ADR-123, `plans/execution/DOCRB-008.actions.md`, `plans/reviews/2026-08-21-docrb-008-public-ia.md`, `docs/guides/documentation-governance.md`, and `docs/guides/` |

| Upstream                                  | Downstream                |
| ----------------------------------------- | ------------------------- |
| `docs/guides/documentation-governance.md` | Developer guide discovery |

How-to guides for developers working on Anvil.

## Available Guides

### Adapter Development

- [Adapter Guide](adapters/README.md) — Building format adapters
- [Adapter Workflow](adapters/workflow-guide.md) — Development workflow

### Edda Stack

- [Edda Memory](edda-memory.md) — Edda memory system usage
- [Ember Candidates](ember-candidates.md) — Ember candidate evaluation
- [Stack Migration](stack-migration.md) — Coordinating schema changes across
  layers

### Command Safety

- [Command Safety](command-safety.md) — Command safety validation
- [Command Safety Configuration](command-safety-configuration.md) — Configuring
  command safety rules

### Architecture

- [Custom Architecture Policies](custom-architecture-policies.md) — Writing
  custom OPA policies
- [Documentation Governance](documentation-governance.md) — Documentation
  authority model, metadata convention, docs-workflow shape, and closeout
  protocol
- [Architecture Diagrams](architecture-diagrams.md) — Mermaid vs Draw.io/SVG,
  when to update, advisory until DOCRB-009

### Testing & Workflow

- [Testing Guide](testing.md) — Test strategy and practices
- [Repository Operations](repository-operations.md) — Local repository
  management, `gx`, and Worktrunk boundaries
- [Branching Strategy](branching-strategy.md) — Main-first branch, Worktrunk,
  and cleanup model
- [Worktree Policy](worktree-policy.md) — How to manage permanent and disposable
  worktrees with Worktrunk, including end-of-task cleanup
- [Git Hook Compatibility Policy](git-hook-compatibility.md) — Baseline and
  rollout policy for file-based and Git 2.54 native config-based hooks

### AI & MCP

- [AI Context Delivery](ai-context-delivery.md) — Wiring Anvil's read-only
  graph-context tools and `graph://` resources into AI assistants (Claude Code,
  Cursor), the identity-only privacy posture, graph states, and how this differs
  from launch-critical write validation

### CLI

- [CLI Output Streams](cli-output-streams.md) — CLI output and stream handling
- [Release Runbook](../runbooks/release-runbook.md) — Operational release
  runbook (relocated to `docs/runbooks/` 2026-05-23, DOCGOV-008 Task 2)
- [Release Doc Checklist](release-doc-checklist.md) — Documentation sync
  checklist per release

## Quick Links

### For New Contributors

1. Start with [Project README](../../README.md)
2. Review [Architecture](../architecture/README.md)
3. Read [Testing Guide](testing.md)

### For Building Adapters

1. [Adapter Framework Overview](adapters/README.md)
2. [Adapter Workflow Guide](adapters/workflow-guide.md)
3. [Adapters Package README](../../packages/adapters/README.md)

## See Also

- [Runbooks](../runbooks/) — Operational playbooks
- [Back to Documentation](../README.md)
