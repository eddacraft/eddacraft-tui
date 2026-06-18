# Anvil Format Adapters

Format adapters for converting between external planning formats (SpecKit, BMAD,
etc.) and Anvil Plan Specification (APS).

## Overview

The adapter framework provides a pluggable architecture for working with
different planning document formats. Users can continue using their preferred
format while benefiting from APS validation and gate execution internally.

## Architecture

### Core Concepts

- **FormatAdapter**: Interface for converting between external formats and APS
- **AdapterRegistry**: Central registry for adapter discovery and lookup
- **Auto-detection**: Content-based format detection with confidence scoring
- **Round-trip fidelity**: Parse and serialize maintain document integrity

### Package Structure

```
packages/adapters/
├── src/
│   ├── base/                 # Framework core
│   │   ├── types.ts          # Core interfaces and types
│   │   ├── registry.ts       # Adapter registry
│   │   ├── utils.ts          # Helper utilities
│   │   └── testing.ts        # Testing utilities
│   ├── speckit/              # SpecKit adapter ✅
│   ├── bmad/                 # BMAD adapter ✅
│   └── generic/              # Generic markdown adapter ✅
└── README.md
```

## Usage

### Using Adapters

```typescript
import { registry } from '@eddacraft/anvil-adapters';
import { SpecKitAdapter } from '@eddacraft/anvil-adapters/speckit';

// Register adapter
const speckit = new SpecKitAdapter();
registry.register(speckit);

// Auto-detect format
const content = await fs.readFile('plan.md', 'utf-8');
const match = registry.detectAdapter(content);

if (match) {
  console.log(`Detected: ${match.adapter.metadata.displayName}`);
  console.log(`Confidence: ${match.detection.confidence}%`);

  // Parse to APS
  const result = await match.adapter.parse(content);
  if (result.success) {
    console.log('Parsed plan:', result.data);
  }
}

// Find adapter by format
const adapter = registry.getAdapterForFormat('speckit');
if (adapter) {
  const result = await adapter.parse(content);
}
```

### Creating Custom Adapters

See [ADAPTER_WORKFLOW_GUIDE.md](./ADAPTER_WORKFLOW_GUIDE.md) for:

- Complete workflow guide with real-world examples
- Technical deep dive into adapter implementation
- Step-by-step guide for adding new adapters
- CLI integration patterns
- Testing strategies

## API Reference

See [packages/adapters/src/base/](./src/base/) for complete API documentation.

### Key Interfaces

- `FormatAdapter` - Main adapter interface
- `AdapterRegistry` - Adapter registration and lookup
- `ParseResult` - Result of parsing external format to APS
- `SerializeResult` - Result of serializing APS to external format
- `DetectionResult` - Result of format detection

## Supported Formats

### SpecKit ✅ COMPLETE

GitHub's official spec-driven development format. Supports the complete spec-kit
workflow with three document types.

- **Extensions**: `spec.md`, `plan.md`, `tasks.md`
- **Status**: ✅ Fully implemented (November 2025)
- **Version**: 2.0.0
- **Code**: ~2,469 lines
- **Tests**: 114 tests (45 format adapter + 69 parsers, all passing ✅)
- **Coverage**: >95%

#### Document Types

**spec.md** - Requirements and user scenarios (WHAT and WHY)

- Feature metadata (branch, date, status)
- User scenarios & testing (prioritized user stories with acceptance criteria)
- Functional requirements (testable requirements with clarification markers)
- Key entities (data model definitions)
- Success criteria (quantitative and qualitative metrics)

**plan.md** - Technical implementation (HOW)

- Summary of technical approach
- Technical context (language, dependencies, constraints)
- Constitution check (compliance with project principles)
- Project structure (directory layout, file organisation)
- Implementation details (API endpoints, database schema, etc.)
- Complexity tracking (design decisions and justifications)

**tasks.md** - Executable breakdown

- Tasks organised by phases with IDs
- Parallel execution markers
- Checkpoints after each phase
- Dependencies and execution order
- Implementation strategies

