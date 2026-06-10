# Lessons from the 2026-06-10 UJ loop run

One-line summary: serialized APS flips via "code-PR-first, flips after the
previous UJ PR merges" avoided every count-cell collision across 7 PRs; the
two recurring traps were oxfmt reflow silently no-opping exact-string patch
scripts and Copilot threads landing minutes after PR creation.

## What worked (keep doing)

- **Deferred-flips pattern for one module's hot count cell.** Open the code
  PR immediately; push the module/index status flips + journal entry to the
  SAME branch only after the previous UJ PR merges, then arm auto-merge.
  Zero rebase conflicts on `plans/index.aps.md` across PRs #2500–#2506.
- **Predicting a PR number for a flip is safe only after `gh pr create`.**
  Cycle 4 predicted #2503 pre-creation and got lucky; flip after creation.
- **Fresh-context verification catches real doc/code drift.** It found a
  nonexistent `anvil workspace remove` (pre-existing config.md bug), a
  misattached doc-comment, and an over-claimed `=1` semantic — all things
  the implementer's context had rationalised.

## Traps (avoid)

- **oxfmt reflow breaks exact-string patch scripts silently.** A python
  `str.replace` that matched the pre-format text no-ops after oxfmt reflows
  the paragraph (Copilot then flags the "fixed" issue). Always `assert old in s`
  in patch scripts, and grep the file after formatting to confirm the edit
  survived.
- **Copilot reviews arrive a few minutes after PR creation and BLOCK
  auto-merge.** Poll review threads once per cycle and resolve via GraphQL
  `resolveReviewThread`; address-or-rebut every thread.
- **The beta licence gate blocks manual transcripts for gated commands**
  (`status`, `start`, `welcome`, `init`, `watch`) in the agent environment.
  Plan validation around unit tests + `--help`/ungated surfaces (`dashboard`
  was ungated). Gate-unavailable is not a content veto.
- **Patching struct initializers by suffix-matching a field line hits the
  WRONG structs** (`WatchData` also has `insights_hint`). Let the compiler
  list E0063 sites and patch only those.
