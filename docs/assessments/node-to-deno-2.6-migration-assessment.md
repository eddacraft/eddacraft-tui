# Node.js to Deno 2.6 Migration Assessment

## EddaCraft Anvil Monorepo — Full End-to-End Impact Analysis

**Date:** 2026-02-03
**Scope:** Complete assessment of migrating the Anvil monorepo from Node.js (>=20.0.0) to Deno 2.6
**Current Stack:** Node.js 24.x | pnpm 10.26.0 | Nx 22.4.3 | TypeScript 5.9.3

---

## Table of Contents

1. [Executive Summary](#1-executive-summary)
2. [Current Architecture Overview](#2-current-architecture-overview)
3. [Deno 2.6 Capabilities Assessment](#3-deno-26-capabilities-assessment)
4. [Module System Compatibility](#4-module-system-compatibility)
5. [Package Manager & Workspace Impact](#5-package-manager--workspace-impact)
6. [Build System & Nx Monorepo Impact](#6-build-system--nx-monorepo-impact)
7. [Framework Compatibility Matrix](#7-framework-compatibility-matrix)
8. [Testing Infrastructure Impact](#8-testing-infrastructure-impact)
9. [CI/CD Pipeline Impact](#9-cicd-pipeline-impact)
10. [Node.js API Usage & Migration Paths](#10-nodejs-api-usage--migration-paths)
11. [Native Addon & FFI Considerations](#11-native-addon--ffi-considerations)
12. [Developer Tooling Impact](#12-developer-tooling-impact)
13. [Security Model Comparison](#13-security-model-comparison)
14. [Performance Considerations](#14-performance-considerations)
15. [Risk Register](#15-risk-register)
16. [Migration Strategy Recommendation](#16-migration-strategy-recommendation)
17. [Decision Matrix](#17-decision-matrix)
18. [References](#18-references)

---

## 1. Executive Summary

### Verdict: **NOT RECOMMENDED at this time**

Migrating the Anvil monorepo from Node.js to Deno 2.6 presents **high risk with limited tangible benefit** given the current project maturity, toolchain dependencies, and ecosystem gaps. While Deno 2.6 has made significant strides in Node.js compatibility, several critical blockers remain:

| Factor | Assessment | Severity |
|--------|-----------|----------|
| Nx monorepo orchestration | No Deno plugin exists | **BLOCKER** |
| pnpm `workspace:*` protocol | Not fully supported in Deno | **BLOCKER** |
| Vitest 4.x compatibility | Known panics with Deno 2.5-2.6 | **HIGH** |
| Docusaurus 3.9 | Partial support; hybrid approach recommended | **HIGH** |
| Next.js 16 | Functional but with friction | **MEDIUM** |
| node-pty (TUI testing) | Requires native addon workarounds | **HIGH** |
| Husky/lint-staged | Not directly compatible | **MEDIUM** |
| 22 workspace packages | Mass migration overhead | **HIGH** |

**Estimated migration effort:** Months of engineering work with ongoing stabilization risk.
**Recommended alternative:** Continue on Node.js; re-evaluate when Deno 3.x ships or Nx adds first-class Deno support.

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

| Metric | Value |
|--------|-------|
| Total npm packages | 22 |
| TypeScript config files | 35 |
| ES Module packages (`"type": "module"`) | 17 |
| Node.js built-in imports (`node:` protocol) | ~397 occurrences |
| `process.*` API calls | ~209 occurrences |
| Native addon dependencies | 2 (node-pty, keytar) |
| CI matrix targets | Node 20.x, 22.x |
| Test frameworks | Vitest 4.0.18, Playwright 1.58.0, tuistory 0.0.9 |

### 2.3 Positive Migration Indicators

The codebase has several characteristics that favor *eventual* Deno migration:

- **ES Modules throughout** — 17/22 packages use `"type": "module"`; zero CommonJS `require()` calls
- **`node:` protocol imports enforced** — ESLint rule `unicorn/prefer-node-protocol: error` ensures all Node.js built-in imports use the `node:` prefix, which Deno natively supports
- **Modern `import.meta` patterns** — Uses `import.meta.url`, `import.meta.dirname` instead of `__dirname`/`__filename`
- **No `.cjs` files** — Complete ES module adoption
- **TypeScript-first** — Deno's native TypeScript support aligns well

---

## 3. Deno 2.6 Capabilities Assessment

### 3.1 Key Deno 2.6 Features Relevant to This Migration

| Feature | Description | Relevance |
|---------|-------------|-----------|
| `dx` command | npx equivalent for running npm/JSR binaries | Replaces `pnpm dlx` / `npx` usage in scripts |
| `@types/node` built-in | Node.js type declarations included by default | Eliminates `@types/node` devDependency |
| `--require` flag | CommonJS module preloading | Useful for SWC register compatibility |
| `deno audit` | Dependency vulnerability scanning | Replaces npm audit / pnpm audit |
| `tsgo` integration | Experimental fast TypeScript type checking (Go-based) | Could accelerate typecheck pipeline |
| Granular permissions | `--ignore-read`, `--ignore-env` | More control than Node.js but adds configuration burden |
| `allowScripts` in `deno.json` | Lifecycle script approval for native addons | Required for node-pty and esbuild |
| JUnit reports | Clean XML output without ANSI codes | CI/CD compatible test reporting |
| Source phase imports | Access raw module sources | Useful for code analysis tooling |

### 3.2 Node.js Compatibility Layer Status

Deno 2.6 supports:
- `node:fs`, `node:path`, `node:crypto`, `node:child_process`, `node:os`, `node:url`, `node:util`, `node:events`, `node:stream`, `node:zlib` — **all used by Anvil**
- `process` global (including `process.env`, `process.cwd()`, `process.argv`, `process.exit()`)
- `Buffer` global
- `package.json` detection and `node_modules` resolution
- npm package imports via `npm:` specifiers
- Subpath imports (`#/` prefix)

**Not fully supported or with known issues:**
- `node:worker_threads` — partial (affects Vitest)
- `node:vm` — partial (affects Vitest, Docusaurus)
- pnpm `workspace:*` protocol in `package.json` — open issue
- Complex lifecycle scripts — requires explicit approval

---

## 4. Module System Compatibility

### 4.1 Current State: Strong ESM Foundation

| Aspect | Status | Migration Impact |
|--------|--------|-----------------|
| ES Module packages | 17/22 use `"type": "module"` | **LOW** — Already compatible |
| CommonJS packages | 5 (website, docs-site, eslint-plugin, vscode-ext, root) | **MEDIUM** — Need conversion or dual-emit |
| `.mjs` config files | 9 files (eslint, next, postcss) | **LOW** — Already compatible |
| `node:` protocol imports | ~397 occurrences | **NONE** — Deno supports `node:` natively |
| `import.meta.url` | Used throughout | **NONE** — Deno supports `import.meta` |
| Dynamic imports | Used in several packages | **LOW** — Generally compatible |

### 4.2 Required Module Changes

**Files requiring attention:**

1. **`apps/docs-site/`** — No `"type": "module"`, Docusaurus expects CJS context
2. **`apps/website/`** — No `"type": "module"`, Next.js manages module resolution
3. **`packages/eslint-plugin-anvil/`** — No `"type": "module"`, ESLint plugin format
4. **`packages/vscode-extension/`** — Must remain CJS (VS Code extension host is Node.js)
5. **Root `package.json`** — No `"type": "module"`, mixed script runner

### 4.3 Import Map Requirements

Deno would require an `importMap` or `deno.json` imports section to resolve:
- `workspace:*` package references (currently 12 inter-package dependencies)
- Path aliases defined in `tsconfig.base.json`
- Bare specifier imports from npm packages

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

### 5.2 Deno Workspace Support

Deno has its own workspace system (since 1.45) configured via `deno.json`:

```jsonc
// Hypothetical deno.json
{
  "workspace": [
    "apps/anvil-cli",
    "packages/anvil/core",
    // ... 20 more entries
  ]
}
```

**Critical incompatibilities:**

| Feature | pnpm | Deno Workspaces | Gap |
|---------|------|----------------|-----|
| `workspace:*` protocol | Native | **Not supported** (open issue #18192) | **BLOCKER** |
| Hoisted `node_modules` | `shamefully-hoist=true` | `"nodeModulesDir": "auto"` | Behavioral differences |
| Lockfile | `pnpm-lock.yaml` | `deno.lock` | Full regeneration required |
| Lifecycle scripts | Runs by default | Requires `allowScripts` approval | Security model change |
| Peer dependency resolution | `auto-install-peers=true` | Different algorithm | Potential resolution conflicts |
| Catalog/overrides | `pnpm.overrides` | Not supported | Must use import maps |
| Built dependency filtering | `onlyBuiltDependencies` | `allowScripts` | Different mechanism |

### 5.3 Migration Path

A migration would require:

1. Replace `pnpm-workspace.yaml` with `deno.json` workspace configuration
2. Convert all `workspace:*` references to explicit version or path specifiers
3. Regenerate lockfile from scratch (`deno.lock`)
4. Configure `allowScripts` for native addons (node-pty, esbuild)
5. Set `"nodeModulesDir": "auto"` for npm package compatibility
6. Replicate `pnpm.overrides` using Deno import maps
7. Update all CI workflows to use `deno install` instead of `pnpm install --frozen-lockfile`

**Risk:** The `workspace:*` protocol incompatibility is a **hard blocker** for the 12 inter-package dependencies in this monorepo. Without this, the workspace package graph cannot be resolved.

---

## 6. Build System & Nx Monorepo Impact

### 6.1 Current Nx Configuration

The project uses Nx 22.4.3 with these plugins:
- `@nx/js/typescript` — TypeScript compilation and typecheck
- `@nx/eslint/plugin` — ESLint integration
- `@nx/vite/plugin` — Vitest integration

Nx provides:
- **Task orchestration** — parallel builds with `dependsOn` graph
- **Computation caching** — skips unchanged targets
- **Code generators** — `@eddacraft/anvil-generators`
- **Affected analysis** — builds only changed packages
- **Release management** — versioning and publishing

### 6.2 Deno Compatibility Assessment

| Nx Feature | Deno Support | Impact |
|-----------|-------------|--------|
| `nx` CLI | Runs on Node.js; **no Deno runtime** | **BLOCKER** |
| `@nx/js/typescript` plugin | Node.js only | **BLOCKER** |
| `@nx/eslint/plugin` | Node.js only | **BLOCKER** |
| `@nx/vite/plugin` | Node.js only | **BLOCKER** |
| Task caching | N/A without Nx | **LOST** capability |
| Affected analysis | N/A without Nx | **LOST** capability |
| Code generators | Node.js API based | **BLOCKER** |
| Verdaccio local registry | npm ecosystem tool | Would need reconfiguration |

### 6.3 Alternatives to Nx in Deno

| Alternative | Maturity | Monorepo Support | Task Caching |
|------------|----------|-----------------|--------------|
| Deno workspaces | Built-in | Basic (since 1.45) | No |
| Deno tasks (`deno task`) | Built-in | Per-package only | No |
| Turborepo | Production | Yes | Yes (but Node.js based) |
| Custom `deno.json` scripts | Manual | Manual wiring | No |
| Moon (moonrepo) | Growing | Yes | Yes (supports Deno) |

**Moon** is the most viable Nx alternative with Deno support, but would require a full migration of all Nx configuration, generators, and target definitions.

### 6.4 Impact Summary

Abandoning Nx means losing:
- Parallel task orchestration with dependency awareness
- Build caching (can save 40-70% CI time on incremental builds)
- Affected commands for selective testing
- Custom code generators (`generate:package`, `generate:anvil-package`)
- Plugin-based automatic target inference
- Release management integration

---

## 7. Framework Compatibility Matrix

### 7.1 Detailed Framework Assessment

| Framework | Version | Deno 2.6 Status | Severity | Notes |
|-----------|---------|-----------------|----------|-------|
| **Next.js** | 16.0.10 | Functional with friction | **MEDIUM** | `proxy.ts` requires Node.js runtime; Turbopack assumes Node.js; Vercel deployment uses Node.js |
| **React** | 19.2.0 | Compatible | **LOW** | Works via npm compatibility layer |
| **Docusaurus** | 3.9.2 | Partial support | **HIGH** | `require` errors during install; Deno recommends hybrid approach (Node.js build + Deno serve) |
| **Ink** | 6.6.0 | Likely compatible | **LOW** | Works via `npm:` specifiers; Deno 2 has `process` global |
| **Commander.js** | 14.0.2 | Compatible | **LOW** | Pure JS/ESM package |
| **Zod** | 4.3.6 | Compatible | **NONE** | Pure JS/ESM, no Node.js APIs |
| **Tailwind CSS** | 4.1.9 | Compatible | **LOW** | PostCSS pipeline may need adjustment |
| **Radix UI** | Various | Compatible | **LOW** | React components, no Node.js APIs |
| **Vite** | 7.3.1 | Partial | **MEDIUM** | Used as test runner via Nx plugin; direct Deno support is limited |
| **ESLint** | 9.39.2 | Functional | **MEDIUM** | Deno has built-in linter; ESLint works via npm compat |
| **Prettier** | 3.8.1 | Functional | **LOW** | Deno has built-in formatter; Prettier works via npm compat |

### 7.2 Next.js 16 Specific Concerns

Next.js 16 introduces:
- `proxy.ts` replacing `middleware.ts` — runs on Node.js runtime explicitly
- Turbopack as default bundler — deeply integrated with Node.js
- Cache Components with PPR — untested on Deno

The `apps/website/next.config.mjs` would need to be validated for Deno compatibility. Vercel deployments assume Node.js runtime, so the production deployment path remains Node.js regardless.

### 7.3 Docusaurus 3.9 Specific Concerns

The official Deno recommendation for Docusaurus is a **hybrid approach**:
> Use Node.js + npm for building the Docusaurus site, then use Deno for serving it.

This means `apps/docs-site/` would likely remain on Node.js even in a Deno migration, creating a split runtime situation.

---

## 8. Testing Infrastructure Impact

### 8.1 Vitest 4.0.18

| Aspect | Current | With Deno 2.6 | Risk |
|--------|---------|---------------|------|
| Test runner | Vitest 4.0.18 via Nx plugin | Known panics (issue #31354) | **HIGH** |
| Coverage | `@vitest/coverage-v8` | v8 coverage may not work | **HIGH** |
| Environment | `happy-dom` | Requires `node:vm` compat | **MEDIUM** |
| Snapshot testing | Works | Likely works | **LOW** |
| Watch mode | Works | Untested with Deno | **MEDIUM** |

**Critical issue:** Deno 2.5.6 panics with Vitest 4.0.10+ (GitHub issue #31354). The Anvil monorepo uses Vitest 4.0.18, which is affected. This is a **hard blocker** for test execution.

**Alternative:** Deno's built-in test runner (`Deno.test()`) would require rewriting all test files across the 22 packages. The test runner lacks Vitest's `test.each`, `vi.mock()` factory patterns, and `happy-dom` environment integration.

### 8.2 Playwright 1.58.0

| Aspect | Current | With Deno 2.6 | Risk |
|--------|---------|---------------|------|
| Browser automation | Works natively | Requires `nodeModulesDir: auto` | **MEDIUM** |
| Config file | `playwright.config.ts` | May need `.mts` extension | **LOW** |
| Browser install | `pnpm exec playwright install` | `deno run -A npm:playwright install` | **LOW** |
| Test isolation | Works with Node.js test runner | Conflicts with Deno test runner | **MEDIUM** |

Playwright works on Deno 2.6 with workarounds:
- Set `"nodeModulesDir": "auto"` in `deno.json`
- May need `PW_DISABLE_TS_ESM=true` environment variable
- Test file discovery must be isolated from Deno's test runner

### 8.3 TUI Testing (tuistory + node-pty)

| Aspect | Current | With Deno 2.6 | Risk |
|--------|---------|---------------|------|
| node-pty | Native addon, compiled in CI | Requires FFI + `allowScripts` | **HIGH** |
| tuistory | npm package | Depends on node-pty | **HIGH** |
| CI integration | Requires `build-essential`, `python3` | Same + Deno FFI permissions | **HIGH** |

The TUI E2E tests depend on `node-pty`, a native C++ addon. While Deno 2.6 supports Node-API addons via FFI, this has not been specifically validated for node-pty and requires:
- `--allow-ffi` runtime permission
- `allowScripts` configuration for build steps
- System build tools (unchanged from current)

### 8.4 Test Migration Effort Estimate

| Test Category | Files | Migration Path | Effort |
|--------------|-------|---------------|--------|
| Vitest unit tests | ~100+ files | Wait for Deno/Vitest fix OR rewrite to `Deno.test()` | **VERY HIGH** |
| Playwright E2E | ~10 files | Add config workarounds | **LOW** |
| TUI E2E (tuistory) | ~5 files | Validate node-pty FFI | **MEDIUM** |
| ink-testing-library | ~5 files | Validate npm compat | **LOW** |

---

## 9. CI/CD Pipeline Impact

### 9.1 Current Pipeline (`.github/workflows/ci.yml`)

```yaml
# Current: Node.js based
- uses: pnpm/action-setup@v4
- uses: actions/setup-node@v4
  with:
    node-version: [20.x, 22.x]
    cache: 'pnpm'
- run: pnpm install --frozen-lockfile
```

### 9.2 Required Changes for Deno

```yaml
# Proposed: Deno based
- uses: denoland/setup-deno@v2
  with:
    deno-version: v2.6.x
- run: deno install  # No --frozen-lockfile equivalent with same behavior
```

### 9.3 Impact Analysis

| CI Feature | Current | With Deno | Impact |
|-----------|---------|-----------|--------|
| Runtime setup | `actions/setup-node@v4` | `denoland/setup-deno@v2` | **LOW** — Drop-in replacement |
| Package install | `pnpm install --frozen-lockfile` | `deno install` | **MEDIUM** — Different lockfile format |
| Dependency caching | `cache: 'pnpm'` | Manual cache configuration | **MEDIUM** — Need custom cache key |
| Matrix testing | Node 20.x, 22.x | Single Deno version | **LOW** — Simplification |
| Build command | `pnpm run build` (via Nx) | Requires replacement | **HIGH** — No Nx equivalent |
| Lint command | `pnpm run lint:check` | `deno lint` or ESLint via npm | **MEDIUM** — Different config |
| Type check | `pnpm run typecheck` (via tsc) | `deno check` or `tsc` via npm | **MEDIUM** — Different behavior |
| Test command | `pnpm run test -- --run --coverage` | Blocked by Vitest issue | **BLOCKER** |
| E2E tests | Playwright via pnpm | Playwright via Deno with workarounds | **MEDIUM** |
| TUI tests | node-pty via pnpm | Requires FFI validation | **HIGH** |

### 9.4 Publish Pipeline Impact

The `publish.yml` workflow publishes `@eddacraft/anvil-cli` to npm:
- `npm publish` remains unchanged (npm registry accepts any package)
- Version verification would need adjustment
- Build step depends on Nx (blocked)

---

## 10. Node.js API Usage & Migration Paths

### 10.1 API Usage Heatmap

| Node.js API | Occurrences | Deno Support | Migration |
|-------------|------------|-------------|-----------|
| `node:fs` / `node:fs/promises` | ~150 | **Full** | No changes needed |
| `node:path` | ~100 | **Full** | No changes needed |
| `node:crypto` | ~30 | **Full** | No changes needed |
| `node:child_process` | ~20 | **Full** | No changes needed |
| `node:os` | ~15 | **Full** | No changes needed |
| `node:url` | ~10 | **Full** | No changes needed |
| `node:stream/promises` | ~5 | **Full** | No changes needed |
| `node:zlib` | ~5 | **Full** | No changes needed |
| `node:events` | ~5 | **Full** | No changes needed |
| `node:util` | ~5 | **Full** | No changes needed |
| `process.cwd()` | ~40 | **Full** | No changes needed |
| `process.env` | ~20 | **Full** | Requires `--allow-env` |
| `process.argv` | ~5 | **Full** | No changes needed |
| `Buffer` | ~25 | **Full** | No changes needed |

### 10.2 Summary

The Node.js API surface used by this project is **well-supported** by Deno 2.6. The codebase's use of `node:` protocol imports means zero source-level changes are needed for Node.js API compatibility. This is the strongest argument in favor of eventual migration.

---

## 11. Native Addon & FFI Considerations

### 11.1 Native Dependencies

| Package | Type | Usage | Deno Path |
|---------|------|-------|-----------|
| `node-pty` 1.1.0 | Node-API (C++) | TUI E2E testing | FFI + `allowScripts` |
| `keytar` | Node-API (C++) | VS Code credential storage | Extension remains Node.js |
| `esbuild` 0.27.2 | Go binary | VS Code extension bundling | Native Deno support exists |
| `@swc/core` ~1.15.11 | Rust (N-API) | Fast compilation in dev | May work via Node-API compat |

### 11.2 esbuild

esbuild has official Deno support (`https://deno.land/x/esbuild`). The VS Code extension build (`packages/vscode-extension/`) would need to:
1. Import esbuild from Deno's esbuild package or use `npm:esbuild`
2. VS Code extensions must still output CJS since the VS Code extension host is Node.js
3. Build scripts would use `deno task` instead of `pnpm run`

### 11.3 node-pty

This is the most problematic native dependency:
- Required for TUI E2E tests
- Compiles C++ code during `npm install`
- Deno 2.3+ supports Node-API addons and FFI in `deno compile`
- Would require `--allow-ffi` permission and `allowScripts` in `deno.json`
- **Not validated** with Deno 2.6 specifically

---

## 12. Developer Tooling Impact

### 12.1 Local Development

| Tool | Current | With Deno | Impact |
|------|---------|-----------|--------|
| Package manager | pnpm 10.26 | `deno install` | **HIGH** — Different workflow |
| Task runner | Nx (`nx run`, `nx affected`) | `deno task` | **HIGH** — Loss of orchestration |
| TypeScript | tsc 5.9.3 (separate compile) | Deno native TS (or `tsgo`) | **MEDIUM** — Behavior differences |
| Linting | ESLint 9.39 + custom plugin | `deno lint` or ESLint via npm | **HIGH** — Custom plugin compat |
| Formatting | Prettier 3.8.1 | `deno fmt` or Prettier via npm | **MEDIUM** — Style differences |
| Git hooks | Husky 9.1.7 + lint-staged 16.2.7 | `deno_hooks` or `core.hooksPath` | **HIGH** — Requires reconfiguration |
| Script runner | tsx 4.21 / ts-node 10.9 | `deno run` (native TS) | **LOW** — Deno is better here |
| VS Code | Node.js Extension Host | Unchanged (extensions are Node.js) | **NONE** |

### 12.2 eslint-plugin-anvil

The custom ESLint plugin (`packages/eslint-plugin-anvil/`) enforces test quality rules. Under Deno:
- ESLint can run via npm compatibility layer
- The plugin itself needs no changes (pure JS)
- `deno lint` has its own rule set; custom rules require Deno's lint plugin API (different from ESLint)
- **Recommendation:** Continue using ESLint via npm compatibility until `deno lint` plugin API stabilizes

### 12.3 IDE Experience

| IDE Feature | Current | With Deno | Impact |
|------------|---------|-----------|--------|
| VS Code TypeScript | Built-in TS server | Deno Language Server | **MEDIUM** — Different behavior |
| VS Code Deno extension | Not used | Required (`deno.enable: true`) | **MEDIUM** — Disables built-in TS |
| Path aliases | Resolved via tsconfig | Must also be in `deno.json` | **LOW** — Dual configuration |
| IntelliSense | Full | Full (different source) | **LOW** |

---

## 13. Security Model Comparison

### 13.1 Permission Model

Deno's security model is fundamentally different from Node.js:

| Permission | Node.js | Deno 2.6 | Anvil Impact |
|-----------|---------|----------|-------------|
| File system | Unrestricted | `--allow-read`, `--allow-write` | Every fs operation needs permission |
| Network | Unrestricted | `--allow-net` | API calls, npm installs |
| Environment | Unrestricted | `--allow-env` | `process.env` usage (~20 occurrences) |
| Child process | Unrestricted | `--allow-run` | `execSync`, `spawn` (~20 occurrences) |
| FFI | N/A | `--allow-ffi` | node-pty, native addons |
| All | Default | `--allow-all` (or `-A`) | Typical dev shortcut |

### 13.2 Practical Impact

For development, most teams use `--allow-all` (`-A`), negating the security benefit. For production:
- The CLI tool (`anvil`) would benefit from granular permissions
- The website and docs-site are deployed to platforms (Vercel, Netlify) that assume Node.js
- CI/CD pipelines would need `--allow-all` for full test execution

### 13.3 Lifecycle Script Security

Deno 2.6's `allowScripts` in `deno.json` provides an **audit trail** for packages with install scripts (like node-pty). This is a genuine security improvement over npm/pnpm's default behavior.

---

## 14. Performance Considerations

### 14.1 Startup Time

| Scenario | Node.js | Deno 2.6 | Difference |
|----------|---------|----------|------------|
| TypeScript execution | Requires tsx/ts-node | Native | **Deno faster** (no compilation step) |
| npm package resolution | pnpm (cached) | Deno cache | **Comparable** |
| Cold start (no cache) | pnpm install | deno install | **pnpm faster** (optimized resolution) |

### 14.2 Runtime Performance

| Scenario | Node.js | Deno 2.6 | Notes |
|----------|---------|----------|-------|
| V8 engine version | Both use V8 | Both use V8 | **Comparable** |
| TypeScript type checking | tsc 5.9.3 | `tsgo` (experimental) | **Deno potentially faster** if tsgo stabilizes |
| File I/O | Optimized | Rust-based ops | **Comparable to slightly faster** |
| Crypto operations | OpenSSL | ring (Rust) | **Comparable** |

### 14.3 Build Performance

**Without Nx**, the project loses:
- Parallel task execution with dependency graph awareness
- Computation caching (the biggest performance factor)
- Affected analysis (skip unchanged packages)

This would likely **increase** CI build times significantly, counteracting any Deno startup improvements.

---

## 15. Risk Register

### 15.1 Blockers (Must be resolved before migration)

| ID | Risk | Likelihood | Impact | Mitigation |
|----|------|-----------|--------|------------|
| R1 | Nx has no Deno plugin or runtime support | Certain | Critical | Migrate to Moon or custom task runner |
| R2 | pnpm `workspace:*` protocol unsupported in Deno | Certain | Critical | Convert to explicit versions or path imports |
| R3 | Vitest 4.x panics on Deno 2.5-2.6 | Confirmed | Critical | Pin Vitest <4.0.10 or rewrite to Deno.test() |

### 15.2 High Risks

| ID | Risk | Likelihood | Impact | Mitigation |
|----|------|-----------|--------|------------|
| R4 | Docusaurus 3.9 build broken on Deno | High | High | Keep docs-site on Node.js (hybrid) |
| R5 | node-pty FFI incompatibility | Medium | High | Validate in isolation; fallback to Node.js for TUI tests |
| R6 | Loss of build caching increases CI time | Certain | High | Implement custom caching or adopt Moon |
| R7 | Developer productivity loss during transition | Certain | High | Gradual migration with parallel Node.js support |
| R8 | Deno LTS end-of-life April 2026 | Certain | High | Must track Deno release schedule; potential support gap |

### 15.3 Medium Risks

| ID | Risk | Likelihood | Impact | Mitigation |
|----|------|-----------|--------|------------|
| R9 | Playwright test runner conflicts with Deno test discovery | Medium | Medium | Isolate via config `testMatch` patterns |
| R10 | ESLint custom plugin compatibility issues | Low | Medium | Test in isolation; ESLint runs via npm compat |
| R11 | Husky/lint-staged replacement fragility | Medium | Medium | Use `deno_hooks` or git `core.hooksPath` |
| R12 | Vercel deployment expects Node.js | Low | Medium | Website deployment unaffected (stays Node.js) |
| R13 | `@swc/core` native addon compatibility | Medium | Medium | Test SWC with Deno FFI; fallback to Deno native TS |

### 15.4 Low Risks

| ID | Risk | Likelihood | Impact | Mitigation |
|----|------|-----------|--------|------------|
| R14 | `import.meta.dirname` behavior differences | Low | Low | Already using Deno-compatible patterns |
| R15 | `Buffer` API differences | Very Low | Low | Deno supports Buffer via Node compat |
| R16 | chalk/ora terminal color compat | Very Low | Low | Work via npm compat layer |

---

## 16. Migration Strategy Recommendation

### 16.1 Recommended: **Do Not Migrate Now — Re-evaluate in 6-12 Months**

Given the three confirmed blockers (Nx, workspace protocol, Vitest), a full migration is not feasible without significant engineering investment and ongoing instability risk.

### 16.2 Alternative: Incremental Adoption Strategy

If the team wants to begin positioning for Deno, a phased approach is possible:

#### Phase 0: Preparation (No Deno Required)

- Continue enforcing `node:` protocol imports (already done via ESLint)
- Maintain ES module purity (already strong)
- Avoid adding new CommonJS dependencies
- Document all Node.js API usage patterns

#### Phase 1: Validate Isolated Packages

- Test pure logic packages in Deno without changing the build:
  - `@eddacraft/anvil-contracts` (zero Node.js API dependencies)
  - `@eddacraft/anvil-kindling-integration` (minimal APIs)
- Run their test suites with `deno test` as an experiment
- No production impact

#### Phase 2: Tooling Scripts

- Migrate standalone scripts to Deno:
  - `tools/scripts/bench-anvil-check.mjs`
  - `test-hash.mjs`
  - Codemod CLI (`tools/codemods/`)
- These don't affect the main build pipeline

#### Phase 3: Evaluate Ecosystem Maturity

Before proceeding further, wait for:
- [ ] Nx Deno plugin or Moon reaching feature parity
- [ ] Deno resolving pnpm `workspace:*` support (issue #18192)
- [ ] Vitest 4.x working reliably on Deno 2.6+
- [ ] Deno 3.x (if announced) with improved monorepo support

#### Phase 4: Full Migration (Future)

Only proceed when all Phase 3 conditions are met:
- Migrate build orchestration to Deno-compatible tool
- Convert workspace package references
- Update CI/CD pipelines
- Migrate test infrastructure
- Update developer documentation

---

## 17. Decision Matrix

### 17.1 Weighted Scoring

| Criterion | Weight | Node.js (Current) | Deno 2.6 | Notes |
|-----------|--------|-------------------|----------|-------|
| Monorepo orchestration (Nx) | 20% | 10/10 | 2/10 | No Nx support in Deno |
| Package management maturity | 15% | 10/10 | 4/10 | workspace:* unsupported |
| Test framework compatibility | 15% | 10/10 | 3/10 | Vitest panics confirmed |
| Framework compatibility | 15% | 10/10 | 6/10 | Next.js works; Docusaurus partial |
| CI/CD pipeline stability | 10% | 10/10 | 5/10 | Significant rework needed |
| Developer experience | 10% | 9/10 | 6/10 | Better TS; worse tooling |
| Security model | 5% | 6/10 | 9/10 | Deno's permissions are superior |
| TypeScript support | 5% | 8/10 | 10/10 | Deno native TS is excellent |
| Performance | 5% | 8/10 | 7/10 | Offset by loss of caching |

**Weighted Score:**
- **Node.js (Current): 9.15 / 10**
- **Deno 2.6: 4.35 / 10**

### 17.2 Break-Even Analysis

Deno becomes viable for this project when:
1. Nx (or equivalent) supports Deno as a runtime — estimated 2026-2027
2. pnpm `workspace:*` protocol is supported — open issue, no timeline
3. Vitest runs reliably on Deno — likely within 3-6 months (active development)
4. Docusaurus 4.x ships with ESM-first architecture — unknown timeline

---

## 18. References

### Deno 2.6
- [Deno 2.6: dx is the new npx](https://deno.com/blog/v2.6)
- [Deno 2.6 Release Notes](https://github.com/denoland/deno/releases/tag/v2.6.7)
- [Announcing Deno 2](https://deno.com/blog/v2.0)

### Compatibility
- [Deno Node and npm Compatibility](https://docs.deno.com/runtime/fundamentals/node/)
- [Deno Workspaces and Monorepos](https://docs.deno.com/runtime/fundamentals/workspaces/)
- [pnpm monorepo support — Deno Issue #28894](https://github.com/denoland/deno/issues/28894)
- [workspace: specifier not supported — Deno Issue #18192](https://github.com/denoland/deno/issues/18192)

### Framework Support
- [Build a Next.js App with Deno](https://docs.deno.com/examples/next_tutorial/)
- [Next.js 16 Release](https://nextjs.org/blog/next-16)
- [Add Deno support to Next.js — Discussion #26428](https://github.com/vercel/next.js/discussions/26428)
- [Docusaurus Deno Compatibility — Issue #24589](https://github.com/denoland/deno/issues/24589)

### Testing
- [Supporting Vitest — Deno Issue #23882](https://github.com/denoland/deno/issues/23882)
- [Deno 2.5.6 panics with Vitest 4.0.10 — Issue #31354](https://github.com/denoland/deno/issues/31354)
- [Deno 2 and Playwright](https://honman.dev/posts/deno-2-and-playwright)
- [Running Playwright with Deno](https://www.kapp.technology/en/blog/run-playwright-on-deno-javascript-runtime/)

### Tooling
- [Deno 2.4: deno bundle is back](https://deno.com/blog/v2.4)
- [Ink Deno Support — Issue #250](https://github.com/vadimdemedes/ink/issues/250)
- [deno_hooks — Git Hooks Manager for Deno](https://github.com/Yakiyo/deno_hooks)

### Ecosystem
- [Deno 2 vs Node.js vs Bun in 2026](https://dev.to/pockit_tools/deno-2-vs-nodejs-vs-bun-in-2026-the-complete-javascript-runtime-comparison-1elm)
- [Nx Wrapping Up 2025](https://nx.dev/blog/wrapping-up-2025)
- [Deno LTS End-of-Life Schedule](https://endoflife.date/deno)
