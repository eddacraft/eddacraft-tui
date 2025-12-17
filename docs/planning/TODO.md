# Anvil Implementation TODO

Actionable task list organised by priority and horizon. For strategic context,
see [ROADMAP.md](./ROADMAP.md) and [PLAN.md](./PLAN.md).

---

## Current Status

| Component                               | Status      | Tests           |
| --------------------------------------- | ----------- | --------------- |
| APS Core (schema, validation, hashing)  | ✅ Complete | 100%            |
| Adapter Framework                       | ✅ Complete | 22 tests        |
| SpecKit Adapter                         | ✅ Complete | 114 tests       |
| BMAD Adapter                            | ✅ Complete | 86 tests        |
| Generic Markdown Adapter                | ✅ Complete | 32 tests        |
| CLI (validate, gate, export, hooks)     | ✅ Complete | 50 tests        |
| Gate v1 (lint, test, coverage, secrets) | ✅ Complete | ✅              |
| Gate Caching & Parallel Execution       | ✅ Complete | —               |
| Watch Mode                              | ✅ Complete | —               |
| OPA Policy Engine                       | ✅ Complete | ✅              |
| **Total Tests**                         |             | **598 passing** |

---

## 🎯 Next Up: Developer Experience Quick Wins

High value, low difficulty. Do these first.

### Git Hooks Integration

- [x] `anvil hooks install` command
  - [x] Add pre-commit hook: `anvil validate`
  - [x] Add pre-push hook: `anvil gate`
  - [x] Integration guide for existing Husky setup (auto-detection + --husky
        flag)
  - [x] `anvil hooks uninstall` for cleanup
  - [x] `anvil hooks status` to show current hook state
  - [x] ANVIL_SKIP_HOOKS environment variable support in hooks

### Gate Flags (Dev Mode)

- [x] `--skip-checks=<gates>` flag to bypass specific gates (already
      implemented)
- [x] `--only-checks=<gates>` flag to run specific gates (already implemented)
- [x] Gate profiles: `--profile=dev|ci|production`
- [x] `--list-profiles` to show available profiles
- [x] Environment variable support: `ANVIL_SKIP_GATES`

### Configuration File

- [x] `.anvilrc` or `.anvil/config.json` support (priority order)
  - [x] Gate configuration (thresholds, enabled checks)
  - [x] Default format preference
  - [x] Coverage threshold override
  - [x] Secret scanning patterns
- [x] `anvil init` to generate default config (already implemented)
- [x] Config validation on load with detailed error reporting

---

## 🚀 Priority 1: Speed & Caching

Critical for adoption. Developers disable slow tools.

### Validation Caching ✅

- [x] Cache validation results by content hash
- [x] Cache location: `.anvil/cache/`
- [x] Cache invalidation on file change (input hash comparison)
- [x] `--no-cache` flag to bypass
- [x] Cache stats in verbose output

### Parallel Execution ✅

- [x] Run gate checks in parallel
- [x] Configurable parallelism: `--parallel=<n>` (0 = sequential)
- [x] Progress reporting for parallel checks (`--progress` flag)

### JSON Output ✅

- [x] `--output json` flag for CI/CD integration
- [x] Structured JSON schema with version, timing, cache stats

### Watch Mode ✅

- [x] `anvil watch` command
- [x] File system watcher (chokidar)
- [x] Git-aware filtering (unstaged changes only)
- [x] Debounce rapid changes (configurable)
- [x] Configurable via `.anvilrc` watch section
- [x] Real-time output with pass/fail tracking
- [ ] Clear terminal on re-run (optional enhancement)

---

## 🚀 Priority 2: CI/CD Integration

### GitHub Action

- [ ] Create `.github/actions/anvil-gate/action.yml`
- [ ] Install Anvil CLI in action
- [ ] Run gate on changed files
- [ ] Post PR summary comment
- [ ] Set commit status (pass/fail)
- [ ] Block merge on failure
- [ ] Configuration via workflow inputs
- [ ] Documentation and examples

### PR Integration

