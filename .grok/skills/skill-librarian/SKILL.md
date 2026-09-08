---
name: skill-librarian
description: >-
  Find, source, evaluate, and recommend skills, agents, profiles, bundles, or
  project-local tweaks for this catalogue. Use when the user asks for a skill
  for a task or domain, a base pack, a source to harvest from, or whether an
  existing skill can be adapted — the front door for skill requests in this
  catalogue, with `find-skills` handling generic third-party installs outside
  catalogue governance.
---

# Skill Librarian

Use this skill as the front door for skill and agent discovery in this repo. It
is a catalogue operating skill: it helps users find what already exists, decide
whether something is a good base, source candidates from outside the repo, and
route the next action to the right catalogue workflow.

This skill is intentionally installed locally for this repo as well as kept in
canonical source form. The repo itself is the knowledge base; cloning it should
bring the librarian with it.

## When To Use

Use for requests like:

- "I need a game development skill for a Godot game"
- "Find me a skill for using Higgsfield to generate motion graphics"
- "What skills should this project have?"
- "Is there already a skill for this?"
- "Can one of these skills be tweaked into what I need?"
- "Should this be a skill, profile, bundle, agent, or local wrapper?"
- "Where should we source a skill for this domain?"

Do not use this for catalogue mutations by itself. When the answer requires
changing the catalogue, hand off to `skill-repo` or the relevant creation
workflow. For project-specific wrappers, route to a retained localisation
workflow (e.g. `localiser`) when one is canonical; otherwise propose an
explicit project-local wrapper.

## Search Order

Search this repo before going online:

1. `skills/eddacraft/`, `skills/engineering/`, `skills/productivity/`,
   `skills/misc/`, and other canonical domain folders.
2. `profiles/*.json` for install policy and base packs.
3. `bundles/*/bundle.meta.json` for package-style installs.
4. `agents/**/AGENT.md` for specialist agents that may be the better answer.
5. `docs/harvest-targets.md` and `docs/skill-agent-sources.md` for known
   external sources.
6. `skills/in-progress/`, `skills/inbox/`, and `skills/deprecated/` only as
   unstable references, never as direct install recommendations.

If this search does not produce a good answer, source externally. Prefer
official docs, first-party repositories, active community skill repositories,
and examples with clear licensing/provenance. Prefer a runtime-provided readable
page extraction capability for ordinary web pages; use raw fetch only for APIs,
PDFs, or when extraction fails. Do not install a scraper without authority.

## Fit Decisions

Classify each candidate with one of these outcomes:

| Outcome             | Meaning                                                        | Next action                                   |
| ------------------- | -------------------------------------------------------------- | --------------------------------------------- |
| exact fit           | Existing skill/profile/bundle covers the request               | recommend use                                 |
| good base           | Existing asset covers most of it but needs context             | recommend a small edit or scoped wrapper plan |
| local wrapper       | Need is project-specific, not catalogue-wide                   | route to a retained localisation workflow (e.g. `localiser`) when one is canonical; otherwise propose an explicit project-local wrapper |
| harvest candidate   | External source has repeatable general value                   | hand off to `skill-repo` neutralisation       |
| new canonical skill | No source exists, but the workflow is reusable                 | propose new skill design                      |
| agent better        | Need is specialist research/review/operation, not a user skill | propose or use an agent                       |
| one-off             | Need is not repeatable enough for a skill                      | answer directly, do not create catalogue work |

Prefer adapting a good base over creating a new skill. Create a new skill only
when the workflow is repeatable, has a clear trigger contract, and is likely to
help more than one project or session.

## Delegation

For simple lookups, search and answer directly. For broad or external sourcing
work, delegate to the `skill-librarian` agent when the harness provides it;
otherwise perform the sourcing inline. When delegating, pass:

- the user's request;
- known project context;
- any must-have runtime or domain constraints;
- whether catalogue mutation is allowed;
- the expected output format below.

The specialist agent should inspect this repo, compare candidates, and update or
recommend updates to `docs/skill-agent-sources.md` when new durable sources are
found.

## Output Format

Return concise recommendations in this shape:

```markdown
## Skill Recommendation

- Best fit:
- Good base:
- Needs tweaking:
- Not suitable:
- Gap:
- Source candidates:
- Recommended next action:
```

Omit empty lines when they add no value. Cite paths for in-repo assets and URLs
for external candidates.

## Routing

- Use `skill-repo` for neutralising, promoting, archiving, duplicate detection,
  or catalogue status.
- Propose a project-local wrapper when a neutral skill needs project context;
  do not silently modify the canonical package.
- Use an available readable-page extraction capability for online docs and fall
  back to raw fetch when necessary.
- In this catalogue, this skill is the front door for skill requests;
  `find-skills` handles generic third-party ecosystem installs outside
  catalogue governance — hand the selected candidate to its workflow when an
  external install is the outcome.
- Use `creating-openclaw-skills` only when the target is specifically an
  openclaw workspace skill.
- Use `planning-workflow` when the recommendation becomes a multi-step project
  change.

## Source Registry

Keep `docs/skill-agent-sources.md` as the durable list of places to find skills,
agents, prompts, workflows, and domain packs. When a new source appears during a
search, recommend adding it there with status and notes. Maintaining
`docs/skill-agent-sources.md` (and `docs/harvest-targets.md`) on explicit user
request is the single direct mutation this skill performs; all other catalogue
changes hand off to `skill-repo`. Keep `docs/harvest-targets.md` focused on
sources already queued for catalogue harvesting.
