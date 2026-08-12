# Architecture Diagram Maintenance

| Type  | Authority     | Owner  | Status | Freshness                                                                                  |
| ----- | ------------- | ------ | ------ | ------------------------------------------------------------------------------------------ |
| Guide | Authoritative | DOCGOV | Live   | Last reviewed 2026-05-23 against `docs/architecture/overview.md` and current diagram drift |

| Upstream                                                                                           | Downstream                                  |
| -------------------------------------------------------------------------------------------------- | ------------------------------------------- |
| `plans/archive/modules/documentation-governance.aps.md`, `docs/guides/documentation-governance.md` | Architecture diagram reviews and PR hygiene |

This guide covers how architecture diagrams are managed, when to update them,
and which tool to use.

## Diagram Inventory

| Diagram           | Format  | Location                                           | What it shows                                                |
| ----------------- | ------- | -------------------------------------------------- | ------------------------------------------------------------ |
| System overview   | Mermaid | `docs/architecture/overview.md`                    | Active runtime, package/crate dependency graph, and surfaces |
| System components | Draw.io | `docs/architecture/anvil-system-components.drawio` | Full component map, shipped/planned/archived status          |
| PPTX workflow     | Draw.io | `docs/architecture/pptx-workflow.drawio`           | Presentation generation flow                                 |

## Tool Choice

**Mermaid** — use for diagrams embedded in Markdown files:

- Dependency graphs
- Sequence diagrams in ADRs or specs
- Simple flow diagrams in guides
- Renders automatically in GitHub; for local preview, use a Mermaid-aware editor
  or CLI

**Draw.io** — use for complex architecture diagrams:

- Multi-layer system component views
- Diagrams with custom styling or dense layout
- Anything that needs precise spatial positioning
- Open with draw.io desktop or VS Code extension

## When to Update Diagrams

Update the relevant diagram when:

- A new crate or package is added to the dependency graph
- A module boundary changes (new surface, removed component)
- An ADR introduces a new architectural layer or pattern
- A module is promoted from Draft to In Progress (new subsystem entering the
  codebase)
- A surface moves between active, planned, deprecated, or archived state

You do **not** need to update diagrams for:

- Internal refactors within an existing module
- Bug fixes that don't change architecture
- Adding tests or documentation

## How to Update

### Mermaid diagrams

1. Find the diagram in the Markdown file (look for ` ```mermaid ` blocks)
2. Edit the Mermaid source directly
3. Preview locally: use a Mermaid-aware editor or `npx @mermaid-js/mermaid-cli`
4. Commit alongside the code change that prompted the update

### Draw.io diagrams

1. Open the `.drawio` file in draw.io (desktop app or VS Code extension)
2. Make changes, save the file
3. Commit the `.drawio` file — do not export PNGs (they drift from source)

## Review Checklist

When reviewing a PR that changes architecture:

- [ ] Does the PR touch a module boundary listed in the diagram inventory?
- [ ] If yes, is the relevant diagram updated?
- [ ] Are new components labelled consistently with existing naming?
- [ ] Do dependency arrows match the actual `Cargo.toml` / `package.json` deps?
- [ ] Are archived surfaces labelled as archived instead of active runtime?
- [ ] Do active surface diagrams match the relevant `*-as-built.md` document?

## Quarterly Audit

As part of the quarterly documentation audit (see
`docs/guides/release-doc-checklist.md`):

- [ ] Verify all Mermaid diagrams render correctly
- [ ] Open each `.drawio` file and check it matches the current codebase
- [ ] Check that new crates/packages added since last audit appear in diagrams
