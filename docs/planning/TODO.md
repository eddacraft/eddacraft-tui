# Anvil Implementation TODO

This document provides a comprehensive task list for implementing Anvil,
following the three-act strategic vision whilst maintaining practical MVP focus.
Tasks are organised by phase and epic with detailed acceptance criteria. For
strategic context, see [PLAN.md](./PLAN.md).

## 🚨 IMMEDIATE NEXT STEPS (Current as of November 9, 2025)

### ✅ Recent Completion: Generic Markdown Adapter + File Discovery

**Status**: ✅ Complete - Full support for non-BMAD/SpecKit planning documents

**Completed (November 9, 2025)**:

- ✅ Generic markdown adapter implementation (198 LOC)
- ✅ File discovery utility for auto-finding planning docs (289 LOC)
- ✅ Support for PRD, TODO, plan, RFC, ADR formats
- ✅ Fallback detection (30-45% confidence)
- ✅ Parser: Markdown → APS (requirements, tasks, features, goals)
- ✅ Serializer: APS → Markdown
- ✅ Comprehensive tests (32 tests, all passing)
- ✅ Test fixtures (4 documents: PRD, TODO, plan, RFC)
- ✅ Auto-registration with priority ordering
- ✅ Total adapter tests: 232 passing ✅

### ✅ Previous Completion: BMAD Comprehensive Testing

**Status**: ✅ Complete - 86 tests exceeding 50+ target

**Completed (November 9, 2025)**:

- ✅ BMAD adapter comprehensive testing (86 tests, target was 50+)
- ✅ Additional test fixtures (6 new: task, minimal PRD, complex PRD, invalid
  formats)
- ✅ Parser tests: 20 (target: 20) ✅
- ✅ Serializer tests: 10 (target: 10) ✅
- ✅ Integration tests: 5 (target: 5) ✅
- ✅ Detection tests: 14 ✅
- ✅ Round-trip tests: 4 ✅
- ✅ Edge case tests: 33 ✅

### ✅ Previous Completion: SpecKit Adapter Migration

**Status**: ✅ Already Complete - Verified November 9, 2025

- ✅ SpecKit adapter already migrated to FormatAdapter interface
- ✅ 45 format adapter tests passing
- ✅ Proper detection, parse, serialize, validate implementation
- ✅ Auto-registered with BMAD and Generic adapters

**Next Steps** (Week 9-10):

- Prepare demo materials for first pilot customers
- Update documentation with adapter examples
- Create customer onboarding guides

### ✅ Week 7 Complete: Polish & Stabilise

**Goal**: Ensure current implementation is production-ready ✅

**Completed**:

- ✅ Fixed vitest configuration - tests run from root (`pnpm test cli/src`)
- ✅ All 36 CLI integration tests passing
- ✅ All 69 adapter tests passing
- ✅ Documentation updated (TODO.md, README.md, cli/README.md)

**Discovered Issues**:

- ⚠️ SpecKit adapters need FormatAdapter migration (see KNOWN_ISSUES.md)
- ⚠️ Hash validation bug in CLI (needs investigation)
- **Decision**: Document limitations, move forward with BMAD adapter

### Current Work: Week 9-10 - Testing & Migration

**Goal**: Complete comprehensive testing and prepare for first pilots

**Next Actions**:

1. ✅ BMAD adapter implementation (COMPLETE)
2. ✅ CLI integration with BMAD (COMPLETE)
3. Add comprehensive unit tests for BMAD adapter (50+ tests)
4. Create additional test fixtures for BMAD documents
5. Migrate SpecKit adapters to FormatAdapter interface (using BMAD as reference)
6. Prepare demo materials for first pilot customers

**After This**: First pilot customers (Week 11), then Apply/Rollback (Weeks
12-15)

---

## Executive Summary

**Current Status**: Phase 2 (APS Core) 100% complete, Phase 2.5 (Adapters) 100%
complete (SpecKit ✅, BMAD ✅), Phase 3 (CLI Integration) 100% complete, Phase 4
(Gate) 100% complete

**Next Critical Path**: SpecKit Migration → Enhanced Testing → First Pilots →
Apply/Rollback

**Target MVP**: 4-6 weeks from current state (November 2025)

### Strategic Priorities (in order)

1. **Testing & Quality** - BMAD unit tests, SpecKit adapter migration
2. **Pilot Readiness** - Documentation, demo materials, customer onboarding
3. **Developer Experience** - CLI commands that work with existing formats ✅
4. **Validation & Safety** - Gate integration with formats ✅
5. **Production Readiness** - Apply/rollback with audit trail (Weeks 11-14)

## Progress Summary

### ✅ Completed Phases (52% overall)

- **Phase 1: Foundations** - Repository structure, CI/CD, quality gates (100%)
- **Phase 2: APS Spine** - Core schema, validation, hash generation, CLI
  integration, documentation (100%)
- **Phase 2.5: Adapters** - Framework complete, SpecKit adapter (100%), BMAD
  adapter (100%), Generic adapter (100%), File discovery (100%)
