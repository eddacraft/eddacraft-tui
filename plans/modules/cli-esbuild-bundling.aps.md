<!--
APS Module: CLI esbuild Bundling
=================================
Bundles all workspace:* dependencies into the CLI dist using esbuild
so the published npm package is self-contained and installable.
-->

# CLI esbuild Bundling

| ID     | Owner | Status  |
| ------ | ----- | ------- |
| BUNDLE | —     | Complete |

## Branch

`feat/cli-esbuild-bundle` (off `main`)

## Purpose

The CLI declares 5 `workspace:*` dependencies (`anvil-core`, `anvil-runtime`,
`anvil-aps`, `anvil-adapters`, `anvil-kindling-integration`) that don't exist on
npm. The current `tsc` build emits separate JS files with unresolved
`import ... from '@eddacraft/anvil-core'` statements at runtime. Anyone
installing `@eddacraft/anvil-cli` from npm gets a broken install.

**This is a hard release blocker for v0.1.0.**

The VS Code extension already uses esbuild in this repo — adapt the same pattern
for the CLI.

## In Scope

### BUNDLE-001: Create esbuild config (P0)

- **File:** `apps/anvil-cli/esbuild.config.mjs` (new)
- **What:** Build script that bundles `src/index.ts` into a single
  `dist/index.js`
- **Config:**
  - `format: 'esm'` — matches `"type": "module"`
  - `platform: 'node'`, `target: 'node20'`
  - `banner: { js: '#!/usr/bin/env node' }` — restores shebang
  - `jsx: 'automatic'` — matches tsconfig `"jsx": "react-jsx"` for Ink
  - `sourcemap: true`
  - `define.__CLI_VERSION__` — injects version from package.json at build time
- **External packages** (native deps, can't be bundled):
  - `@eddacraft/kindling-core`
  - `@eddacraft/kindling-store-sqlite`
  - `@eddacraft/kindling-provider-local`
- **Everything else gets bundled** into the single output file: all 5 workspace
  packages, all pure-JS npm deps (chalk, commander, ink, react, zod, etc.)
- **Confidence:** high
- **Priority:** P0

### BUNDLE-002: Update package.json (P0)

- **File:** `apps/anvil-cli/package.json`
- **Changes:**
  - `scripts.build`: `tsc` → `node esbuild.config.mjs`
  - `scripts.build:types`: add `tsc --emitDeclarationOnly` (optional, for
    downstream consumers)
  - Remove all 5 `workspace:*` entries from `dependencies` (now bundled)
  - Remove all pure-JS npm deps from `dependencies` (now bundled): chalk,
    commander, ink, ink-select-input, ink-spinner, ink-text-input, inquirer, ora,
    react, yaml, zod, glob, fs-extra, chokidar, beautiful-mermaid
  - Keep only external runtime deps:
    ```
    "@eddacraft/kindling-core": "0.1.1",
    "@eddacraft/kindling-provider-local": "0.1.1",
    "@eddacraft/kindling-store-sqlite": "0.1.1"
    ```
  - Add `esbuild` to `devDependencies`
  - Remove `"main"` and `"types"` fields (CLI binary, not a library)
- **Confidence:** high
- **Priority:** P0

### BUNDLE-003: Inject CLI version at build time (P0)

- **File:** `apps/anvil-cli/src/index.ts` (lines 38-43)
- **What:** Replace runtime `readJsonFileSync(package.json)` with build-time
  constant `__CLI_VERSION__` injected by esbuild `define`
- **Before:**
  ```ts
  const __dirname = dirname(fileURLToPath(import.meta.url));
  const packageJson =
    readJsonFileSync<Record<string, string>>(join(__dirname, '..', 'package.json')) ?? {};
  const CLI_VERSION = packageJson.version || '0.0.0-unknown';
  ```
- **After:**
  ```ts
  declare const __CLI_VERSION__: string;
  const CLI_VERSION = typeof __CLI_VERSION__ !== 'undefined'
    ? __CLI_VERSION__
    : '0.0.0-dev';
  ```
- Remove unused imports (`join`, `dirname`, `fileURLToPath`, `readJsonFileSync`)
  if no longer referenced elsewhere in the file
- **Confidence:** high
- **Priority:** P0

### BUNDLE-004: No changes needed (informational)

These files use `import.meta.url` for file-path resolution but already have
inline fallbacks that activate when the file-based paths fail:

- `services/hook-installer.ts` — `getScriptsPath()` fails gracefully →
  `getEmbeddedScript()` provides inline shell scripts (lines 95-181)
- `commands/policy.ts` — `getExamplePoliciesPath()` returns `''` on failure →
  `EXAMPLE_POLICIES` inline fallback (lines 75+)

In the bundle, file-path resolution will fail (flattened directory structure) and
the embedded fallbacks activate automatically. Working as designed.

## Out of Scope

- Publishing the 5 internal packages to npm (no longer needed — they're bundled)
- Minification (not needed for a CLI; add later if desired)
- Tree-shaking optimisation (esbuild handles this automatically)
- Changing module format to CJS (stay ESM)

## Verification

| Step | Command | Expected |
| ---- | ------- | -------- |
| 1 | `cd apps/anvil-cli && node esbuild.config.mjs` | Produces `dist/index.js` with shebang on line 1 |
| 2 | `node dist/index.js --version` | Prints `0.1.0` |
| 3 | `node dist/index.js --help` | Lists all commands |
| 4 | `node dist/index.js doctor` | Runs diagnostics without crash |
| 5 | `npm pack --dry-run` | Only `dist/`, `README.md`, `package.json`; no `workspace:*` in deps |
| 6 | `pnpm vitest run` (from CLI package) | Unit tests pass (import source, not bundle) |
| 7 | `pnpm run typecheck` | tsc --noEmit passes |

## Dependencies

None — this module is self-contained.

## Risks

| Risk | Mitigation |
| ---- | ---------- |
| Some npm package doesn't bundle cleanly | esbuild warns on unresolvable imports; fix or mark external |
| Dynamic `import()` calls break | esbuild preserves dynamic imports in ESM; they resolve from node_modules at runtime |
| Bundle size too large | Acceptable for a CLI tool; not a library. Can add minify later |
| Kindling packages not published on npm | They're pinned to 0.1.1, not `workspace:*` — separate concern |
