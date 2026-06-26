---
id: workflow
title: Workflow
description: The APS planning lifecycle — Plan, Execute, Validate, Learn.
sidebar_position: 4
---

# APS Workflow

| Type        | Authority | Owner   | Status | Freshness                                               |
| ----------- | --------- | ------- | ------ | ------------------------------------------------------- |
| Public docs | Derived   | DOCSYNC | Live   | Last reviewed 2026-06-22 against anvil-plan-spec v0.4.0 |

| Upstream                                                                  | Downstream            |
| ------------------------------------------------------------------------- | --------------------- |
| [anvil-plan-spec](https://github.com/EddaCraft/anvil-plan-spec) `docs/**` | APS docs-site section |

APS follows compound engineering: each unit of work should make future work
easier. The workflow has four phases that loop back to planning:

```
Plan → Execute → Validate → Learn → Plan again
  ↑                                      │
  └──────────────────────────────────────┘
```

| Phase        | What happens                | APS artefacts                       |
| ------------ | --------------------------- | ----------------------------------- |
| **Plan**     | Define scope and success    | Designs, Index, Modules, Work Items |
| **Execute**  | Work against specs          | Action Plans, status updates        |
| **Validate** | Check outcomes against spec | Review notes, checklist             |
| **Learn**    | Document solutions          | Solution docs in `docs/solutions/`  |

**The 80/20 principle:** Spend 80% of effort on planning and validation, 20% on
execution.

## Driving the loop with the CLI

```bash
aps next                              # Plan → discover next ready work item
# edit status to In Progress, implement, run validation
# edit status to Complete: YYYY-MM-DD, capture learnings
aps next                              # Loop
```

The native v0.4.0 binary resolves `next` and lints the plan. The bash
fallback/vendored runtime also provides `aps start` and `aps complete`, which
enforce the state machine (`Ready` → `In Progress` → `Complete`), check
dependencies, generate context packages, and write status lines consistently.

## Work item status vocabulary

Canonical values: `Draft`, `Ready`, `In Progress`, `Complete`, `Blocked`.

Accepted aliases (tools normalise internally; your files are not rewritten):

| Alias      | Canonical  | Notes                      |
| ---------- | ---------- | -------------------------- |
| `Proposed` | `Draft`    | Not yet actionable         |
| `Done`     | `Complete` | Terminal / compacted items |

Terminal compaction may also use `Merged` or `Released/Shipped` in Anvil's
release operating model. Lint treats those terminal labels as completed for
field-completeness checks. Orchestration dependency checks (`aps start`) only
accept `Complete` or `Done` on prerequisites — `Merged`/`Released/Shipped`
remain terminal but do not satisfy dependencies.

## Starting a feature

1. **Assess scope** — Single module or multiple? Create an Index for
   multi-module work.
2. **Write a design (optional)** — For non-obvious architecture, capture the
   "why this approach" in `plans/designs/`.
3. **Create the Index** — Problem, success criteria, module table.
4. **Draft modules** — Purpose and scope; leave Work Items empty while
   exploring.
5. **Get approval** — Share the Index; discuss scope and risks.
6. **Move to Ready** — Change module status to Ready; add work items.
7. **Execute** — Work through items; create action plans for complex ones.

## Mid-implementation

When you discover missing work:

With the bash fallback/vendored runtime, use `aps start AUTH-000` and
`aps complete AUTH-000 --learning "Migrations must run before schema-aware tests"`.
With the native-only v0.4.0 binary, hand-edit the status and learning fields.

Capture blockers in `plans/issues.md` rather than inventing new status values.
APS does not have a `Blocked` CLI transition — blocking is a planning question.

## Completion and archival

1. **Validate** — Run each work item's validation command. The bash
   fallback/vendored runtime's `aps audit <module>` does this mechanically.
2. **Mark module complete** — Bump the metadata table to `Complete` once every
   work item is Complete.
3. **Update the Index** — Reflect module status.
4. **Roll into completed archive** — Copy the work-item table into
   `plans/completed.aps.md`, grouped by release.
5. **Write a release narrative** — For multi-module releases, use
   `plans/releases/v<version>.md`.

## Review specs in PRs

A PR that implements a work item should carry:

- The status change (`In Progress` → `Complete`, with date) — `aps complete`
  does this
- A `Results:` line for non-trivial items, and any learnings captured
- New `Draft` items for work discovered along the way, or `ISS-NNN` entries in
  `plans/issues.md`
- A green `aps lint plans`

If the plan files did not change, ask whether the plan still matches reality —
that is exactly the drift `aps audit` exists to catch in the bash
fallback/vendored runtime.

## Learn phase

After solving non-trivial problems, document solutions in `docs/solutions/`
organised by category. Inline learnings from `aps complete --learning "..."` are
a good seed for a solution doc.

---

**Next:** [Terminology →](./terminology.md)