- **Phase 3: CLI Integration** - Format detection, validate/gate/export commands
  (100%)
- **Phase 4: Gate v1** - ESLint, coverage, secret scanning (100%)
- **Phase 4.5: Comprehensive Testing** - BMAD (86 tests), SpecKit (114 tests),
  Generic (32 tests), total 232 tests (100%)

### ✅ Previous Sprint (Week 5-7) - COMPLETE

**Goal**: Complete CLI integration with SpecKit adapter ✅

- [x] Adapter framework (types, registry, testing utilities) ✅
- [x] SpecKit parser (spec.md, plan.md, tasks.md parsers) ✅
- [x] SpecKit import adapter (v1 and v2) ✅
- [x] SpecKit export adapter ✅
- [x] SpecKit tests (69 tests, all passing) ✅
- [x] CLI format auto-detection ✅
- [x] CLI `gate` command with adapter support ✅
- [x] CLI `validate` command with adapter support ✅
- [x] CLI `export` command for format conversion ✅
- [~] Evidence bundle integration (deferred to post-MVP)

### ✅ Week 8 Sprint - COMPLETE

**Goal**: Implement BMAD adapter with correct FormatAdapter interface ✅

- [x] BMAD format research (Context7, 3001 snippets) ✅
- [x] BMAD adapter specification (BMAD_ADAPTER_SPEC.md) ✅
- [x] Format detection with confidence scoring ✅
- [x] Parser implementation (BMAD → APS) ✅
- [x] Serializer implementation (APS → BMAD) ✅
- [x] Validation implementation ✅
- [x] Registry integration ✅
- [x] CLI integration verification ✅

### ✅ Week 8-9 Sprint - COMPLETE

**Goal**: Complete BMAD testing and prepare for pilot customers ✅

- [x] Add comprehensive unit tests for BMAD adapter (86 tests, exceeded 50+
      target) ✅
- [x] Create test fixtures (6 new BMAD fixtures + 4 generic fixtures) ✅
- [x] Verify SpecKit adapter FormatAdapter migration (already complete) ✅
- [x] Implement Generic Markdown adapter for broad format support ✅
- [x] Add file discovery utility for auto-finding planning docs ✅
- [ ] Update documentation with adapter examples (in progress)
- [ ] Prepare Customer #1 and #2 demo materials

### 🎯 Next 3 Sprints (Weeks 9-11)

**Goal**: First pilots and execution pipeline

- Week 9-10: Complete testing, first pilot customers with validation-only
  workflow
- Weeks 11-12: Dry-run system (preview changes before applying)
- Weeks 13-14: Apply/rollback with snapshots (complete execution pipeline)

### 📋 Remaining Work (Phases 5-12)

- Phase 5: Policy engine (OPA/Rego)
- Phase 6: Sidecar & dry-run
- Phase 7: Apply & rollback
- Phase 8: GitHub integration
- Phases 9-12: Productioniser, hardening, release

---

## Phase 1: Foundations ✅ COMPLETE

### Epic: Infrastructure Setup

#### Repository Structure ✅

- [x] Initialize Nx monorepo structure
- [x] Configure `pnpm-workspace.yaml`
- [x] Ensure folder structure: `core/`, `cli/`, `gate/`, `adapters/`
- **Date Completed**: 2025-09-22
- **Date Committed**: 2025-09-22

#### CI/CD Pipeline ✅

- [x] GitHub Actions workflow for lint + test
- [x] Status badges in README
- [x] Node.js version pinning (>=18.0.0)
- [x] Cache pnpm dependencies
- **Date Completed**: 2025-09-22
- **Date Committed**: 2025-09-22

#### Quality Gates ✅

- [x] ESLint configuration with TypeScript rules
- [x] Prettier formatting rules
- [x] Husky pre-commit hooks
- [x] TypeScript strict mode configuration
- **Date Completed**: 2025-09-22
- **Date Committed**: 2025-09-22

---

## Phase 2: APS Spine ✅ COMPLETE

### Epic: APS Core Implementation

#### Schema Definition ✅

- [x] Create schema directory structure
- [x] Define APS Zod Schema with all required fields
- [x] Export JSON Schema for compatibility
- [x] TypeScript type generation
- **Date Completed**: 2025-09-26
- **Status**: Core schema complete, pending final integration

#### Hash Generation ✅

- [x] Implement canonicalisation utilities
- [x] SHA-256 hash generation
- [x] Hash verification functions
- [x] Plan ID generation (aps-[8 hex])
- **Date Completed**: 2025-09-26
- **Status**: Complete and tested

#### Validation Implementation ✅

- [x] Create validation module structure
- [x] APS Validator class with Zod
- [x] Error formatting for CLI
- [x] Comprehensive test coverage
- **Date Completed**: 2025-09-26
- **Status**: Complete and tested

#### Integration & Deployment ✅

