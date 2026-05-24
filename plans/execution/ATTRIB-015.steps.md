# ATTRIB-015 — Anvil adopts a node-devtools attribution block

## Purpose

Land the first real production consumer of the Node ecosystem driver
that ATTRIB-012 shipped. Anvil itself doesn't publish JS to npm
(`project_npm_published_surface` memory), but it does depend on a
non-trivial Node devtools stack (Nx, husky, oxlint, oxfmt, vitest,
Playwright, TypeScript, markdownlint) to build itself. ATTRIB-015
attributes those devtools in Anvil's own `ACKNOWLEDGEMENTS.md` via a
second `[[blocks]]` entry alongside the existing Rust block.

Two parallel side-effects:

- Migrates Anvil's `attribution.toml` from the back-compat flat
  `[rust]` shim to the canonical `[[blocks]]` schema. The Rust block
  keeps the same scope; the marker pair in `ACKNOWLEDGEMENTS.md`
  gains per-block name suffixes.
- Exercises the ATTRIB-012 driver against a real (non-fixture)
  workspace for the first time, surfacing any rough edges before
  downstream consumers (ATTRIB-009 little-termi port, future
  ecosystem drivers) hit them.

Full design at
[`plans/specs/2026-05-22-acknowledgements-multi-block-and-multi-eco.md`](../specs/2026-05-22-acknowledgements-multi-block-and-multi-eco.md)
§ "Anvil adopts a Node devtools attribution block (ATTRIB-015)".

## Decisions (recorded on kickoff, 2026-05-24)

| Question | Decision | Rationale |
| --- | --- | --- |
| Scope: root `package.json` vs curated devtools manifest | **Curated minimal** at `tools/dev/package.json` declaring 8 core build/test/lint tools | Spec open question (line 358-363) resolved against a live `license-checker` survey. Root with `prod_only = false` → 2034 transitive packages including Nx Powerpack (proprietary `Custom: https://nx.dev/powerpack`), Copilot CLI (proprietary `Custom: https://docs.github.com/copilot/...`), UNKNOWN-licenced packages, and ~13 BlueOak / LGPL entries — would force major allow-list expansion AND ship a noisy attribution that misrepresents what Anvil actually ships. Curated minimal → 231 transitive packages, three new permissive licences to allow, no proprietary noise. **Key dev deps and tools beyond the curated minimum go in the hand-curated `## Thanks` section** of `ACKNOWLEDGEMENTS.md` per the kit's "hand curation is a feature" stance. |
| Curated tool shortlist | `nx`, `husky`, `oxlint`, `oxfmt`, `vitest`, `@playwright/test`, `typescript`, `markdownlint-cli` (8 tools, copied verbatim from root devDependencies version specs) | The eight tools that actually fire during build/test/lint — the ones a contributor needs available locally to make a PR pass CI. Larger devDep stack (changesets, eslint plugins, SWC, Nx subpackages, typescript-eslint family) are transitively pulled in OR are not directly invoked at build time; surface them via the hand-curated `## Thanks` section if attribution is wanted. |
| Manifest location | `tools/dev/package.json` (NOT root) | Spec line 332-334 explicit. Keeps the dev-tooling intent declared separately from the workspace root manifest (which is the pnpm-workspace manifest, not a "what we depend on at build time" surface). New installer step (`pnpm install --filter @eddacraft/anvil-devtools...`) covers this; pre-existing `pnpm install` at root continues to populate every workspace's deps. |
| Licences to add to `licences.toml` for ATTRIB-015 | `Python-2.0` (argparse@2.0.1), `BlueOak-1.0.0` (npm-owned minimatch@10.2.5), `0BSD` (Microsoft tslib@2.8.1) | All four packages flagged by the spike are mainstream, widely-used, OSI-approved permissive (BlueOak / 0BSD) or Python Software Foundation (Python-2.0). Adding to `about = true; deny = true` keeps the canonical allow-list authoritative across all consumers (Rust + Node + future Go/Python). Compound expression `(BSD-2-Clause OR MIT OR Apache-2.0)` on `run-con@1.3.2` should pass `license-checker --onlyAllow` without intervention (all three components allowed). |
| Workflow trigger | Existing `acknowledgements-kit.yml` already fires on `licences.toml` changes; existing `rust.yml::acknowledgements-diff` already fires on Cargo deps + workspace edits | No CI wiring change needed. The new `tools/dev/package.json` is excluded from CI's existing Node-test paths (it's not in pnpm-workspace.yaml unless we add it). |
| pnpm-workspace inclusion | `tools/dev` included in `pnpm-workspace.yaml` so `pnpm install` at root populates `tools/dev/node_modules` automatically | Otherwise contributors would have to `cd tools/dev && pnpm install` separately to make `generate-acknowledgements.sh --check` work locally. Inclusion has no runtime cost — it's just dep resolution. |

