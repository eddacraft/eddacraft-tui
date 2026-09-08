---
name: docs-workflow
description: Use when creating, editing, reviewing, or closing out documentation changes, especially Markdown under docs/**, plans/**, README files, runbooks, ADRs, guides, workflow docs, or docs-only PRs. Enforces project-specific docs formatting, linting, link checks, and evidence before completion claims.
---

# Docs Workflow

## Purpose

This skill complements `documentation-writing`, which improves the content; this
skill owns the workflow, validation evidence, and PR closeout discipline.

## Activation

Invoke this skill when:

- The user asks to create, update, review, or fix docs.
- Any change touches `docs/**`, `plans/**`, `README.md`, `CHANGELOG.md`,
  runbooks, ADRs, guides, tutorials, help text, or other Markdown files.
- A PR is docs-only or mostly docs.
- CI reports docs formatting, Markdown lint, link, or anchor failures.
- You are about to claim that documentation work is complete.

If the task also changes code, use `dev-loop` (or the project's implementation
workflow) for the implementation path, but keep this skill active for the docs
files and closeout evidence.

## Router

| Situation                           | Use                                                           | Result                                                   |
| ----------------------------------- | ------------------------------------------------------------- | -------------------------------------------------------- |
| New or rewritten docs               | `documentation-writing` when available                        | reader, doc type, structure, and tone are explicit       |
| Existing docs edited for accuracy   | current code, CLI output, tests, schemas, ADRs, and workflows | prose reflects source truth                              |
| Markdown formatting or lint failure | this skill                                                    | run the repo's docs gate, not only the failed subcommand |
| Broken links or anchors             | focused docs validation plus source inspection                | links point to current canonical headings and files      |
| Docs PR closeout                    | Docs Closeout template below                                  | validation is stated with command evidence               |

## Mandatory Command Discovery

Before committing, opening a PR, or saying docs validation passed, discover the
project's docs gate from repo-local truth. Check, in order:

1. `AGENTS.md`, `CLAUDE.md`, or other assistant instructions.
2. `package.json`, `justfile`, `Makefile`, CI workflows, or docs scripts.
3. Markdown lint, formatter, link-checker, or docs build configuration.

Prefer a single repo-provided closeout command when one exists. For example,
`pnpm docs:closeout` is better than cherry-picking individual subcommands.

## Command Rules

| Repo evidence                                        | Required action                                                                                            |
| ---------------------------------------------------- | ---------------------------------------------------------------------------------------------------------- |
| CI Docs Lint enforces Markdown formatting            | Run the formatter check, such as `pnpm run format:check`; `lint:md` alone is insufficient.                 |
| A docs closeout script exists                        | Run it and report the exact command and exit status.                                                       |
| Markdown files changed but no docs script exists     | Run the repo's formatter/linter pair, or state that no repo docs gate was found and list what you checked. |
| Links or anchors changed                             | Run the repo link/anchor check if present; otherwise manually inspect the changed targets.                 |
| Code examples, commands, or generated output changed | Verify the examples against current source, CLI output, tests, or schemas.                                 |

Never claim validation passed without exit 0 evidence from the command that was
actually run. If a command cannot be run, say why and name the remaining risk.

## Workflow

1. **Classify the doc change.** Name the affected doc type: guide, reference,
   runbook, ADR, release note, README, plan, or workflow doc.
2. **Identify the reader and job.** Use `documentation-writing` for substantial
   new content or rewrites.
3. **Inventory source truth.** Check the current implementation, CLI output,
   tests, schemas, CI workflows, ADRs, plans, or existing docs that the prose
   depends on.
4. **Edit narrowly.** Preserve the existing docs structure and voice unless the
   task is explicitly a rewrite.
5. **Check links and anchors.** Treat heading renames as API changes for docs:
   update inbound links or avoid the rename.
6. **Run docs validation.** Use the project-specific mandatory command table
   above.
7. **Close out with evidence.** Return the Docs Closeout block.

## Docs Closeout

Use this closeout block for docs tasks, PR descriptions, or handoffs:

```markdown
## Docs Closeout

- Changed files:
- Doc type:
- Reader/job:
- Source truth checked:
- Links/anchors checked:
- Commands run:
- Result: passed | failed | not run
- Remaining risk:
```

For `Commands run`, include exact commands and whether they exited 0. Do not use
aspirational wording such as "should pass" or "not needed" without explaining
the evidence.

## Common Failures

- Running `lint:md` while skipping the formatter check that CI enforces.
- Fixing the visible page but leaving inbound links or anchors stale.
- Treating plan prose as truth when the current code, CI, or command output has
  drifted.
- Marking a PR checklist item before the command has actually run.
- Using a generic validation command when the repo has a docs-specific closeout
  script.
