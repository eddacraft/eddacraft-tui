# ATTRIB-008 — Multi-block dispatcher + driver-per-ecosystem architecture

## Purpose

Land the keystone refactor of `tools/starters/acknowledgements/`. The
generator becomes a dispatcher that reads a `[[blocks]]` array from
`attribution.toml` and routes each block to an ecosystem-specific
driver script under `drivers/<ecosystem>.sh`. The Rust block is
extracted from the current generator body into `drivers/rust.sh` with
no behaviour change.

Unblocks ATTRIB-012 (Node), ATTRIB-013 (Go), ATTRIB-014 (Python),
and ATTRIB-015 (Anvil's own Node devtools block).

Full design contract at
[`plans/specs/2026-05-22-acknowledgements-multi-block-and-multi-eco.md`](../../specs/2026-05-22-acknowledgements-multi-block-and-multi-eco.md).

## Decisions (recorded on kickoff, 2026-05-24)

| Question | Decision | Rationale |
| --- | --- | --- |
| Driver runtime | Bash, per-ecosystem scripts under `drivers/` | Pinned in the module Decisions table 2026-05-22; revisit only if a single driver outgrows shell. |
| Back-compat shape | Flat `[rust]` top-level table auto-promotes to a single unnamed block | Existing consumers (Anvil today, `eddacraft-tui` via the mirror) don't migrate. |
| Mixed schema policy | Flat `[rust]` and `[[blocks]]` array in the same file fails preflight with an actionable error | Avoid silent precedence rules; force the operator to pick a shape. |
| Atomic-write granularity | Single whole-file `mv` at the end of the splice loop (not per-block) | Preserves the no-partial-clobber invariant cleanly; matches the existing single-block contract. Per-block writes would multiply failure modes. |
| Stub-ecosystem driver | Each test stands up its own stub drivers under an `mktemp -d` fixture and points the dispatcher at them via `ATTRIB_DRIVERS_DIR` | Keeps the production `drivers/` set clean and avoids checking test-only files into the kit tree. Equivalent to the earlier "tests/fixtures/" plan but per-test ephemeral. |
| Driver invocation contract | `drivers/<ecosystem>.sh <block-config-json> <output-temp-path>` | Argv form matches the spec; JSON config keeps the driver/dispatcher boundary narrow. |

## Actions

### 1. Lock the dispatcher + driver-author contracts

- **Purpose:** Cite the existing spec as the authoritative design. No
  redesign in this PR.
- **Produces:** A short README section in the kit pointing at
  `plans/specs/2026-05-22-acknowledgements-multi-block-and-multi-eco.md`
  for the schema, dispatcher contract, and driver-author contract.
- **Checkpoint:** README cross-reference resolves; spec needs no edits.

### 2. Write failing dispatcher tests

- **Purpose:** Pin the new behaviour with red tests before refactoring.
- **Produces:** Two new test files (back-compat shim coverage comes
  from the pre-existing `tests/strict-license-field.sh`, which uses a
  flat-`[rust]` config and exercises the shim end-to-end through the
  rust driver — a dedicated `dispatcher-shim.sh` would be redundant):
  - `tests/dispatcher-schema-validation.sh` — 5 scenarios: mixed
    flat-`[rust]` + `[[blocks]]`, missing `name`, missing `ecosystem`,
    unknown `ecosystem` (no matching `drivers/<eco>.sh`), duplicate
    block name.
  - `tests/dispatcher-two-block.sh` — 3 scenarios using stub drivers
    (planted under each test's own `mktemp -d`, surfaced via the
    `ATTRIB_DRIVERS_DIR` env override): two-block round-trip +
    idempotency + `--check`; partial regeneration leaves the other
    block byte-identical; driver failure leaves the on-disk target
    byte-identical.
- **Checkpoint:** Both new test files fail against the current
  single-block generator; both pass against the dispatcher refactor
  in Action 4.

### 3. Extract `drivers/rust.sh` with no behaviour change

- **Purpose:** Carve out the cargo-about invocation into the driver
  shape the dispatcher expects, without changing what it does.
- **Produces:** `drivers/rust.sh` accepting
  `<block-config-json> <output-temp-path>`. Reads `manifest_path`,
  `template_path`, `config_path` from the JSON; runs `cargo about
  generate --fail` exactly as the existing generator does; writes
  rendered markdown to the output path.
- **Checkpoint:** Existing single-block flow continues to pass
  `tools/starters/acknowledgements/generate-acknowledgements.sh --check`
  against Anvil's real Cargo graph (no diff to `ACKNOWLEDGEMENTS.md`).

### 4. Refactor the generator into a dispatcher

- **Purpose:** Move the top-level shape from "one cargo-about call" to
  "parse → loop blocks → invoke driver → splice → atomic mv".
- **Produces:** Updated
  `tools/starters/acknowledgements/generate-acknowledgements.sh`:
  reads `[[blocks]]` array from `attribution.toml`; auto-promotes flat
  `[rust]` to a single unnamed block; rejects mixed schemas;
  per-block marker-count gate; per-block splice into a working temp;
  single whole-file `mv` at end. CLI surface unchanged (`--check`,
  `--output`, `--config`, `-h`/`--help`).
- **Checkpoint:** All three dispatcher tests from Action 2 now pass,
  AND all pre-existing tests (`tests/strict-license-field.sh`,
  `tests/licences-drift.sh`) still pass.

### 5. Wire per-block markers + back-compat suffix rules

- **Purpose:** Markers carry the block name as a suffix; empty name
  (back-compat shim) uses the unsuffixed legacy default.
- **Produces:** Dispatcher generates marker strings via
  `marker_begin + (name ? " " + name : "") + marker_close` (and
  similarly for END). `[project].marker_begin` / `marker_end`
  overrides continue to work; per-block marker-count gate uses the
  composed marker text.
- **Checkpoint:** Two-block fixture splice-loop emits
  `<!-- BEGIN AUTO-GENERATED rust -->` and
  `<!-- BEGIN AUTO-GENERATED _stub -->` correctly; back-compat fixture
  still emits the legacy unsuffixed pair.

### 6. Document the dispatcher + driver-author contract in the kit README

- **Purpose:** Consumers adopting the kit need to know the new shape;
  driver authors need to know what their script must satisfy.
- **Produces:** README additions covering: `[[blocks]]` schema with
  examples; back-compat shim; dispatcher steps; driver-author contract
  (preflight / render / strict-license / no-side-effects on target);
  monorepo manifest-scoping pattern. Pointer at the v3.2 spec for
  fuller design rationale.
- **Checkpoint:** `markdownlint` clean; cross-references resolve;
  README reads as drop-in for downstream subtree consumers.

### 7. Verify the full kit suite + drift + format

- **Purpose:** Prove no regression before opening the PR.
- **Produces:** Green output from every kit test
  (`bash tools/starters/acknowledgements/tests/*.sh`), drift-check
  clean (`node scripts/aps/drift-check.mjs`), `pnpm run format`
  produces no untracked changes to the kit.
- **Checkpoint:** All checks green locally; ready for CI.
- **Validate:**
  `bash tools/starters/acknowledgements/tests/dispatcher-schema-validation.sh && bash tools/starters/acknowledgements/tests/dispatcher-two-block.sh && bash tools/starters/acknowledgements/tests/strict-license-field.sh && bash tools/starters/acknowledgements/tests/licences-drift.sh && tools/starters/acknowledgements/generate-acknowledgements.sh --check && node scripts/aps/drift-check.mjs`

### 8. Open the PR

- **Purpose:** Ship the refactor; clear the keystone gate for
  ATTRIB-012/013/014/015.
- **Produces:** PR with atomic commits per logical chunk (tests; rust
  driver extraction; dispatcher refactor; README); body cites the
  v3.2 spec, summarises the dispatcher and driver shapes, lists the
  full verification suite from Action 7, calls out that the mirror to
  `eddacraft/acknowledgements-starter` will pick up the schema change
  on next push (back-compat shim means `eddacraft-tui` keeps working
  without migration).
- **Checkpoint:** PR opened against `main`; all CI green; mirror
  workflow either runs on merge or is queued (existing mirror trigger
  policy applies — see ATTRIB-011 execution notes).

## Post-merge follow-ups (not in scope for this PR)

- ATTRIB-012/013/014: each adds a new `drivers/<eco>.sh` against the
  contract this PR establishes.
- ATTRIB-015: Anvil's own `ACKNOWLEDGEMENTS.md` grows a
  `node-devtools` block; depends on ATTRIB-012.
- A separate operator-facing migration note in the mirror's
  `MIRROR-README.md` if/when an external consumer adopts a non-Rust
  block (no migration needed for current Rust-only consumers).
