# DOCGOV-009 Owner and Freshness Rubric

**Purpose:** Provide the operator-approved mapping rules for DOCGOV-009 metadata
backfill before any live documents under `docs/**` are edited.

**Status:** Approved 2026-05-25. Task 2 backfill may start from this rubric.

**Approval note:** Default owner rules are accepted. Use `@aneki`
(`aneki@eddacraft.ai`) as the backup owner or when administrative ownership is
needed.

**Upstream authority:** `docs/guides/documentation-governance.md` field meanings,
status values, authority values, and freshness rules.

## Backfill Defaults

- Add the metadata table immediately after the H1.
- Use `Status: Live` only when the document is current operational, product, or
  discovery guidance.
- Use `Status: Deprecated` when freshness cannot be established but the document
  must remain in an active path.
- Do not guess unresolved owner or freshness fields; route unclear cases to the
  judgement bucket in Task 4.
- Do not add new tags outside `docs/governance/tags-catalogue.md`; either use an
  approved tag, propose a catalogue addition with rationale, or drop the tag.
- For governed `As-built` and `Runbook` documents, every backtick-wrapped source
  path used in freshness, upstream, downstream, or body references must resolve
  on disk.

## Owner Mapping

Choose `Owner` in this order:

| Priority | Use when | Owner value |
| -------- | -------- | ----------- |
| 1 | A document is directly owned by active APS work | APS module ID, for example `DOCGOV`, `MLP2`, `TUIR` |
| 2 | A document is durable architecture or decision-support owned by a module family | Stable module or programme ID, for example `RELORCH`, `OPMODEL`, `CICD` |
| 3 | A document is a repo-wide operational procedure without a narrower module | `@aneki` (`aneki@eddacraft.ai`) until a narrower owner exists |
| 4 | A document is public user-facing behaviour | Product/docs publishing owner; default `@aneki` (`aneki@eddacraft.ai`) |
| 5 | A document is generated, derived, or an index | `Docs governance` |
| 6 | A document is historical but intentionally active | Owning historical module if clear; otherwise route to Task 4 |

Do not use `Docs governance` as a catch-all owner for ordinary live prose. It is
reserved for derived/generated governance surfaces and metadata/index mechanics.

## Authority Mapping

| Surface | Default authority | Notes |
| ------- | ----------------- | ----- |
| `docs/architecture/**/*-as-built.md` | `Derived` | Source code, schemas, tests, and generated artefacts remain implementation truth. |
| `docs/architecture/**` conceptual architecture | `Authoritative` when it owns design guidance; otherwise `Advisory` | If it summarises ADRs or code, use `Derived`. |
| `docs/runbooks/**` | `Authoritative` | Runbooks own operational procedure for their scope. |
| `docs/guides/**` | `Authoritative` when it defines repo practice; `Advisory` when it is explanatory | Developer workflow guides that agents must follow are authoritative. |
| `docs/policies/**` | `Authoritative` | Policies own operational constraints unless an ADR or APS item supersedes them. |
| `docs/public/**` | `Authoritative` for user-visible behaviour; `Derived` for public indexes/overviews | Public docs must match released product state. |
| `docs/indexes/**` | `Derived` | Generated from metadata; owner is `Docs governance`. |
| `docs/marketing/**`, `docs/strategy/**`, `docs/vision/**` | `Advisory` unless explicitly declared as scope guard or product authority | `docs/vision/anvil-scope-guard.md` remains authoritative for scope. |
| `docs/specs/**` | `Advisory` before implementation; `Historical` when superseded; `Authoritative` only if it is the active approved spec | Prefer APS/ADR links for durable authority. |
| `docs/observability/**`, `docs/testing/**`, `docs/internal/**`, `docs/reviews/**`, `docs/plans/**` | Match the document's actual role | Use `Derived` for reports/maps, `Authoritative` for executable procedures, `Advisory` for guidance, `Historical` for retained records. |

## Freshness Mapping

Freshness must include a review date and a concrete anchor. Use ISO dates.

