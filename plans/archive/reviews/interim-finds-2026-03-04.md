# Interim Review Findings (Systematic Pass 1)

**Date:** 2026-03-04
**Reviewer:** Codex
**Branch:** `PR-1`
**Scope:** Package-by-package triage (`packages/*`), then shared utils/libs,
then apps (`apps/*`)
**Method:** Static analysis and manual code inspection (execution checks
blocked by dependency install/network failure)

---

## Confirmed Findings

### F-001: Release smoke check uses Unix `ls` in preflight (cross-platform break)

- **Priority:** P1
- **Impact:** High (release flow fails on Windows shells)
- **Confidence:** High
- **Area:** `apps/anvil-cli`
- **Reference:** `apps/anvil-cli/src/services/release-preflight.ts:65`
- **Details:** `runSmokeCheck()` calls `runCommand('ls', [tmpDir], workspaceRoot)`.
  This is Unix-specific and can fail even when `pnpm pack` succeeds.
- **Status:** ✅ Resolved — commit `0198d82a`. `release-preflight.ts:71` now uses
  `readdirSync(tmpDir)` instead of Unix `ls`.

### F-002: Workflow monitor can mark non-publish run as exact match

- **Priority:** P1
- **Impact:** Medium-High (release telemetry/watching may target wrong run)
- **Confidence:** High
- **Area:** `apps/anvil-cli`
- **Reference:** `apps/anvil-cli/src/services/release-monitor.ts:48-49`
- **Details:** Any run with `headBranch === tagName` is returned with
  `exact: true`, even if workflow name is not `Publish to NPM`.
- **Status:** ✅ Resolved — commit `0198d82a`. `release-monitor.ts:46` now checks
  `r.name === 'Publish to NPM' && r.headBranch === tagName` before setting
  `exact: true`.

### F-003: Template rendering builds regex from unescaped variable names

- **Priority:** P2
- **Impact:** Medium (incorrect rendering or runtime regex errors)
- **Confidence:** High
- **Area:** `apps/anvil-cli`
- **Reference:** `apps/anvil-cli/src/services/template-loader.ts:278,284`
- **Details:** Variable names are interpolated into `new RegExp(...)` without
  escaping metacharacters.
- **Status:** ✅ Resolved — PR #513. Regex metacharacters are now escaped before
  interpolation.

---

## Systematic Package Sweep (Pass 1)

### packages/adapters

- **Status:** No critical defects confirmed in pass 1.
- **Potential work item:** Replace direct `console.error` in library discovery
  paths with structured debug logger for consistency.
- **Reference:** `packages/adapters/src/base/file-discovery.ts:221,227`

### packages/anvil/contracts

- **Status:** No findings in pass 1.

### packages/anvil/core

- **Status:** No critical defects confirmed in pass 1.
- **Potential work item:** Route library-level `console.error` usage through
  shared debug/log abstraction to avoid mixed output channels.
- **Reference:** `packages/anvil/core/src/architecture/baseline.ts:64,75`,
  `packages/anvil/core/src/drift/snapshot-storage.ts:94`

### packages/anvil/policy

- **Status:** No new critical defects confirmed in pass 1.
- **Potential work item:** Add explicit schema validation wrappers around
  remaining plain `JSON.parse` callsites in bundle index/manifest loading paths.
- **Reference:** `packages/anvil/policy/src/bundle-manager.ts:468`,
  `packages/anvil/policy/src/bundle-verifier.ts:237`

### packages/anvil/ports

- **Status:** No findings in pass 1.

### packages/anvil/runtime

- **Status:** No new critical defects confirmed in pass 1.
- **Potential work item 1:** Replace direct stderr logs in checks/config with
  shared logger (`createDebugger`) for consistent CLI/runtime output policy.
- **Reference:** `packages/anvil/runtime/src/gate/checks/secret.check.ts:301`,
  `packages/anvil/runtime/src/gate/gate-config.ts:99`
- **Potential work item 2:** Validate and categorise config parse failures
  as typed diagnostics (not only warning text) for deterministic downstream
  handling.

### packages/aps

- **Status:** No critical defects confirmed in pass 1.
- **Potential work item:** Replace shell-string `execSync(...)` git calls with
  argument-based invocation (`execFileSync`) for stricter command hardening.
- **Reference:** `packages/aps/src/state/index.ts:395,410`

### packages/edda-stack

- **Status:** No findings in pass 1.

### packages/eslint-plugin-anvil