#### Usage Example

```typescript
import { SpecKitImportAdapterV2 } from '@eddacraft/anvil-adapters/speckit';

const adapter = new SpecKitImportAdapterV2();

// Import complete spec-kit feature directory
const result = await adapter.convertToAPS({
  format: 'speckit',
  version: '2.0.0',
  content: {
    spec: { content: specMdContent },
    plan: { content: planMdContent },
    tasks: { content: tasksMdContent },
  },
});

if (result.success) {
  // APS plan with full metadata from all documents
  const plan = result.data;

  // Access spec.md data
  console.log(plan.metadata?.userScenarios);
  console.log(plan.metadata?.clarifications);

  // Access plan.md data
  console.log(plan.metadata?.technicalContext);
  console.log(plan.metadata?.implementationDetails);

  // Access tasks.md data
  console.log(plan.metadata?.phases);
  console.log(plan.metadata?.taskDependencies);
}
```

#### APS Mapping

- **User Scenarios** → `proposed_changes` with scenario metadata (user story,
  acceptance criteria, edge cases)
- **Functional Requirements** → `metadata.requirements.functional[]`
- **Key Entities** → `metadata.requirements.entities[]`
- **Success Criteria** → `metadata.successCriteria`
- **Clarifications** → `metadata.clarifications[]` (all `[NEEDS CLARIFICATION]`
  markers)
- **Technical Context** → `metadata.technicalContext`
- **Project Structure** → `metadata.projectStructure`
- **Implementation Details** → `metadata.implementationDetails`
- **Phases & Tasks** → `metadata.phases[]`, `metadata.taskDependencies[]`

### BMAD ✅ COMPLETE

Business Model and Architecture Document format - enterprise requirements and
PRD format.

- **Extensions**: `.md` (PRD, Architecture, Epic, Story formats)
- **Status**: ✅ Fully implemented (November 2025)
- **Version**: 0.1.2
- **Code**: ~800 lines
- **Tests**: 86 tests (all passing ✅) - exceeds 50+ target
- **Coverage**: >95%
- **CLI Integration**: 100% (validate, gate, export commands)

#### Document Types Supported

**PRD (Product Requirements Document)**

- Functional Requirements (FR-01, FR-02, etc.)
- Non-Functional Requirements (NFR-01, NFR-02, etc.)
- Epics and User Stories (US-01, US-02, etc.)
- YAML front-matter metadata
- Change log tables

**Architecture Documents**

- Technical Summary
- High Level Architecture
- System Components
- Tech Stack
- API Specifications

**Epic and Story Documents**

- Epic goals and related stories
- User story format: "As a... I want... so that..."
- Acceptance criteria
- Implementation details

#### Format Detection

Confidence-based algorithm with 5 weighted indicators (100-point scale):

- YAML front-matter: 30 points
- Requirement identifiers (FR/NFR/US): 25 points
- User story format: 20 points
- Change log table: 15 points
- Document title: 10 points
- **Detection threshold**: 50%
- **Typical confidence**: 90-100% for valid BMAD documents

#### Usage Example

```typescript
import { BMADFormatAdapter } from '@eddacraft/anvil-adapters/bmad';

const adapter = new BMADFormatAdapter();

// Detect BMAD format
const detection = adapter.detect(content);
console.log(`Confidence: ${detection.confidence}%`);

// Parse BMAD to APS
const parseResult = await adapter.parse(content);
if (parseResult.success) {
  console.log('Parsed plan:', parseResult.data);
}

// Serialize APS to BMAD
const serializeResult = await adapter.serialize(apsPlan);
if (serializeResult.success) {
  console.log('Generated BMAD:', serializeResult.content);
}
```

#### CLI Integration