- [x] **Integrate APS with CLI infrastructure**
  - [x] Export all APS utilities from core package
  - [x] Update CLI package.json to use core package
  - [x] Fix TypeScript configuration for proper build output
  - [x] Align Gate types with APS schema (PlanData → APSPlan)
  - [x] Update gate checks to use new schema fields (path instead of target)
  - [x] Update all test fixtures to use APS schema v0.1.0
  - [x] Add integration tests for CLI + APS
  - **Acceptance**: CLI can import and use APS validation ✅
  - **Status**: Complete with comprehensive test coverage
  - **Date Completed**: 2025-10-13

- [x] **Documentation for APS Core**
  - [x] API documentation for all exported functions
  - [x] Usage examples for developers
  - [x] Migration guide from manual JSON
  - **Acceptance**: Developers can use APS without source code inspection ✅
  - **Status**: Complete - API.md, EXAMPLES.md, MIGRATION.md created
  - **Date Completed**: 2025-10-13

---

## Phase 2.5: Adapters (NEW - CRITICAL PATH)

### Epic: Format Interoperability

**Strategic Rationale**: Users won't adopt a new format. We must work with
existing planning formats (SpecKit, BMAD) whilst using APS internally for
validation and execution.

#### Adapter Architecture ✅

- [x] **Create adapter framework** (`adapters/src/base/`)
  - [x] Define `FormatAdapter` interface with detection, parse, serialize,
        validate
  - [x] Implement adapter registry for format detection
  - [x] Add adapter testing utilities
  - [x] Create adapter documentation
  - [x] Implement comprehensive framework tests (22 tests passing)

- **Acceptance**: Framework supports pluggable adapters ✅
- **Dependencies**: APS core complete ✅
- **Date Completed**: 2025-10-13

#### SpecKit Adapter (Customer #1) ✅ COMPLETE

- [x] **Implement SpecKit parser** (`adapters/src/speckit/`)
  - [x] Parse `spec.md` / `plan.md` / `tasks.md` formats
  - [x] Extract intent from spec structure
  - [x] Map SpecKit sections to APS proposed_changes
  - [x] Handle SpecKit metadata (authors, versions, status)
  - [x] Preserve round-trip fidelity
  - [x] Support both v1 (simple) and v2 (official spec-kit) formats
  - [x] Specialised parsers for spec/plan/tasks documents
  - **Acceptance**: Valid SpecKit documents convert to valid APS ✅
  - **Date Completed**: 2025-10-14

- [x] **Implement SpecKit serialiser**
  - [x] Convert APS back to SpecKit format
  - [x] Preserve original formatting where possible
  - [x] Generate spec.md, plan.md, and tasks.md
  - [x] Support metadata injection
  - **Acceptance**: Round-trip conversion preserves intent ✅
  - **Date Completed**: 2025-10-14

- [x] **SpecKit adapter tests**
  - [x] Fixture: Valid SpecKit documents (5+ examples)
  - [x] Fixture: Official GitHub spec-kit format examples
  - [x] Import/export tests (v1 and v2)
  - [x] Parser tests (spec-parser, plan-parser, tasks-parser)
  - [x] Registry tests (22 tests, 100% passing)
  - **Test Results**: 69 total tests, all passing ✅
  - **Acceptance**: >95% test coverage achieved ✅
  - **Date Completed**: 2025-10-21

- [x] **CLI integration for SpecKit** ✅ COMPLETE
  - [x] Auto-detect SpecKit format in CLI
  - [x] `anvil gate spec.md` works end-to-end
  - [x] `anvil validate plan.md` provides feedback
  - [x] `anvil export spec.md --to=aps` format conversion
  - [~] Evidence updates append to SpecKit files (deferred to post-MVP)
  - **Acceptance**: SpecKit users can validate plans ✅
  - **Demo**: Ready for Customer #1
  - **Date Completed**: 2025-10-23

#### BMAD Adapter (Customer #2) ✅ COMPLETE

- [x] **Implement BMAD parser** (`adapters/src/bmad/`) ✅
  - [x] Parse PRD/architecture doc formats ✅
  - [x] Extract requirements and acceptance criteria ✅
  - [x] Map BMAD structure to APS proposed_changes ✅
  - [x] Handle BMAD metadata and versioning ✅
  - [x] Support multiple BMAD document types ✅
  - **Acceptance**: Valid BMAD documents convert to valid APS ✅
  - **Date Completed**: 2025-11-05

- [x] **Implement BMAD serialiser** ✅
  - [x] Convert APS back to BMAD format ✅
  - [x] Preserve document structure ✅
  - [x] Generate YAML front-matter and change log tables ✅
  - [x] Roundtrip fidelity verified ✅
  - **Acceptance**: Round-trip conversion works correctly ✅
  - **Date Completed**: 2025-11-05

