# Component Documentation Guide and Template

| Type  | Authority     | Owner | Status | Freshness                                                                                                                       |
| ----- | ------------- | ----- | ------ | ------------------------------------------------------------------------------------------------------------------------------- |
| Guide | Authoritative | DOCRB | Live   | Last reviewed 2026-08-20 against ADR-123, `docs/guides/documentation-governance.md`, and `plans/modules/docs-rebaseline.aps.md` |

| Upstream                                                                                                                 | Downstream                                                                                       |
| ------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------ |
| `plans/decisions/123-documentation-authority-and-diagram-model.md`, `docs/guides/documentation-governance.md`, DOCRB-003 | `AGENTS.md`, `docs/architecture/README.md`, component-root README and architecture doc authoring |

## Purpose and use

This is the authoring source for **component-root** documentation under ADR-123.
Do not create a new component as-built under `docs/architecture/**`. Copy the
relevant shape below into the component's `README.md` and, when its internals
warrant a separate explanation, `ARCHITECTURE.md`.

Before writing, use the authoritative
[component documentation standard](../guides/documentation-governance.md#component-documentation-standard)
and the source-pinned
[corpus disposition](../../plans/specs/2026-08-17-docrb-corpus-disposition.md).
For a component absent from that inventory, apply the standard's fallback
classification; absence is not an exemption.

## Component `README.md` shape

A material component has a root README unless its evidenced classification says
otherwise. Keep it concise enough to orient a contributor before they read
internals:

1. **Purpose and scope** — what the component owns and explicitly does not own.
2. **Owner and entry points** — responsible module/team and supported public
   interfaces.
3. **Local development** — the narrow build, test, lint, or run commands that
   prove changes.
4. **Architecture and authorities** — link local `ARCHITECTURE.md` when present,
   governing ADRs, cross-system views, runbooks, and public documentation.
5. **Orientation diagram, only when useful** — a small inline Mermaid map when
   it materially improves navigation.

Do not copy cross-system explanations, ADR rationale, public guidance, or
generated reference material into the README.

## Component `ARCHITECTURE.md` shape

Add this file when component internals warrant source-linked as-built
explanation. Cover only the concerns that help maintainers verify the component:

1. **Scope and boundaries** — owned responsibilities, dependencies, consumers,
   and extension points.
2. **Invariants and decisions** — load-bearing rules and links to governing
   ADRs.
3. **Material data and control flow** — entry points, state transitions,
   persistence or emit boundaries, and lifecycle.
4. **Trust, failure, and fallback behaviour** — validation boundaries, degraded
   modes, recovery, and fail-open or fail-closed posture.
5. **Source references and related authorities** — repository paths for
   load-bearing claims plus links to the one cross-system or public authority
   for wider concerns.

Cite current source or contracts for every load-bearing claim. Record a known
gap only when it has an owner and an APS or GitHub issue; do not leave inline
deferred-work markers. Do not create an empty `ARCHITECTURE.md` for a
README-only or evidenced exempt component.

## Metadata shape

Both files use the
[governance metadata convention](../guides/documentation-governance.md#metadata-convention).
A component README normally owns orientation; an architecture document is a
source-linked derivation of implementation truth.

Copy the relevant fenced example and replace every placeholder.

```markdown
# <Component name>

| Type   | Authority     | Owner         | Status | Freshness                                                      |
| ------ | ------------- | ------------- | ------ | -------------------------------------------------------------- |
| README | Authoritative | <MODULE-CODE> | Live   | Last reviewed YYYY-MM-DD against <tag-or-sha> and source paths |

| Upstream                            | Downstream                          |
| ----------------------------------- | ----------------------------------- |
| <source paths, contracts, and ADRs> | <consumers and dependent documents> |
```

```markdown
# <Component name> Architecture

| Type         | Authority | Owner         | Status | Freshness                                                      |
| ------------ | --------- | ------------- | ------ | -------------------------------------------------------------- |
| Architecture | Derived   | <MODULE-CODE> | Live   | Last reviewed YYYY-MM-DD against <tag-or-sha> and source paths |

| Upstream                            | Downstream                          |
| ----------------------------------- | ----------------------------------- |
| <source paths, contracts, and ADRs> | <consumers and dependent documents> |
```

Use repository-relative inline-code paths so humans and future tooling can trace
the source references. Today `pnpm docs:check` checks this central authoring
source, including its metadata, Markdown links, and generated-index freshness.
That coverage does not extend to metadata, cited paths, or links copied into
component-root `README.md` and `ARCHITECTURE.md` files. Manually trace those
three concerns in each completed copy until DOCRB-009 authorises enforcement.

## Diagram guidance

Follow ADR-123 and the
[architecture-diagram guide](../guides/architecture-diagrams.md):

- use inline Mermaid for component internals when relationships are materially
  easier to verify visually;
- name the concern the diagram owns and link important nodes to source paths in
  adjacent prose;
- keep the diagram small enough to review as text;
- link to central cross-system views instead of copying them;
- use curated Draw.io source plus an accessible sibling SVG only for governed
  public or public-facing views.

A prose-only component document is valid when a diagram adds no explanatory
value. ASCII is not a required precursor to Mermaid.

## Validation

Run the repository documentation gates after copying and completing the local
files:

```bash
pnpm format:check
pnpm docs:check
```

Also trace each documented node, edge, invariant, trust boundary, and material
flow to the cited source or contract.
