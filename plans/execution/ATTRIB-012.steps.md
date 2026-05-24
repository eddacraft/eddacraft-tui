# ATTRIB-012 — Node ecosystem driver

## Purpose

Add the first non-Rust ecosystem driver against the multi-block
dispatcher contract that ATTRIB-008 landed. `drivers/node.sh` shells
`license-checker` against a single `package.json`, emits deterministic
markdown sorted by package name, and enforces the canonical allow-list
through `license-checker --onlyAllow`.

Unblocks ATTRIB-015 (Anvil's own `node-devtools` block in
`ACKNOWLEDGEMENTS.md`) and proves the dispatcher contract generalises
beyond Rust.

Full design contract at
[`plans/specs/2026-05-22-acknowledgements-multi-block-and-multi-eco.md`](../specs/2026-05-22-acknowledgements-multi-block-and-multi-eco.md)
§ "Node — `drivers/node.sh` (ATTRIB-012)".

## Decisions (recorded on kickoff, 2026-05-24)

| Question | Decision | Rationale |
| --- | --- | --- |
| Strict-license CLI shape | `license-checker --onlyAllow '<semicolon-list>'`, not `--failOn` | Spec's open question (line 364-367) resolved against the live CLI: `license-checker@25.0.1` exposes both `--failOn` (deny-list) and `--onlyAllow` (allow-list), both semicolon-separated. `--onlyAllow` matches `licences.toml`'s positive-allow shape directly — no deny-side inversion needed. |
| Allow-list fragment shape | Plain text file `licences.node-allow.txt` at project root, single line, `;`-joined SPDX list, emitted by the ATTRIB-006 expander between dedicated BEGIN/END markers | Node has no native config-file home equivalent to `about.toml` / `deny.toml`. A standalone single-line text file keeps the driver invocation simple (`--onlyAllow "$(cat licences.node-allow.txt)"`), keeps the expander generic, and gives the consumer a stable file to commit. |
| Markdown render path | jq pipeline against `license-checker --json` output, formatted to the same column structure the Rust block uses in `ACKNOWLEDGEMENTS.md` | `license-checker` ships no markdown output (json / csv / summary only). A jq filter keeps the driver shell-only (no node script) and gives byte-stable output sorted by package name. |
| Block keys | `manifest_path` (required, absolute or project-relative `package.json`); `node_allow_path` (required, points at the consumer's `licences.node-allow.txt`); `prod_only` (default `true`); `exclude` (optional semicolon-separated `package@version` list — matches `license-checker --excludePackages`'s native format; glob support is a future driver extension) | Matches spec § Node, sharpened to the live `license-checker@25.0.1` CLI surface. `prod_only=true` defaults to `--production` to keep dev-tooling out of consumer ACKNOWLEDGEMENTS; consumers wanting devtools (e.g. ATTRIB-015) set `prod_only=false` explicitly. |
| Preflight scope | Verify `jq` + `license-checker` on PATH; verify `manifest_path` exists; verify `node_modules` exists under the manifest's directory | Empty `node_modules` would silently produce a one-entry block (the consumer's own package). Hard error with `pnpm install` / `npm install` actionable hint instead. |
| Fixture installer | `npm install --no-audit --no-fund --prefer-offline` against a two-package `package.json` checked into `tests/fixtures/node-two-pkg/` | `npm` ships with Node, `pnpm`/`yarn` don't — keeps the test runnable on a stock Node install without extra bootstrap. Production consumers can use whatever installer they want; the driver only cares about a populated `node_modules`. |
| ATTRIB-006 expander extension scope | One new fragment emitter + a new BEGIN/END marker pair in `licences.node-allow.txt`; expander still supports `--check` | Mechanical extension of the existing `render_fragment` helper. Keeps the canonical `licences.toml` as the single source of truth across all ecosystems. |

## Actions

### 1. Re-read the design contract + survey the live CLI surface

- **Purpose:** Confirm the spec's open questions resolve against the
  current `license-checker@25.0.1` shape before writing tests.
- **Produces:** No file changes — verifies that `--onlyAllow` is
  semicolon-separated (not comma per spec § Node line 257), and that
  `license-checker --json` output is the most stable input for a
  deterministic jq render pipeline.
- **Checkpoint:** Decisions table above matches the live CLI; no
  spec-doc edits required (open question gets closed in the PR body
  with the kickoff decision, not by editing the spec mid-flight).