- [x] **BMAD adapter comprehensive testing** ✅
  - [x] Basic fixture: Valid BMAD documents (docs/prd.md) ✅
  - [x] Additional fixtures: 6 new valid BMAD documents ✅
  - [x] Fixture: Invalid BMAD documents (edge cases) ✅
  - [x] Detection tests: 14 tests ✅
  - [x] Parser tests: 20 tests (target 20) ✅
  - [x] Serializer tests: 10 tests (target 10) ✅
  - [x] Integration tests: 5 tests (target 5) ✅
  - [x] Round-trip tests: 4 tests ✅
  - [x] Edge case tests: 33 tests ✅
  - **Test Results**: 86 total tests, all passing (exceeded 50+ target) ✅
  - **Acceptance**: >95% test coverage achieved ✅
  - **Date Completed**: 2025-11-09

- [x] **CLI integration for BMAD** ✅
  - [x] Auto-detect BMAD format in CLI (100% confidence) ✅
  - [x] `anvil validate docs/prd.md` works end-to-end ✅
  - [x] `anvil gate docs/prd.md` works end-to-end ✅
  - [x] `anvil export docs/prd.md --to aps` works ✅
  - [x] Roundtrip verification (parse → serialize → parse) ✅
  - **Acceptance**: BMAD users can validate plans ✅
  - **Demo**: Ready for Customer #2 ✅
  - **Date Completed**: 2025-11-05

#### SpecKit Adapter Migration ✅

- [x] **Verify SpecKit FormatAdapter implementation** ✅
  - [x] Confirmed SpecKit already uses FormatAdapter interface ✅
  - [x] Proper detection, parse, serialize, validate methods ✅
  - [x] Auto-registration with BMAD and Generic adapters ✅
  - [x] 45 format adapter tests + 69 parser tests = 114 total ✅
  - **Status**: Migration already complete, no work needed ✅
  - **Date Verified**: 2025-11-09

#### Generic Markdown Adapter ✅ NEW

- [x] **Implement Generic Markdown adapter** ✅
  - [x] Support PRD, TODO, plan, spec, RFC, ADR formats ✅
  - [x] Fallback detection (30-45% confidence) ✅
  - [x] Parser: extract requirements, tasks, features, goals ✅
  - [x] Serializer: APS → generic markdown ✅
  - [x] Handle documents without strict format requirements ✅
  - **Acceptance**: Any markdown planning document works ✅
  - **Date Completed**: 2025-11-09

- [x] **File discovery utility** ✅
  - [x] Search repositories for planning documents ✅
  - [x] Pattern matching (prd, plan, todo, spec, rfc, adr) ✅
  - [x] Confidence scoring for auto-detection ✅
  - [x] Sort by confidence and recency ✅
  - [x] Group files by pattern ✅
  - **Acceptance**: Auto-find planning docs in any repo ✅
  - **Date Completed**: 2025-11-09

- [x] **Generic adapter tests** ✅
  - [x] Fixture: PRD, TODO, plan, RFC documents ✅
  - [x] Detection tests (all document types) ✅
  - [x] Parser tests (requirements, tasks, features) ✅
  - [x] Serializer tests ✅
  - [x] Integration tests ✅
  - [x] Validation tests ✅
  - **Test Results**: 32 tests, all passing ✅
  - **Acceptance**: Generic format fully tested ✅
  - **Date Completed**: 2025-11-09

**Adapter Summary (as of 2025-11-09)**:

- ✅ SpecKit: 114 tests (45 format adapter + 69 parsers)
- ✅ BMAD: 86 tests (comprehensive coverage)
- ✅ Generic: 32 tests (fallback support)
- ✅ **Total: 232 adapter tests passing**

#### Format Detection ✅ COMPLETE

- [x] **Implement format auto-detection** ✅
  - [x] Content-based detection (not just file extension)
  - [x] Confidence scoring for format detection
  - [x] Fallback to APS native format
  - [x] Clear error messages for unknown formats
  - [x] FormatDetectionService with AdapterRegistry integration
  - [x] Support for explicit format override (`--format` flag)
  - [x] Support for SpecKit format ✅
  - [x] Support for BMAD format ✅
  - [x] Support for APS format ✅
  - **Acceptance**: `anvil gate <any-format>` just works ✅
  - **Date Completed**: 2025-10-23

---

## Phase 3: CLI Foundation ✅ COMPLETE

### Epic: CLI Interface

**Status**: All core commands implemented with multi-format adapter support
(Completed Week 6)

#### Core Commands

- [ ] **Implement `anvil plan <intent>`** (DEFERRED)
  - [ ] Accept format flag: `--format=speckit|bmad|aps`
  - [ ] Generate plan in specified format
  - [ ] Save to `.anvil/plans/` directory
  - [ ] Display plan summary
  - [ ] Support interactive mode for missing details
  - **Acceptance**: Users can create plans in their preferred format
  - **Dependencies**: Adapter framework ✅
  - **Target**: Week 9 (post-BMAD adapter)
  - **Status**: Deferred until after BMAD adapter complete

