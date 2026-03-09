# Node.js to Deno 2.6 Migration Assessment

## EddaCraft Anvil Monorepo — Full End-to-End Impact Analysis

**Date:** 2026-02-03 **Scope:** Complete assessment of migrating the Anvil
monorepo from Node.js (>=20.0.0) to Deno 2.6 **Current Stack:** Node.js 24.x |
pnpm 10.26.0 | Nx 22.4.3 | TypeScript 5.9.3

---

## Table of Contents

1. [Executive Summary](#1-executive-summary)
2. [Current Architecture Overview](#2-current-architecture-overview)
3. [Deno 2.6 Capabilities Assessment](#3-deno-26-capabilities-assessment)
4. [Module System Compatibility](#4-module-system-compatibility)
5. [Package Manager & Workspace Impact](#5-package-manager--workspace-impact)
6. [Build System & Nx Audit](#6-build-system--nx-audit)
7. [Hybrid Runtime Strategy](#7-hybrid-runtime-strategy)
8. [Framework Compatibility Matrix](#8-framework-compatibility-matrix)
9. [Testing Infrastructure Impact](#9-testing-infrastructure-impact)
10. [CI/CD Pipeline Impact](#10-cicd-pipeline-impact)
11. [Node.js API Usage & Migration Paths](#11-nodejs-api-usage--migration-paths)
12. [Native Addon & FFI Considerations](#12-native-addon--ffi-considerations)
13. [Developer Tooling Impact](#13-developer-tooling-impact)
14. [Security Model Comparison](#14-security-model-comparison)
15. [Performance Considerations](#15-performance-considerations)
16. [Risk Register](#16-risk-register)
17. [Migration Strategy Recommendation](#17-migration-strategy-recommendation)
18. [Decision Matrix](#18-decision-matrix)
19. [References](#19-references)

---

## 1. Executive Summary

### Verdict: **FEASIBLE with hybrid approach — proceed with phased migration**

Migrating the Anvil monorepo from Node.js to Deno 2.6 is more viable than it
initially appears. A detailed audit reveals that:

- **Nx is lightweight in this project** — it provides topological build ordering
  via `nx run-many -t build` but no packages use Nx executors, no computation
  caching is active, and CI does not invoke Nx directly. Deno workspaces can
  replace this.
- **Deno supports hybrid workspaces** — members can be `package.json`-only and
  continue running on Node.js. Apps like `docs-site` (Docusaurus) and `website`
  (Next.js/Vercel) stay on Node.js with zero migration cost.
- **The codebase is already Deno-ready** — 17/22 packages use ES Modules, all
  Node.js imports use `node:` protocol, and `import.meta` patterns are standard.

| Factor                      | Assessment                                                 | Severity    |
| --------------------------- | ---------------------------------------------------------- | ----------- |
| Nx monorepo orchestration   | Lightweight usage; Deno workspaces replace it              | **LOW**     |
| pnpm `workspace:*` protocol | Deno supports workspace protocol in `package.json` members | **LOW**     |
| Vitest 4.x compatibility    | Known panics with Deno 2.5-2.6 (issue #31354)              | **BLOCKER** |
| Docusaurus 3.9              | Stays on Node.js as hybrid workspace member                | **NONE**    |
| Next.js 16 / Vercel         | Stays on Node.js as hybrid workspace member                | **NONE**    |
| node-pty (TUI testing)      | Requires FFI validation; not yet confirmed                 | **HIGH**    |
| VS Code extension           | Stays on Node.js (extension host is Node.js)               | **NONE**    |
| Husky/lint-staged           | Needs reconfiguration                                      | **MEDIUM**  |

**Primary blocker:** Vitest 4.0.18 panics on Deno 2.5-2.6. This must be resolved
(upstream fix or Vitest pin) before migration can proceed.

**Recommended approach:** Phased migration — migrate core packages and CLI to
Deno while keeping Node.js-bound apps (website, docs-site, VS Code extension) as
hybrid `package.json` workspace members running on Node.js.

---

## 2. Current Architecture Overview

### 2.1 Repository Structure

```
anvil-001/                          # pnpm + Nx monorepo
├── apps/
│   ├── anvil-cli/                  # Commander.js + Ink TUI (ES Module)
│   ├── website/                    # Next.js 16.0.10 + React 19.2
│   ├── docs-site/                  # Docusaurus 3.9.2
│   ├── e2e/                        # Playwright 1.58.0
│   ├── anvil-api/                  # Placeholder
│   └── anvil-ui/                   # Placeholder
├── packages/
│   ├── anvil/{contracts,core,ports,runtime,policy}
│   ├── platform/{config,crypto,storage}
│   ├── adapters/
│   ├── aps/
│   ├── edda-stack/
│   ├── kindling-integration/
│   ├── eslint-plugin-anvil/
│   ├── vscode-extension/           # esbuild bundled
│   └── tooling/{eslint-config,tsconfig}
└── tools/{generators,codemods}
```

### 2.2 Key Metrics

| Metric                                      | Value                                            |
| ------------------------------------------- | ------------------------------------------------ |
| Total npm packages                          | 22                                               |
| TypeScript config files                     | 35                                               |
| ES Module packages (`"type": "module"`)     | 17                                               |
| Node.js built-in imports (`node:` protocol) | ~397 occurrences                                 |
| `process.*` API calls                       | ~209 occurrences                                 |
| Native addon dependencies                   | 2 (node-pty, keytar)                             |
| CI matrix targets                           | Node 20.x, 22.x                                  |
| Test frameworks                             | Vitest 4.0.18, Playwright 1.58.0, tuistory 0.0.9 |

### 2.3 Migration-Favorable Characteristics

The codebase is already well-positioned for Deno:

- **ES Modules throughout** — 17/22 packages use `"type": "module"`; zero
  CommonJS `require()` calls in source code
- **`node:` protocol imports enforced** — ESLint rule
  `unicorn/prefer-node-protocol: error` ensures all Node.js built-in imports use
  the `node:` prefix, which Deno natively supports
- **Modern `import.meta` patterns** — Uses `import.meta.url`,
  `import.meta.dirname` instead of `__dirname`/`__filename`
- **No `.cjs` files** — Complete ES module adoption
- **TypeScript-first** — Deno's native TypeScript support aligns directly
- **Clean Node.js API surface** — All `node:` APIs used are fully supported by
  Deno 2.6's compatibility layer (see Section 11)

---

## 3. Deno 2.6 Capabilities Assessment

### 3.1 Key Deno 2.6 Features Relevant to This Migration

| Feature                       | Description                                           | Relevance                                              |
| ----------------------------- | ----------------------------------------------------- | ------------------------------------------------------ |
| `dx` command                  | npx equivalent for running npm/JSR binaries           | Replaces `pnpm dlx` / `npx` usage in scripts           |
| `@types/node` built-in        | Node.js type declarations included by default         | Eliminates `@types/node` devDependency                 |
| `--require` flag              | CommonJS module preloading                            | Useful for SWC register compatibility                  |
| `deno audit`                  | Dependency vulnerability scanning                     | Replaces npm audit / pnpm audit                        |
| `tsgo` integration            | Experimental fast TypeScript type checking (Go-based) | Could accelerate typecheck pipeline                    |
| Granular permissions          | `--ignore-read`, `--ignore-env`                       | More control than Node.js; useful for CLI distribution |
| `allowScripts` in `deno.json` | Lifecycle script approval for native addons           | Required for node-pty and esbuild                      |
| JUnit reports                 | Clean XML output without ANSI codes                   | CI/CD compatible test reporting                        |
| Hybrid workspaces             | `package.json` members alongside `deno.json` members  | Enables gradual migration                              |

### 3.2 Node.js Compatibility Layer Status

Deno 2.6 supports:

- `node:fs`, `node:path`, `node:crypto`, `node:child_process`, `node:os`,
  `node:url`, `node:util`, `node:events`, `node:stream`, `node:zlib` — **all
  used by Anvil**
- `process` global (including `process.env`, `process.cwd()`, `process.argv`,
  `process.exit()`)
- `Buffer` global
- `package.json` detection and `node_modules` resolution
- npm package imports via `npm:` specifiers
- Subpath imports (`#/` prefix)
- Workspace protocol specifiers in `package.json` files

**Known issues:**

- `node:worker_threads` — partial (affects Vitest)
- `node:vm` — partial (affects Vitest)
- Vitest 4.0.10+ panics on Deno 2.5-2.6 (issue #31354)

---

## 4. Module System Compatibility

### 4.1 Current State: Strong ESM Foundation

| Aspect                   | Status                                                  | Migration Impact                          |
| ------------------------ | ------------------------------------------------------- | ----------------------------------------- |
| ES Module packages       | 17/22 use `"type": "module"`                            | **NONE** — Already compatible             |
| CommonJS packages        | 5 (website, docs-site, eslint-plugin, vscode-ext, root) | **NONE** — Stay as hybrid members         |
| `.mjs` config files      | 9 files (eslint, next, postcss)                         | **NONE** — Already compatible             |
| `node:` protocol imports | ~397 occurrences                                        | **NONE** — Deno supports `node:` natively |
| `import.meta.url`        | Used throughout                                         | **NONE** — Deno supports `import.meta`    |
| Dynamic imports          | Used in several packages                                | **LOW** — Generally compatible            |

### 4.2 Packages Requiring No Module Changes

The 5 packages without `"type": "module"` are all candidates for hybrid
membership (stay on Node.js):

1. **`apps/docs-site/`** — Stays on Node.js (Docusaurus)
2. **`apps/website/`** — Stays on Node.js (Next.js / Vercel)
3. **`packages/eslint-plugin-anvil/`** — ESLint plugin, runs via npm compat
4. **`packages/vscode-extension/`** — Must remain CJS (VS Code extension host)
5. **Root `package.json`** — Becomes hybrid workspace root

### 4.3 Import Map Requirements

Deno workspace members that use `deno.json` would need:

- Path aliases from `tsconfig.base.json` replicated in `deno.json` `imports`
- Bare specifier imports from npm packages (auto-resolved with
  `nodeModulesDir: auto`)

Cross-member imports resolve automatically by `name` field — no import map
entries needed for workspace packages.

---

## 5. Package Manager & Workspace Impact

### 5.1 Current Setup: pnpm 10.26.0

```yaml
# pnpm-workspace.yaml
packages:
  - 'apps/*'
  - 'packages/**'
  - 'tools/*'
```

```ini
# .npmrc
node-linker=hoisted
shamefully-hoist=true
link-workspace-packages=true
onlyBuiltDependencies=["node-pty"]
```

### 5.2 Deno Workspace Replacement

Deno workspaces (since 1.45) support hybrid monorepos where members can have
`deno.json`, `package.json`, or both:

```jsonc
// deno.json (root)
{
  "workspace": [
    // Deno-native members (migrated)
    "packages/anvil/contracts",
    "packages/anvil/core",
    "packages/anvil/ports",
    "packages/anvil/runtime",
    "packages/anvil/policy",
    "packages/platform/config",
    "packages/platform/crypto",
    "packages/platform/storage",
    "packages/adapters",
    "packages/aps",
    "packages/edda-stack",
    "packages/kindling-integration",
    "apps/anvil-cli",

    // Node.js hybrid members (package.json only — unchanged)
    "apps/website",
    "apps/docs-site",
    "apps/e2e",
    "packages/vscode-extension",
    "packages/eslint-plugin-anvil",
    "packages/tooling/eslint-config",
    "packages/tooling/tsconfig",
  ],
  "nodeModulesDir": "auto",
}
```

### 5.3 Feature Comparison

| Feature                    | pnpm                      | Deno Workspaces                             | Gap                   |
| -------------------------- | ------------------------- | ------------------------------------------- | --------------------- |
| `workspace:*` protocol     | Native                    | Supported in `package.json` members         | **NONE**              |
| Hybrid members             | N/A                       | `package.json`-only members stay on Node.js | **Advantage**         |
| Cross-member imports       | Via `workspace:*`         | Auto-resolved by `name` field               | **NONE**              |
| Hoisted `node_modules`     | `shamefully-hoist=true`   | `"nodeModulesDir": "auto"`                  | Behavioral difference |
| Lockfile                   | `pnpm-lock.yaml`          | `deno.lock`                                 | Full regeneration     |
| Lifecycle scripts          | Runs by default           | Requires `allowScripts` approval            | Security improvement  |
| Peer dependency resolution | `auto-install-peers=true` | Different algorithm                         | Low risk              |
| Shared root dependencies   | Via root `package.json`   | Via root `deno.json` `imports`              | **NONE**              |

### 5.4 Migration Path

1. Create root `deno.json` with `workspace` array listing all members
2. Set `"nodeModulesDir": "auto"` in root `deno.json`
3. Add `deno.json` to each migrating package with `name` and `version`
4. Leave Node.js-bound apps with `package.json` only (hybrid members)
5. Configure `allowScripts` for native addons (node-pty, esbuild)
6. Generate `deno.lock` from scratch
7. Update CI to use `deno install`

---

## 6. Build System & Nx Audit

### 6.1 Actual Nx Usage (Audit Findings)

A detailed audit of every `project.json`, `package.json` script, and CI workflow
reveals Nx's footprint is **lightweight**:

**What Nx IS doing:**

- `nx run-many -t build` — topological build ordering via
  `dependsOn: ["^build"]`
- Auto-inferring test targets from `vitest.config.ts` via `@nx/vite/plugin`
- Auto-inferring lint targets from ESLint configs via `@nx/eslint/plugin`
- Auto-inferring build targets from `tsconfig.lib.json` via `@nx/js/typescript`
- Two custom generators for scaffolding new packages

**What Nx is NOT doing:**

| Capability                 | Status         | Evidence                                                                      |
| -------------------------- | -------------- | ----------------------------------------------------------------------------- |
| `@nx/js:tsc` executor      | **Not used**   | Zero packages reference this executor; all use `tsc` directly via npm scripts |
| Computation caching        | **Not active** | Configured for `@nx/js:tsc` which nothing uses; no `.nx/cache`                |
| `nx affected` in CI        | **Not used**   | CI runs `pnpm run build`, `pnpm run lint:check`, `pnpm run typecheck`         |
| Direct Nx invocation in CI | **Not used**   | All CI steps go through `pnpm run` scripts                                    |
| Complex target configs     | **Not used**   | All but 2 `project.json` files have `targets: {}` (empty)                     |

**Every package builds the same way:**

```json
// Typical package.json scripts (all packages)
{
  "build": "tsc -p tsconfig.lib.json",
  "test": "vitest run",
  "typecheck": "tsc --noEmit"
}
```

### 6.2 Replacing Nx with Deno

| Nx Feature (as used)                 | Deno Replacement                                                                         | Effort   |
| ------------------------------------ | ---------------------------------------------------------------------------------------- | -------- |
| `nx run-many -t build` (topological) | `deno task` per member + build script or [Monodeno](https://jsr.io/@jurassicjs/monodeno) | **LOW**  |
| `workspace:*` resolution             | Deno workspace auto-resolution by `name`                                                 | **NONE** |
| Auto-inferred targets                | Each member defines `deno task` entries                                                  | **LOW**  |
| Custom generators                    | Template files + script (or keep as standalone Node.js tool)                             | **LOW**  |
| Verdaccio local registry             | Not needed if publishing to JSR; keep as Node.js tool if needed                          | **NONE** |

### 6.3 The Build Simplification Opportunity

Deno runs TypeScript natively. For packages that are only consumed within the
monorepo (not published to npm), the `tsc -p tsconfig.lib.json` build step can
be **eliminated entirely**. Deno imports `.ts` files directly.

Packages that still need a build step:

- `apps/anvil-cli` — published to npm, needs `.js` + `.d.ts` output
- `apps/website` — Next.js build (stays on Node.js)
- `apps/docs-site` — Docusaurus build (stays on Node.js)
- `packages/vscode-extension` — esbuild bundle (stays on Node.js)

Internal-only packages (`anvil-contracts`, `anvil-core`, `anvil-ports`, etc.)
could skip compilation entirely under Deno, reducing build time significantly.

---

## 7. Hybrid Runtime Strategy

### 7.1 Core Principle

Deno 2 is designed to run alongside Node.js. Workspace members with only a
`package.json` continue using Node.js tooling. Members with a `deno.json` (or
both) use Deno. This is not a workaround — it is Deno's intended architecture
for gradual adoption.

### 7.2 Proposed Runtime Assignment

| Component                       | Runtime     | Rationale                                         |
| ------------------------------- | ----------- | ------------------------------------------------- |
| `packages/anvil/*` (5 pkgs)     | **Deno**    | Pure logic, ES Modules, `node:` imports only      |
| `packages/platform/*` (3 pkgs)  | **Deno**    | Pure logic, ES Modules, `node:` imports only      |
| `packages/adapters`             | **Deno**    | ES Module, minimal deps                           |
| `packages/aps`                  | **Deno**    | ES Module, remark/unified ecosystem               |
| `packages/edda-stack`           | **Deno**    | ES Module, minimal deps                           |
| `packages/kindling-integration` | **Deno**    | ES Module, zero Node.js APIs                      |
| `apps/anvil-cli`                | **Deno**    | ES Module, Commander.js + Ink work via npm compat |
| `apps/website`                  | **Node.js** | Next.js 16, Vercel deployment                     |
| `apps/docs-site`                | **Node.js** | Docusaurus 3.9, CJS-dependent                     |
| `apps/e2e`                      | **Node.js** | Playwright, test isolation                        |
| `packages/vscode-extension`     | **Node.js** | VS Code extension host is Node.js                 |
| `packages/eslint-plugin-anvil`  | **Node.js** | ESLint ecosystem                                  |
| `packages/tooling/*`            | **Node.js** | ESLint/TS config sharing                          |
| `tools/generators`              | **Node.js** | Nx generators (can be kept as standalone tool)    |
| `tools/codemods`                | **Deno**    | ts-morph works via npm compat                     |

**Result:** 13 packages migrate to Deno, 9 stay on Node.js as hybrid members.

### 7.3 Cross-Runtime Package Consumption

Node.js hybrid members (like `apps/website`) that depend on Deno-native packages
(like `@eddacraft/anvil-core`) can import them because:

1. Deno workspace members are resolvable by `name` field
2. With `nodeModulesDir: auto`, packages are available in `node_modules`
3. Node.js members continue using their existing resolution

The reverse also works: Deno members can import `package.json`-only members via
bare specifiers.

---

## 8. Framework Compatibility Matrix

### 8.1 Detailed Framework Assessment

| Framework        | Version | Strategy                        | Severity    | Notes                                                    |
| ---------------- | ------- | ------------------------------- | ----------- | -------------------------------------------------------- |
| **Next.js**      | 16.0.10 | Stay on Node.js (hybrid member) | **NONE**    | Vercel deployment is Node.js; no migration needed        |
| **React**        | 19.2.0  | Compatible                      | **NONE**    | Works via npm compatibility layer                        |
| **Docusaurus**   | 3.9.2   | Stay on Node.js (hybrid member) | **NONE**    | No migration needed                                      |
| **Ink**          | 6.6.0   | Migrate with CLI                | **LOW**     | Works via `npm:` specifiers; Deno 2 has `process` global |
| **Commander.js** | 14.0.2  | Migrate with CLI                | **NONE**    | Pure JS/ESM package                                      |
| **Zod**          | 4.3.6   | Compatible                      | **NONE**    | Pure JS/ESM, no Node.js APIs                             |
| **Tailwind CSS** | 4.1.9   | Stay on Node.js (website)       | **NONE**    | Part of Next.js build                                    |
| **Radix UI**     | Various | Stay on Node.js (website)       | **NONE**    | Part of Next.js build                                    |
| **Vite**         | 7.3.1   | See Vitest section              | **BLOCKER** | Vitest 4.x panic issue                                   |
| **ESLint**       | 9.39.2  | Run via npm compat              | **LOW**     | Or adopt `deno lint` over time                           |
| **Prettier**     | 3.8.1   | Replace with `deno fmt`         | **LOW**     | Or keep via npm compat                                   |

### 8.2 Key Insight: Hybrid Eliminates Most Framework Concerns

By keeping Next.js and Docusaurus as Node.js hybrid members, their entire
dependency trees (Turbopack, PostCSS, Tailwind, Radix UI, MDX, etc.) are
unaffected. The migration scope is limited to the packages that actually move to
Deno.

---

## 9. Testing Infrastructure Impact

### 9.1 Vitest 4.0.18 — PRIMARY BLOCKER

| Aspect           | Current                     | With Deno 2.6               | Risk        |
| ---------------- | --------------------------- | --------------------------- | ----------- |
| Test runner      | Vitest 4.0.18 via Nx plugin | Known panics (issue #31354) | **BLOCKER** |
| Coverage         | `@vitest/coverage-v8`       | v8 coverage may not work    | **HIGH**    |
| Environment      | `happy-dom`                 | Requires `node:vm` compat   | **MEDIUM**  |
| Snapshot testing | Works                       | Likely works                | **LOW**     |

**This is the single hard blocker.** Deno 2.5.6+ panics with Vitest 4.0.10+. The
project uses Vitest 4.0.18 across ~100+ test files.

**Resolution paths:**

1. **Wait for upstream fix** — Both Deno and Vitest teams are aware (issues
   [#31354](https://github.com/denoland/deno/issues/31354),
   [#23882](https://github.com/denoland/deno/issues/23882))
2. **Pin Vitest < 4.0.10** — May work but loses 4.x features
3. **Use Deno's built-in test runner** — Requires rewriting tests; loses
   `vi.mock()`, `test.each`, `happy-dom` environment
4. **Keep tests on Node.js** — Run `vitest` via Node.js even for Deno packages
   (pragmatic short-term approach)

**Recommended:** Option 4 short-term (tests run on Node.js while source runs on
Deno), then transition to option 1 when the upstream fix lands.

### 9.2 Playwright 1.58.0

| Aspect             | Current                        | With Deno 2.6                        | Risk    |
| ------------------ | ------------------------------ | ------------------------------------ | ------- |
| Browser automation | Works natively                 | Requires `nodeModulesDir: auto`      | **LOW** |
| Config file        | `playwright.config.ts`         | May need `.mts` extension            | **LOW** |
| Browser install    | `pnpm exec playwright install` | `deno run -A npm:playwright install` | **LOW** |

Playwright works on Deno 2.6 with `nodeModulesDir: auto`. However, since
`apps/e2e` stays as a Node.js hybrid member, **no changes are needed** — it
continues running via `pnpm exec playwright test`.

### 9.3 TUI Testing (tuistory + node-pty)

| Aspect   | Current                      | With Deno 2.6                 | Risk     |
| -------- | ---------------------------- | ----------------------------- | -------- |
| node-pty | Native addon, compiled in CI | Requires FFI + `allowScripts` | **HIGH** |
| tuistory | npm package                  | Depends on node-pty           | **HIGH** |

The TUI E2E tests depend on `node-pty`, a native C++ addon. Deno 2.3+ supports
Node-API addons, but this has not been specifically validated for node-pty.

**Mitigation:** TUI tests can remain a Node.js-invoked step in CI even if the
CLI itself runs on Deno. The test harness spawns the CLI as a subprocess, so the
CLI's runtime is independent of the test runner's runtime.

### 9.4 Test Strategy Summary

| Test Category       | Files | Strategy                                           | Effort                |
| ------------------- | ----- | -------------------------------------------------- | --------------------- |
| Vitest unit tests   | ~100+ | Keep on Node.js short-term; migrate when fix lands | **NONE** (short-term) |
| Playwright E2E      | ~10   | Stay on Node.js (hybrid member)                    | **NONE**              |
| TUI E2E (tuistory)  | ~5    | Stay on Node.js (test harness)                     | **NONE**              |
| ink-testing-library | ~5    | Validate npm compat                                | **LOW**               |

---

## 10. CI/CD Pipeline Impact

### 10.1 Current Pipeline (`.github/workflows/ci.yml`)

```yaml
# Current: Node.js based
- uses: pnpm/action-setup@v4
- uses: actions/setup-node@v4
  with:
    node-version: [20.x, 22.x]
    cache: 'pnpm'
- run: pnpm install --frozen-lockfile
- run: pnpm run lint:check
- run: pnpm run typecheck
- run: pnpm run test -- --run --coverage
- run: pnpm run build
```

### 10.2 Proposed Hybrid CI Pipeline

```yaml
# Proposed: Hybrid Deno + Node.js
- uses: denoland/setup-deno@v2
  with:
    deno-version: v2.6.x
- uses: actions/setup-node@v4 # Still needed for hybrid members
  with:
    node-version: 22.x
- run: deno install # Resolves all workspace dependencies
- run: deno task lint # deno lint + ESLint for Node.js members
- run: deno task typecheck # deno check for Deno members + tsc for hybrid
- run: deno task test # Vitest via Node.js (short-term)
- run: deno task build # Only published packages need builds
```

### 10.3 Impact Analysis

| CI Feature      | Current                          | With Deno Hybrid                        | Impact                                   |
| --------------- | -------------------------------- | --------------------------------------- | ---------------------------------------- |
| Runtime setup   | `setup-node` only                | `setup-deno` + `setup-node`             | **LOW** — One extra step                 |
| Package install | `pnpm install --frozen-lockfile` | `deno install`                          | **LOW** — Deno 90% faster with hot cache |
| Matrix testing  | Node 20.x, 22.x                  | Single Deno version + Node for hybrid   | **LOW** — Simplification                 |
| Build command   | `pnpm run build` (via Nx)        | `deno task build` (only published pkgs) | **LOW** — Fewer packages to build        |
| Lint command    | `pnpm run lint:check`            | `deno lint` + ESLint via npm compat     | **MEDIUM**                               |
| Type check      | `pnpm run typecheck` (via tsc)   | `deno check` or `tsgo` (experimental)   | **MEDIUM**                               |
| Test command    | `pnpm run test` (Vitest)         | Vitest via Node.js (short-term)         | **NONE** — Same runner                   |
| E2E tests       | Playwright via pnpm              | Unchanged (hybrid member)               | **NONE**                                 |
| TUI tests       | node-pty via pnpm                | Unchanged (Node.js test harness)        | **NONE**                                 |

### 10.4 Publish Pipeline Impact

The `publish.yml` workflow publishes `@eddacraft/anvil-cli` to npm:

- `deno task build` replaces `nx run-many -t build`
- `npm publish` remains unchanged (npm registry accepts any package)
- Additionally can publish to JSR with `deno publish`

---

## 11. Node.js API Usage & Migration Paths

### 11.1 API Usage Heatmap

| Node.js API                    | Occurrences | Deno Support | Migration              |
| ------------------------------ | ----------- | ------------ | ---------------------- |
| `node:fs` / `node:fs/promises` | ~150        | **Full**     | No changes needed      |
| `node:path`                    | ~100        | **Full**     | No changes needed      |
| `node:crypto`                  | ~30         | **Full**     | No changes needed      |
| `node:child_process`           | ~20         | **Full**     | No changes needed      |
| `node:os`                      | ~15         | **Full**     | No changes needed      |
| `node:url`                     | ~10         | **Full**     | No changes needed      |
| `node:stream/promises`         | ~5          | **Full**     | No changes needed      |
| `node:zlib`                    | ~5          | **Full**     | No changes needed      |
| `node:events`                  | ~5          | **Full**     | No changes needed      |
| `node:util`                    | ~5          | **Full**     | No changes needed      |
| `process.cwd()`                | ~40         | **Full**     | No changes needed      |
| `process.env`                  | ~20         | **Full**     | Requires `--allow-env` |
| `process.argv`                 | ~5          | **Full**     | No changes needed      |
| `Buffer`                       | ~25         | **Full**     | No changes needed      |

### 11.2 Summary

The Node.js API surface used by this project is **fully supported** by Deno 2.6.
The codebase's use of `node:` protocol imports means **zero source-level
changes** are needed for Node.js API compatibility. This is the strongest
argument for migration feasibility.

---

## 12. Native Addon & FFI Considerations

### 12.1 Native Dependencies

| Package              | Type           | Usage                      | Strategy                        |
| -------------------- | -------------- | -------------------------- | ------------------------------- |
| `node-pty` 1.1.0     | Node-API (C++) | TUI E2E testing            | Keep test harness on Node.js    |
| `keytar`             | Node-API (C++) | VS Code credential storage | Extension stays on Node.js      |
| `esbuild` 0.27.2     | Go binary      | VS Code extension bundling | Has native Deno support         |
| `@swc/core` ~1.15.11 | Rust (N-API)   | Fast compilation in dev    | Not needed — Deno has native TS |

### 12.2 esbuild

esbuild has official Deno support. The VS Code extension stays on Node.js, so
its esbuild usage is unaffected. If other build scripts need esbuild, it works
via `npm:esbuild` or `deno bundle` (which uses esbuild internally).

### 12.3 node-pty

By keeping the TUI test harness on Node.js, node-pty is unaffected. The Anvil
CLI itself can run on Deno — the test harness spawns it as a subprocess
regardless of runtime.

### 12.4 @swc/core

SWC is used for fast TypeScript compilation in development. Under Deno, this is
unnecessary — Deno compiles TypeScript natively with no separate step. This
dependency can be removed for Deno-migrated packages.

---

## 13. Developer Tooling Impact

### 13.1 Local Development

| Tool            | Current                          | With Deno Hybrid                              | Impact                       |
| --------------- | -------------------------------- | --------------------------------------------- | ---------------------------- |
| Package manager | pnpm 10.26                       | `deno install`                                | **LOW** — Similar workflow   |
| Task runner     | `pnpm run` (via Nx for build)    | `deno task`                                   | **LOW** — Direct replacement |
| TypeScript      | tsc 5.9.3 (separate compile)     | Deno native TS (no compile needed)            | **Improvement**              |
| Script runner   | tsx 4.21 / ts-node 10.9          | `deno run` (native TS)                        | **Improvement**              |
| Linting         | ESLint 9.39 + custom plugin      | `deno lint` + ESLint via npm for custom rules | **MEDIUM**                   |
| Formatting      | Prettier 3.8.1                   | `deno fmt` or Prettier via npm                | **LOW**                      |
| Git hooks       | Husky 9.1.7 + lint-staged 16.2.7 | `deno_hooks` or git `core.hooksPath`          | **MEDIUM**                   |
| VS Code         | Node.js Extension Host           | Deno extension for Deno members               | **MEDIUM**                   |

### 13.2 eslint-plugin-anvil

The custom ESLint plugin stays as a Node.js hybrid member. ESLint can run via
npm compatibility layer on Deno members, or those members can use `deno lint`
with its built-in rules. Custom Anvil-specific rules would need to continue
running via ESLint until `deno lint` supports custom plugins.

### 13.3 IDE Experience

VS Code supports per-folder Deno enablement via `.vscode/settings.json`:

```json
{
  "deno.enablePaths": [
    "packages/anvil",
    "packages/platform",
    "packages/adapters",
    "packages/aps",
    "apps/anvil-cli"
  ]
}
```

This gives Deno Language Server to migrated packages while keeping the built-in
TypeScript service for Node.js hybrid members (website, docs-site, etc.).

---

## 14. Security Model Comparison

### 14.1 Permission Model

| Permission    | Node.js      | Deno 2.6                        | Anvil Impact                          |
| ------------- | ------------ | ------------------------------- | ------------------------------------- |
| File system   | Unrestricted | `--allow-read`, `--allow-write` | Every fs operation needs permission   |
| Network       | Unrestricted | `--allow-net`                   | API calls, npm installs               |
| Environment   | Unrestricted | `--allow-env`                   | `process.env` usage (~20 occurrences) |
| Child process | Unrestricted | `--allow-run`                   | `execSync`, `spawn` (~20 occurrences) |
| FFI           | N/A          | `--allow-ffi`                   | node-pty (stays Node.js)              |
| All           | Default      | `--allow-all` (or `-A`)         | Typical dev shortcut                  |

### 14.2 Practical Impact

For the **Anvil CLI** specifically, Deno's permission model is a genuine
improvement — the CLI is distributed to end users who would benefit from knowing
exactly what file system and network access the tool requires.

For development, `deno task` scripts can embed permissions in `deno.json`:

```json
{
  "tasks": {
    "dev": "deno run --allow-read --allow-write --allow-env src/main.ts",
    "test": "deno run -A npm:vitest run"
  }
}
```

### 14.3 Lifecycle Script Security

Deno 2.6's `allowScripts` in `deno.json` provides an **explicit audit trail**
for packages with install scripts. This is a genuine security improvement over
pnpm's default behavior where lifecycle scripts run without approval.

---

## 15. Performance Considerations

### 15.1 Gains

| Area                           | Impact                 | Notes                                 |
| ------------------------------ | ---------------------- | ------------------------------------- |
| TypeScript execution           | **Faster**             | No tsx/ts-node compilation step       |
| Build step elimination         | **Significant**        | Internal packages skip `tsc` entirely |
| Dependency install (hot cache) | **90% faster**         | Deno vs npm/pnpm benchmarks           |
| `deno check` / `tsgo`          | **Potentially faster** | Experimental Go-based type checker    |

### 15.2 Neutral

| Area                | Impact         | Notes                                  |
| ------------------- | -------------- | -------------------------------------- |
| V8 engine           | **Same**       | Both runtimes use V8                   |
| Runtime performance | **Same**       | Both execute JS at same speed          |
| File I/O            | **Comparable** | Deno uses Rust ops; Node.js uses libuv |

### 15.3 No Longer Relevant

The original assessment noted loss of Nx computation caching as a performance
concern. The audit shows **caching was never active** — it was configured for
`@nx/js:tsc` which no package uses. There is no performance to lose.

---

## 16. Risk Register

### 16.1 Blockers

| ID  | Risk                              | Likelihood | Impact   | Mitigation                                              |
| --- | --------------------------------- | ---------- | -------- | ------------------------------------------------------- |
| R1  | Vitest 4.x panics on Deno 2.5-2.6 | Confirmed  | Critical | Keep tests on Node.js short-term; wait for upstream fix |

### 16.2 High Risks

| ID  | Risk                                          | Likelihood | Impact | Mitigation                                           |
| --- | --------------------------------------------- | ---------- | ------ | ---------------------------------------------------- |
| R2  | node-pty FFI incompatibility                  | Medium     | High   | TUI test harness stays on Node.js                    |
| R3  | Developer productivity loss during transition | Medium     | High   | Phased migration; hybrid members reduce blast radius |

### 16.3 Medium Risks

| ID  | Risk                                                      | Likelihood | Impact | Mitigation                                          |
| --- | --------------------------------------------------------- | ---------- | ------ | --------------------------------------------------- |
| R4  | Playwright test runner conflicts with Deno test discovery | Medium     | Medium | E2E tests stay as Node.js hybrid member             |
| R5  | ESLint custom plugin compatibility issues                 | Low        | Medium | Plugin stays on Node.js; ESLint runs via npm compat |
| R6  | Husky/lint-staged replacement fragility                   | Medium     | Medium | Use `deno_hooks` or git `core.hooksPath`            |
| R7  | VS Code Deno extension per-folder configuration           | Low        | Medium | `deno.enablePaths` in workspace settings            |

### 16.4 Low Risks

| ID  | Risk                                       | Likelihood | Impact | Mitigation                             |
| --- | ------------------------------------------ | ---------- | ------ | -------------------------------------- |
| R8  | `import.meta.dirname` behavior differences | Low        | Low    | Already using Deno-compatible patterns |
| R9  | `Buffer` API differences                   | Very Low   | Low    | Deno supports Buffer via Node compat   |
| R10 | chalk/ora terminal color compat            | Very Low   | Low    | Work via npm compat layer              |

### 16.5 Risks Eliminated by Hybrid Approach

These were identified in the initial assessment but are no longer applicable:

| Original Risk                       | Why Eliminated                                              |
| ----------------------------------- | ----------------------------------------------------------- |
| Nx has no Deno support              | Nx is lightweight; Deno workspaces + `deno task` replace it |
| `workspace:*` protocol unsupported  | Deno supports workspace protocol in `package.json` members  |
| Docusaurus 3.9 build broken on Deno | Stays on Node.js as hybrid member                           |
| Next.js 16 Vercel deployment        | Stays on Node.js as hybrid member                           |
| Loss of build caching               | Caching was never active                                    |
| VS Code extension compat            | Stays on Node.js                                            |

---

## 17. Migration Strategy Recommendation

### 17.1 Recommended: **Phased Hybrid Migration**

The hybrid workspace model makes migration feasible with controlled risk. The
Vitest blocker is mitigated by keeping tests on Node.js short-term.

### 17.2 Phase Plan

#### Phase 1: Foundation

- Create root `deno.json` with workspace configuration
- Set `nodeModulesDir: auto`
- Configure `allowScripts` for native addons
- Validate `deno install` resolves all dependencies
- Add VS Code `deno.enablePaths` configuration
- **No code changes** — just configuration

#### Phase 2: Pure Logic Packages

Migrate packages with zero Node.js-specific dependencies:

- `@eddacraft/anvil-contracts` — schemas, types, events (zero deps)
- `@eddacraft/anvil-kindling-integration` — contracts only
- `@eddacraft/anvil-edda-stack` — memory stack contracts

For each:

1. Add `deno.json` with `name`, `version`, `exports`
2. Verify `deno check` passes
3. Verify existing Vitest tests still pass via Node.js
4. Remove `tsc` build step (Deno imports `.ts` directly)

#### Phase 3: Core Domain Packages

- `@eddacraft/anvil-core` — domain logic
- `@eddacraft/anvil-ports` — interfaces
- `@eddacraft/anvil-runtime` — orchestration (uses `node:fs`, `node:crypto`)
- `@eddacraft/anvil-policy` — OPA wrappers
- `packages/platform/*` — config, crypto, storage

These use `node:` APIs extensively — all fully supported by Deno 2.6.

#### Phase 4: CLI Application

- `apps/anvil-cli` — Commander.js + Ink
- Validate CLI runs under `deno run -A`
- Update `bin` entry for Deno execution
- Test npm publish still works (`deno task build` for npm output)
- TUI tests stay on Node.js test harness

#### Phase 5: Ecosystem Packages

- `packages/adapters` — format converters
- `packages/aps` — APS parser (remark/unified)
- `tools/codemods` — ts-morph (validate via npm compat)

#### Phase 6: CI/CD Transition

- Add `setup-deno` to CI alongside `setup-node`
- Migrate build commands to `deno task`
- Keep Vitest running via Node.js until upstream fix lands
- Update publish workflow

#### Phase 7: Test Migration (When Vitest Fix Lands)

- Migrate Vitest to run under Deno
- OR adopt `Deno.test()` for Deno-native packages
- Remove tsx/ts-node dependencies
- Remove Nx dependencies

### 17.3 What Stays on Node.js Permanently

| Component                      | Reason                      |
| ------------------------------ | --------------------------- |
| `apps/website`                 | Next.js + Vercel deployment |
| `apps/docs-site`               | Docusaurus                  |
| `apps/e2e`                     | Playwright test harness     |
| `packages/vscode-extension`    | VS Code extension host      |
| `packages/eslint-plugin-anvil` | ESLint ecosystem            |
| `packages/tooling/*`           | ESLint/TS config sharing    |

---

## 18. Decision Matrix

### 18.1 Weighted Scoring (Revised)

| Criterion               | Weight | Node.js (Current) | Deno 2.6 Hybrid | Notes                                          |
| ----------------------- | ------ | ----------------- | --------------- | ---------------------------------------------- |
| Monorepo support        | 15%    | 10/10             | 8/10            | Deno workspaces replace Nx for this project    |
| Package management      | 10%    | 10/10             | 8/10            | Hybrid workspace protocol supported            |
| Test framework compat   | 20%    | 10/10             | 5/10            | Vitest blocker mitigated by Node.js fallback   |
| Framework compatibility | 10%    | 10/10             | 10/10           | Hybrid members eliminate framework concerns    |
| CI/CD pipeline          | 10%    | 10/10             | 7/10            | Hybrid pipeline, moderate rework               |
| Developer experience    | 10%    | 9/10              | 8/10            | Native TS is better; some tooling churn        |
| Security model          | 10%    | 6/10              | 9/10            | CLI benefits from granular permissions         |
| TypeScript support      | 10%    | 8/10              | 10/10           | Native TS, no build step for internal packages |
| Performance             | 5%     | 8/10              | 9/10            | Fewer build steps, faster installs             |

**Weighted Score:**

- **Node.js (Current): 9.10 / 10**
- **Deno 2.6 Hybrid: 7.85 / 10**

The gap has narrowed significantly from the initial assessment (was 4.35/10) to
7.85/10 with the hybrid approach. The remaining delta is primarily the Vitest
blocker (temporary) and migration effort (one-time).

### 18.2 Break-Even Timeline

The migration becomes net-positive when:

1. **Vitest works on Deno** — likely within 3-6 months (active development)
2. **Build steps eliminated** — immediate benefit for internal packages
3. **Developer familiarity** — 2-4 weeks of team adjustment
4. **CI stabilization** — 1-2 sprints of pipeline hardening

---

## 19. References

### Deno 2.6

- [Deno 2.6: dx is the new npx](https://deno.com/blog/v2.6)
- [Deno 2.6 Release Notes](https://github.com/denoland/deno/releases/tag/v2.6.7)
- [Announcing Deno 2](https://deno.com/blog/v2.0)

### Workspaces & Monorepos

- [Deno Workspaces and Monorepos](https://docs.deno.com/runtime/fundamentals/workspaces/)
- [Building a Deno v2 Monorepo](https://www.britrunner.xyz/post/building-a-deno-v2-monorepo-part-1)
- [Deno 1.45: Workspace and Monorepo Support](https://deno.com/blog/v1.45)
- [Monodeno — Task Runner for Deno Workspaces](https://jsr.io/@jurassicjs/monodeno)

### Compatibility

- [Deno Node and npm Compatibility](https://docs.deno.com/runtime/fundamentals/node/)
- [deno.json and package.json](https://docs.deno.com/runtime/fundamentals/configuration/)
- [Hybrid Monorepo Cross-Member Imports](https://questions.deno.com/m/1288791037275537468)

### Testing

- [Vitest Panics on Deno 2.5.6 — Issue #31354](https://github.com/denoland/deno/issues/31354)
- [Supporting Vitest — Deno Issue #23882](https://github.com/denoland/deno/issues/23882)
- [Deno 2 and Playwright](https://honman.dev/posts/deno-2-and-playwright)
- [Running Playwright with Deno](https://www.kapp.technology/en/blog/run-playwright-on-deno-javascript-runtime/)

### Tooling

- [Deno 2.4: deno bundle is back](https://deno.com/blog/v2.4)
- [deno_hooks — Git Hooks Manager for Deno](https://github.com/Yakiyo/deno_hooks)
- [Deno in 2024](https://deno.com/blog/deno-in-2024)

### Ecosystem

- [Deno 2 vs Node.js vs Bun in 2026](https://dev.to/pockit_tools/deno-2-vs-nodejs-vs-bun-in-2026-the-complete-javascript-runtime-comparison-1elm)
- [Deno LTS End-of-Life Schedule](https://endoflife.date/deno)