### 2. Write failing Node-driver tests

- **Purpose:** Pin the new behaviour with red tests before implementing.
- **Produces:** Three new test files under
  `tools/starters/acknowledgements/tests/`:
  - `node-driver-render.sh` — two-package fixture under
    `tests/fixtures/node-two-pkg/` round-trips through the driver +
    dispatcher; second invocation is byte-identical (`--check` exit 0).
  - `node-driver-strict.sh` — fixture with a disallowed-licence
    dependency (e.g. a stub `package.json` declaring `"license":
    "GPL-3.0"`) trips `--onlyAllow` and exits non-zero with an
    actionable error referencing the offending package@version.
  - `node-driver-preflight.sh` — missing `node_modules` produces the
    actionable `pnpm install / npm install` hint and exits non-zero;
    missing `license-checker` produces the actionable
    `npm i -g license-checker` hint and exits non-zero.
- **Checkpoint:** All three tests fail against the current tree (no
  `drivers/node.sh` exists); all three pass after Action 4 lands.

### 3. Extend the ATTRIB-006 expander to emit a Node-shaped fragment

- **Purpose:** Single-source the Node allow-list off the canonical
  `licences.toml` so Rust and Node share the same SPDX truth.
- **Produces:**
  - `tools/starters/acknowledgements/expand-licences.sh` gains a
    third fragment emitter that writes a one-line `;`-joined SPDX
    list to `licences.node-allow.txt` between dedicated BEGIN/END
    markers (e.g. `# BEGIN AUTO-GENERATED licences.node-allow` /
    `# END AUTO-GENERATED licences.node-allow` — `#` comment-style to
    keep the file shell-cat-friendly).
  - `tools/starters/acknowledgements/licences.node-allow.txt`
    seeded with the marker pair around an initial expansion of the
    canonical allow-list.
  - `tools/starters/acknowledgements/tests/licences-drift.sh`
    extended with a fourth scenario covering the Node fragment
    (clean expand + consumer-side drift detection).
- **Checkpoint:** `expand-licences.sh --check` exit 0 against the
  seeded file; `licences-drift.sh` all four scenarios green;
  hand-editing `licences.node-allow.txt` between the markers is
  detected as drift.

### 4. Implement `drivers/node.sh`

- **Purpose:** Add the actual Node driver per the driver-author
  contract documented in `drivers/rust.sh:15-20`.
- **Produces:** `tools/starters/acknowledgements/drivers/node.sh`
  accepting `<block-config-json> <output-temp-path>`:
  - **Preflight**: `jq`, `license-checker` on PATH; `manifest_path`
    exists; `node_modules` exists under the manifest's directory.
  - **Strict-license**: `license-checker --onlyAllow "$(cat
    licences.node-allow.txt-without-marker-lines)" --start
    <manifest-dir>` ahead of render. Non-zero exit forwards the
    license-checker error verbatim plus the actionable hint
    `update licences.toml + re-run expand-licences.sh`.
  - **Render**: `license-checker --json --start <manifest-dir>
    [--production]` → jq pipeline producing one markdown row per
    package sorted by `name@version` ascending, with columns
    `Package | Version | License | Repository`. Output written to
    `<output-temp-path>`. Optional `exclude` block-config key applied
    as `--exclude '<comma-list>'`.
  - **No side effects on the splice target**: dispatcher already
    enforces this by passing a temp path; driver simply respects it.