- [x] **Implement `anvil validate <plan>`** ✅
  - [x] Auto-detect plan format
  - [x] Convert to APS for validation
  - [x] Run schema + hash validation
  - [x] Display validation results
  - [x] Support `--format` flag for explicit format
  - [x] Support `--native` flag for APS-only mode
  - **Acceptance**: Validates any supported format ✅
  - **Date Completed**: 2025-10-23

- [x] **Implement `anvil gate <plan>`** ✅
  - [x] Auto-detect plan format
  - [x] Convert to APS if needed
  - [x] Run all configured checks (lint, test, coverage, secrets)
  - [x] Collect evidence
  - [~] Update source file with results (deferred to post-MVP)
  - [x] Display summary table
  - [x] Exit with appropriate code
  - **Acceptance**: Gate works with all supported formats ✅
  - **Date Completed**: 2025-10-23

- [x] **Implement `anvil export <plan>`** ✅
  - [x] Export to different formats: `--to=speckit|aps|json|yaml`
  - [x] Preserve all data during conversion
  - [x] Validate exported format
  - [x] Support `--output` flag for custom paths
  - [x] Support `--from` flag for explicit source format
  - **Acceptance**: Plans can be converted between formats ✅
  - **Date Completed**: 2025-10-23

#### CLI User Experience

- [x] **Pretty printing** ✅
  - [x] Colourised output for validation results (chalk)
  - [x] Table formatting for gate summaries
  - [x] Progress indicators for long operations (ora spinners)
  - [x] Clear error messages with suggestions
  - **Acceptance**: CLI output is professional and helpful ✅
  - **Date Completed**: 2025-10-23

- [ ] **Interactive prompts** (DEFERRED)
  - [ ] Prompt for missing plan details
  - [ ] Confirmation for destructive operations
  - [ ] Format selection when ambiguous
  - **Acceptance**: CLI guides users through workflows
  - **Target**: Post-MVP (Week 12+)
  - **Status**: Nice-to-have, not blocking pilot customers

---

## Phase 4: Gate v1 ✅ COMPLETE

### Epic: Quality Checks

**Status**: All checks implemented, CLI integration complete, tests passing

#### Integration Tasks ✅

- [x] **Connect Gate to CLI commands**
  - [x] Wire up `anvil gate` command to gate runner
  - [x] Support gate configuration file
  - [x] Align gate types with APS schema
  - [x] Update checks to use new schema fields
  - [ ] Add check selection flags: `--checks=lint,test`
  - [ ] Support check exclusion: `--skip=coverage`
  - **Acceptance**: Gate runs via CLI with all checks ✅
  - **Status**: Core integration complete
  - **Date Completed**: 2025-10-10

- [~] **Evidence bundle integration** (DEFERRED TO POST-MVP)
  - [x] Collect evidence during gate execution ✅
  - [x] Store evidence in APS format ✅
  - [ ] Append evidence to source files (SpecKit/BMAD)
  - [ ] Evidence injection via `--inject` flag
  - **Acceptance**: Evidence properly attached to plans
  - **Target**: Post-MVP (Week 12+)
  - **Status**: Core evidence collection complete; injection deferred as not
    blocking pilot customers
  - **Rationale**: Validation-only workflow sufficient for initial pilots

- [ ] **Gate configuration**
  - [ ] Support `.anvilrc` configuration file
  - [ ] Check-specific configuration (coverage thresholds, etc.)
  - [ ] Per-project policy overrides
  - [ ] Configuration validation
  - **Acceptance**: Users can configure gate behaviour
  - **Target**:

---

## Phase 5: OPA/Rego Integration

### Epic: Policy Engine

#### OPA Foundation

- [ ] **Vendor OPA binary**
  - [ ] Download OPA for Linux, macOS, Windows
  - [ ] Version pinning (latest stable)
  - [ ] Checksum verification
  - [ ] Binary execution wrapper
  - **Acceptance**: OPA available on all platforms
  - **Target**:

- [ ] **Policy bundle structure**
  - [ ] Define policy directory structure: `.anvil/policies/`
  - [ ] Create example policies:
    - `coverage_min.rego` - Enforce minimum coverage
    - `client_side_flags.rego` - Flag risk policies
    - `change_scope.rego` - Limit change scope
  - [ ] Policy versioning strategy
  - [ ] Policy testing framework
  - **Acceptance**: Policies can be defined and versioned
  - **Target**:

#### Policy Integration

- [ ] **Policy evaluation in Gate**
  - [ ] Call OPA with plan data
  - [ ] Collect policy violations
  - [ ] Format policy results as evidence
  - [ ] Support policy warnings vs. failures
  - **Acceptance**: Policies enforced during gate execution
  - **Dependencies**: OPA binary, Gate v1
  - **Target**:

- [ ] **Policy CLI commands**
  - [ ] `anvil policy validate` - Check policy syntax
  - [ ] `anvil policy test` - Run policy tests
  - [ ] `anvil policy list` - Show active policies
  - **Acceptance**: Users can manage policies via CLI
  - **Target**:

---

## Phase 6: Sidecar Development

### Epic: Execution Runtime

