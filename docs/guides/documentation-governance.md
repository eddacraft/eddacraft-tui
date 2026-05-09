# Documentation Governance

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

| Type        | Purpose                               | Location                                  |
| ----------- | ------------------------------------- | ----------------------------------------- |
| APS index   | Module discovery and active state     | `plans/index.aps.md`                      |
| APS module  | Execution authority                   | `plans/modules/*.aps.md`                  |
| ADR         | Durable decision rationale            | `plans/decisions/*.md`                    |
| Spec        | Intended design before or during work | `plans/specs/`, `docs/specs/`             |
| As-built    | Current implementation map            | `docs/architecture/*-as-built.md`         |
| Runbook     | Operational procedure                 | `docs/runbooks/*.md`                      |
| Guide       | Developer practice                    | `docs/guides/*.md`                        |
| README      | Local orientation                     | nearest package, crate, app, or directory |
| Public docs | User-facing behaviour                 | `docs/public/**/*.md`                     |
| Archive     | Historical reference                  | `docs/archive/`, `plans/archive/`         |

Current migration exception: the evergreen release runbook remains at
`docs/guides/release-runbook.md`. Treat it as runbook authority until DOCGOV-008
either moves it, renames it, or records why the exception remains.

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

DOCGOV will replace these manual checks with explicit validators over time.

## Drift Rule

Known documentation drift must not be left as tribal knowledge. Resolve it in
the same change, mark it stale, or create/link an APS work item.
