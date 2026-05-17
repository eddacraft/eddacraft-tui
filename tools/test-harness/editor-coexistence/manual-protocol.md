# Boring Week — Desktop Editor Coexistence Protocol

Run once per editor row in
[`docs/policies/editor-coexistence.md`](../../../docs/policies/editor-coexistence.md#coexistence-matrix-v1)
during the candidate's Boring Week. Record results in
`plans/releases/<version>-boring-week.md`.

## Pre-flight

1. Clean working tree on the candidate Anvil version (`anvil --version` matches
   the candidate tag).
2. `anvil install` against a clone of this repo or of
   `tools/test-harness/editor-coexistence/fixtures/<lang>/`.
3. `anvil watch --source` running in a separate terminal.

## Per-editor scenario

For each editor cell, perform the actions in order. Anything other than "happy
path completed silently" is a fail.

### VS Code / Cursor

1. Open the fixture folder; let `rust-analyzer` / `tsserver` / `pyright` index
   fully (status bar idle).
2. Open one source file from each language fixture.
3. Save with no edit (Ctrl+S / Cmd+S). Expected: no Anvil event reported, no
   editor toast.
4. Add a trailing newline; save. Expected: Anvil emits one event for the touched
   file, the formatter does not loop, the LSP re-indexes once.
5. Run **Format Document** (Shift+Alt+F / Shift+Option+F). Expected: file
   reformats once; Anvil emits at most one event; no `EBUSY` toast.
6. Run **ESLint: Fix All** (or the language equivalent). Expected: same as
   step 5.

### JetBrains IDEA / RustRover / WebStorm / PyCharm

1. Open the fixture as a project; let indexing complete.
2. Repeat the four save / format / fix steps above using JetBrains commands (⌥⇧L
   on macOS).
3. Trigger a **Code Cleanup** on the project. Expected: project-wide reformat
   completes; Anvil events are bounded by the number of changed files; no Anvil
   errors in its log.

### Neovim (nvim-lspconfig)

1. `nvim` into the fixture; let `:LspInfo` show an attached server.
2. `:write` an unchanged buffer (no-op save).
3. Edit a single line, `:write`. Expected: Anvil reports one event.
4. `:lua vim.lsp.buf.format()`. Expected: one save, one Anvil event.
5. `:!ruff check .` / `:!prettier --check .` / `:!eslint .` in a buffer shell.
   Expected: target exits 0; Anvil log shows no errors.

## Pass criteria

A row in the matrix is **pass** for the candidate when:

- Every step above completed without a user-visible Anvil error.
- The editor's normal LSP / format flow worked as it would with Anvil
  uninstalled.
- `anvil watch`'s log shows no `EBUSY`, `EAGAIN`, lock-contention, or panic
  messages.

## Failure handling

A single failed row blocks the candidate. The triage path:

1. Capture the `anvil watch` log and the editor's LSP log.
2. Open an ADOPT-006 follow-up entry in the module spec naming the editor, OS,
   and tool versions.
3. Either (a) reproduce headlessly and add a runner to `targets/<tool>.sh` so
   the harness catches it next time, or (b) document a precise exclusion in the
   matrix policy.
