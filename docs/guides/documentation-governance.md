# Documentation Governance

| Type  | Authority     | Owner  | Status | Freshness                                                                        |
| ----- | ------------- | ------ | ------ | -------------------------------------------------------------------------------- |
| Guide | Authoritative | DOCGOV | Live   | Last reviewed 2026-05-11 against `plans/modules/documentation-governance.aps.md` |

| Upstream                                                                           | Downstream                                                                           |
| ---------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------ |
| `plans/modules/documentation-governance.aps.md`, `AGENTS.md`, `plans/aps-rules.md` | `docs/README.md`, `docs/guides/README.md`, `AGENTS.md`, future `docs-workflow` skill |

Documentation is operational knowledge for humans and agents. It exists to make
engineering behaviour deterministic: what to read, what to trust, what to
update, and what must be verified before work is closed.

This guide is owned by APS module `DOCGOV` and is the seed for a future
`docs-workflow` skill.

## Authority Model

| Question                                      | Authoritative source                      |
| --------------------------------------------- | ----------------------------------------- |
| What work is authorised?                      | APS work item in `plans/modules/*.aps.md` |
| What work is active or planned?               | `plans/index.aps.md`                      |
| Why was an architectural choice made?         | ADR in `plans/decisions/`                 |
| What is actually implemented?                 | Code, schemas, tests, generated artefacts |
| How does the implemented system fit together? | Source-pinned as-built doc                |
| How is a system operated?                     | Runbook in `docs/runbooks/`               |
| How should developers work?                   | Guide in `docs/guides/`                   |
| What do users see?                            | Public docs in `docs/public/`             |
| What happened historically?                   | Archive or release evidence               |

No document should duplicate another document's authority. Link to the upstream
source instead.

## Document Types

| Type              | Purpose                                   | Location                                  |
| ----------------- | ----------------------------------------- | ----------------------------------------- |
| APS index         | Module discovery and active state         | `plans/index.aps.md`                      |
| APS module        | Execution authority                       | `plans/modules/*.aps.md`                  |
| Release plan      | Current release-slate summary             | `RELEASE-PLAN.md`                         |
| ADR               | Durable decision rationale                | `plans/decisions/*.md`                    |
| Spec              | Intended design before or during work     | `plans/specs/`, `docs/specs/`             |
| As-built          | Current implementation map                | `docs/architecture/*-as-built.md`         |
| Runbook           | Operational procedure                     | `docs/runbooks/*.md`                      |
| Guide             | Developer practice and operational policy | `docs/guides/*.md`, `docs/policies/*.md`  |
| README            | Local orientation                         | nearest package, crate, app, or directory |
| Contributor guide | Contribution workflow and expectations    | `CONTRIBUTING.md`                         |
| Public docs       | User-facing behaviour                     | `docs/public/**/*.md`                     |
| Archive           | Historical reference                      | `docs/archive/`, `plans/archive/`         |

Current migration exception: the evergreen release runbook remains at
`docs/guides/release-runbook.md`. Treat it as runbook authority until DOCGOV-008
either moves it, renames it, or records why the exception remains.

## Metadata Convention

New documents and materially touched non-APS documentation should declare their
governance metadata immediately after the H1 title. Existing documents do not
need a metadata-only migration until DOCGOV-005 adds validation.

APS modules, APS indexes, and ADRs keep their native metadata formats unless
their own schema or process explicitly adopts this table. Do not add this table
to APS files just because their plan state changed.

Use this compact table immediately after the title:

```markdown
| Type  | Authority     | Owner                       | Status | Freshness                                            |
| ----- | ------------- | --------------------------- | ------ | ---------------------------------------------------- |
| Guide | Authoritative | APS module, team, or handle | Live   | Last reviewed YYYY-MM-DD against tag/SHA/source path |

| Upstream                                         | Downstream                                          |
| ------------------------------------------------ | --------------------------------------------------- |
| Canonical source(s) this doc must not contradict | Docs, tooling, or workflows that depend on this doc |
```

Keep values short and link to canonical sources when useful. For active non-APS
docs, fill every field in the table; do not replace the table with prose
elsewhere in the document. Archive-only documents may omit `Downstream` when no
live document depends on them.

### Field Meanings

| Field      | Meaning                                                                         |
| ---------- | ------------------------------------------------------------------------------- |
| Type       | The document type from the table above                                          |
| Authority  | Whether the document is source-of-truth, derived, advisory, or historical       |
| Owner      | Who keeps the document correct; prefer APS module IDs for active work           |
| Status     | Current lifecycle state of the document itself                                  |
| Freshness  | Review date plus tag, SHA, source path, release, or other check anchor          |
| Upstream   | Documents, code, schemas, tests, release records, or ADRs this doc derives from |
| Downstream | Documents, generated indexes, runbooks, agents, or workflows that read this doc |

### Status Values

| Value      | Use when                                                  |
| ---------- | --------------------------------------------------------- |
| Draft      | Content is being shaped and is not yet safe as guidance   |
| Proposed   | Direction is reviewed but not yet operational authority   |
| Ready      | Content is approved for use but not yet live practice     |
| Live       | Content is current operational guidance or discovery      |
| Deprecated | Content is stale or superseded but remains in active path |
| Archived   | Content is historical reference only                      |

### Authority Values

