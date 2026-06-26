---
id: getting-started
title: Getting Started
description:
  Adopt APS in your project — templates, scaffolding, and first work item.
sidebar_position: 2
---

# Getting Started with APS

| Type        | Authority | Owner   | Status | Freshness                                               |
| ----------- | --------- | ------- | ------ | ------------------------------------------------------- |
| Public docs | Derived   | DOCSYNC | Live   | Last reviewed 2026-06-22 against anvil-plan-spec v0.4.0 |

| Upstream                                                                  | Downstream            |
| ------------------------------------------------------------------------- | --------------------- |
| [anvil-plan-spec](https://github.com/EddaCraft/anvil-plan-spec) `docs/**` | APS docs-site section |

This guide walks you through adopting APS in your project.

## Which template should I use?

| Situation                                     | Template       | Time to value |
| --------------------------------------------- | -------------- | ------------- |
| **Just trying APS**                           | quickstart     | 5 minutes     |
| **Small feature** (1–3 work items)            | simple         | 15 minutes    |
| **Module with boundaries** (interfaces, deps) | module         | 30 minutes    |
| **Multi-module initiative**                   | index          | 1 hour        |
| **Large initiative** (6+ modules)             | index-expanded | 1–2 hours     |
| **Monorepo** (multiple packages/apps)         | index-monorepo | 1–2 hours     |
| **Breaking a work item into actions**         | actions        | 15 minutes    |
| **Technical/architectural design**            | design         | 30 minutes    |
| **Tracking dev-time discoveries**             | issues         | 10 minutes    |

Templates ship in the
[anvil-plan-spec repository](https://github.com/EddaCraft/anvil-plan-spec/tree/main/templates).

## Quick start

**Want to see APS in action first?** Check the worked examples in
anvil-plan-spec:

- [User Authentication](https://github.com/EddaCraft/anvil-plan-spec/tree/main/examples/user-auth)
- [OpenCode Companion](https://github.com/EddaCraft/anvil-plan-spec/tree/main/examples/opencode-companion)

**Solo developer?** You do not need the full ceremony:

- Use `simple.template.md` for most features
- Skip formal modules — go straight to work items
- Only create an Index if you are planning weeks of work
- Action plans are optional — use when a work item feels complex

**Ready to scaffold?** Run this in your project:

```bash
curl -fsSL https://raw.githubusercontent.com/EddaCraft/anvil-plan-spec/main/scaffold/install | bash
```

Then run `aps init` — the Ratatui-based wizard walks you through agent ports,
modules, and project context. It creates `plans/` with templates and
`aps-rules.md` for AI guidance.

## Manual setup

If you prefer manual setup over the scaffold script:

### 1. Create folder structure

```text
your-project/
├── plans/
│   ├── index.aps.md
│   ├── issues.md
│   ├── modules/
│   ├── execution/
│   ├── designs/
│   └── decisions/
└── .aps/
    └── config.yml
```

### 2. Create your Index

Copy `index.template.md` to `plans/index.aps.md`. Fill in:

1. **Problem** — What are you solving?
2. **Success Criteria** — How do you know you are done?
3. **Modules** — List each bounded area of work

The Index is non-executable. Focus on intent, not implementation.

### 3. Create modules

For each module, create a file in `plans/modules/`:

- `module.template.md` — For modules with interfaces and dependencies
- `simple.template.md` — For small, self-contained features

Fill in Purpose, Scope, and leave Work Items empty until Ready.

### 4. Write a design (optional)

For complex work where the architecture is not obvious, create a design doc in
`plans/designs/` before defining work items. Name it
`YYYY-MM-DD-slug.design.md`.

### 5. Add work items when ready

Work items are **execution authority**. Only add them when:

- The module scope is clear
- Dependencies are resolved
- You are ready to implement

Each work item needs:

- **Intent** — One sentence on what it achieves
- **Expected Outcome** — Testable result
- **Validation** — How to verify completion

### 6. Generate action plans (optional)

For complex work items, create an actions file in `plans/execution/` named
`{WORK-ITEM-ID}.actions.md`. Each action has a **checkpoint** (observable
state). Actions can be grouped into **waves** for concurrent agents.

### 7. Track issues and questions

Log dev-time discoveries in `plans/issues.md`:

- **Issues (ISS-NNN)** — Bugs, limitations, tech debt
- **Questions (Q-NNN)** — Unknowns that need answers

## Drive work with the CLI

Once you have a plan with at least one `Ready` work item:

```bash
aps next                              # next ready item across modules
```

In the bash fallback/vendored runtime, `aps start` enforces that every dependency
is `Complete`, marks the item `In Progress`, and writes a focused context package
at `.aps/context/<ID>.md`. The native v0.4.0 binary currently exposes `next` but
not `start`/`complete`/`graph`; with the native-only install, edit the work-item
status in markdown and run its `Validation` command directly.

## Working with AI assistants

`aps init` scaffolds `plans/aps-rules.md` — point your agent at it and APS
conventions are followed by default. APS ships first-class agent definitions for
Claude Code, Codex, GitHub Copilot, OpenCode, and Gemini.

See [AI Agents →](./guides/ai-agents.md) for the full agent guide.

---

**Next:** [Installation →](./installation.md)
