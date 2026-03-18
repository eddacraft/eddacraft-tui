# Restore Welcome Screen & Hands-On Tutorial

Module: **WELCOME**
Status: Ready
Owner: EddaCraft

## Problem

The Ratatui port (PORT-010, PORT-040–044) shipped a welcome screen and tutorial
that lost the "wow moment" from the original Ink version. The old flow scanned
the user's actual project, surfaced real findings, and guided them through a fix.
The new version is a static menu → generic tutorial with no project discovery.

Additionally:
- The scan in development only ever hits `__fixtures__/` test files (intentional
  anti-patterns), which isn't useful for demos or real users
- The fix step asked users to switch to their editor to fix a file, which is
  friction — they might not have the file open or know where it is
- Feature tutorials (Policy, Architecture, Drift, CI) are "show and tell" rather
  than hands-on with real codebase findings

## Design Decisions

### DD-1: Scan filtering

Exclude paths matching these patterns from tutorial scan results:
- `__fixtures__/`, `__mocks__/`, `__tests__/`, `test-data/`, `fixtures/`
- `*.test.ts`, `*.spec.ts`, `*.test.rs`, `*_test.rs`
- `node_modules/`, `target/`, `.git/`

If after filtering the scan finds zero real warnings, enter **showcase mode**:
show a curated set of example findings with clear "[Example]" labels so the user
still sees what Anvil catches, but knows these aren't from their project.

### DD-2: Inline editor

When the fix step presents a warning, offer to open the file in a minimal
embedded editor panel within the TUI (read-only context + editable region around
the warning line). This uses the existing `TextInputState` widget extended to a
multi-line `EditorState` widget. User can make the fix without leaving the TUI.

If the user prefers their own editor, show the full file path (clickable in
terminals that support OSC 8 hyperlinks) and wait for the file to change on disk.

### DD-3: Feature tutorial integration

Each feature tutorial path (Policy, Architecture, Drift, CI) gets an optional
**"In Your Project"** step that runs a targeted scan for findings relevant to
that feature. E.g.:
- Policy tutorial: scan for policy violations
- Architecture tutorial: scan for import/boundary violations
- Drift tutorial: if snapshots exist, show drift diff
- CI tutorial: check for existing CI config, show what Anvil would add

If no relevant findings exist, show a curated example instead.

## Work Items

### Phase 1: Discovery Scan Surface

- **WELCOME-001**: Create `ScanFilter` in `anvil-checks` that excludes test
  fixture paths from results. Add unit tests for pattern matching.

- **WELCOME-002**: Create `DiscoverySurface` in
  `crates/anvil-tui/src/surfaces/tutorial/discovery.rs`. Phases: Scanning
  (spinner + progress from kernel events) → Results (warning list with
  file:line, message, suggestion) → Continue. Thread `ScanResults` into
  tutorial state.

- **WELCOME-003**: Implement **showcase mode** fallback. When filtered scan
  returns zero warnings, display 3–4 curated example findings covering
  different check types (secret detection, anti-pattern, architecture
  violation). Each clearly labelled "[Example]" with muted styling.

- **WELCOME-004**: Wire discovery into welcome flow. After user selects
  "Interactive Tutorial" from welcome menu, run discovery scan before
  entering tutorial path selection. Pass `ScanResults` through to tutorial
  state.

### Phase 2: Inline Editor Widget

- **WELCOME-005**: Create `EditorState` / `EditorWidget` in
  `crates/eddacraft-tui/src/widgets/editor.rs`. Multi-line text editing
  with: line numbers, syntax-aware line highlighting (current line), scroll,
  cursor movement (hjkl/arrows, Home/End, PgUp/PgDn), insert/delete/
  backspace. Load from file path, save back to file.

- **WELCOME-006**: Create fix step in tutorial that presents the top warning
  with context (5 lines above/below), opens inline editor focused on the
  warning line, and validates the fix by re-running the check on save.
  Fallback: show file path + watch for external changes.

### Phase 3: Hands-On Feature Tutorials

- **WELCOME-007**: Add **"In Your Project"** step to Policy tutorial path.
  Run policy-specific checks against the user's codebase. Display real
  policy violations if found, otherwise show curated example. Step content
  adapts based on findings.

- **WELCOME-008**: Add **"In Your Project"** step to Architecture tutorial
  path. Run architecture checks (import rules, module boundaries). Display
  real violations or curated example.

- **WELCOME-009**: Add **"In Your Project"** step to Drift tutorial path.
  Look for existing snapshots in `.anvil/snapshots/`. If found, show drift
  between latest two. Otherwise, capture a baseline and explain what drift
  detection does.

- **WELCOME-010**: Add **"In Your Project"** step to CI tutorial path.
  Detect existing CI configuration (`.github/workflows/`, `.gitlab-ci.yml`,
  `Jenkinsfile`). Show what Anvil hooks/checks would integrate. If no CI
  found, show setup instructions.

### Phase 4: Welcome Flow Polish

- **WELCOME-011**: Restore watch demo step in core tutorial. After discovery
  scan, launch file watcher showing real-time check results. Progressive
  hints after 10s/20s/30s. Skip with 's'.

- **WELCOME-012**: Wire `ScanResults` threading through all tutorial phases.
  Results from discovery flow into feature tutorials so "In Your Project"
  steps can reference already-found issues rather than re-scanning.

- **WELCOME-013**: Add tutorial progress persistence to
  `~/.anvil/tutorial-progress.json` (already exists in Ratatui tutorial but
  needs to include discovery completion state and scan results cache).

## File Map

```text
crates/anvil-checks/src/filter.rs: WELCOME-001
crates/anvil-checks/src/filter_test.rs: WELCOME-001
crates/anvil-tui/src/surfaces/tutorial/discovery.rs: WELCOME-002, WELCOME-003
crates/anvil-tui/src/surfaces/tutorial/discovery_render.rs: WELCOME-002, WELCOME-003
crates/anvil-tui/src/surfaces/tutorial/showcase.rs: WELCOME-003
crates/anvil-tui/src/surfaces/welcome/mod.rs: WELCOME-004
crates/anvil-tui/src/surfaces/welcome/render.rs: WELCOME-004
crates/eddacraft-tui/src/widgets/editor.rs: WELCOME-005
crates/anvil-tui/src/surfaces/tutorial/fix.rs: WELCOME-006
crates/anvil-tui/src/surfaces/tutorial/fix_render.rs: WELCOME-006
crates/anvil-tui/src/surfaces/tutorial/paths.rs: WELCOME-007, WELCOME-008, WELCOME-009, WELCOME-010
crates/anvil-tui/src/surfaces/tutorial/mod.rs: WELCOME-004, WELCOME-011, WELCOME-012, WELCOME-013
crates/anvil-tui/src/surfaces/tutorial/render.rs: WELCOME-011, WELCOME-012
```

## Dependencies

- `run_embedded()` from `anvil-kernel` for project scanning
- `events_to_gate_result()` from gate surface for event conversion
- `TextInputState` from `eddacraft-tui` as base for editor widget
- Existing `EddaCraftTheme` for consistent styling

## Risks

- **Scan performance**: `run_embedded()` on large projects could be slow.
  Mitigate: show progress spinner with file count, cap scan to first 500
  files, allow skip with 's'.
- **Editor complexity**: A full inline editor is a significant widget.
  Mitigate: Phase 2 can ship with a minimal version (no syntax highlighting)
  and iterate.
- **Showcase mode staleness**: Curated examples could drift from actual check
  capabilities. Mitigate: derive examples from `__fixtures__/` at build time
  so they stay current.