| Value         | Use when                                                                     |
| ------------- | ---------------------------------------------------------------------------- |
| Authoritative | The document owns the answer for its declared scope                          |
| Derived       | The document summarises or maps implementation truth from upstream sources   |
| Advisory      | The document offers practice guidance but must defer to stronger sources     |
| Historical    | The document is preserved for context and should not be edited as live truth |

### Freshness Rules

- **As-built docs:** cite a tag or SHA and source paths reviewed.
- **Runbooks:** cite the last successful dry-run, release, incident, or command
  review, plus executable source paths where the procedure depends on scripts or
  command surfaces.
- **Guides:** cite the upstream rule, APS item, ADR, or source path reviewed.
- **Public docs:** cite the release or product version they describe.
- **Archives:** cite the superseding document or archive date.

When freshness cannot be established, mark the document `Status: Deprecated` or
track the gap in APS instead of leaving it ambiguous.

### Source-Reference Validation

`pnpm docs:check` validates source references for governed `As-built` and
`Runbook` documents through the `asbuilt-paths` surface. The validator reads the
metadata table, requires a `YYYY-MM-DD` freshness date, extracts
backtick-wrapped repository paths from freshness/upstream/downstream/body
references, and checks that each path resolves in the repository. Markdown
anchors are allowed and are resolved to the owning file; placeholder paths using
angle brackets are treated as examples and ignored.

Use `docs/architecture/_as-built-template.md` for implementation maps and
`docs/guides/runbook-template.md` for operational procedures.

## Docs Workflow Skill Shape

A future `docs-workflow` skill should be a router, not a bureaucracy layer. It
should classify the request, load the right rules, and require closeout.

| Intent                     | Route                                                    |
| -------------------------- | -------------------------------------------------------- |
| Planning or execution docs | APS rules and module state                               |
| Architecture change        | Decision log, ADR need, as-built impact                  |
| ADR work                   | Numbering, status, supersession, decision-log entry      |
| As-built update            | Source references, gaps, APS/ADR/runbook links           |
| Runbook update             | Owner, trigger, commands, success/failure, rollback      |
| Guide update               | Audience, lifecycle, authority, upstream source          |
| Public docs                | Release/version alignment and user-facing behaviour      |
| Release docs               | Evergreen guide versus version-specific runbook evidence |
| Archive or retirement      | Supersession, stale markers, index updates               |
| Validation                 | Links, APS, ADRs, metadata, source references            |

The skill must answer three questions before editing:

1. What type of document is this?
2. What authority does it have?
3. What upstream source must it not duplicate or contradict?

## Closeout Protocol

Closeout is mandatory for documentation-affecting work. It prevents agents from
doing the visible edit and skipping hygiene.

Before final response, check:

- **Classification:** each changed document has an understood type and
  authority.
- **APS alignment:** active work is tracked in APS, and `plans/index.aps.md` is
  updated when module status or progress changes.
- **ADR alignment:** architecture decisions update ADRs and
  `plans/decisions/DECISION-LOG.md` when needed.
- **As-built alignment:** implementation claims cite code, schema, config, test,
  release, or generated artefacts.
- **Runbook alignment:** operational docs include executable commands, success
  output, failure modes, and rollback or safety notes.
- **Index alignment:** local README indexes and documentation maps still point
  to the authoritative entrypoints.
- **Stale-state handling:** stale or superseded information is marked inline,
  archived, fixed, or tracked in APS.
- **Validation:** relevant checks are run, or skipped with a reason.

Final responses for documentation changes should include a short closeout note:

```markdown
Docs Closeout:

- Authority checked: yes
- Indexes updated: yes/not needed
- Cross-links checked: yes
- Validation: <command or reason skipped>
- Residual drift risk: <none or short note>
```

## Minimal Validation Baseline

Until dedicated tooling exists, use the smallest relevant check set:

| Change         | Minimum validation                                             |
| -------------- | -------------------------------------------------------------- |
| Markdown only  | `pnpm format:check` plus manual link/source check              |
| APS files      | `pnpm format:check` plus manual status/progress reconciliation |
| ADR files      | `pnpm format:check` plus decision-log and numbering check      |
| Public docs    | `pnpm format:check` plus release/version source check          |
| Package README | `pnpm format:check` plus link/source path check                |
| Runbook        | `pnpm format:check` plus command review                        |

DOCGOV is replacing these manual checks with explicit validators over time.

## Automated Indexing Requirement

Documentation indexes must be generated, not manually maintained. The only
manual indexing input should be document-local metadata and, when needed, adding
a new approved tag to the tag catalogue.

Current generated-index flow:

```text
document metadata -> docs:index -> docs/indexes/ -> docs:check freshness check
```

The generator:

- scans governed documentation sources
- parses document-local metadata
- infers safe fields such as title and path
- generates `docs/indexes/` by type, authority, owner, status, and tag
- rejects unknown tags unless they exist in the approved tag catalogue
- relies on `pnpm docs:check` metadata validation for required metadata
- fails CI when generated indexes are stale

Required commands:

```bash
pnpm docs:index        # regenerate generated indexes
pnpm docs:index:check  # fail if generated indexes are stale
pnpm docs:check        # metadata, tags, links, and index freshness
```

Generated indexes must be marked as generated and must not contain hand-written
authority prose. They are discovery surfaces over canonical documents, not a new
source of truth.

## Drift Rule

Known documentation drift must not be left as tribal knowledge. Resolve it in
the same change, mark it stale, or create/link an APS work item.