```bash
# Validate BMAD PRD
anvil validate docs/prd.md
# ✓ Detected format: bmad (100% confidence)

# Run quality gates on BMAD document
anvil gate docs/architecture.md
# ✓ All quality gates passed

# Convert BMAD to APS
anvil export docs/prd.md --to aps
# ✓ Export complete (6 functional requirements, 3 non-functional requirements)

# Roundtrip verification
# Parse → Serialize → Parse preserves document structure
```

#### APS Mapping

- **Functional Requirements (FR-XX)** → `proposed_changes[]` with type
  `file_create` or `file_update`
- **Non-Functional Requirements (NFR-XX)** → `proposed_changes[]` with type
  `config_update`
- **User Stories (US-XX)** → `proposed_changes[]` with acceptance criteria in
  description
- **YAML Front-matter** → `provenance` (author, date, version)
- **Document Title/Summary** → `intent.description`
- **Change Log** → `metadata.changeLog[]`

#### Implementation Details

**Files**:

- `format-adapter.ts` - Main FormatAdapter implementation
- `parser.ts` - BMAD → APS conversion
- `serializer.ts` - APS → BMAD generation
- `types.ts` - BMAD-specific TypeScript types
- `utils.ts` - Helper functions (metadata extraction, requirement parsing)

**Registry Integration**:

- Auto-registered with `AdapterRegistry` on module import
- Discoverable by CLI's `FormatDetectionService`

**Validation**:

- Content validation without full parse
- Returns `ValidationIssue[]` with severity levels (error, warning, info)
- Checks format indicators and document structure

**Next Steps**:

- ✅ Comprehensive testing complete (86 tests, exceeding 50+ target)
- ✅ Test fixtures created (6 valid + invalid BMAD documents)
- ✅ Serves as reference implementation for FormatAdapter interface

### Generic Markdown ✅ COMPLETE

Fallback adapter for generic planning documents that don't match SpecKit or BMAD
formats. Provides broad compatibility for PRDs, TODOs, RFCs, and ADRs.

- **Extensions**: `.md` (PRD, TODO, plan, spec, RFC, ADR formats)
- **Status**: ✅ Fully implemented (November 2025)
- **Version**: 1.0.0
- **Code**: ~198 lines
- **Tests**: 32 tests (all passing ✅)
- **Coverage**: >95%
- **Detection**: Fallback adapter (30-45% confidence)

#### Supported Document Types

- **PRD (Product Requirements Document)** - Generic product requirements
- **TODO** - Task lists and action items
- **Plan** - Implementation plans
- **Spec** - Technical specifications
- **RFC (Request for Comments)** - Design proposals
- **ADR (Architecture Decision Records)** - Architecture decisions

#### Format Detection

Generic adapter uses fallback detection with lower confidence (30-45%):

- Generic markdown structure: 15 points
- Common planning keywords: 10 points
- Section headers (Requirements, Tasks, etc.): 10 points
- **Detection threshold**: 30%
- **Typical confidence**: 30-45% (intentionally lower than specific formats)

#### Usage Example

```typescript
import { GenericMarkdownAdapter } from '@eddacraft/anvil-adapters/generic';

const adapter = new GenericMarkdownAdapter();

// Detect generic markdown
const detection = adapter.detect(content);
console.log(`Confidence: ${detection.confidence}%`);

// Parse generic markdown to APS
const parseResult = await adapter.parse(content);
if (parseResult.success) {
  // Extracted requirements, tasks, features, goals
  console.log('Parsed plan:', parseResult.data);
}
```

#### CLI Integration

```bash
# Validate generic TODO document
anvil validate TODO.md
# ✓ Detected format: generic (45% confidence)

# Run gates on generic RFC
anvil gate docs/RFC-001.md
# ✓ Format: generic markdown

# Works as fallback for unknown formats
anvil validate docs/custom-plan.md
# ✓ Falling back to generic markdown adapter
```

#### APS Mapping

- **Requirements sections** → `proposed_changes[]` with type `requirement`
- **Task lists** → `proposed_changes[]` with type `task`
- **Features** → `proposed_changes[]` with type `feature`
- **Goals/Objectives** → `intent.goals[]`
- **Document title** → `intent.description`

