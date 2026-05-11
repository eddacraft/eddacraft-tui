# Development Guides

| Type   | Authority | Owner  | Status | Freshness                                                                                     |
| ------ | --------- | ------ | ------ | --------------------------------------------------------------------------------------------- |
| README | Advisory  | DOCGOV | Live   | Last reviewed 2026-05-11 against `docs/guides/documentation-governance.md` and `docs/guides/` |

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

### Testing & Workflow

- [Testing Guide](testing.md) — Test strategy and practices
- [Branching Strategy](branching-strategy.md) — Git branching model
- [Worktree Policy](worktree-policy.md) — How to manage permanent and disposable
  worktrees
- [Git Hook Compatibility Policy](git-hook-compatibility.md) — Baseline and
  rollout policy for file-based and Git 2.54 native config-based hooks

### CLI

- [CLI Output Streams](cli-output-streams.md) — CLI output and stream handling
- [Release Runbook](release-runbook.md) — Operational release runbook retained
  here during the DOCGOV migration
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
