# Architecture Diagram Maintenance

| Type  | Authority     | Owner | Status | Freshness                                                                                                     |
| ----- | ------------- | ----- | ------ | ------------------------------------------------------------------------------------------------------------- |
| Guide | Authoritative | DOCRB | Live   | Last reviewed 2026-08-20 against ADR-123, `docs/architecture/`, and `docs/guides/documentation-governance.md` |

| Upstream                                                                                                                                              | Downstream                                  |
| ----------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------- |
| ADR-123, `docs/guides/documentation-governance.md`, `plans/specs/2026-08-16-docs-rebaseline.md`, `plans/specs/2026-08-17-docrb-corpus-disposition.md` | Architecture diagram reviews and PR hygiene |

This guide covers diagram format and maintenance procedure. The durable contract
is ADR-123. Documentation governance owns the
[change-impact trigger and exemptions](documentation-governance.md#change-impact-review)
and the
[component documentation standard](documentation-governance.md#component-documentation-standard);
`AGENTS.md` carries only a thin advisory link to that authority.

## Inventory authority

The source-pinned
[`DOCRB-002 corpus disposition`](../../plans/specs/2026-08-17-docrb-corpus-disposition.md)
owns the diagram inventory, authority, and retain/move/redraw/merge/retire
decisions. Do not maintain a second per-diagram list here. Before changing a
diagram, start from its corpus entry and verify the referenced files against the
current tree; `docs/architecture/README.md` provides discovery only.

## Tool choice

**Mermaid** — default for internal and engineering diagrams:

- component maps placed under the
  [component documentation standard](documentation-governance.md#component-documentation-standard);
- crate or package boundaries, request/data/control flow, state machines;
- sequence diagrams in ADRs or specs;
- central cross-system views that must be reviewable as text.

Renders automatically on GitHub. For local preview, use a Mermaid-aware editor
or the repository's supported Mermaid CLI. Syntax must render in that toolchain.

**Draw.io plus SVG** — default for public diagrams, and for a central view only
when layout, typography, visual grouping, or brand consistency materially
improves comprehension:

- curated public journeys under `docs/public/**`;
- dense multi-layer maps that cannot stay reviewable as Mermaid.

Each governed public (or public-facing central) diagram commits:

1. the editable `.drawio` source;
2. a deterministic `.svg` export with the matching base name;
3. alt text or an adjacent textual explanation of the same meaning;
4. ownership and upstream metadata through the containing document.

SVG is the default export. Raster export is an explicit exception. Do not commit
a PNG in place of the SVG pair. Source-export parity is enforced from DOCRB-007;
until then, new public diagrams still commit both files.

**One diagram per concern.** A second audience gets a linked or deliberately
simplified view, not a copied diagram that can drift.

## When to update diagrams

Start with the authoritative
[change-impact review](documentation-governance.md#change-impact-review). When
that review is triggered, update the one authoritative diagram only if its
depicted nodes, edges, boundaries, states, flow, user workflow, or lifecycle
change. Otherwise, record the diagram as unaffected with a brief reason. A
documentation update may still be required when the diagram is unaffected.

Until DOCRB-009 this review is **advisory**. Do not fail CI for a missing
diagram update under ADR-123.

## How to update

### Mermaid diagrams

1. Edit the Mermaid source in the owning document (component `ARCHITECTURE.md`
   for local internals; `docs/architecture/**` for cross-system views).
2. Name the concern the diagram owns. Link important nodes to source paths in
   the adjacent prose.
3. Preview locally with a Mermaid-aware editor or `npx @mermaid-js/mermaid-cli`.
4. Commit alongside the code change that prompted the update.
5. Do not copy a central view into a component doc. Link instead.

### Draw.io diagrams

1. Open the `.drawio` file in draw.io (desktop app or VS Code extension).
2. Make changes and save the source.
3. Export the sibling `.svg` through the documented export path (DOCRB-007).
   Until that path exists, export SVG from the same source and commit both files
   together.
4. Confirm alt text or adjacent prose still matches the picture.
5. Do not commit a PNG instead of the SVG pair.

## Review checklist

When reviewing a PR that changes architecture or a public user journey:

- [ ] Does the change match the governance
      [change-impact trigger](documentation-governance.md#change-impact-review)?
- [ ] If yes, is the **authoritative** diagram for that concern updated or
      explicitly called out as unaffected?
- [ ] Is the diagram in the right home (component vs `docs/architecture/**` vs
      `docs/public/**`)?
- [ ] Are new components labelled consistently with existing naming?
- [ ] Do dependency arrows match the actual `Cargo.toml` / `package.json` deps?
- [ ] Are archived surfaces labelled as archived instead of active runtime?
- [ ] For public diagrams, are `.drawio` and `.svg` both present and named
      together?
- [ ] Does the change avoid creating a second authority for the same concern?

Until DOCRB-005, central `*-as-built.md` files may still hold component detail.
Prefer linking to them rather than copying their diagrams.

## Quarterly audit

As part of the quarterly documentation audit (see
`docs/guides/release-doc-checklist.md`):

- [ ] Verify all Mermaid diagrams render correctly
- [ ] Open each `.drawio` file and check it matches the current codebase
- [ ] Confirm each retained Draw.io source has a sibling SVG (after DOCRB-007)
- [ ] Check that new crates/packages added since last audit appear in the
      authoritative diagram that owns that concern
