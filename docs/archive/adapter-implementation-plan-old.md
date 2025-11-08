# Comprehensive Adapter Implementation Plan

## Executive Summary

This document provides a comprehensive plan for implementing SpecKit and BMAD
adapters for Anvil, based on thorough analysis of both standards. The plan
ensures proper implementation, extensibility, and addresses all facets of how
these formats impact Anvil usage.

**Key Findings:**

- SpecKit: **3-document format** (spec.md, plan.md, tasks.md) - **Partially
  Implemented** (V2 exists but needs validation)
- BMAD: **Multi-document format** (PRD, Architecture, Stories, Epics, QA) -
  **Not Implemented**
- Both formats have rich metadata and structured relationships
- Significant differences in philosophy and workflow

---

## Table of Contents

1. [Format Analysis](#format-analysis)
2. [Current Implementation Status](#current-implementation-status)
3. [Gap Analysis](#gap-analysis)
4. [Unified Adapter Architecture](#unified-adapter-architecture)
5. [Implementation Roadmap](#implementation-roadmap)
6. [Impact on Anvil Features](#impact-on-anvil-features)
7. [Testing Strategy](#testing-strategy)
8. [Future Extensibility](#future-extensibility)

---

## 1. Format Analysis

> **Note**: Full template examples are available in:
>
> - [SpecKit Templates](formats/speckit-templates.md)
> - [BMAD Templates](formats/bmad-templates.md)

### 1.1 SpecKit Format (GitHub Official)

**Philosophy**: Spec-Driven Development with AI agents **Primary Use Case**:
AI-assisted feature development with clear WHAT/WHY/HOW separation

#### Document Structure

SpecKit uses a **3-document format**:

1. **spec.md** - Requirements (WHAT and WHY)
   - Metadata: Branch, date, status
   - User scenarios with priorities (P1, P2, P3+)
   - User story format: As a/I want to/So that
   - Acceptance scenarios and edge cases
   - Functional requirements (FR-XXX)
   - Key entities with attributes and relationships
   - Success criteria (quantitative, qualitative, performance, security)
   - Clarification markers: `[NEEDS CLARIFICATION: ...]`

2. **plan.md** - Implementation (HOW)
   - Summary and technical context
   - Constitution check (✅ PASSED / ⚠️ REVIEWED / ❌ BLOCKED)
   - Project structure (documentation + source code)
   - Implementation details (database, API, components)
   - Complexity tracking table

3. **tasks.md** - Execution Breakdown
   - Prerequisites checklist
   - Task ID format: `TASK-###`, `[~]` for parallel execution, `[STORY-###]` for
     links
   - Phases: Setup → Foundational → User Stories (P1) → User Stories (P2) →
     Polish
   - Checkpoints at key milestones
   - Dependencies and execution order (mermaid diagrams)
   - Implementation strategies (MVP first, independent stories)

#### Key Characteristics

1. **Priority-Driven**: P1 (high), P2 (medium), P3+ (low)
2. **Clarification Markers**: `[NEEDS CLARIFICATION: ...]` throughout
3. **Independent Testability**: Each user scenario must be independently
   testable
4. **Constitution Check**: Gating mechanism for architectural decisions
5. **Phase-Based Tasks**: Setup → Foundational → Stories → Polish
6. **Parallel Execution**: Tasks marked with `[~]` can run in parallel

---

### 1.2 BMAD Method Format

**Philosophy**: AI-Agent Framework for Agile Development **Primary Use Case**:
Agent-driven planning with human-in-the-loop validation

#### Document Structure

BMAD uses a **multi-document format**:

1. **PRD (docs/prd.md)** - Product Requirements
   - Change log and version control
   - Goals and background context
   - Functional (FR-XXX) and non-functional (NFR-XXX) requirements
   - UI design goals (UX vision, interaction paradigms, accessibility, branding,
     platforms)
   - Technical assumptions (repo structure, service architecture, testing
     requirements)
   - Epic list with single-sentence goals
   - Epic details with user stories (US-XXX) and acceptance criteria
   - Next steps for agents

2. **Architecture (docs/architecture.md)** - Technical Design
   - Change log and references to PRD/frontend spec
   - Project overview and starter template assessment
   - High-level architecture (style, repo structure, data flows)
   - Project diagram (mermaid)
   - Architectural patterns with rationale
   - Tech stack table (DEFINITIVE - pinned versions, NO "latest")
   - Data models with purpose, attributes, relationships, design decisions
   - Components with responsibilities, APIs, dependencies, diagrams
   - External APIs documentation
   - Core workflows with sequence diagrams
   - REST API specification (OpenAPI YAML)
   - Deployment, monitoring, scaling strategies
   - Security and performance considerations

3. **Story Files (docs/stories/{epic}.{story}.md)** - Individual Stories
   - Epic, ID, priority
   - User story format
   - Acceptance criteria (Given/When/Then)
   - Implementation notes
   - Dev/QA notes (carry forward between iterations)
   - Links to PRD, Architecture, QA Assessment

4. **QA Assessments
   (docs/qa/assessments/{epic}.{story}-risk-profile-YYYYMMDD.md)**
   - Risk assessment matrix (probability × impact)
   - Test strategy (unit, integration, E2E with priorities)
   - Requirements traceability (FR/NFR → tests)
   - NFR validation with evidence

5. **Quality Gates (docs/qa/gates/{epic}.{story}-{slug}.yml)**
   - Gate validation results (PASS/CONCERNS/FAIL/WAIVED)
   - Concerns with severity and recommendations
   - Waiver reasons if applicable

#### Key Characteristics

1. **Agent-Driven**: Analyst, PM, Architect, Dev, QA agents collaborate
2. **YAML Templates**: All documents generated from YAML templates with embedded
   prompts
3. **Validation Workflow**: PO runs master checklist to ensure PRD/Architecture
   alignment
4. **Document Sharding**: PRD/Architecture can be split into individual Epic and
   Story files
5. **QA Integration**: Built-in risk assessment and quality gates
6. **Traceability**: Requirements tracked through stories to tests
7. **Version Control**: Change logs in every document

---

## 2. Current Implementation Status

### 2.1 SpecKit Adapter (V2)

**Location**: `packages/adapters/src/speckit/`

**Files**:

- ✅ `parser.ts` - Core markdown parser (330 LOC)
- ✅ `import.ts` - V1 import adapter (284 LOC)
- ✅ `import-v2.ts` - V2 official format adapter (424 LOC)
- ✅ `export.ts` - Export adapter (462 LOC)
- ✅ `parsers/spec-parser.ts` - Spec.md parser (378 LOC)
- ✅ `parsers/plan-parser.ts` - Plan.md parser (342 LOC)
- ✅ `parsers/tasks-parser.ts` - Tasks.md parser (246 LOC)

**Test Status**: 51 tests (49 passing, 2 failing)

**Coverage**: >95%

#### What's Implemented

**Spec.md Parsing**:

- ✅ Metadata extraction (branch, date, status)
- ✅ User scenarios with priority (P1, P2, P3)
- ✅ User story components (As a, I want to, So that)
- ✅ Acceptance scenarios
- ✅ Edge cases
- ✅ Functional requirements (FR-XXX)
- ✅ Key entities with attributes and relationships
- ✅ Success criteria (quantitative, qualitative, security, performance)
- ✅ Clarification markers (`[NEEDS CLARIFICATION: ...]`)

**Plan.md Parsing**:

- ✅ Technical context (language, dependencies, storage, testing)
- ✅ Constitution check
- ✅ Project structure (documentation + source code)
- ✅ Implementation details (database, API, components)
- ✅ Complexity tracking

**Tasks.md Parsing**:

- ✅ Phases extraction
- ✅ Task IDs and descriptions
- ✅ Dependencies
- ✅ Parallel execution markers
- ✅ Story links
- ✅ Implementation strategies

**APS Conversion**:

- ✅ Intent building from user scenarios
- ✅ Proposed changes from user scenarios + plan + tasks
- ✅ Metadata preservation (all parsed data stored in metadata)
- ✅ Provenance tracking

**Export**:

- ✅ APS → spec.md
- ✅ APS → plan.md
- ✅ APS → tasks.md
- ✅ Evidence injection

### 2.2 BMAD Adapter

**Status**: ❌ **NOT IMPLEMENTED**

**Required Files**:

- ❌ `bmad/parser.ts` - Core YAML/markdown parser
- ❌ `bmad/prd-parser.ts` - PRD parser
- ❌ `bmad/architecture-parser.ts` - Architecture document parser
- ❌ `bmad/story-parser.ts` - Story file parser
- ❌ `bmad/qa-parser.ts` - QA assessment parser
- ❌ `bmad/import.ts` - BMAD → APS adapter
- ❌ `bmad/export.ts` - APS → BMAD adapter

---

## 3. Gap Analysis

### 3.1 SpecKit Gaps

#### Critical Gaps

1. **Missing Template Validation**
   - Current: Templates generated but not validated against official spec-kit
     format
   - Needed: Validate generated templates match GitHub's official structure
   - Impact: Generated files may not work with official spec-kit CLI

2. **Constitution Check Not Enforced**
   - Current: Constitution check parsed but not used in gate validation
   - Needed: Gate should fail if constitution check is ❌ BLOCKED
   - Impact: Architectural violations not caught during validation

3. **Task Dependency Execution**
   - Current: Dependencies parsed but not used in execution planning
   - Needed: Anvil sidecar should respect task dependencies and parallel markers
   - Impact: Tasks may execute in wrong order

4. **Clarification Workflow Missing**
   - Current: `[NEEDS CLARIFICATION: ...]` markers extracted
   - Needed: Anvil should block apply if clarifications exist, provide
     clarification workflow
   - Impact: Ambiguous requirements proceed to implementation

5. **Phase Checkpoints Not Validated**
   - Current: Checkpoints parsed but not enforced
   - Needed: Gate should validate checkpoints before proceeding to next phase
   - Impact: Foundational work may be incomplete

#### Minor Gaps

1. **Research.md Support**
   - Current: Not parsed
   - Needed: Parse research findings and include in metadata
   - Impact: Loss of background research context

2. **Data-model.md Support**
   - Current: Not parsed
   - Needed: Parse data model and validate against entities
   - Impact: Data model inconsistencies not detected

3. **Contract Files Support**
   - Current: Not parsed
   - Needed: Parse API contracts and validate against implementation
   - Impact: API contract violations not detected

4. **Quickstart.md Support**
   - Current: Not parsed
   - Needed: Parse quickstart instructions and include in metadata
   - Impact: Onboarding documentation missing

### 3.2 BMAD Gaps

#### Critical Gaps (Everything)

1. **No PRD Parser**
   - Needed: Parse PRD YAML structure
   - Extract: Goals, background, FR/NFR, UI design goals, technical assumptions,
     epics, user stories
   - Impact: Cannot import BMAD projects

2. **No Architecture Parser**
   - Needed: Parse Architecture YAML structure
   - Extract: Tech stack, data models, components, API specs, workflows
   - Impact: Technical context lost

3. **No Story Parser**
   - Needed: Parse individual story markdown files
   - Extract: User story, acceptance criteria, dev/QA notes
   - Impact: Story-level detail lost

4. **No QA Integration**
   - Needed: Parse risk profiles, test strategies, quality gates
   - Extract: Risk scores, test coverage, NFR validation
   - Impact: Quality assurance data lost

5. **No Agent Workflow Support**
   - Needed: Support BMAD's agent collaboration model
   - Extract: Agent notes, master checklist results, validation reports
   - Impact: Agent-driven workflow incompatible with Anvil

### 3.3 Cross-Format Gaps

#### Semantic Differences

1. **Priority Systems**
   - SpecKit: P1, P2, P3+ (user scenario priority)
   - BMAD: P0, P1, P2 (story priority + test priority)
   - Gap: Need unified priority mapping in APS

2. **Requirement IDs**
   - SpecKit: FR-XXX (functional requirements)
   - BMAD: FR-XXX (functional), NFR-XXX (non-functional)
   - Gap: APS should support both conventions

3. **Task Granularity**
   - SpecKit: Tasks are implementation-level (TASK-XXX)
   - BMAD: Stories are implementation-level (sharded from epics)
   - Gap: APS needs flexible change granularity

4. **Validation Philosophy**
   - SpecKit: Constitution check + clarification markers
   - BMAD: Master checklist + QA gates
   - Gap: Anvil gate needs to support both validation models

---

## 4. Unified Adapter Architecture

### 4.1 Design Principles

1. **Format Agnostic Core**: APS must not favour any format
2. **Lossless Conversion**: Round-trip conversion preserves all data
3. **Metadata Rich**: All format-specific data stored in `metadata`
4. **Extensible**: Easy to add new formats (ADR, RFC, etc.)
5. **Validation Aware**: Support format-specific validation rules

### 4.2 Enhanced APS Schema

**Current APS**:

```typescript
interface APSPlan {
  id: string;
  schema_version: string;
  hash: string;
  intent: string;
  proposed_changes: ProposedChange[];
  provenance: Provenance;
  validations?: Validation;
  evidence?: Evidence[];
  metadata?: Record<string, unknown>;
}
```

**Enhanced APS** (to support both formats):

```typescript
interface APSPlan {
  id: string;
  schema_version: string;
  hash: string;
  intent: string;
  proposed_changes: ProposedChange[];
  provenance: Provenance;
  validations?: Validation;
  evidence?: Evidence[];

  // Enhanced metadata to support rich format information
  metadata?: {
    source_format: 'speckit-v2' | 'bmad-v2' | 'aps';
    format_version?: string;

    // SpecKit-specific
    userScenarios?: UserScenario[];
    clarifications?: Clarification[];
    constitutionCheck?: ConstitutionCheck;
    phases?: Phase[];
    taskDependencies?: TaskDependency[];

    // BMAD-specific
    epics?: Epic[];
    stories?: Story[];
    riskProfiles?: RiskProfile[];
    qualityGates?: QualityGate[];
    agentNotes?: AgentNote[];

    // Shared
    requirements?: {
      functional?: Requirement[];
      nonFunctional?: Requirement[];
      entities?: Entity[];
    };
    technicalContext?: TechnicalContext;
    successCriteria?: SuccessCriteria;
    architectureDetails?: ArchitectureDetails;

    // Original format data (for perfect round-trip)
    _original?: Record<string, unknown>;
  };
}
```

**Enhanced ProposedChange**:

```typescript
interface ProposedChange {
  type: ChangeType;
  path: string;
  description: string;
  content?: string;
  diff?: string;

  // Enhanced metadata
  metadata?: {
    // SpecKit
    priority?: 'P1' | 'P2' | 'P3+';
    userStory?: UserStoryComponents;
    taskId?: string;
    phase?: string;
    dependencies?: string[];
    parallelizable?: boolean;
    checkpoint?: string;

    // BMAD
    epicId?: string;
    storyId?: string;
    requirementIds?: string[];
    riskScore?: number;
    testStrategy?: TestStrategy;

    // Shared
    requirementId?: string;
    acceptanceCriteria?: string[];
    technicalNotes?: string[];
  };
}
```

### 4.3 Adapter Interface (Revised)

**Base Adapter** (enhanced from current implementation):

```typescript
interface FormatAdapter {
  readonly metadata: AdapterMetadata;

  // Core methods (existing)
  detect(content: string): DetectionResult;
  parse(
    content: string,
    context?: ParseContext,
    options?: AdapterOptions
  ): Promise<ParseResult>;
  serialize(plan: APSPlan, options?: AdapterOptions): Promise<SerializeResult>;
  validate(
    content: string,
    options?: AdapterOptions
  ): Promise<ValidationResult>;

  // New: Multi-document support
  parseMultiple?(documents: DocumentSet): Promise<ParseResult>;
  serializeMultiple?(plan: APSPlan): Promise<DocumentSet>;

  // New: Format-specific validation
  validateFormatRules?(plan: APSPlan): Promise<ValidationResult>;

  // Existing
  canImport(format: string): boolean;
  canExport(format: string): boolean;
}

interface DocumentSet {
  primary: Document;
  related?: Document[];
  metadata?: Record<string, unknown>;
}

interface Document {
  path: string;
  content: string;
  type: string;
}
```

### 4.4 Adapter Implementations

#### SpecKit Adapter (Enhanced)

**Files** (additions to existing):

```
speckit/
├── index.ts              # Exports
├── parser.ts             # Core markdown parser (REUSE EXISTING)
├── import-v2.ts          # V2 import (ENHANCE EXISTING)
├── export.ts             # Export adapter (ENHANCE EXISTING)
├── parsers/
│   ├── spec-parser.ts    # EXISTING - no changes needed
│   ├── plan-parser.ts    # EXISTING - no changes needed
│   ├── tasks-parser.ts   # EXISTING - no changes needed
│   ├── research-parser.ts     # NEW - parse research.md
│   ├── datamodel-parser.ts    # NEW - parse data-model.md
│   └── contract-parser.ts     # NEW - parse contracts/*.md
├── validators/               # NEW - format-specific validation
│   ├── constitution-validator.ts
│   ├── clarification-validator.ts
│   └── checkpoint-validator.ts
└── generators/               # NEW - template generation
    ├── spec-generator.ts
    ├── plan-generator.ts
    └── tasks-generator.ts
```

**Key Enhancements**:

1. Parse additional documents (research, data-model, contracts)
2. Validate constitution check in format-specific validator
3. Extract and validate clarifications
4. Validate phase checkpoints
5. Generate validated templates

#### BMAD Adapter (New)

**Files** (all new):

```
bmad/
├── index.ts              # Exports
├── parser.ts             # Core YAML/markdown parser
├── import.ts             # BMAD → APS adapter
├── export.ts             # APS → BMAD adapter
├── parsers/
│   ├── prd-parser.ts          # Parse PRD YAML template
│   ├── architecture-parser.ts  # Parse Architecture YAML template
│   ├── story-parser.ts        # Parse story markdown files
│   ├── epic-parser.ts         # Parse epic markdown files
│   └── qa-parser.ts           # Parse QA assessments and gates
├── validators/
│   ├── master-checklist.ts    # PO master checklist validation
│   ├── alignment-validator.ts # PRD/Architecture alignment check
│   └── qa-gate-validator.ts   # Quality gate validation
└── generators/
    ├── prd-generator.ts
    ├── architecture-generator.ts
    ├── story-generator.ts
    └── qa-generator.ts
```

**Implementation Strategy**:

1. **Phase 1**: PRD parser (goals, background, FR/NFR, epics, stories)
2. **Phase 2**: Architecture parser (tech stack, data models, components, APIs)
3. **Phase 3**: Story/Epic parsers (individual file support)
4. **Phase 4**: QA parsers (risk profiles, quality gates)
5. **Phase 5**: Validators (master checklist, alignment, QA gates)
6. **Phase 6**: Generators (template generation)

---

## 5. Implementation Roadmap

### Phase 1: SpecKit Enhancement (Week 1-2)

**Goals**: Fix existing gaps, complete SpecKit adapter

**Tasks**:

1. **Fix Failing Tests** (2 minor spec-parser fixes)
   - Priority: HIGH
   - Effort: 2 hours

2. **Add Missing Document Parsers**
   - `research-parser.ts`: Parse research.md
   - `datamodel-parser.ts`: Parse data-model.md
   - `contract-parser.ts`: Parse API contracts
   - Priority: MEDIUM
   - Effort: 1 day

3. **Implement Format-Specific Validators**
   - `constitution-validator.ts`: Validate constitution check status
   - `clarification-validator.ts`: Block on unresolved clarifications
   - `checkpoint-validator.ts`: Validate phase checkpoints
   - Priority: HIGH
   - Effort: 1 day

4. **Enhance Template Generators**
   - Validate templates against official spec-kit format
   - Add constitution check template
   - Add clarification markers in templates
   - Priority: MEDIUM
   - Effort: 0.5 days

5. **Integration Testing**
   - Test with official spec-kit examples
   - Test round-trip conversion
   - Test evidence injection
   - Priority: HIGH
   - Effort: 1 day

**Deliverables**:

- ✅ All 51 tests passing
- ✅ 100% coverage
- ✅ Complete SpecKit adapter with all features
- ✅ Format-specific validation working

### Phase 2: BMAD Foundation (Week 3-4)

**Goals**: Implement core BMAD parsing and conversion

**Tasks**:

1. **PRD Parser** (Priority: CRITICAL)
   - Parse YAML template structure
   - Extract goals, background, FR/NFR
   - Parse epics and user stories
   - Parse UI design goals and technical assumptions
   - Effort: 2 days

2. **Architecture Parser** (Priority: CRITICAL)
   - Parse YAML template structure
   - Extract tech stack (with version pinning)
   - Parse data models
   - Parse components and APIs
   - Parse workflows and diagrams
   - Effort: 2 days

3. **BMAD Import Adapter** (Priority: CRITICAL)
   - Convert PRD + Architecture → APS
   - Map FR/NFR to proposed changes
   - Map epics/stories to changes
   - Preserve all metadata
   - Generate provenance
   - Effort: 2 days

4. **BMAD Export Adapter** (Priority: HIGH)
   - Convert APS → PRD
   - Convert APS → Architecture
   - Preserve YAML structure
   - Generate valid YAML templates
   - Effort: 2 days

5. **Testing**
   - Unit tests for each parser
   - Integration tests for import/export
   - Round-trip tests
   - Effort: 1 day

**Deliverables**:

- ✅ PRD ↔ APS conversion working
- ✅ Architecture ↔ APS conversion working
- ✅ >95% test coverage
- ✅ Round-trip fidelity validated

### Phase 3: BMAD Advanced (Week 5-6)

**Goals**: Implement story sharding and QA integration

**Tasks**:

1. **Story/Epic Parsers** (Priority: HIGH)
   - Parse individual story files
   - Parse individual epic files
   - Link stories to epics
   - Preserve dev/QA notes
   - Effort: 1 day

2. **QA Parsers** (Priority: MEDIUM)
   - Parse risk profile assessments
   - Parse test strategies
   - Parse quality gate YAML files
   - Calculate risk scores
   - Effort: 1 day

3. **Multi-Document Support** (Priority: HIGH)
   - Implement `parseMultiple()` for document sets
   - Implement `serializeMultiple()` for sharded output
   - Handle document relationships
   - Effort: 2 days

4. **Agent Workflow Support** (Priority: LOW)
   - Parse agent notes from story files
   - Extract master checklist results
   - Preserve agent collaboration context
   - Effort: 1 day

5. **Testing**
   - Test story sharding
   - Test QA integration
   - Test multi-document workflows
   - Effort: 1 day

**Deliverables**:

- ✅ Full document sharding support
- ✅ QA integration complete
- ✅ Multi-document parsing working
- ✅ >95% test coverage

### Phase 4: BMAD Validation (Week 7)

**Goals**: Implement BMAD-specific validation

**Tasks**:

1. **Master Checklist Validator** (Priority: HIGH)
   - Implement PO master checklist
   - Validate PRD completeness
   - Validate Architecture completeness
   - Effort: 1 day

2. **Alignment Validator** (Priority: HIGH)
   - Check PRD/Architecture alignment
   - Validate FR coverage in architecture
   - Validate epic/story consistency
   - Generate alignment report
   - Effort: 1 day

3. **QA Gate Validator** (Priority: MEDIUM)
   - Validate risk profiles exist
   - Check quality gate status
   - Block on FAIL status (unless WAIVED)
   - Effort: 1 day

4. **Integration with Anvil Gate** (Priority: CRITICAL)
   - Wire validators into gate runner
   - Add BMAD-specific gate checks
   - Display validation results
   - Effort: 1 day

5. **Testing**
   - Test all validators
   - Test gate integration
   - Test with real BMAD projects
   - Effort: 1 day

**Deliverables**:

- ✅ All BMAD validators working
- ✅ Gate integration complete
- ✅ Validation reports generated

### Phase 5: Polish & Documentation (Week 8)

**Goals**: Complete adapter framework, documentation, and examples

**Tasks**:

1. **Adapter Guide Updates**
   - Update ADAPTER_WORKFLOW_GUIDE.md
   - Add BMAD examples
   - Document all validators
   - Effort: 1 day

2. **Example Projects**
   - Create example SpecKit project
   - Create example BMAD project
   - Add conversion examples
   - Effort: 1 day

3. **CLI Integration**
   - Update `anvil gate` to support both formats
   - Add format detection
   - Add validation reports
   - Effort: 1 day

4. **Performance Optimization**
   - Optimize large document parsing
   - Add caching for repeated parses
   - Benchmark performance
   - Effort: 1 day

5. **Final Testing & QA**
   - End-to-end testing
   - Customer validation
   - Bug fixes
   - Effort: 1 day

**Deliverables**:

- ✅ Complete documentation
- ✅ Example projects
- ✅ CLI fully integrated
- ✅ Performance optimized

---

## 6. Impact on Anvil Features

### 6.1 Gate Validation

**Current Gate**:

```typescript
interface GateRunner {
  run(plan: APSPlan): Promise<GateResult>;
}
```

**Enhanced Gate** (supports format-specific validation):

```typescript
interface GateRunner {
  run(plan: APSPlan, options?: GateOptions): Promise<GateResult>;
}

interface GateOptions {
  enforceFormatRules?: boolean; // Default: true
  formatValidators?: FormatValidator[];
}

interface FormatValidator {
  name: string;
  validate(plan: APSPlan): Promise<ValidationResult>;
}
```

**SpecKit Format Validation**:

1. **Constitution Check**: Fail gate if constitution is ❌ BLOCKED
2. **Clarifications**: Warn or fail if unresolved `[NEEDS CLARIFICATION: ...]`
   markers
3. **Phase Checkpoints**: Validate checkpoints before proceeding

**BMAD Format Validation**:

1. **Master Checklist**: Run PO master checklist on PRD/Architecture
2. **Alignment Check**: Validate PRD/Architecture alignment
3. **Quality Gates**: Check QA gate status (PASS/CONCERNS/FAIL/WAIVED)
4. **Risk Profiles**: Ensure risk assessments exist for high-risk changes

### 6.2 Sidecar Execution

**Current Sidecar**:

```typescript
interface SidecarEngine {
  dryRun(plan: APSPlan): Promise<DryRunResult>;
  apply(plan: APSPlan): Promise<ApplyResult>;
  rollback(planId: string): Promise<RollbackResult>;
}
```

**Enhanced Sidecar** (respects format-specific execution rules):

```typescript
interface SidecarEngine {
  dryRun(plan: APSPlan, options?: ExecutionOptions): Promise<DryRunResult>;
  apply(plan: APSPlan, options?: ExecutionOptions): Promise<ApplyResult>;
  rollback(planId: string, options?: RollbackOptions): Promise<RollbackResult>;
}

interface ExecutionOptions {
  respectDependencies?: boolean; // Default: true
  parallelExecution?: boolean; // Default: true
  phaseCheckpoints?: boolean; // Default: true (SpecKit)
  qaGating?: boolean; // Default: true (BMAD)
}
```

**SpecKit Execution Rules**:

1. **Task Dependencies**: Respect `TASK-XXX` dependencies
2. **Parallel Execution**: Execute tasks marked with `[~]` in parallel
3. **Phase Checkpoints**: Validate checkpoints before next phase
4. **Story Independence**: Each user scenario can be deployed independently

**BMAD Execution Rules**:

1. **Epic Sequencing**: Execute epics in documented order
2. **Story Dependencies**: Respect story dependencies
3. **QA Gating**: Check QA gates before proceeding
4. **Agent Workflow**: Support agent collaboration workflow

### 6.3 Evidence Injection

**SpecKit Evidence** (injected as markdown comments):

```markdown
# Feature: Authentication

<!-- ANVIL EVIDENCE
Gate Status: PASSED
Timestamp: 2025-10-16T10:00:00Z
Plan Hash: abc123def456
Constitution Check: ✅ PASSED
Clarifications: 0 unresolved
Checks:
  - Lint: ✅ Passed
  - Tests: ✅ Passed (51/51)
  - Coverage: ✅ 85%
-->

**Branch**: `feature/auth` ...
```

**BMAD Evidence** (injected as YAML front matter):

```yaml
---
anvil_evidence:
  gate_status: PASSED
  timestamp: 2025-10-16T10:00:00Z
  plan_hash: abc123def456
  master_checklist: PASSED
  alignment_check: PASSED
  checks:
    - name: Lint
      status: PASSED
    - name: Tests
      status: PASSED
      details: '51/51 passing'
    - name: Coverage
      status: PASSED
      details: '85%'
---
# Product Requirements Document
```

### 6.4 Clarification Workflow (SpecKit-specific)

**New Feature**: Interactive clarification resolution

```bash
# User runs gate on spec with clarifications
$ anvil gate specs/auth-feature/spec.md

⚠️  Clarifications Required

Found 3 unresolved clarifications:

1. [spec.md:45] What should happen when user enters invalid email?
2. [spec.md:67] Should we support OAuth providers other than Google/GitHub?
3. [plan.md:32] Which ORM should we use: Prisma or TypeORM?

Options:
  --allow-clarifications  Proceed despite clarifications (warning only)
  --resolve               Start interactive clarification resolution

$ anvil gate specs/auth-feature/spec.md --resolve

┌─ Clarification 1 of 3 ─────────────────────────────────────┐
│ Location: spec.md:45                                        │
│ Question: What should happen when user enters invalid email│
│                                                             │
│ Your answer:                                                │
│ > Show error message "Invalid email format"                │
│                                                             │
│ Update spec.md with this answer? [Y/n]                     │
└─────────────────────────────────────────────────────────────┘
```

**Impact**: Reduces ambiguity before implementation

### 6.5 Multi-Format Team Collaboration

**Scenario**: Team uses both SpecKit and BMAD

```bash
# Developer 1 (uses SpecKit)
$ anvil plan "Add user auth" --format speckit
# → Generates specs/auth-feature/spec.md, plan.md, tasks.md

$ anvil gate specs/auth-feature/spec.md
# → Validates SpecKit format

# Product Manager (uses BMAD)
$ anvil convert specs/auth-feature/spec.md --to bmad
# → Generates docs/prd.md and docs/architecture.md

# PM edits PRD, adds NFRs and QA requirements
$ vim docs/prd.md

# Developer 2 (needs latest from PM)
$ anvil convert docs/prd.md --to speckit
# → Updates specs/auth-feature/ with PM's changes

# Both formats stay in sync through APS!
```

**Impact**: Teams can collaborate across format boundaries

---

## 7. Testing Strategy

### 7.1 Unit Tests

**Parser Tests** (each parser):

```typescript
describe('PRDParser', () => {
  it('should parse goals section', () => {
    /* ... */
  });
  it('should extract FR/NFR requirements', () => {
    /* ... */
  });
  it('should parse epics with user stories', () => {
    /* ... */
  });
  it('should extract UI design goals', () => {
    /* ... */
  });
  it('should parse technical assumptions', () => {
    /* ... */
  });
});
```

**Validator Tests**:

```typescript
describe('ConstitutionValidator', () => {
  it('should fail gate if constitution is BLOCKED', () => {
    /* ... */
  });
  it('should pass gate if constitution is PASSED', () => {
    /* ... */
  });
  it('should warn if constitution is REVIEWED', () => {
    /* ... */
  });
});
```

### 7.2 Integration Tests

**Adapter Tests**:

```typescript
describe('SpecKit Adapter Integration', () => {
  it('should convert full SpecKit directory to APS', async () => {
    const docs = {
      spec: { content: await readFile('fixtures/auth-feature/spec.md') },
      plan: { content: await readFile('fixtures/auth-feature/plan.md') },
      tasks: { content: await readFile('fixtures/auth-feature/tasks.md') },
    };

    const result = await adapter.parseMultiple(docs);
    expect(result.success).toBe(true);
    expect(result.data.proposed_changes.length).toBeGreaterThan(0);
  });
});
```

**Round-Trip Tests**:

```typescript
describe('Round-Trip Conversion', () => {
  it('should preserve all data through SpecKit → APS → SpecKit', async () => {
    const original = await readFixture('speckit/auth-feature/spec.md');
    const aps = await speckitImport.parse(original);
    const regenerated = await speckitExport.serialize(aps.data);
    const aps2 = await speckitImport.parse(regenerated.content);

    expect(aps2.data.hash).toBe(aps.data.hash);
  });

  it('should preserve all data through BMAD → APS → BMAD', async () => {
    const originalPRD = await readFixture('bmad/prd.md');
    const originalArch = await readFixture('bmad/architecture.md');
    const aps = await bmadImport.parseMultiple({
      primary: { path: 'prd.md', content: originalPRD },
      related: [{ path: 'architecture.md', content: originalArch }],
    });
    const regenerated = await bmadExport.serializeMultiple(aps.data);
    const aps2 = await bmadImport.parseMultiple(regenerated);

    expect(aps2.data.hash).toBe(aps.data.hash);
  });
});
```

### 7.3 Fixture-Based Tests

**SpecKit Fixtures**:

```
__tests__/fixtures/speckit/
├── auth-feature/
│   ├── spec.md               # Complete spec with all sections
│   ├── plan.md               # Complete plan with all sections
│   ├── tasks.md              # Complete tasks with phases
│   ├── research.md           # Research findings
│   ├── data-model.md         # Data model
│   └── contracts/
│       └── auth-api.md       # API contract
├── minimal-feature/
│   └── spec.md               # Minimal valid spec
└── invalid-feature/
    └── spec.md               # Missing required sections
```

**BMAD Fixtures**:

```
__tests__/fixtures/bmad/
├── complete-project/
│   ├── docs/
│   │   ├── prd.md
│   │   ├── architecture.md
│   │   ├── epics/
│   │   │   ├── epic-001.md
│   │   │   └── epic-002.md
│   │   ├── stories/
│   │   │   ├── epic-001.story-001.md
│   │   │   └── epic-001.story-002.md
│   │   └── qa/
│   │       ├── assessments/
│   │       │   └── epic-001.story-001-risk-profile-20251016.md
│   │       └── gates/
│   │           └── epic-001.story-001-unit-tests.yml
├── minimal-project/
│   └── docs/
│       └── prd.md            # Minimal valid PRD
└── invalid-project/
    └── docs/
        └── prd.md            # Invalid PRD structure
```

### 7.4 Format Validation Tests

**SpecKit Official Compatibility**:

```typescript
describe('SpecKit Official Compatibility', () => {
  it('should work with official spec-kit generated files', async () => {
    // Test with real files from github/spec-kit repository
    const officialSpec = await fetchFromGitHub(
      'github/spec-kit/examples/auth/spec.md'
    );
    const result = await adapter.parse(officialSpec);
    expect(result.success).toBe(true);
  });

  it('should generate files compatible with spec-kit CLI', async () => {
    const aps = createTestPlan();
    const generated = await adapter.serialize(aps);

    // Validate with official spec-kit validator (if available)
    const validation = await validateWithSpecKit(generated.content);
    expect(validation.valid).toBe(true);
  });
});
```

**BMAD Template Compatibility**:

```typescript
describe('BMAD Template Compatibility', () => {
  it('should parse PRD generated from official YAML template', async () => {
    const prd = await generateFromTemplate('prd-tmpl.yaml');
    const result = await adapter.parse(prd);
    expect(result.success).toBe(true);
  });

  it('should generate valid YAML that matches template structure', async () => {
    const aps = createTestPlan();
    const generated = await adapter.serialize(aps);

    const yamlValid = validateYAML(generated.content);
    expect(yamlValid).toBe(true);
  });
});
```

### 7.5 Performance Tests

**Large Document Tests**:

```typescript
describe('Performance', () => {
  it('should parse large SpecKit spec (<100ms)', async () => {
    const largeSpec = generateLargeSpec(100); // 100 user scenarios
    const start = Date.now();
    await adapter.parse(largeSpec);
    const duration = Date.now() - start;
    expect(duration).toBeLessThan(100);
  });

  it('should parse BMAD with 50 epics/200 stories (<500ms)', async () => {
    const largePRD = generateLargePRD(50, 200);
    const start = Date.now();
    await adapter.parse(largePRD);
    const duration = Date.now() - start;
    expect(duration).toBeLessThan(500);
  });
});
```

---

## 8. Future Extensibility

### 8.1 Additional Format Support

With the unified adapter architecture, adding new formats is straightforward:

**ADR (Architecture Decision Records)**:

```
adapters/src/adr/
├── parser.ts
├── import.ts
├── export.ts
└── validators/
    └── adr-status-validator.ts
```

**RFC (Request for Comments)**:

```
adapters/src/rfc/
├── parser.ts
├── import.ts
├── export.ts
└── validators/
    └── rfc-review-validator.ts
```

**Confluence/Notion Pages**:

```
adapters/src/confluence/
├── api-client.ts
├── parser.ts
├── import.ts
└── export.ts
```

### 8.2 Adapter Development Kit

**Reusable Components**:

```
adapters/src/shared/
├── markdown-parser.ts      # Common markdown parsing
├── yaml-parser.ts          # Common YAML parsing
├── id-extractor.ts         # Extract requirement IDs
├── metadata-builder.ts     # Build APS metadata
└── validators/
    ├── base-validator.ts
    └── common-rules.ts
```

**Adapter Template**:

```typescript
// Template for new adapters
export class NewFormatAdapter extends BaseFormatAdapter {
  readonly metadata = {
    name: 'new-format',
    version: '1.0.0',
    displayName: 'New Format',
    description: 'Description',
    extensions: ['.new'],
    formats: ['new-format'],
  };

  detect(content: string): DetectionResult {
    // Implement detection logic
  }

  async parse(content: string): Promise<ParseResult> {
    // Implement parsing logic
  }

  async serialize(plan: APSPlan): Promise<SerializeResult> {
    // Implement serialization logic
  }

  async validate(content: string): Promise<ValidationResult> {
    // Implement validation logic
  }
}
```

### 8.3 Plugin System (Future)

**Concept**: Allow third-party adapter development

```typescript
// ~/.anvil/plugins/my-format/adapter.ts
import { BaseFormatAdapter } from '@anvil/adapters';

export default class MyFormatAdapter extends BaseFormatAdapter {
  // Custom adapter implementation
}
```

**Registration**:

```bash
$ anvil plugin install my-format-adapter
$ anvil plugin list
- speckit (built-in)
- bmad (built-in)
- my-format (plugin)
```

---

## 9. Recommendations

### 9.1 Immediate Actions (Next Sprint)

1. **Fix SpecKit failing tests** (2 hours)
   - Critical for quality

2. **Implement SpecKit validators** (1 day)
   - Constitution check
   - Clarifications
   - Checkpoints

3. **Start BMAD PRD parser** (2 days)
   - Foundation for BMAD support

### 9.2 Strategic Decisions Needed

1. **Should Anvil enforce format-specific rules by default?**
   - Option A: Yes (strict) - Better quality, may frustrate users
   - Option B: No (lenient) - More flexible, may miss issues
   - **Recommendation**: Yes by default, with `--skip-format-validation` flag

2. **Should Anvil support partial document conversion?**
   - Example: Only spec.md without plan.md/tasks.md
   - **Recommendation**: Yes, with warnings about missing context

3. **Should Anvil support mixed formats in one project?**
   - Example: SpecKit specs + BMAD PRD
   - **Recommendation**: No, require single format per project for consistency

4. **How should Anvil handle format version evolution?**
   - Example: SpecKit v2 → v3, BMAD v2 → v3
   - **Recommendation**: Support multiple versions per adapter, auto-detect
     version

### 9.3 Success Metrics

**Quality Metrics**:

- Test Coverage: >95% for all adapters
- Test Pass Rate: 100%
- Round-Trip Fidelity: 100% (data preservation)

**Performance Metrics**:

- Parse Time: <100ms for typical documents
- Serialize Time: <50ms for typical documents
- Detection Time: <10ms

**Adoption Metrics**:

- Customer #1: Validates SpecKit adapter with real projects
- Customer #2: Validates BMAD adapter with real projects
- Format Conversion: Both customers can convert between formats

---

## 10. Conclusion

This comprehensive plan provides:

1. ✅ **Deep Format Understanding**: Complete analysis of SpecKit and BMAD
   formats
2. ✅ **Gap Analysis**: Clear identification of what's missing
3. ✅ **Unified Architecture**: Extensible adapter framework
4. ✅ **Implementation Roadmap**: 8-week plan to completion
5. ✅ **Impact Analysis**: How adapters affect all Anvil features
6. ✅ **Testing Strategy**: Comprehensive test coverage plan
7. ✅ **Future-Proofing**: Easy to add new formats

**Next Steps**:

1. Review and approve this plan
2. Begin Phase 1: SpecKit Enhancement
3. Customer validation at each phase
4. Iterate based on feedback

**Timeline**: 8 weeks to full adapter support for both formats
