# ADR-122: Public-doc sections split in-tree product vs copied import

## Status

Accepted 2026-08-13 (owner). Executes DOCFRESH-008. Follows ADR-119 D5/D6/D9.

## Date

2026-08-13

## Context

ADR-119 D6 assumes a public page's `upstream` lives in this repository, so
release readiness can run `git diff v<verified_against> <candidate> -- <upstream>`.
DOCFRESH-005 treated `docs/public/kindling/**`, `docs/public/aps/**`, and
`docs/public/edda-stack/**` as one "out of tree" bucket and left them owner-only
until this decision.

That bucket was wrong on two counts:

1. All three sections already live in this repository as published pages under
   `docs/public/<section>/**`.
2. **edda-stack is an in-repo product.** It is the internal name for the Edda
   and Ember substrates in `packages/edda-stack/`. It is not an external project.
   `packages/aps/` and `packages/kindling-integration/` also exist here, but they
   are not the source of the public kindling or APS *product* docs.

kindling and APS *are* external products today. Their public pages in this tree
are **copies** (DOCSYNC-023 / DOCSYNC-024), not live checkouts of those
repositories. `git diff` of the kindling or anvil-plan-spec product cannot run
from this repo. The copy can still carry a version pin.

DOCFRESH-005 also recorded a half-governed hole: an "external" page that
declared `upstream` without `verified_against` passed silently.

## Decision

Split the three sections by **product source of truth**, not by the existence of
a `docs/public/` folder.

| Section | Product source | Governance |
| --- | --- | --- |
| `docs/public/edda-stack/**` | This monorepo (`packages/edda-stack/` and related CLI) | Full triple: `owner`, in-repo `upstream`, `verified_against` of the shipped Anvil version. Same D6 model as `anvil` / `start-here` / `beta`. |
| `docs/public/kindling/**` | External product; pages are a copy | `owner` plus `verified_against` as the **imported product version the copy was last synced to**. In-repo `upstream` is not required. If a page does declare `upstream`, it must resolve **and** `verified_against` is still required. |
| `docs/public/aps/**` | External spec (`anvil-plan-spec`); pages are a copy. `packages/aps/` is local tooling, not that public spec | Same copied-section rule as kindling. |

Coverage stays visible. The checker reports how many copied-section pages are
attested against an imported product version. Owner-only is no longer a valid
end state.

A copied page without `verified_against` is an error, not a pass. That closes
the half-governed hole.

Cross-repo checks against `eddacraft/kindling` or `anvil-plan-spec` are
optional later work. They are not required to stop the silent gap.

## Rationale

edda-stack never needed a special model. Treating it as external hid an in-repo
product behind a count of "pending DOCFRESH-008".

For copies, inventing in-repo `upstream` paths would either point at the pages
themselves (circular) or at `packages/aps` / `packages/kindling-integration`
(the wrong product). The honest pin is the imported version the copy claims.

### Alternatives Considered

| Option | Pros | Cons |
| --- | --- | --- |
| **Chosen: split in-tree vs copy-attestation** | Matches repo truth; D6 works for edda-stack; copies stay honest | Copied `verified_against` is a manual sync pin, not a git diff |
| Cross-repo check now | Could verify kindling/APS against their real trees | Needs network, credentials, and a pin to a foreign tag; not needed to close the gap |
| Keep all three owner-only | No frontmatter work | Silent hole; wrong for edda-stack |
| Point kindling/APS `upstream` at `packages/aps` or `packages/kindling-integration` | Looks like a full triple | Checks the wrong product |

## Consequences

- **Positive:** edda-stack joins the D6 release-boundary check. Copied sections
  carry a visible version pin. Half-governed pages fail closed.
- **Negative:** kindling/APS pins rot unless DOCSYNC bumps them on the next
  import. That is the existing sync obligation, now machine-visible.
- **Risks:** a copy's `verified_against` can be set without actually re-reading
  the page (same honesty limit as any `verified_against`).
- **Mitigations:** DOCSYNC owns the copy; the checker only requires the pin to
  be present and well-formed. Do not invent in-repo paths for foreign products.

## References

- Related ADRs: [ADR-119](119-documentation-freshness-from-declared-upstream.md),
  [ADR-117](117-repo-state-checks-are-not-per-pr-gates.md)
- APS modules: DOCFRESH-008, DOCFRESH-005, DOCSYNC-023, DOCSYNC-024