**Strategic Note**: The sidecar is where plans become changes. This is the trust
boundary.

#### Dry-run System

- [ ] **Implement dry-run** (`sidecar/src/dry-run/`)
  - [ ] Parse proposed_changes from APS
  - [ ] Generate file diffs without applying
  - [ ] Collect logs and evidence
  - [ ] Create preview bundle
  - [ ] Support rollback preview
  - **Acceptance**: `anvil dry-run plan.json` shows what would happen
  - **Target**:
  - **Demo**: This is the "wow moment"

- [ ] **Dry-run CLI command**
  - [ ] `anvil dry-run <plan>` command
  - [ ] Display diffs with syntax highlighting
  - [ ] Show impact summary (files changed, LOC, etc.)
  - [ ] Support `--output` for saving preview
  - **Acceptance**: Users can preview changes safely
  - **Dependencies**: Dry-run system
  - **Target**:

#### Sidecar Daemon

- [ ] **Daemon process** (`sidecar/src/daemon/`)
  - [ ] Background process management
  - [ ] Job queue for apply operations
  - [ ] Status monitoring
  - [ ] Graceful shutdown
  - **Acceptance**: Sidecar runs as background service
  - **Target**

- [ ] **Evidence collection**
  - [ ] Immutable evidence appending
  - [ ] Structured evidence format
  - [ ] Evidence verification
  - [ ] Audit trail generation
  - **Acceptance**: All operations produce evidence
  - **Target**:

---

## Phase 7: Apply & Rollback

### Epic: Transactional Execution

**Critical**: This is where Anvil's core value proposition is delivered - safe,
auditable, reversible changes.

#### Apply System

- [ ] **Implement idempotent apply** (`sidecar/src/apply/`)
  - [ ] Parse proposed_changes from APS
  - [ ] Apply changes transactionally
  - [ ] Create snapshots before applying
  - [ ] Record all applied changes
  - [ ] Generate apply evidence
  - [ ] Support partial application with clear errors
  - **Acceptance**: Changes apply successfully with audit trail
  - **Target**:

- [ ] **Apply CLI command**
  - [ ] `anvil apply <plan>` command
  - [ ] Require gate pass before applying
  - [ ] Require approval flag: `--approved`
  - [ ] Display apply progress
  - [ ] Show summary of applied changes
  - **Acceptance**: Users can apply validated plans safely
  - **Dependencies**: Apply system, Gate integration
  - **Target**:

#### Rollback System

- [ ] **Implement rollback** (`sidecar/src/rollback/`)
  - [ ] Load snapshot from apply
  - [ ] Reverse applied changes
  - [ ] Verify rollback integrity
  - [ ] Generate rollback evidence
  - [ ] Support partial rollback
  - **Acceptance**: Changes can be rolled back to previous state
  - **Target**:

- [ ] **Rollback CLI command**
  - [ ] `anvil rollback <plan-id>` command
  - [ ] Display what will be rolled back
  - [ ] Require confirmation
  - [ ] Show rollback progress
  - [ ] Verify system state after rollback
  - **Acceptance**: Users can safely undo applied changes
  - **Dependencies**: Rollback system
  - **Target**:

#### Safety Guards

- [ ] **Apply guards**
  - [ ] Verify gate passed before apply
  - [ ] Check approval status
  - [ ] Validate plan hasn't been modified
  - [ ] Prevent concurrent applies
  - [ ] Timeout protection
  - **Acceptance**: Apply operations are safe by default
  - **Target**:

---

## Phase 8: GitHub Integration

### Epic: CI/CD Integration

**Goal**: Make Anvil a natural part of the development workflow

#### GitHub Action

- [ ] **Create GitHub Action** (`.github/actions/anvil-gate/`)
  - [ ] Action definition (action.yml)
  - [ ] Install Anvil CLI
  - [ ] Run gate on changed files
  - [ ] Post results as PR comment
  - [ ] Set status check (pass/fail)
  - [ ] Support configuration via workflow inputs
  - **Acceptance**: Action can be used in any repository
  - **Target**:

- [ ] **PR Integration**
  - [ ] Detect SpecKit/BMAD files in PR
  - [ ] Run gate automatically
  - [ ] Block merge on gate failure
  - [ ] Clear merge on gate pass
  - [ ] Support override via comment: `/anvil override`
  - **Acceptance**: PRs are automatically validated
  - **Target**:

- [ ] **Status checks**
  - [ ] Report individual check results
  - [ ] Provide links to detailed evidence
  - [ ] Show validation summary
  - [ ] Support required vs. optional checks
  - **Acceptance**: PR status clearly shows validation state
  - **Target**:

#### Documentation & Examples

- [ ] **GitHub Action documentation**
  - [ ] Setup guide for repositories
  - [ ] Configuration examples
  - [ ] Troubleshooting guide
  - [ ] Best practices
  - **Acceptance**: Teams can integrate Anvil easily
  - **Target**:

---

## Phase 9: Feature Flags Pack (DEFERRED)

