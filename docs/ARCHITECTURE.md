# Anvil Architecture

**Version**: 2.0.0 **Last Updated**: 30 September 2025  
**Status**: Living Document

---

## Table of Contents

1. [Overview](#overview)
2. [Core Principles](#core-principles)
3. [System Architecture](#system-architecture)
4. [Component Deep Dive](#component-deep-dive)
5. [Data Flow](#data-flow)
6. [Technology Stack](#technology-stack)
7. [Security Model](#security-model)
8. [Performance Considerations](#performance-considerations)
9. [Extension Points](#extension-points)
10. [Future Considerations](#future-considerations)

---

## Overview

Anvil is a deterministic development automation platform that transforms
AI/human intent into validated, auditable, and reversible changes. The
architecture is designed around a core insight: **making all changes flow
through a deterministic plan specification (APS) enables validation, governance,
and safety**.

### Key Architectural Decisions

1. **APS as Internal Format** - Universal interchange format for validation and
   execution
2. **Adapter Pattern for Interoperability** - Support existing planning formats
   (SpecKit, BMAD, etc.)
3. **Immutable Evidence Trail** - All validations and executions append
   evidence, never modify
4. **Separation of Concerns** - Parse → Validate → Execute are distinct,
   composable stages
5. **Safety by Default** - All operations are reversible; rollback is a
   first-class concern

---

## Core Principles

### 1. Determinism

**Principle**: Same input → same output, always.

**Implementation**:

- Hash-stable canonicalisation of plans
- Deterministic validation (no race conditions)
- Reproducible execution environments
- Immutable evidence (append-only)

**Why It Matters**: Determinism enables trust. Teams can review a plan once and
know it won't change unexpectedly.

### 2. Interoperability

**Principle**: Work with existing formats, don't force adoption of new ones.

**Implementation**:

- Pluggable adapter system
- Format auto-detection
- Round-trip conversion (parse → APS → serialize)
- Preserve original formatting and structure

**Why It Matters**: Adoption is the biggest risk. By supporting existing
formats, we eliminate that barrier.

### 3. Safety

**Principle**: All changes are validated before application and reversible
after.

**Implementation**:

- Mandatory gate validation before apply
- Snapshot creation before any change
- Rollback as first-class operation
- Audit trail for all operations

**Why It Matters**: Production incidents are expensive. Anvil makes changes safe
by default.

### 4. Transparency

**Principle**: All operations produce auditable evidence.

**Implementation**:

- Immutable evidence bundles
- Structured evidence format (JSON/YAML)
- Provenance tracking (who, what, when, why)
- Evidence signatures (future: cryptographic)

**Why It Matters**: Compliance and debugging require complete audit trails.

### 5. Composability

**Principle**: Components are independent and can be composed flexibly.

**Implementation**:

- Adapter → APS → Validator → Gate → Executor pipeline
- Each component has clear interfaces
- Components usable independently (library mode)
- CLI orchestrates but doesn't couple components

**Why It Matters**: Future extensibility requires loose coupling.

---

## System Architecture

### High-Level Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                     External World                           │
│  • User Formats (SpecKit, BMAD, etc.)                       │
│  • VCS (Git)                                                 │
│  • CI/CD (GitHub Actions, GitLab CI)                        │
│  • IDEs (VS Code, Cursor)                                   │
└────────────────────────┬────────────────────────────────────┘
                         │
                         │ Files, Commands
                         ↓
┌─────────────────────────────────────────────────────────────┐
│                     CLI Layer (Commander.js)                 │
│  • Command parsing and routing                               │
│  • User interaction and prompts                              │
│  • Output formatting and display                             │
│  • Error handling and recovery                               │
└────────────────────────┬────────────────────────────────────┘
                         │
                         │ Commands
                         ↓
┌─────────────────────────────────────────────────────────────┐
│                   Adapter Layer                              │
│                                                              │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐     │
│  │   SpecKit    │  │     BMAD     │  │   Native     │     │
│  │   Adapter    │  │   Adapter    │  │     APS      │     │
│  └──────────────┘  └──────────────┘  └──────────────┘     │
│                                                              │
│  • Format detection                                          │
│  • Parse → APS conversion                                    │
│  • APS → Format serialization                                │
│  • Round-trip fidelity                                       │
└────────────────────────┬────────────────────────────────────┘
                         │
                         │ APS (Internal Format)
                         ↓
┌─────────────────────────────────────────────────────────────┐
│                   Core Layer (APS)                           │
│                                                              │
│  ┌────────────────────────────────────────────────┐         │
│  │  APS Schema (Zod)                              │         │
│  │  • TypeScript types                            │         │
│  │  • Runtime validation                          │         │
│  │  • JSON Schema export                          │         │
│  └────────────────────────────────────────────────┘         │
│                                                              │
│  ┌────────────────────────────────────────────────┐         │
│  │  Hash & Canonicalization                       │         │
│  │  • Stable serialization                        │         │
│  │  • SHA-256 hashing                             │         │
│  │  • Hash verification                           │         │
│  └────────────────────────────────────────────────┘         │
│                                                              │
│  ┌────────────────────────────────────────────────┐         │
│  │  Validation Engine                             │         │
│  │  • Schema validation                           │         │
│  │  • Hash validation                             │         │
│  │  • Business rule validation                    │         │
│  └────────────────────────────────────────────────┘         │
└────────────────────────┬────────────────────────────────────┘
                         │
                         │ Validated APS
                         ↓
┌─────────────────────────────────────────────────────────────┐
│                   Gate Layer                                 │
│                                                              │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐   │
│  │  Lint    │  │  Test    │  │ Coverage │  │ Secrets  │   │
│  │  Check   │  │  Check   │  │  Check   │  │  Check   │   │
│  └──────────┘  └──────────┘  └──────────┘  └──────────┘   │
│                                                              │
│  ┌──────────────────────────────────────────┐               │
│  │  Policy Engine (OPA)                     │               │
│  │  • Rego policy evaluation                │               │
│  │  • Custom rules                          │               │
│  │  • Policy versioning                     │               │
│  └──────────────────────────────────────────┘               │
│                                                              │
│  ┌──────────────────────────────────────────┐               │
│  │  Evidence Collector                      │               │
│  │  • Aggregate check results               │               │
│  │  • Format evidence bundle                │               │
│  │  • Append to plan (immutable)            │               │
│  └──────────────────────────────────────────┘               │
└────────────────────────┬────────────────────────────────────┘
                         │
                         │ Validated Plan + Evidence
                         ↓
┌─────────────────────────────────────────────────────────────┐
│                   Sidecar Layer                              │
│                                                              │
│  ┌──────────────────────────────────────────┐               │
│  │  Dry-Run Engine                          │               │
│  │  • Preview changes                       │               │
│  │  • Generate diffs                        │               │
│  │  • Collect logs                          │               │
│  │  • No side effects                       │               │
│  └──────────────────────────────────────────┘               │
│                                                              │
│  ┌──────────────────────────────────────────┐               │
│  │  Apply Engine                            │               │
│  │  • Snapshot creation                     │               │
│  │  • Transactional execution               │               │
│  │  • Audit trail generation                │               │
│  │  • Idempotent operations                 │               │
│  └──────────────────────────────────────────┘               │
│                                                              │
│  ┌──────────────────────────────────────────┐               │
│  │  Rollback Engine                         │               │
│  │  • Snapshot restoration                  │               │
│  │  • Change reversal                       │               │
│  │  • Integrity verification                │               │
│  │  • Rollback evidence                     │               │
│  └──────────────────────────────────────────┘               │
└────────────────────────┬────────────────────────────────────┘
                         │
                         │ Execution Results
                         ↓
┌─────────────────────────────────────────────────────────────┐
│                   Storage Layer                              │
│                                                              │
│  • Plans (.anvil/plans/)                                     │
│  • Evidence bundles (.anvil/evidence/)                       │
│  • Snapshots (.anvil/snapshots/)                             │
│  • Audit logs (.anvil/audit/)                                │
│  • Configuration (.anvilrc, .anvil/config.json)              │
└─────────────────────────────────────────────────────────────┘
```

---

## Component Deep Dive

### 1. Adapter Layer

**Purpose**: Bridge between user formats and internal APS representation.

#### Adapter Interface

```typescript
interface FormatAdapter {
  // Metadata
  name: string;
  version: string;
  supportedExtensions: string[];

  // Detection
  detect(content: string, filename?: string): DetectionResult;

  // Parsing (User Format → APS)
  parse(content: string, options?: ParseOptions): Promise<APSPlan>;

  // Serialization (APS → User Format)
  serialize(plan: APSPlan, options?: SerializeOptions): Promise<string>;

  // Validation
  validate(content: string): Promise<ValidationResult>;

  // Evidence injection
  injectEvidence(content: string, evidence: Evidence): Promise<string>;
}

interface DetectionResult {
  detected: boolean;
  confidence: number; // 0-1
  format: string;
  version?: string;
}
```

#### SpecKit Adapter ✅ COMPLETE

**Responsibility**: Parse and serialize GitHub's official SpecKit format
(`spec.md`, `plan.md`, `tasks.md`).

**Status**: Fully implemented (October 2025)

- **Code Size**: 2,469 lines of code
- **Tests**: 51 tests (49 passing, 2 minor fixes pending)
- **Coverage**: >95%

**SpecKit Format Support**:

The adapter supports GitHub's complete spec-kit workflow with three document
types:

1. **spec.md** - Requirements and user scenarios (WHAT and WHY)
   - Feature metadata (branch, date, status)
   - User scenarios with acceptance criteria
   - Functional requirements with clarification markers
   - Key entities (data model)
   - Success criteria

2. **plan.md** - Technical implementation (HOW)
   - Summary of technical approach
   - Technical context (language, dependencies, constraints)
   - Constitution check (compliance with project principles)
   - Project structure
   - Implementation details

3. **tasks.md** - Executable breakdown
   - Tasks organized by phases with IDs
   - Parallel execution markers
   - Checkpoints after each phase
   - Dependencies and execution order

**Mapping to APS**:

- User Scenarios → `proposed_changes[]` with scenario metadata
- Functional Requirements → `metadata.requirements.functional[]`
- Key Entities → `metadata.requirements.entities[]`
- Success Criteria → `metadata.successCriteria`
- Clarifications → `metadata.clarifications[]` (all `[NEEDS CLARIFICATION]`
  markers)
- Technical Context → `metadata.technicalContext`
- Project Structure → `metadata.projectStructure`
- Implementation Details → `metadata.implementationDetails`
- Phases & Tasks → `metadata.phases[]`, `metadata.taskDependencies[]`

**Round-trip Preservation**:

- Preserve markdown formatting
- Maintain comment structure
- Keep custom sections
- Inject evidence as markdown comments
- Support for v1 (simple) and v2 (official) formats

**Implementation Structure**: `adapters/src/speckit/`

```
speckit/
├── index.ts              # Exports
├── parser.ts             # Core markdown parser (330 LOC)
├── import.ts             # V1 import adapter (284 LOC)
├── import-v2.ts          # V2 official format adapter (424 LOC)
├── export.ts             # Export adapter (462 LOC)
└── parsers/              # Specialized parsers (966 LOC)
    ├── spec-parser.ts    # Spec.md parser (378 LOC)
    ├── plan-parser.ts    # Plan.md parser (342 LOC)
    └── tasks-parser.ts   # Tasks.md parser (246 LOC)
```

#### BMAD Adapter ⏳ PLANNED

**Responsibility**: Parse and serialize BMAD format (Business Model and
Architecture Documents, PRDs).

**Status**: Not yet implemented (Planned for Week 5-6, November 2025)

**Expected BMAD Format Structure**:

```markdown
# Product Requirements Document

## Problem Statement

[Description of the problem being solved]

## Requirements

### Functional

- REQ-001: [Requirement description]
- REQ-002: [Requirement description]

### Non-Functional

- PERF-001: [Performance requirement]
- SEC-001: [Security requirement]

## Architecture

[Technical approach and design decisions]

## Acceptance Criteria

[Testing and validation criteria]
```

**Planned Mapping to APS**:

- `Problem Statement` → `intent`
- Functional Requirements (REQ-XXX) → `proposed_changes[]`
- Non-Functional Requirements (PERF-XXX, SEC-XXX) → `metadata.nfr[]`
- `Architecture` → `metadata.architecture`
- `Acceptance Criteria` → `metadata.acceptance_criteria[]`
- Requirement IDs → `metadata.requirement_ids[]`

**Planned Implementation Location**: `adapters/src/bmad/`

```
bmad/
├── index.ts              # Exports
├── parser.ts             # Markdown parser with REQ-ID extraction
├── import.ts             # BMAD → APS adapter
└── export.ts             # APS → BMAD adapter
```

**Target Completion**: Week 5-6

#### Native APS Adapter

**Responsibility**: Handle native APS JSON/YAML format.

**Features**:

- Direct parsing (no transformation)
- Schema validation
- Pretty printing
- YAML ↔ JSON conversion

**Implementation Location**: Native APS handling is built into core
(`@anvil/core`) and base adapter framework (`adapters/src/base/`)

### 2. Core Layer (APS)

**Purpose**: Define and validate the Anvil Plan Specification.

#### APS Schema (Zod)

**Schema Definition**: `core/src/schema/aps.schema.ts`

**Key Fields**:

```typescript
interface APSPlan {
  // Identity
  schema_version: '1.0.0';
  id: string; // aps-[8 hex chars]
  hash: string; // SHA-256 of canonical form

  // Core content
  intent: string; // What we're trying to achieve
  proposed_changes: Change[];

  // Context
  provenance: Provenance; // Who, when, how
  context: Context; // Repository, branch, etc.

  // Validation
  validation: Validation; // Required checks, policies
  evidence: Evidence[]; // Gate results (append-only)

  // Lifecycle
  approval: Approval; // Approval status
  executions: Execution[]; // Apply/rollback history
}
```

**Schema Validation**:

- Zod for runtime validation
- JSON Schema for interoperability
- TypeScript types for development

#### Hash & Canonicalization

**Purpose**: Generate deterministic hashes for plans.

**Algorithm**:

1. **Canonicalize**: Stable JSON serialization
   - Sorted keys
   - No whitespace variations
   - Consistent encoding
   - Excludes `hash` and `evidence` fields

2. **Hash**: SHA-256 of canonical form

3. **Verify**: Compare computed hash with stored hash

**Implementation**: `core/src/hash/`

**Critical Property**: Same plan content → same hash, always.

#### Validation Engine

**Purpose**: Validate plans against schema and business rules.

**Validation Levels**:

1. **Schema Validation** (Zod)
   - Type checking
   - Required fields
   - Format validation (regex patterns)
   - Enum validation

2. **Hash Validation**
   - Recompute hash
   - Compare with stored hash
   - Detect tampering

3. **Business Rule Validation**
   - Intent length (10-500 chars)
   - Required fields present
   - Valid provenance
   - Reasonable proposed changes

**Error Handling**:

- Structured error objects
- User-friendly messages
- Actionable suggestions
- CLI-formatted output

**Implementation**: `core/src/validation/`

### 3. Gate Layer

**Purpose**: Enforce quality standards before changes are applied.

#### Architecture

```typescript
interface GateRunner {
  run(plan: APSPlan, config: GateConfig): Promise<GateResult>;
}

interface GateConfig {
  checks: CheckConfig[];
  policies: string[]; // OPA policy files
  failFast: boolean; // Stop on first failure
  parallel: boolean; // Run checks in parallel
}

interface GateResult {
  overall_status: 'passed' | 'failed' | 'partial';
  checks: CheckResult[];
  evidence: Evidence;
  summary: string;
}
```

#### Check Types

**1. Lint Check (ESLint)**

- Run ESLint on proposed changes
- Configurable rules
- Support for custom configs
- Output formatted errors

**2. Test Check (Vitest)**

- Run test suite
- Require all tests pass
- Support for test patterns
- Coverage integration

**3. Coverage Check**

- Measure test coverage
- Enforce minimum threshold (default: 80%)
- Per-file coverage analysis
- Delta coverage (changed files only)

**4. Secret Scanning**

- Regex-based detection
- Common secret patterns (API keys, tokens, passwords)
- Future: Entropy-based detection
- Configurable patterns

**5. Policy Check (OPA)**

- Rego policy evaluation
- Custom business rules
- Example policies:
  - Coverage minimum
  - Change scope limits
  - Risk classification

#### Evidence Collection

**Purpose**: Aggregate all check results into immutable evidence bundle.

**Evidence Structure**:

```typescript
interface Evidence {
  gate_version: string;
  timestamp: string;
  overall_status: 'passed' | 'failed' | 'partial';
  checks: CheckResult[];
  summary: string;
  artifacts: Record<string, string>; // Links to detailed reports
}
```

**Properties**:

- **Immutable**: Never modified, only appended
- **Comprehensive**: All check results included
- **Timestamped**: When validation occurred
- **Versioned**: Which gate version ran

**Storage**: Appended to `evidence[]` array in APS plan.

**Implementation**: `gate/src/`

### 4. Sidecar Layer

**Purpose**: Execute validated plans safely with rollback capability.

#### Dry-Run Engine

**Responsibility**: Preview changes without applying them.

**Process**:

1. Parse `proposed_changes` from APS
2. Generate diffs for each change
3. Collect execution logs (simulated)
4. Create preview bundle
5. Return preview to user

**No side effects**: Dry-run never modifies files or state.

**Implementation**: `sidecar/src/dry-run/`

#### Apply Engine

**Responsibility**: Apply validated plans transactionally.

**Process**:

1. **Pre-flight checks**:
   - Verify gate passed
   - Check approval status
   - Validate plan hash

2. **Snapshot creation**:
   - Capture current state
   - Store in `.anvil/snapshots/`
   - Include timestamp and plan ID

3. **Transactional application**:
   - Apply changes sequentially
   - Collect execution evidence
   - Rollback on failure (optional)

4. **Post-execution**:
   - Generate audit trail
   - Update plan with execution record
   - Store evidence

**Idempotency**: Re-applying same plan should have no additional effect.

**Implementation**: `sidecar/src/apply/`

#### Rollback Engine

**Responsibility**: Revert applied changes to previous state.

**Process**:

1. Load snapshot from apply
2. Verify snapshot integrity
3. Reverse changes sequentially
4. Verify system state matches snapshot
5. Generate rollback evidence

**Safety**: Rollback itself is reversible (can "undo rollback").

**Implementation**: `sidecar/src/rollback/`

### 5. Storage Layer

**Purpose**: Persist plans, evidence, and snapshots.

#### Directory Structure

```
.anvil/
├── plans/
│   ├── aps-a1b2c3d4.json
│   ├── aps-e5f6g7h8.json
│   └── index.json          # Plan registry
├── evidence/
│   ├── aps-a1b2c3d4/
│   │   ├── gate-001.json
│   │   └── gate-002.json
│   └── index.json
├── snapshots/
│   ├── aps-a1b2c3d4/
│   │   ├── pre-apply.tar.gz
│   │   └── metadata.json
│   └── index.json
├── audit/
│   ├── 2025-09-30.log
│   └── index.json
└── config.json             # Anvil configuration
```

#### Plan Storage

**Format**: JSON (pretty-printed)

**Naming**: `aps-[plan-id].json`

**Indexing**: `index.json` maintains registry of all plans

**Retrieval**: By plan ID or hash

#### Evidence Storage

**Format**: JSON (per gate execution)

**Organization**: By plan ID, then chronological

**Retention**: Indefinite (immutable audit trail)

#### Snapshot Storage

**Format**: Compressed tar.gz + metadata JSON

**Content**: All files affected by plan

**Retention**: Configurable (default: keep until plan superseded)

#### Audit Logs

**Format**: Structured JSON logs

**Content**: All operations (plan, gate, apply, rollback)

**Rotation**: Daily files, configurable retention

---

## Data Flow

### 1. Plan Creation Flow

```
User Intent
    ↓
┌───────────────────┐
│  anvil plan       │
│  "add feature"    │
└─────────┬─────────┘
          │
          ↓
┌───────────────────┐
│ Format Selection  │  ← User specifies or auto-detect
└─────────┬─────────┘
          │
          ↓
┌───────────────────┐
│ Generate Plan     │  ← Create in selected format
│ (Adapter)         │     (SpecKit, BMAD, or APS)
└─────────┬─────────┘
          │
          ↓
┌───────────────────┐
│ Convert to APS    │  ← Internal representation
│ (if needed)       │
└─────────┬─────────┘
          │
          ↓
┌───────────────────┐
│ Validate Schema   │  ← Zod validation
└─────────┬─────────┘
          │
          ↓
┌───────────────────┐
│ Generate Hash     │  ← Deterministic hash
└─────────┬─────────┘
          │
          ↓
┌───────────────────┐
│ Serialize & Save  │  ← Store in user format
│ (.anvil/plans/)   │     + APS internally
└───────────────────┘
```

### 2. Validation Flow (Gate)

```
Plan File
    ↓
┌───────────────────┐
│  anvil gate       │
│  plan.md          │
└─────────┬─────────┘
          │
          ↓
┌───────────────────┐
│ Detect Format     │  ← Auto-detect or explicit
└─────────┬─────────┘
          │
          ↓
┌───────────────────┐
│ Parse to APS      │  ← Adapter parses format
└─────────┬─────────┘
          │
          ↓
┌───────────────────┐
│ Validate APS      │  ← Schema + hash check
└─────────┬─────────┘
          │
          ↓
┌───────────────────┐
│ Run Gate Checks   │  ← Parallel execution
│  • Lint           │
│  • Tests          │
│  • Coverage       │
│  • Secrets        │
│  • Policies       │
└─────────┬─────────┘
          │
          ↓
┌───────────────────┐
│ Collect Evidence  │  ← Aggregate results
└─────────┬─────────┘
          │
          ↓
┌───────────────────┐
│ Update Plan       │  ← Inject evidence into
│ (with Evidence)   │     user format + APS
└─────────┬─────────┘
          │
          ↓
┌───────────────────┐
│ Display Results   │  ← Pretty table output
└───────────────────┘
```

### 3. Apply Flow

```
Validated Plan
    ↓
┌───────────────────┐
│  anvil apply      │
│  plan.md          │
└─────────┬─────────┘
          │
          ↓
┌───────────────────┐
│ Load & Parse      │  ← Load plan, convert to APS
└─────────┬─────────┘
          │
          ↓
┌───────────────────┐
│ Pre-flight Checks │
│  • Gate passed?   │
│  • Approved?      │
│  • Hash valid?    │
└─────────┬─────────┘
          │
          ↓
┌───────────────────┐
│ Create Snapshot   │  ← Capture current state
└─────────┬─────────┘
          │
          ↓
┌───────────────────┐
│ Apply Changes     │  ← Sequential execution
│ (Transactional)   │
└─────────┬─────────┘
          │
          ↓
┌───────────────────┐
│ Generate Evidence │  ← Execution logs
└─────────┬─────────┘
          │
          ↓
┌───────────────────┐
│ Update Plan       │  ← Add execution record
└─────────┬─────────┘
          │
          ↓
┌───────────────────┐
│ Audit Log         │  ← Record operation
└───────────────────┘
```

### 4. Rollback Flow

```
Plan ID or Hash
    ↓
┌───────────────────┐
│  anvil rollback   │
│  aps-a1b2c3d4     │
└─────────┬─────────┘
          │
          ↓
┌───────────────────┐
│ Load Snapshot     │  ← Find pre-apply snapshot
└─────────┬─────────┘
          │
          ↓
┌───────────────────┐
│ Verify Integrity  │  ← Check snapshot hash
└─────────┬─────────┘
          │
          ↓
┌───────────────────┐
│ Preview Changes   │  ← Show what will revert
└─────────┬─────────┘
          │
          ↓
┌───────────────────┐
│ Confirm?          │  ← User confirmation
└─────────┬─────────┘
          │
          ↓
┌───────────────────┐
│ Restore State     │  ← Apply snapshot
└─────────┬─────────┘
          │
          ↓
┌───────────────────┐
│ Verify Rollback   │  ← Check state matches
└─────────┬─────────┘
          │
          ↓
┌───────────────────┐
│ Generate Evidence │  ← Rollback evidence
└─────────┬─────────┘
          │
          ↓
┌───────────────────┐
│ Audit Log         │  ← Record rollback
└───────────────────┘
```

---

## Technology Stack

### Core Technologies

**Language**: TypeScript 5.9+

- Strict mode enabled
- Path aliases for imports
- Comprehensive type coverage

**Runtime**: Node.js >=18.0.0

- Native ESM support
- Modern JavaScript features
- Cross-platform compatibility

**Package Manager**: pnpm

- Workspace support
- Efficient disk usage
- Fast installs

**Build System**: Nx 21.5+

- Monorepo management
- Incremental builds
- Task caching

### Key Libraries

#### Schema & Validation

- **Zod** (^3.x): Runtime validation, TypeScript types
- **zod-to-json-schema** (^3.x): JSON Schema export
- **Ajv** (^8.17): JSON Schema validation (compatibility)

#### CLI

- **Commander.js** (^11.x): Command-line interface
- **Enquirer** (^2.x): Interactive prompts
- **Chalk** (^5.x): Terminal colors
- **Ora** (^7.x): Spinners and progress

#### Parsing & Serialization

- **js-yaml** (^4.1): YAML parsing
- **marked** (^9.x): Markdown parsing
- **turndown** (^7.x): HTML to Markdown

#### Testing

- **Vitest** (^1.x): Unit and integration tests
- **@vitest/coverage-v8**: Coverage reporting

#### Code Quality

- **ESLint** (^8.x): Linting
- **Prettier** (^3.x): Code formatting
- **TypeScript ESLint**: TypeScript linting

#### Security

- **detect-secrets** (patterns): Secret detection
- **semver** (^7.x): Version comparison

#### Policy Engine

- **Open Policy Agent** (OPA): Policy evaluation
- Bundled binary (platform-specific)
- Rego policy language

### Development Tools

**Version Control**: Git + GitHub

- Conventional commits
- Branch protection
- PR templates

**CI/CD**: GitHub Actions

- Automated testing
- Lint checks
- Security scanning

**Code Quality**: Husky + lint-staged

- Pre-commit hooks
- Automated formatting
- Lint on commit

---

## Security Model

### Threat Model

**Threats**:

1. Malicious plans (code injection, path traversal)
2. Tampering with validated plans
3. Unauthorized apply operations
4. Secret leakage in logs/evidence
5. Supply chain attacks

**Mitigations**:

1. **Input validation**: All inputs sanitized and validated
2. **Hash verification**: Detect plan tampering
3. **Approval requirements**: Multi-step validation before apply
4. **Secret scanning**: Prevent secrets in plans and logs
5. **Dependency scanning**: Regular security audits

### Trust Boundaries

**Boundary 1**: User Input → Adapter

- Validate all file inputs
- Sanitize user-provided data
- Prevent path traversal

**Boundary 2**: Adapter → APS Core

- Schema validation
- Hash verification
- Business rule enforcement

**Boundary 3**: Gate → Sidecar

- Require gate pass
- Check approval status
- Verify plan integrity

**Boundary 4**: Sidecar → File System

- Sandbox execution (future)
- Limited file system access
- Audit all operations

### Secrets Handling

**Policy**: Never log or store secrets.

**Implementation**:

- Secret scanning in gate
- Redaction in logs
- Secure evidence storage (optional encryption)
- Environment variable isolation

**Detection Patterns**:

- API keys (AWS, GitHub, etc.)
- Private keys (RSA, SSH)
- Passwords and tokens
- Database connection strings

### Audit Trail

**Requirements**:

- All operations logged
- Immutable audit log
- Timestamp and actor tracking
- Evidence integrity verification

**Storage**: `.anvil/audit/`

**Format**: Structured JSON logs

**Retention**: Configurable (default: 90 days)

---

## Performance Considerations

### Scalability Targets

**Plan Size**:

- Max plan size: 10 MB
- Max proposed changes: 1000
- Max evidence entries: 100

**Repository Size**:

- Support repositories up to 10 GB
- Incremental analysis (changed files only)
- Caching for repeated checks

**Execution Time**:

- Gate execution: <2 minutes for typical repos
- Dry-run: <30 seconds
- Apply: <5 minutes

### Optimization Strategies

#### 1. Parallel Execution

**Gate checks run in parallel**:

```typescript
const results = await Promise.all([
  lintCheck.run(plan),
  testCheck.run(plan),
  coverageCheck.run(plan),
  secretsCheck.run(plan),
  policyCheck.run(plan),
]);
```

**Benefits**: 3-5x faster than sequential execution.

#### 2. Caching

**Cache Strategy**:

- Cache check results by plan hash
- Cache adapter parsing by file hash
- Invalidate on file changes

**Storage**: `.anvil/cache/`

**Eviction**: LRU with configurable size limit

#### 3. Incremental Analysis

**Strategy**: Analyze only changed files.

**Implementation**:

- Git diff to identify changed files
- Run checks only on changed files
- Aggregate results with previous checks

**Benefits**: 10x faster for large repos with small changes.

#### 4. Streaming Output

**Strategy**: Stream check results as they complete.

**Implementation**:

- Event-based architecture
- Real-time UI updates
- Interruptible operations

**Benefits**: Better UX, faster perceived performance.

### Future Optimizations

**Post-MVP**:

- Rust/Go rewrite for performance-critical components
- Native binary for faster startup
- Parallel file parsing
- Distributed gate execution (for CI)

---

## Extension Points

### 1. Custom Adapters

**Interface**: `FormatAdapter`

**Use Cases**:

- Support additional planning formats (ADR, RFC, etc.)
- Company-specific formats
- Legacy system integration

**Registration**:

```typescript
import { AdapterRegistry } from '@anvil/adapters';
import { MyCustomAdapter } from './my-adapter';

AdapterRegistry.register(new MyCustomAdapter());
```

### 2. Custom Gate Checks

**Interface**: `GateCheck`

```typescript
interface GateCheck {
  name: string;
  run(plan: APSPlan, config: CheckConfig): Promise<CheckResult>;
}
```

**Use Cases**:

- Custom linting rules
- Company-specific validations
- Integration with external tools

**Registration**:

```typescript
import { GateRunner } from '@anvil/gate';
import { MyCustomCheck } from './my-check';

const gate = new GateRunner({
  checks: [...standardChecks, new MyCustomCheck()],
});
```

### 3. Custom Policies

**Format**: Rego (OPA)

**Location**: `.anvil/policies/`

**Use Cases**:

- Business-specific rules
- Compliance requirements
- Risk classification

**Example**:

```rego
package anvil.policies

deny[msg] {
  input.proposed_changes[_].type == "file_create"
  not has_tests(input)
  msg = "New files must include tests"
}
```

### 4. Event Hooks

**Future Feature**: Subscribe to lifecycle events.

**Events**:

- `plan.created`
- `gate.started`, `gate.completed`
- `apply.started`, `apply.completed`
- `rollback.started`, `rollback.completed`

**Use Cases**:

- Notifications (Slack, email)
- External system integration
- Custom workflow automation

---

## Future Considerations

### Phase 2 Features (Post-MVP)

#### 1. Packs System

**Concept**: Reusable modules for common infrastructure needs.

**Examples**:

- Feature flags pack
- Telemetry pack
- Authentication pack

**Architecture**:

```
Pack
  ├── Templates (code generation)
  ├── Policies (OPA rules)
  ├── Tests (generated tests)
  └── Documentation
```

**Distribution**: npm packages, versioned.

#### 2. Memory Layer (RAG)

**Concept**: Store and retrieve context from past plans.

**Components**:

- Vector store (embeddings)
- Metadata indices
- Retrieval APIs

**Use Cases**:

- "Show plans related to authentication"
- "What changes did we make to the API last month?"
- Context for AI-assisted plan generation

#### 3. AI-Assisted Features

**Plan Generation**:

```bash
anvil plan --assist "add user authentication"
# AI generates structured plan based on context
```

**Plan Review**:

```bash
anvil review plan.md --assist
# AI suggests improvements, finds issues
```

**Productioniser Enhancement**:

```bash
anvil productionise --assist
# AI suggests architectural improvements
```

#### 4. VS Code Extension

**Features**:

- Inline plan validation
- Provenance decorations (gutter badges)
- Command palette integration
- Diff preview in editor

**Architecture**: Language Server Protocol (LSP) for Anvil.

#### 5. MCP Integration

**Concept**: Expose Anvil as MCP (Model Context Protocol) tools.

**Tools**:

- `anvil_plan`: Create plans
- `anvil_gate`: Validate plans
- `anvil_apply`: Execute plans
- `anvil_search`: Search past plans

**Use Cases**: AI assistants (Cursor, Claude Projects) can use Anvil natively.

### Act 2 Expansion

**Document Adapters**:

- Word/Google Docs
- Confluence
- Notion
- Markdown

**Analysis Adapters**:

- Excel/Google Sheets
- Jupyter notebooks
- Tableau

**Validation Checks**:

- Citation verification
- Data source validation
- Formula correctness
- Logic checking

### Act 3 Platform

**Enterprise Features**:

- SSO integration
- RBAC (role-based access control)
- Advanced audit dashboard
- Compliance reporting

**Platform Integrations**:

- ServiceNow
- Jira
- Confluence
- Slack

**Governance**:

- Policy marketplace
- Compliance frameworks (SOC2, ISO)
- Regulatory support

---

## Architecture Decision Records (ADRs)

### ADR-001: APS as Internal Format

**Status**: Accepted

**Context**: Users won't adopt a new planning format. Existing formats (SpecKit,
BMAD) are entrenched.

**Decision**: APS is internal only. Adapters convert between user formats and
APS.

**Consequences**:

- ✅ Easier adoption (work with existing formats)
- ✅ Clean separation of concerns
- ❌ Adapter maintenance burden
- ❌ Conversion complexity

### ADR-002: Immutable Evidence

**Status**: Accepted

**Context**: Evidence must be trustworthy for compliance and debugging.

**Decision**: Evidence is append-only, never modified.

**Consequences**:

- ✅ Complete audit trail
- ✅ Tamper-proof
- ❌ Storage growth
- ❌ Cannot "fix" old evidence

### ADR-003: Hash-Stable Plans

**Status**: Accepted

**Context**: Plans must be reproducible for validation and caching.

**Decision**: Plans have deterministic hashes based on canonical form.

**Consequences**:

- ✅ Detect tampering
- ✅ Enable caching
- ✅ Reproducible validation
- ❌ Canonicalisation complexity

### ADR-004: TypeScript Everywhere

**Status**: Accepted

**Context**: Need fast iteration, strong typing, cross-platform support.

**Decision**: Use TypeScript for all components. Consider Go/Rust post-MVP for
performance.

**Consequences**:

- ✅ Fast development
- ✅ Strong typing
- ✅ Good ecosystem
- ❌ Slower than compiled languages
- ❌ May need rewrite later

### ADR-005: OPA for Policies

**Status**: Accepted

**Context**: Need flexible, safe policy engine.

**Decision**: Use OPA with Rego for policy evaluation.

**Consequences**:

- ✅ Industry standard
- ✅ Safe evaluation
- ✅ Good tooling
- ❌ Another language to learn
- ❌ Binary dependency

---

## Glossary

**APS (Anvil Plan Specification)**: The internal, hash-stable format for
representing plans.

**Adapter**: Component that converts between user formats and APS.

**Gate**: Validation system that enforces quality standards.

**Evidence**: Immutable record of validation and execution results.

**Sidecar**: Execution runtime that applies and rolls back plans.

**Provenance**: Metadata about who, what, when, why for a plan.

**Canonical Form**: Stable, deterministic serialization of a plan.

**Round-trip**: Parse → APS → Serialize preserving original format.

**Snapshot**: Captured state before applying changes (for rollback).

**Idempotent**: Re-applying has no additional effect.

---

## Contributing to Architecture

This architecture is a living document. As we build and learn, it will evolve.

**Process for Architecture Changes**:

1. Propose change as GitHub issue
2. Discuss with team
3. Create ADR if significant decision
4. Update this document
5. Implement change

**Questions About Architecture?**

- Open a GitHub discussion
- Ask in team chat
- Review existing ADRs

---

**Document Version**: 2.0  
**Last Updated**: 30 September 2025  
**Next Review**: Weekly during MVP development