## Actions

### 1. Add the curated devtools manifest

- **Purpose:** Surface the curated dev-tooling list as a real
  workspace member so `pnpm install` populates `node_modules` and
  `license-checker` can walk the dependency graph.
- **Produces:**
  - `tools/dev/package.json` — `@eddacraft/anvil-devtools` private
    package declaring the 8 curated tools as `devDependencies`
    (version specs copied verbatim from root `package.json` to keep
    transitive resolution identical to what contributors already
    have locally).
  - `pnpm-workspace.yaml` — add `tools/dev` to the `packages:` list.
- **Checkpoint:** `pnpm install` from repo root succeeds and
  populates `tools/dev/node_modules`. `tools/dev/node_modules/.bin`
  is not used by the driver — `license-checker` resolves through the
  hoisted workspace `node_modules`.

### 2. Extend `licences.toml` for the three new permissive licences

- **Purpose:** Single-source the allow-list across every ecosystem.
- **Produces:** `licences.toml` gains three `[[licences]]` entries:
  - `Python-2.0` (about = true, deny = true, note explaining
    `argparse@2.0.1` is the canonical pull-in)
  - `BlueOak-1.0.0` (about = true, deny = true, note: npm's own
    relicensing of `minimatch`)
  - `0BSD` (about = true, deny = true, note: Microsoft tslib)
- **Checkpoint:** `expand-licences.sh` regenerates `about.toml`,
  `deny.toml`, and `licences.node-allow.txt` with the three new
  entries. `expand-licences.sh --check` exits 0 against the
  regenerated state.

### 3. Migrate `attribution.toml` from flat `[rust]` to `[[blocks]]`

- **Purpose:** Add the Node block alongside Rust. Dispatcher rejects
  mixed flat-`[rust]` + `[[blocks]]` in the same file, so the
  migration is required, not optional.
- **Produces:** `attribution.toml`:
  - Existing `[rust]` table replaced with a `[[blocks]]` entry
    (`name = "rust"`, `ecosystem = "rust"`, same `manifest_path` /
    `template_path` / `config_path`).
  - New `[[blocks]]` entry (`name = "node-devtools"`,
    `ecosystem = "node"`, `manifest_path = "tools/dev/package.json"`,
    `node_allow_path = "licences.node-allow.txt"`,
    `prod_only = false`).
- **Checkpoint:** TOML lints clean; dispatcher's schema validation
  (`tests/dispatcher-schema-validation.sh`) still passes.

### 4. Migrate `ACKNOWLEDGEMENTS.md` markers from unsuffixed to per-block

- **Purpose:** `[[blocks]]` schema requires per-block marker names
  (`<!-- BEGIN AUTO-GENERATED <name> -->`); the unsuffixed pair is
  only emitted by the flat-`[rust]` back-compat shim.