**Note**: This is deferred post-MVP. Including spec for completeness.

### Epic: Feature Flag Management

- [ ] Feature flag library implementation
- [ ] CLI commands for flag management
- [ ] OpenFeature provider
- [ ] FeatureBoard adapter preparation
- [ ] Test generation
- [ ] Documentation

**Target**: Post-MVP ()

---

## Phase 10: Productioniser (MINIMAL MVP VERSION)

### Epic: Repository Governance

**MVP Scope**: Basic scanning with safe recommendations only. Full heuristics
engine deferred.

#### Minimal Scanner

- [ ] **Basic repository scanner** (`productioniser/src/scanner/`)
  - [ ] Test coverage check
  - [ ] Documentation presence check
  - [ ] Lint configuration check
  - [ ] Basic security scan (secrets, known vulnerabilities)
  - [ ] Simple scoring system
  - **Acceptance**: Scanner identifies obvious gaps
  - **Target**:

- [ ] **Safe recommendations**
  - [ ] Suggest adding tests where missing
  - [ ] Suggest adding README if absent
  - [ ] Suggest lint setup if missing
  - [ ] Flag potential security issues
  - [ ] No automatic fixes, only suggestions
  - **Acceptance**: Recommendations are safe and valuable
  - **Target**:

#### Productioniser Command

- [ ] **Implement `anvil productionise`**
  - [ ] Scan repository
  - [ ] Generate report
  - [ ] Optionally create remediation plan
  - [ ] Support `--fix` flag for safe auto-fixes only
  - **Acceptance**: Command outputs useful assessment
  - **Target**:

---

## Phase 11: Hardening & Documentation

### Epic: Production Readiness

#### Performance Optimisation

- [ ] **Gate performance**
  - [ ] Parallel check execution
  - [ ] Caching for repeated checks
  - [ ] Incremental validation
  - [ ] Memory optimisation
  - **Acceptance**: Gate runs efficiently on large repositories
  - **Target**:

- [ ] **CLI responsiveness**
  - [ ] Fast startup time
  - [ ] Streaming output for long operations
  - [ ] Interrupt handling
  - **Acceptance**: CLI feels fast and responsive
  - **Target**:

#### Security Hardening

- [ ] **Input validation**
  - [ ] Sanitise all user inputs
  - [ ] Validate file paths
  - [ ] Prevent path traversal
  - [ ] Rate limiting for operations
  - **Acceptance**: CLI is secure against common attacks
  - **Target**:

- [ ] **Secrets handling**
  - [ ] Never log sensitive data
  - [ ] Secure evidence storage
  - [ ] Audit trail encryption (optional)
  - **Acceptance**: No secrets leaked in logs or evidence
  - **Target**:

#### Documentation

- [ ] **Developer documentation** (`docs/`)
  - [ ] Getting started guide
  - [ ] Architecture overview
  - [ ] Adapter development guide
  - [ ] API reference
  - [ ] Troubleshooting guide
  - **Acceptance**: Developers can contribute effectively
  - **Target**:

- [ ] **User documentation**
  - [ ] Installation guide
  - [ ] CLI reference
  - [ ] Configuration guide
  - [ ] Best practices
  - [ ] Examples and tutorials
  - **Acceptance**: Users can use Anvil without support
  - **Target**:

- [ ] **Policy cookbook**
  - [ ] Common policy examples
  - [ ] Policy writing guide
  - [ ] Policy testing guide
  - **Acceptance**: Users can write effective policies
  - **Target**:

---

## Phase 12: Release Candidate

### Epic: Release Preparation

#### Release Engineering

- [ ] **Version management**
  - [ ] Semantic versioning setup
  - [ ] Changelog generation
  - [ ] Version bumping automation
  - [ ] Git tagging strategy
  - **Acceptance**: Versions managed consistently
  - **Target**:

- [ ] **Artifact signing**
  - [ ] Package signing setup
  - [ ] Checksum generation
  - [ ] Provenance documentation
  - [ ] SBOM generation
  - **Acceptance**: Artifacts are signed and verifiable
  - **Target**:

#### Release Testing

- [ ] **End-to-end validation**
  - [ ] Complete workflow testing
  - [ ] Performance benchmarking
  - [ ] Security validation
  - [ ] Cross-platform testing (Linux, macOS, Windows)
  - **Acceptance**: Release candidate is production-ready
  - **Target**:

- [ ] **Sample walkthrough**
  - [ ] Video demonstration
  - [ ] Written tutorial
  - [ ] Example repository
  - **Acceptance**: New users have clear onboarding
  - **Target**:

#### Release Documentation

- [ ] **Day-0 runbook**
  - [ ] Initial deployment guide
  - [ ] Configuration recommendations
  - [ ] Common issues and solutions
  - **Acceptance**: Teams can deploy Anvil quickly
  - **Target**:

- [ ] **Release notes**
  - [ ] Feature summary
  - [ ] Breaking changes
  - [ ] Migration guide
  - [ ] Known issues
  - **Acceptance**: Release notes are clear and complete
  - **Target**:

---

## Post-MVP: Future Phases

### Deferred Features (Post-Week 24)

**These are explicitly out of scope for MVP but documented for future
planning:**

#### Advanced Features

- [ ] Rust/Go worker for performance
- [ ] React dashboard for plan approval
- [ ] Additional packs (telemetry, observability, infrastructure)
- [ ] Full productioniser with heuristics engine
- [ ] Memory layer (RAG + provenance store)
- [ ] MCP façade for agentic interoperability

#### Enterprise Features

- [ ] Multi-language support (Python, Java, Go)
- [ ] SSO authentication
- [ ] RBAC authorisation
- [ ] Advanced audit logging
- [ ] Compliance reporting
- [ ] Packs marketplace

#### Act 2 & Act 3 Expansion

- [ ] Document validation adapters (Word, Confluence, Notion)
- [ ] Analysis validation adapters (Excel, Jupyter, Tableau)
- [ ] Horizontal platform expansion (consultants, analysts, legal)

---

## Success Metrics

### MVP Success Criteria

We've achieved MVP when:

1. 🚧 **Interoperability**: SpecKit and BMAD users can validate plans without
   changing formats (SpecKit ✅, BMAD in progress)
2. 🚧 **Validation**: Gate enforces quality standards (lint, test, coverage,
   secrets, policies) (Gate v1 ✅, CLI integration in progress)
3. ⏳ **Safety**: Apply and rollback work reliably with full audit trails
4. ⏳ **Integration**: GitHub Action blocks PRs that fail validation
5. ⏳ **Adoption**: 15-20 teams using Anvil in production

### Quality Gates for Each Phase

Each phase must meet:

- [ ] > 90% test coverage for new code
- [ ] All integration tests passing
- [ ] Documentation complete and reviewed
- [ ] Security review completed
- [ ] Performance benchmarks met

---

## Sprint Planning Template

### Current Sprint: (October 2025)

**Goal**: Complete CLI integration with SpecKit adapter

**Tasks**:

- [x] Complete SpecKit adapter (parser, import, export) ✅
- [ ] Implement format auto-detection in CLI
- [ ] Implement `anvil validate` with adapter support
- [ ] Implement `anvil gate` with adapter support
- [ ] Implement `anvil export` with format conversion
- [ ] Evidence bundle integration with SpecKit

**Blockers**:

- None

**Demo**:

- Show `anvil validate spec.md` working end-to-end
- Show `anvil gate plan.md` with SpecKit format
- Show `anvil export spec.md --to=aps` format conversion

### Recent Progress (October 14-21, 2025)

**Completed**:

- SpecKit adapter framework (586 LOC, 22 tests passing)
- SpecKit parser (2,469 LOC, 51 tests)
- V1 and V2 format support
- Import and export adapters
- Comprehensive test coverage
- Documentation cleanup (extracted templates to separate files)
- Fixed 2 spec-parser test failures (metadata and user story parsing)
- Fixed import test loader errors (built @anvil/core)

**In Progress (October 21, 2025)**:

- **CURRENT**: Fixing 3 remaining export adapter test failures (48/51 tests
  passing)
  - Test 1: "should handle APS with execution history" - expects "Error: Build
    failed" in output
  - Test 2: "should validate specs before export" - expects `result.errors` to
    be defined
  - Test 3: "should warn about empty changes" - expects `result.valid` to be
    true
  - **Files**: `packages/adapters/src/__tests__/speckit-export.test.ts` lines
    165-220
  - **Issue**: `validateSpec` method in `export.ts:20` not matching test
    expectations
  - **Next**: Read validateSpec implementation and fix to match test
    expectations

**Blocked Until Tests Pass**:

- CLI integration with adapter framework
- Format auto-detection
- Command implementation (validate, gate, export)

---

## Notes

### MVP Philosophy

**Ship Fast, Ship Value:**

- Focus on interoperability (adapters) before fancy features
- Validation and safety before AI assistance
- Working software before perfect software

**Defer Strategically:**

- Advanced features (packs, memory, MCP) come after validation works
- Enterprise features come after product-market fit
- Act 2/3 expansion comes after Act 1 success

### Key Architectural Decisions

1. **APS is internal** - Users never see it unless they want to
2. **Adapters are the wedge** - Work with existing formats
3. **Gate is the trust boundary** - All validation happens here
4. **Evidence is immutable** - Full audit trail always
5. **Safety first** - Rollback capability is non-negotiable

---

## Version History

- **2025-10-18**: Updated current state - SpecKit adapter complete (51 tests, 49
  passing), CLI integration in progress (Week 5-6 sprint), BMAD deferred to
  weeks 7-8
- **2025-09-30**: Major revision for interoperability strategy, updated
  progress, aligned with three-act vision
- **2025-09-26**: Initial comprehensive TODO with phase breakdown
- **2025-09-22**: Repository foundations established