#### File Discovery Utility

Automatically finds planning documents in repositories:

```typescript
import { findPlanningDocuments } from '@eddacraft/anvil-adapters/utils';

// Search for planning docs in current directory
const docs = await findPlanningDocuments();

// Returns sorted by confidence and recency
docs.forEach((doc) => {
  console.log(`${doc.path} - ${doc.pattern} (confidence: ${doc.confidence}%)`);
});

// Example output:
// docs/prd.md - prd (high confidence)
// TODO.md - todo (medium confidence)
// docs/RFC-001.md - rfc (medium confidence)
```

**Next Steps**:

- ✅ Generic adapter complete with full test coverage
- ✅ File discovery utility implemented
- ✅ Provides broad compatibility for any markdown planning document

## Development

### Running Tests

```bash
pnpm test                # Run all tests (232 tests)
pnpm test:watch          # Watch mode
pnpm test:coverage       # Run with coverage report
```

**Current Test Status** (as of November 2025):

- **Total: 232 tests** ✅ All passing
  - SpecKit: 114 tests (45 format adapter + 69 parsers)
  - BMAD: 86 tests (exceeds 50+ target)
  - Generic: 32 tests
- **Coverage**: >95% across all adapters

### Type Checking

```bash
pnpm typecheck
```

### Building

```bash
npx nx build adapters
```

### Project Structure

```
packages/adapters/
├── src/
│   ├── base/                     # Framework core (586 LOC)
│   │   ├── types.ts              # FormatAdapter interface, base classes
│   │   ├── registry.ts           # Adapter registry with detection
│   │   ├── utils.ts              # Helper utilities
│   │   ├── testing.ts            # Testing utilities
│   │   └── __tests__/
│   │       └── registry.test.ts  # 22 tests (100% passing)
│   ├── common/                   # Legacy adapter types (deprecated)
│   │   ├── types.ts              # Old SpecToolAdapter interface
│   │   ├── registry.ts           # Old registry implementation
│   │   └── index.ts
│   ├── speckit/                  # SpecKit adapter (2,469 LOC)
│   │   ├── index.ts              # Exports
│   │   ├── parser.ts             # Core markdown parser (330 LOC)
│   │   ├── import.ts             # V1 import adapter (284 LOC)
│   │   ├── import-v2.ts          # V2 official format (424 LOC)
│   │   ├── export.ts             # Export adapter (462 LOC)
│   │   └── parsers/              # Specialized parsers (966 LOC)
│   │       ├── spec-parser.ts    # Spec.md parser (378 LOC)
│   │       ├── plan-parser.ts    # Plan.md parser (342 LOC)
│   │       └── tasks-parser.ts   # Tasks.md parser (246 LOC)
│   ├── __tests__/                # Test suite
│   │   ├── fixtures/             # Test fixtures
│   │   │   ├── speckit/          # Sample SpecKit documents
│   │   │   ├── speckit-official/ # Official spec-kit examples
│   │   │   └── aps/              # APS test fixtures
│   │   ├── speckit-import.test.ts       # V1 import tests
│   │   ├── speckit-import-v2.test.ts    # V2 import tests
│   │   ├── speckit-export.test.ts       # Export tests
│   │   ├── speckit-parser.test.ts       # Core parser tests
│   │   └── speckit-spec-parser.test.ts  # Spec parser tests
│   └── index.ts                  # Main package exports
└── README.md                     # This file
```

## Design Principles

1. **Format Agnostic**: Users work with their preferred format
2. **APS Internal**: Validation and execution always use APS
3. **Round-trip Fidelity**: Parse → Serialize → Parse preserves intent
4. **Content Detection**: Auto-detect format from content
5. **Extensible**: Easy to add new format adapters
6. **Type Safe**: Full TypeScript support

## Licence

Copyright (c) 2026 eddacraft. All rights reserved. See [LICENSE](../../LICENSE)
for details.