- **Produces:** `ACKNOWLEDGEMENTS.md`:
  - Existing `<!-- BEGIN AUTO-GENERATED -->` / `END` pair renamed to
    `<!-- BEGIN AUTO-GENERATED rust -->` / `END`.
  - New `<!-- BEGIN AUTO-GENERATED node-devtools -->` / `END` pair
    added in a dedicated `### Node devtools` (or similar) section.
  - Hand-curated `## Thanks` content (key dev deps + tools NOT in the
    curated minimal — eslint, SWC, vitest's sibling tools, etc.)
    preserved / extended outside the markers per the kit's "hand
    curation is a feature" stance.
- **Checkpoint:** Both marker pairs present, exactly once each. No
  hand-curated content lost. Markdown lint passes.

### 5. Regenerate and verify

- **Purpose:** Populate both blocks; prove --check is clean.
- **Produces:** `ACKNOWLEDGEMENTS.md` updated with rendered content
  in both auto-generated blocks. The Rust block content is unchanged
  (same cargo-about output as the flat shim path produced — the kit
  preserves byte-identicality across the shim → `[[blocks]]`
  migration). The Node block carries the 231 attributed packages
  sorted by name@version.
- **Checkpoint:**
  `tools/starters/acknowledgements/generate-acknowledgements.sh --check`
  exits 0; second invocation byte-identical; existing kit self-tests
  still pass.
- **Validate:**
  `tools/starters/acknowledgements/generate-acknowledgements.sh --check && tools/starters/acknowledgements/expand-licences.sh --check && bash tools/starters/acknowledgements/tests/dispatcher-schema-validation.sh && bash tools/starters/acknowledgements/tests/dispatcher-two-block.sh && bash tools/starters/acknowledgements/tests/strict-license-field.sh && bash tools/starters/acknowledgements/tests/licences-drift.sh && bash tools/starters/acknowledgements/tests/node-driver-preflight.sh && bash tools/starters/acknowledgements/tests/node-driver-render.sh && bash tools/starters/acknowledgements/tests/node-driver-strict.sh && node scripts/aps/drift-check.mjs`

### 6. Update the ATTRIB-015 APS entry

- **Purpose:** Reflect the work in flight per the APS rule "mark its
  status In Progress before starting".
- **Produces:**
  `plans/modules/attribution-pipeline-v3.aps.md` ATTRIB-015
  `Status: Pending` → `Status: In Progress`; new `Execution plan:`
  line pointing at this file.
- **Checkpoint:** `node scripts/aps/drift-check.mjs` clean.

### 7. Open the PR

- **Purpose:** Ship the first production Node-driver consumer; close
  the ATTRIB-015 work item.
- **Produces:** PR with atomic commits (curated manifest + workspace
  inclusion; licences.toml expansion + regenerated consumer files;
  attribution.toml migration + ACKNOWLEDGEMENTS.md marker rename;
  regenerated ACKNOWLEDGEMENTS.md content). Body cites the spec §
  ATTRIB-015, summarises the kickoff decisions (especially the
  curated-vs-root and licences additions), lists the full
  verification suite from Action 5.
- **Checkpoint:** PR opened against `main`; all CI green; mirror
  workflow either runs on merge or is queued.

## Post-merge follow-ups (not in scope for this PR)

- **Post-merge APS reconciliation**: ATTRIB-015 `In Progress` →
  `Merged via PR #N`; ATTRIB module Done-count bump in
  `plans/index.aps.md` (9/15 → 10/15). Separate small docs(aps) PR
  per the established reconciliation pattern (cf. #1907 for
  ATTRIB-012).
- **ATTRIB-013 (Go driver) + ATTRIB-014 (Python driver)**: same
  recipe as ATTRIB-012; this PR proves the consumer pattern at the
  same time as the driver pattern.
- **ATTRIB-009 (little-termi port)**: this PR is implicit
  byte-identicality re-verification for Anvil's Rust block under the
  shim → `[[blocks]]` migration. Cross-repo little-termi port is the
  external acceptance test.
