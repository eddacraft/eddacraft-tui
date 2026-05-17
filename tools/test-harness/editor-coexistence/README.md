# Editor Coexistence Test Harness

Verifies that `anvil watch` runs against the same working tree as common
language servers and formatters without conflict. Owned by ADOPT-006; policy at
[`docs/policies/editor-coexistence.md`](../../../docs/policies/editor-coexistence.md).

## What it checks

For each supported target tool (rust-analyzer, tsserver, pyright, ruff,
prettier, eslint) and a matching fixture under `fixtures/<lang>/`, the harness:

1. Starts `anvil watch --source` against the fixture and waits for the initial
   scan to settle.
2. Runs the target tool in its non-mutating mode (LSP `initialize` + `shutdown`
   for language servers, `--check` for formatters).
3. Stops `anvil watch` (SIGTERM) and waits for clean exit.
4. Asserts: the target tool exited 0, `anvil watch` exited cleanly, no `EBUSY` /
   `EAGAIN` lock-contention messages appear in either log, and `anvil watch`
   reported no save-time findings against the fixture baseline.

A target is **skipped** if its binary is not on PATH on the current machine. The
list of targets that must be present on a CI runner lives in
`required-targets.txt`; the runner fails if more than the allowed number of
targets skip.

## Layout

```
editor-coexistence/
├── README.md
├── manual-protocol.md          # Boring Week desktop-editor protocol
├── required-targets.txt        # CI presence floor
├── run-harness.sh              # entry point — emits JSON verdict
├── fixtures/
│   ├── rust/                   # cargo crate
│   ├── typescript/             # tsc-checkable package
│   └── python/                 # pyright/ruff-checkable module
└── targets/
    ├── rust-analyzer.sh
    ├── tsserver.sh
    ├── pyright.sh
    ├── ruff.sh
    ├── prettier.sh
    └── eslint.sh
```

Each `targets/<name>.sh` is a self-contained runner that:

- Exits 0 if the target is unavailable (the harness records that as a `skip`).
- Otherwise runs the target against the fixture and exits 0 on pass, non-zero on
  fail.

## Running locally

```bash
# From repo root, with an Anvil binary built:
ANVIL_BIN=$(pwd)/target/debug/anvil \
  tools/test-harness/editor-coexistence/run-harness.sh \
  > editor-coexistence-verdict.json
```

The verdict shape and CI assertion contract are documented in
[`docs/policies/editor-coexistence.md`](../../../docs/policies/editor-coexistence.md#verification-protocol).

## Adding a new target

1. Add a `targets/<tool>.sh` runner that respects the "unavailable → exit 0"
   convention.
2. If the tool requires its own fixture (e.g. a new language), add the fixture
   under `fixtures/<lang>/` and reference it from the runner.
3. Update `required-targets.txt` if CI should fail when the tool is missing.
4. Add the new column to the matrix in
   [`docs/policies/editor-coexistence.md`](../../../docs/policies/editor-coexistence.md#coexistence-matrix-v1).

## Why not GUI editors here

The matrix policy doc explains the trade-off. tl;dr: a headless xvfb harness for
VS Code / JetBrains / Cursor / Neovim costs more maintenance hours than the bugs
it would catch. They live in the manual Boring Week protocol
(`manual-protocol.md`) instead.