- **Status:** No findings in pass 1.

### packages/kindling-integration

- **Status:** No findings in pass 1.

### packages/mcp-server

- **Status:** No new critical defects confirmed in pass 1.
- **Potential work item:** Separate normal startup messages (stdout/info) from
  error messages (stderr/error) for cleaner process supervision integration.
- **Reference:** `packages/mcp-server/src/bin-http.ts:28`

### packages/platform/config

- **Status:** No findings in pass 1.

### packages/platform/crypto

- **Status:** No findings in pass 1.

### packages/platform/storage

- **Status:** No findings in pass 1.

### packages/tooling/eslint-config

- **Status:** No findings in pass 1.

### packages/tooling/tsconfig

- **Status:** No findings in pass 1.

### packages/vscode-extension

- **Status:** No critical defects confirmed in pass 1.
- **Potential work item:** Standardise extension diagnostic output via VS Code
  output channel wrapper instead of direct `console.error`.
- **Reference:** `packages/vscode-extension/src/services/planWatcher.ts:101`

---

## Shared Utils/Libs Sweep (Pass 1)

### apps/anvil-cli `src/utils`

- **Potential work item:** Tighten JSON/YAML loading with schema gate where
  data currently flows through generic parse helpers.
- **Reference:** `apps/anvil-cli/src/utils/file-io.ts:16,87`

### apps/website `lib`

- **Potential work item:** Reduce raw error logging and ensure provider errors
  are consistently sanitised to avoid accidental sensitive value leakage.
- **Reference:** `apps/website/lib/email.ts:63,70`

---

## Apps Sweep (Pass 1)

### apps/anvil-api

- **Status:** No findings in pass 1.

### apps/anvil-cli

- **Confirmed findings:** `F-001`, `F-002`, `F-003`
- **Potential work item:** Continue CRB-019 stream/logging convergence for
  command/service-level `console.*` usage not yet migrated.

### apps/docs-site

- **Status:** No findings in pass 1.

### apps/e2e

- **Status:** No production findings (test-only shell usage observed).

### apps/website

- **Potential work item 1:** Add abuse controls on waitlist endpoint
  (rate limiting/challenge) to reduce automated signup spam risk.
- **Reference:** `apps/website/app/api/waitlist/route.ts`
- **Potential work item 2:** Consider `next/image` for hero brand image for
  built-in optimisation consistency.
- **Reference:** `apps/website/components/hero-section.tsx:14`

---

## Prioritised Work Item Backlog

| ID     | Priority | Impact       | Confidence | Area                    | Work Item |
| ------ | -------- | ------------ | ---------- | ----------------------- | --------- |
| W-001  | P1       | High         | High       | apps/anvil-cli          | Replace Unix `ls` in smoke check with Node fs directory listing |
| W-002  | P1       | Medium-High  | High       | apps/anvil-cli          | Fix workflow run matching logic (`exact`) to require workflow-name match |
| W-003  | P2       | Medium       | High       | apps/anvil-cli          | Escape template variable names before regex construction |
| W-004  | P2       | Medium       | Medium     | apps/website            | Add waitlist endpoint abuse controls (rate limiting/challenge) |
| W-005  | P2       | Medium       | Medium     | packages/aps            | Replace `execSync` shell-string git calls with argument-based execution |
| W-006  | P2       | Medium       | Medium     | packages/anvil/policy   | Add schema wrappers for remaining plain JSON.parse in bundle metadata paths |
| W-007  | P3       | Low-Medium   | High       | packages/anvil/runtime  | Route secret check/gate config logging through shared logger |
| W-008  | P3       | Low-Medium   | High       | packages/adapters       | Replace file-discovery direct console errors with structured debug logging |
| W-009  | P3       | Low-Medium   | High       | packages/anvil/core     | Standardise library logging (baseline/snapshot) via logger abstraction |
| W-010  | P3       | Low          | Medium     | packages/vscode-extension | Route validation errors via extension output channel abstraction |
| W-011  | P3       | Low          | Medium     | apps/website            | Harden/sanitise email delivery error logging |
| W-012  | P3       | Low          | Medium     | apps/website            | Optimise hero image via `next/image` where compatible with design constraints |

---

## Execution Constraints During Review

- `pnpm install --frozen-lockfile` failed due registry DNS/network errors
  (`EAI_AGAIN`), so `build/test/typecheck` could not be executed in this pass.
- Findings above are based on static code analysis and manual inspection.