| Type | Freshness rule | Example anchor |
| ---- | -------------- | -------------- |
| As-built | Cite the tag or SHA and source paths reviewed | `Last reviewed 2026-05-25 against v0.7.1-beta and crates/anvil-daemon/src/session.rs` |
| Runbook | Cite the last successful dry-run, release, incident, or command review plus executable source paths | `Last reviewed 2026-05-25 against v0.7.1-beta release dry-run and scripts/release/prepare.mjs` |
| Guide | Cite the upstream rule, APS item, ADR, or source path reviewed | `Last reviewed 2026-05-25 against DOCGOV-009 and docs/guides/documentation-governance.md` |
| Public docs | Cite the release or product version described | `Last reviewed 2026-05-25 against v0.7.1-beta user behaviour` |
| Spec | Cite the owning APS module, ADR, or supersession state | `Last reviewed 2026-05-25 against MLP2 active module state` |
| README | Cite the local package/crate/app source or canonical index it orients readers to | `Last reviewed 2026-05-25 against package.json and docs/indexes/by-type.md` |
| Archive or active historical reference | Cite the superseding document or archive date | `Archived 2026-05-24; superseded by docs/indexes/by-type.md` |

## Directory Examples

| Directory | Type | Owner | Authority | Freshness pattern |
| --------- | ---- | ----- | --------- | ----------------- |
| `docs/architecture/` | `As-built` or `Guide` | Owning module ID | `Derived` for as-built, otherwise mapped by role | Tag/SHA plus source paths or ADR reviewed |
| `docs/runbooks/` | `Runbook` | Owning module ID or `@aneki` (`aneki@eddacraft.ai`) | `Authoritative` | Last dry-run/release/incident plus script or command surface |
| `docs/guides/` | `Guide` | Owning module ID | `Authoritative` or `Advisory` | APS rule, ADR, source path, or guide authority reviewed |
| `docs/policies/` | `Guide` | Owning module ID or `@aneki` (`aneki@eddacraft.ai`) | `Authoritative` | Policy review date plus APS/ADR/source authority |
| `docs/public/` | `Public docs` | Public docs owner, default `@aneki` (`aneki@eddacraft.ai`) | `Authoritative` or `Derived` | Product version or release record described |
| `docs/indexes/` | `Guide` or generated index role | `Docs governance` | `Derived` | Generator run or metadata source reviewed |
| `docs/marketing/`, `docs/strategy/`, `docs/vision/` | `Guide` or `Spec` | Product/module owner | `Advisory` except explicit scope authority | Owning APS module, RELEASE-PLAN entry, or stated aspirational status |
| `docs/specs/` | `Spec` | Owning APS module | `Advisory`, `Authoritative`, or `Historical` by state | Active APS/ADR link or supersession date |
| `docs/observability/`, `docs/testing/`, `docs/internal/`, `docs/reviews/`, `docs/plans/` | Role-specific | Owning module or `@aneki` | Role-specific | Most recent canonical source, test command, report, or APS item |

## Ambiguous Cases for Task 4

Route a document to Task 4 instead of the high-authority sweep when any of these
apply:

- No clear owner exists after checking APS, ADRs, release records, and nearby
  README/index references.
- The document appears stale but still has live inbound links.
- The document is aspirational, marketing-oriented, or strategic and may not
  describe current product behaviour.
- The document has mixed authority, for example a historical narrative with live
  operational commands.
- The document cites a command, script, crate, package, release, or path that no
  longer exists.
- The document needs a tag not present in `docs/governance/tags-catalogue.md`
  and the right action is unclear.

Task 4 outcomes are: operator supplies owner/freshness metadata, the document is
marked `Deprecated`, the document is routed back to archival work, or the tag
catalogue is extended with explicit rationale.

## Sign-off Gate

Operator confirmed 2026-05-25:

- The default public-docs owner is acceptable.
- `@aneki` (`aneki@eddacraft.ai`) is acceptable as the fallback owner for
  repo-wide operational docs and when administrative ownership is needed.
- Ambiguous docs should be held for Task 4 rather than guessed during high-
  authority and public-doc sweeps.
- `Docs governance` should remain restricted to generated/derived governance
  surfaces.
