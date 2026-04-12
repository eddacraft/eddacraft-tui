# Welcome & First-User Experience Fix

**Date:** 2026-04-12
**Branch:** fix/welcome
**Module:** WELCOME (reopen from Complete → In Progress)

## Problem

The WELCOME module (18/18 marked Complete) has 4 stub implementations and 1
partial, plus several UX issues reported by beta testers. The intended "wow
moment" — where a new user sees Anvil scan their actual codebase, find real
issues, and fix one live — does not work. Users see fake findings about
nonexistent files, a config wizard that doesn't save, a fix button that doesn't
respond, and a watch mode that tells them to use the CLI.

## Issues (from beta testing)

| # | Type        | Summary                                              | Root Cause                                       |
|---|-------------|------------------------------------------------------|--------------------------------------------------|
| 1 | improvement | Prompt during install to reset tutorial progress     | No reset prompt exists                           |
| 2 | improvement | Print first command after install script completes    | install.sh has no post-install guidance           |
| 3 | ux          | "Add existing project" not visible as an option      | Hidden behind "Start guided setup" label          |
| 4 | bug         | Discovery shows example data, not real project       | `showcase_findings()` hardcoded; real scan absent |
| 5 | bug         | Fix action ('f') shown but doesn't work              | FixState not wired into tutorial state machine    |
| 6 | ux          | Esc navigation confusing — multiple menus            | Double-esc required; flow not obvious             |
| 7 | improvement | Doctor TUI should let you action/fix interactively   | Fixes only via `--fix` CLI flag                   |
| 8 | improvement | `doctor --fix` should cover more checks              | Only 3/8 checks auto-fixable                     |
| 9 | bug         | "Start Watch" in welcome hub does nothing            | Handler is a stub, shows message only             |

## Stub/Incomplete Work Items

| Work Item    | What's Missing                                              |
|--------------|-------------------------------------------------------------|
| WELCOME-003  | Config never written after init wizard (`TODO` at L156)     |
| WELCOME-007  | Showcase not wired as fallback for zero real findings       |
| WELCOME-013  | Using `showcase_findings()` instead of real scan            |
| WELCOME-014  | Watch mode stub in welcome hub                              |
| WELCOME-010  | Fix surface built but never integrated into tutorial flow   |

## Design Decisions

### DD-A: Real scan uses secret + antipattern only

Architecture and policy violations require user-defined rules (boundaries,
policies) which don't exist on first run. The discovery scan runs only:

- **Secret scanner** — detects leaked API keys, tokens, credentials
- **Anti-pattern scanner** — detects TODOs, code smells, naming issues

Both work on any codebase with zero configuration. Uses `ScanFilter` (already
built in WELCOME-004) to exclude test fixtures, node_modules, target, .git.

Architecture/policy showcase examples appear **separately** below real results
as "Unlock more checks by configuring rules in the tutorial" — never mixed with
real findings.

### DD-B: Onboarding label clarity

Replace the three onboarding options:

**Before:**
1. "Start guided setup" — "Configure Anvil for your project step by step"
2. "Skip to tutorial" — "Jump straight into the interactive tutorial"
3. "Skip entirely" — "Go to the command menu"

**After:**
1. "Set up this project" — "Add Anvil to your codebase and scan for issues"
2. "Explore the tutorial" — "Learn what Anvil can do with a guided walkthrough"
3. "Go to command menu" — "Skip setup — you can always come back with `anvil start`"

The init wizard's Mode step already has `New / Existing / Minimal` — the
relabelled option makes it clear this works for existing projects.

### DD-C: Previous install detection

On first run, before showing onboarding options, check for existing state:

- `~/.anvil/tutorial-progress.json` exists → offer "Reset previous progress?"
- `.anvil/` directory exists but no config → offer "Previous install detected —
  start fresh?"

This handles the case where an old/broken install left state behind.

### DD-D: Post-install message

After the cargo-dist installer completes successfully, `install.sh` prints:

```
  Anvil installed successfully!

  Get started:
    cd your-project/
    anvil start

  Or run anvil --help for all commands.
```

### DD-E: Watch mode from welcome hub

Wire watch into the welcome hub the same way gate, audit, and doctor are wired:
initialise the kernel watcher, create a `WatchState`, run the surface. The
infrastructure exists in `watch.rs` — it's a plumbing task.

### DD-F: Fix integration in tutorial

When the discovery scan finds real findings, offer a "Fix the top issue" step
before entering tutorial path selection. This uses the existing `FixState`
surface with the top-severity finding from real scan results. Wire it into the
flow between discovery and path selection.

For tutorial steps that have fixable findings from the domain scan, add an 'f'
key handler in `handle_running()` that opens the fix surface for the relevant
finding.

### DD-G: Navigation simplification

- Esc from any tutorial phase exits the tutorial entirely (back to welcome hub),
  not to path select. Path select is reached via the welcome hub "Interactive
  tutorial" option.
- Remove the double-esc pattern. One esc = one level back, always.
- Welcome hub is the single "home" screen. All surfaces return to it on esc.

### DD-H: Doctor TUI actionability

Add key handlers in the doctor TUI surface:
- 'f' on a fixable check → run the auto-fix inline, update status
- Enter on a failing check → show details + suggestion

Extend auto-fixable checks to include:
- `hooks-installed` → offer to create `.husky/pre-commit`
- `plans-dir` → create `plans/` directory

## Scope Boundary

**In scope:** Items 1–9 from the issues table, reopened WELCOME work items,
navigation fixes, doctor improvements.

**Out of scope:** New tutorial paths, new check types, syntax highlighting in
the inline editor, dashboard integration.

## Success Criteria

A new user who runs `install.sh` → `anvil start` in an existing project should:

1. See a clear post-install message telling them what to do
2. Choose "Set up this project" and complete the init wizard
3. See their config written to disk (`.anvil.yaml`)
4. See Anvil scan their actual codebase and find real issues (or showcase if clean)
5. Fix at least one issue via the inline editor or file watcher
6. See a completion summary with actionable next steps
7. Access watch mode, doctor, gate, audit from the welcome hub — all functional

Total time from install to first fix: under 3 minutes.
