# Architecture Diagram Maintenance

| Type  | Authority     | Owner | Status | Freshness                                                                                                                                                              |
| ----- | ------------- | ----- | ------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Guide | Authoritative | DOCRB | Live   | Last reviewed 2026-08-20 against ADR-123, `scripts/docs/public-diagrams.json`, `scripts/docs/check-public-diagrams.mjs`, and `docs/guides/documentation-governance.md` |

| Upstream                                                                                                                                                                                                                             | Downstream                                  |
| ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------- |
| ADR-123, `docs/guides/documentation-governance.md`, `plans/specs/2026-08-16-docs-rebaseline.md`, `plans/specs/2026-08-17-docrb-corpus-disposition.md`, `scripts/docs/public-diagrams.json`, `scripts/docs/check-public-diagrams.mjs` | Architecture diagram reviews and PR hygiene |

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
3. meaningful alt text or an explicitly associated adjacent textual explanation
   of the same meaning;
4. ownership and upstream metadata through the containing document.

SVG is the default export. The manifest explicitly governs only
`assets/diagrams` below each mounted family; ordinary product screenshots and
legacy diagram-like files elsewhere in the family remain outside this contract.
A raster within a governed diagram directory needs an exact-path entry in
`rasterExceptions`, the consumer limitation, an `ADR-123` review, and a real
accessible Markdown/MDX reference. Do not silently commit a PNG in place of the
SVG pair.

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

The governed roots are the families mounted by the two production renderers:
`docs/public/anvil` and `docs/public/beta` through `apps/anvil-docs-private`;
`docs/public/aps`, `docs/public/kindling`, and `docs/public/edda-stack` through
`apps/docs-public`. The disabled `docs/public/start-here`, rollback-only
`apps/docs-site`, legacy `docs/architecture/**.drawio`, and component Mermaid
enforcement are outside this pipeline.

1. Install
   [Draw.io Desktop 31.1.8](https://github.com/jgraph/drawio-desktop/releases/tag/v31.1.8).
   The version is pinned because Desktop export changes can alter committed SVG
   bytes. The wrapper accepts only the exact stdout `31.1.8` with no stderr,
   prefix, suffix, or second version. Selecting the executable remains an
   operator-trusted boundary: `--drawio-bin` must name an authentic local
   Draw.io Desktop binary.
2. Create or edit a single-page, lower-kebab `.drawio` source in the
   manifest-declared `docs/public/<family>/assets/diagrams` directory. The
   source, every ancestor below the repository root, and any existing output
   must be regular non-symlink paths. On the `<mxfile>` element, set non-empty
   `anvil-title` and `anvil-description` attributes; these become the SVG
   accessible name and description.
3. Export with
   `pnpm docs:public:diagrams:export -- docs/public/<family>/assets/diagrams/<name>.drawio`.
   The wrapper verifies Desktop 31.1.8 and invokes
   `--export --format svg --embed-diagram --crop --border 0`. It writes the
   same-name sibling `.svg`, adds `role="img"`, `<title>`, and `<desc>`, and
   verifies that the canonical embedded Draw.io XML equals the sibling source,
   records raw-source and canonical-embedded hashes, the final export hash,
   exact observed version output, version, and flags, then creates the
   destination through a same-directory exclusive temporary file and atomic
   rename. Do not hand-edit the SVG.
4. Reference the SVG from Markdown in the same family using meaningful,
   non-empty alt text. For intentionally empty alt text, bind the adjacent
   explanation explicitly:

   ```markdown
   <!-- diagram-description: system-context.svg -->

   The diagram shows ...

   ![](system-context.svg)
   ```

   Arbitrary nearby prose, code-fence examples, comments, and one-word alt text
   do not satisfy the reference contract.

5. Run `pnpm docs:public:diagrams`. It rejects unpaired or non-lower-kebab
   assets, governed raster candidates without a reviewed exception, stale or
   missing provenance, embedded-source mismatch, symlink traversal, inaccessible
   SVG, and unreferenced exports. Namespace-aware XML DOM inspection fails
   closed on declarations, processing instructions, custom entities, active or
   namespaced elements/attributes, external/non-fragment references, and CSS
   imports or external URLs after entity, percent, and CSS-escape decoding.
   Draw.io sibling and embedded XML are parsed structurally with one
   non-namespaced `mxfile` root and exactly one valid diagram page. The checker
   verifies the recorded source, embedded-source, export, exact version-output,
   version, and flag provenance; it does not invoke Draw.io. Markdown and MDX
   raw HTML fragments are parsed as DOM, so commented, hidden, code, and
   attribute-text image decoys do not count. The manifest enumerates only the
   five governed `assets/diagrams` directories. Both production documentation
   builds prove renderer integration. `pnpm docs:check` runs the same validator
   as its `public-diagrams` surface.

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
