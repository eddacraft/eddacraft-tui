# ADR-047: eddacraft-tui canonical source and public mirror

## Status

Approved

## Date

2026-05-19

## Context

`eddacraft-tui` was extracted from Anvil into a standalone open-source crate so
the shared Ratatui widget library could be published, inspected, and reused
outside the private Anvil monorepo. That extraction succeeded: the public repo
exists, the crate is published to crates.io, and Anvil consumes
`eddacraft-tui = "0.2.2"` as an external dependency.

The practical ownership model is different from the original assumption. Anvil
is the primary consumer and the main source of product-driven requirements for
the library. Realistically, external use and contribution are expected to be low;
the library's main public value is source visibility, trust, and reuse if someone
wants it, not community co-development.

The current model makes Anvil pay an unnecessary integration tax for its own TUI
substrate:

- A widget or theme change that is needed by Anvil must land in
  `eddacraft/eddacraft-tui` first.
- The crate must then be released to crates.io.
- Anvil must bump the dependency and re-verify the consuming surfaces.
- Cross-cutting changes cannot land atomically with Anvil's own TUI surfaces.

The acknowledgements starter kit introduced a better topology for this class of
asset: keep the canonical source in Anvil, mirror a public read-only repository
for downstream visibility and consumption, and make the primary consumer's local
development path the easiest path.

## Decision

Move the canonical source of `eddacraft-tui` back into the Anvil monorepo while
keeping the public `eddacraft/eddacraft-tui` repository open source as a
read-only mirror.

The target topology is:

```text
eddacraft/anvil-001:crates/eddacraft-tui
  -> eddacraft/eddacraft-tui:main
  -> crates.io package eddacraft-tui
```

Anvil will consume `eddacraft-tui` as an in-workspace/path crate. External users
will continue consuming the published crates.io package.

The public repository remains Apache-2.0 and visible, but its `main` branch is
mirror-managed. Public contributions are not taken directly against the mirror;
issues may be filed there, but source changes land in Anvil first and are then
mirrored out.

Mirror and release policy:

- Mirror `crates/eddacraft-tui/` from Anvil to `eddacraft/eddacraft-tui:main`.
- Document the public repo as a read-only mirror.
- Do not rewrite release tags.
- Keep crates.io as the supported external distribution channel.
- Publish crates.io releases from the canonical Anvil source, either through a
  dedicated workflow or as an explicit step in the existing release process.
- Preserve independent `eddacraft-tui` crate versioning; this ADR changes the
  canonical source location, not the public package's semantic version policy.

## Rationale

This aligns the source-of-truth model with the real ownership model. The library
is a reusable primitive, but Anvil is the load-bearing user. Keeping the source
inside Anvil makes Anvil-driven changes cheaper and safer because API changes,
surface changes, snapshot updates, and release validation can happen in one
branch.

The public mirror preserves the strategic benefits that justified open sourcing
the crate:

- The code remains inspectable.
- External users can still depend on the crate from crates.io.
- The eddacraft Terminal Standard and Ratatui widget implementation remain a
  public trust surface.
- Future extraction back to a fully independent repo remains possible because the
  mirrored tree is still a clean crate boundary.

The key trade-off is accepting that `eddacraft-tui` is not primarily a community
project. Optimising for hypothetical external contributors creates real friction
for the actual primary consumer.

### Alternatives Considered

| Option | Pros | Cons |
|--------|------|------|
| Chosen: canonical source in Anvil, public read-only mirror | Anvil gets path-crate development and atomic changes; public source and crates.io package remain; ownership model is honest | Requires mirror automation and release discipline; public PR flow becomes indirect |
| Keep `eddacraft-tui` fully independent | Clean OSS contribution story; no mirror machinery; current crates.io flow stays simple | Anvil keeps paying release-and-bump tax; cross-repo changes are slow; primary consumer is not optimised |
| Re-vendor the code privately and stop publishing | Simplest Anvil development loop | Loses public trust surface, breaks external users, contradicts ADR-018's open primitive surface |
| Use a git dependency from Anvil to the public repo | Avoids crates.io bump for some changes | Still makes the public repo canonical; weakens reproducibility; does not allow atomic Anvil + TUI changes |
| Keep separate repo but automate dependency bumps | Preserves independent repo while reducing some toil | Still requires a release boundary before Anvil can consume changes; does not solve atomicity |

## Consequences

- **Positive:** Anvil TUI changes that need shared widget changes can land in one
  branch and one verification pass.
- **Positive:** `crates/anvil-tui` can depend on a workspace `eddacraft-tui`
  crate instead of a crates.io version, reducing local iteration friction.
- **Positive:** The public mirror keeps `eddacraft-tui` visible and reusable
  without pretending it is a community-governed upstream.
- **Positive:** Crates.io remains the stable contract for external users.
- **Negative:** The public repo becomes less welcoming to drive-by pull requests;
  contribution instructions must be explicit about the mirror model.
- **Negative:** Release automation must handle a source tree whose canonical home
  is private but whose package is public.
- **Risks:** Force-pushing the mirror could disrupt users who depend directly on
  the public git `main` branch.
- **Mitigations:** Treat crates.io as the supported external consumption path,
  protect release tags, document `main` as mirror-managed, and avoid rewriting
  tags or published crate versions.

## References

- Amends: ADR-018's `eddacraft-tui` contribution/source-topology assumption;
  ADR-020's `eddacraft-tui` canonical-source assumption only. Independent crate
  versioning remains unchanged.
- Related ADRs: ADR-025 (package manager distribution strategy)
- APS modules: TUIEXTRACT (archived), ATTRIB-011 mirror pattern. Implementation
  requires a new APS module or work item before code, mirror automation, or
  release-flow changes begin.
- Repos: `eddacraft/anvil-001`, `eddacraft/eddacraft-tui`