- **Checkpoint:** All three Action-2 tests pass; existing tests
  (`dispatcher-schema-validation.sh`, `dispatcher-two-block.sh`,
  `strict-license-field.sh`, `licences-drift.sh`) still pass;
  `generate-acknowledgements.sh --check` against Anvil's real
  `attribution.toml` still exits 0 (no Node block declared there
  yet — that's ATTRIB-015).

### 5. Document the Node driver + monorepo pattern in the kit README

- **Purpose:** Consumers adopting the Node block need to know the
  block-config keys, the `licences.node-allow.txt` contract, and the
  pnpm-workspace pattern for multi-package monorepos.
- **Produces:** `tools/starters/acknowledgements/README.md` gains:
  - A "Node ecosystem driver" subsection under the existing
    driver-list, mirroring the Rust subsection's structure.
  - A worked example `[[blocks]]` entry with `ecosystem = "node"` and
    `manifest_path`.
  - A "Monorepo guidance" subsection covering the spec's "one block
    per shipping `package.json`" pattern with a worked pnpm-workspace
    example, plus the `manifest_path = "package.json"` workspace-wide
    escape hatch.
- **Checkpoint:** Cross-references resolve; README reads as drop-in
  for downstream subtree consumers; `oxfmt --check` clean on the
  edited README.

### 6. Verify the full kit suite + drift + format

- **Purpose:** Prove no regression before opening the PR.
- **Produces:** Green output from every kit test, drift-check clean,
  `pnpm run format` produces no untracked changes to the kit.
- **Checkpoint:** All checks green locally; ready for CI.
- **Validate:**
  `bash tools/starters/acknowledgements/tests/dispatcher-schema-validation.sh && bash tools/starters/acknowledgements/tests/dispatcher-two-block.sh && bash tools/starters/acknowledgements/tests/strict-license-field.sh && bash tools/starters/acknowledgements/tests/licences-drift.sh && bash tools/starters/acknowledgements/tests/node-driver-render.sh && bash tools/starters/acknowledgements/tests/node-driver-strict.sh && bash tools/starters/acknowledgements/tests/node-driver-preflight.sh && tools/starters/acknowledgements/generate-acknowledgements.sh --check && tools/starters/acknowledgements/expand-licences.sh --check && node scripts/aps/drift-check.mjs`

### 7. Wire the new tests into CI

- **Purpose:** Make the Node-driver tests part of the Acknowledgements
  freshness job so future kit changes can't silently break Node
  consumers.
- **Produces:** `.github/workflows/rust.yml` (or whichever workflow
  hosts the existing kit fixture tests — confirm during the action)
  gains invocations of the three new test scripts. `license-checker`
  installed in the job via `npm i -g license-checker` if not already
  pinned.
- **Checkpoint:** Workflow YAML lints clean; existing kit fixture
  tests still run; new tests added next to them.

### 8. Update the ATTRIB-012 APS entry

- **Purpose:** Reflect the work in flight per the APS rule "mark its
  status In Progress before starting".
- **Produces:** `plans/modules/attribution-pipeline-v3.aps.md`
  ATTRIB-012 `Status: Pending` → `Status: In Progress`; new
  `Execution plan:` line pointing at this file; `Last reviewed`
  bumped to 2026-05-24. `plans/index.aps.md` narrative gains an
  ATTRIB-012 "In Progress" callout (no Done-count change yet —
  that's the post-merge reconciliation).
- **Checkpoint:** `node scripts/aps/drift-check.mjs` clean.

### 9. Open the PR

- **Purpose:** Ship the Node driver; clear the gate for ATTRIB-015.
- **Produces:** PR with atomic commits (failing tests; expander
  extension + Node allow-list file; driver implementation; README
  docs; CI wire-up; APS In-Progress marker). Body cites the v3.2
  spec § Node, summarises the decisions table above (especially the
  `--onlyAllow` vs `--failOn` resolution), lists the full
  verification suite from Action 6, calls out that the mirror to
  `eddacraft/acknowledgements-starter` will pick up the new driver
  + expander extension + allow-list file on next push (back-compat
  shim means existing Rust-only consumers keep working without
  migration — no `[[blocks]]` entry of ecosystem = "node" means no
  Node behaviour fires).
- **Checkpoint:** PR opened against `main`; all CI green; mirror
  workflow either runs on merge or is queued.

## Post-merge follow-ups (not in scope for this PR)

- **ATTRIB-013 (Go driver)**: independent — does not depend on Node
  decisions beyond the dispatcher contract that ATTRIB-008 already
  pinned. Will reuse the same expander-extension pattern from this
  PR's Action 3.
- **ATTRIB-014 (Python driver)**: independent — same pattern as Go.
- **ATTRIB-015 (Anvil adopts a Node devtools block)**: depends on
  ATTRIB-012. Decides between a root-`package.json` `prod_only=false`
  block vs a curated `tools/dev/package.json` block based on
  attribution surface cleanliness (spec § Open questions).
- **Post-merge APS reconciliation**: ATTRIB-012 `In Progress` →
  `Merged via PR #N`; ATTRIB module Done-count bump in
  `plans/index.aps.md`. Separate small docs(aps) PR per the
  established reconciliation pattern (cf. PR #1893 for ATTRIB-008).