- [ ] Detect plan files in PR diff
- [ ] Inline comments on validation issues
- [ ] `/anvil override` comment command
- [ ] Re-run on PR update

---

## 🚀 Priority 3: Security Gates

### Dependency Vulnerability Scanning ✅ COMPLETE

- [x] Implement `dependency-check.ts`
  - [x] Integrate `pnpm audit` / `npm audit` / `yarn audit`
  - [x] Parse vulnerability reports (CVE, severity)
  - [x] Configurable severity threshold
  - [x] Fix suggestions with upgrade commands
- [x] Add to gate runner
- [x] CVE links in output

### Enhanced Secret Scanning

- [ ] Entropy-based detection (Shannon entropy)
- [ ] Git history scanning (detect secrets in commits)
- [ ] Configurable allowlist for false positives
- [ ] Remediation suggestions

### SAST Integration (Semgrep)

- [ ] Vendor Semgrep binary
- [ ] Run with `--config=auto`
- [ ] Parse JSON output
- [ ] Map to CWE/OWASP Top 10
- [ ] Severity filtering
- [ ] Custom rule support

---

## 🚀 Priority 4: Architecture Gates

Unique differentiator — no competitor has this.

### Dependency Analysis ✅ COMPLETE

- [x] Integrate `dependency-cruiser`
- [x] Detect circular dependencies
- [x] Validate dependency direction (via `.dependency-cruiser.js` rules)
- [x] Orphaned module detection
- [x] Configurable severity thresholds
- [x] Graceful skip when dependency-cruiser not installed

### Layer Boundary Validation

- [x] Define architecture layers in `.dependency-cruiser.js` config
- [x] Validate no reverse dependencies (via rules)
- [ ] Support clean/hexagonal/onion pattern templates
- [ ] Custom layer rules UI

### Anti-pattern Detection

- [ ] God class detection (>500 LOC, high complexity)
- [ ] Tight coupling detection (high fan-out)
- [ ] AST analysis with `ts-morph`
- [ ] Configurable thresholds
- [ ] Refactoring suggestions

---

## 🚀 Priority 5: IDE Integration

### VS Code Extension

- [ ] Extension scaffold (TypeScript, esbuild)
- [ ] Inline validation status
- [ ] Gate results panel
- [ ] One-click gate run
- [ ] Problem panel integration (diagnostics)
- [ ] Status bar indicator
- [ ] CodeLens for plan files

### MCP Server

- [ ] MCP server implementation
- [ ] Expose validation as tool
- [ ] Expose gate results as resource
- [ ] Real-time validation during AI generation

---

## 🚀 Priority 6: Execution & Rollback

### Safe Apply

- [ ] `anvil apply <plan>` command
- [ ] Snapshot before changes (`.anvil/snapshots/`)
- [ ] Transactional file operations
- [ ] Dry-run mode: `--dry-run`
- [ ] Pre-flight validation
- [ ] Apply evidence generation

### Rollback

- [ ] `anvil rollback <plan-id>` command
- [ ] Load snapshot and restore
- [ ] Partial rollback support
- [ ] Rollback verification
- [ ] Rollback audit trail

---

## 🚀 Priority 7: Visual & Reporting

### Visual Diff Preview

- [ ] `anvil preview --html` command
- [ ] Interactive HTML diff report
- [ ] Side-by-side file comparison
- [ ] Syntax highlighting
- [ ] File tree visualisation

### Blast Radius Analysis

- [ ] Dependency graph of affected files
- [ ] Impact score calculation
- [ ] Interactive graph (D3.js)
- [ ] Risk highlighting

---

## 🚀 Priority 8: Policy Engine ✅ COMPLETE

### OPA Integration

- [x] Auto-download OPA binary (Linux, macOS, Windows)
- [x] Policy bundle structure: `.anvil/policies/`
- [x] Example policies:
  - [x] `coverage_min.rego`
  - [x] `change_scope.rego`
  - [x] `security_baseline.rego`
- [x] Policy testing framework
- [x] PolicyCheck integrated with gate runner

### Policy CLI

- [x] `anvil policy validate` — check Rego syntax
- [x] `anvil policy test` — run policy tests
- [x] `anvil policy list` — show active policies
- [x] `anvil policy init` — initialise policy directory

---

## Deferred (Post-MVP)

### APS Planning Docs Integration (In Progress - See docs/planning/aps-spinout-v0.3.aps.md)

- [ ] Integrate `@anvil/aps` for Markdown planning doc support
- [ ] Add `anvil plan validate` command (wraps `@anvil/aps`)
- [ ] Add `anvil plan load` command (scope-based loading)
- [ ] Add `anvil plan lock/unlock/status` commands (task state management)
- [ ] Rename `.anvil/plans/` to `.anvil/executions/`
- [ ] Update `cli/src/utils/file-io.ts` paths
- [ ] Update `cli/src/commands/plan.ts` output path

### CLI Enhancements

- [ ] `anvil plan <intent>` — generate plan from intent
- [ ] Interactive prompts for missing details
- [ ] `anvil history` — plan audit trail
- [ ] `anvil diff <plan-1> <plan-2>` — compare plans

### Advanced Features

- [ ] Rust/Go worker for performance
- [ ] React dashboard for plan approval
- [ ] Memory layer (RAG + provenance)
- [ ] Evidence injection into source files

### Enterprise

- [ ] Multi-language support (Python, Java, Go)
- [ ] SSO authentication
- [ ] RBAC authorisation
- [ ] Compliance reporting
- [ ] License compliance scanning
- [ ] IaC security scanning (Dockerfile, K8s, Terraform)

---

## ✅ Completed

### Phase 1: Foundations ✅

- [x] Nx monorepo structure
- [x] CI/CD pipeline (GitHub Actions)
- [x] ESLint, Prettier, Husky
- [x] TypeScript strict mode

### Phase 2: APS Core ✅

- [x] Zod schema (v0.1.0)
- [x] SHA-256 deterministic hashing
- [x] Plan ID generation
- [x] Validation with error formatting
- [x] JSON Schema export
- [x] API documentation

### Phase 3: Adapters ✅

- [x] FormatAdapter interface
- [x] Adapter registry with auto-detection
- [x] SpecKit adapter (v1 + v2, 114 tests)
- [x] BMAD adapter (86 tests)
- [x] Generic markdown adapter (32 tests)
- [x] File discovery utility
- [x] Round-trip verification

### Phase 4: CLI ✅

- [x] `anvil validate <plan>`
- [x] `anvil gate <plan>`
- [x] `anvil export <plan> --to <format>`
- [x] Format auto-detection
- [x] Pretty printing (chalk, ora, tables)

### Phase 5: Gate v1 ✅

- [x] ESLint check
- [x] Vitest check
- [x] Coverage check
- [x] Secret scanning (regex patterns)
- [x] Evidence collection

---

## Quality Standards

Every feature must meet:

- [ ] > 90% test coverage
- [ ] Integration tests passing
- [ ] Documentation complete
- [ ] No security vulnerabilities
- [ ] Performance acceptable (<2s cached, <10s cold)

---

## Key Decisions

1. **APS is internal** — users work in their format, we convert
2. **Adapters are the wedge** — meet users where they are
3. **Gate is the trust boundary** — all validation here
4. **Evidence is immutable** — append-only audit trail
5. **Safety first** — rollback is non-negotiable
6. **Speed matters** — slow tools get disabled
7. **Planning docs are mutable, execution is per-task immutable** — The APS
   planning doc (Markdown) is a living document that can be edited at any time.
   Immutability and hashing apply at the **task level** when a task is locked
   for execution. This allows:
   - Ongoing planning while work is in flight
   - Multiple tasks executing in parallel with independent provenance
   - Clear separation between "open for editing" and "locked for execution"
   - One source of truth (the planning doc) that drives execution
   - See `docs/planning/aps-spinout-v0.3.aps.md` for full spec

---

_Last updated: December 2025_
