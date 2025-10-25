# BMAD Adapter Specification

## Overview

BMAD (Breakthrough Method for Agile AI-Driven Development) adapter for
converting BMAD format documents (PRDs, Architecture docs, Epics, Stories)
to/from Anvil Plan Specification (APS).

**Version**: 1.0.0 **Status**: Design Phase (Week 8) **Target**: FormatAdapter
interface compliance from the start

---

## BMAD Format Research Summary

### Document Types

BMAD uses structured markdown documents with YAML front-matter:

1. **PRD (Product Requirements Document)** - `docs/prd.md`
   - Functional Requirements (FRs) - numbered FR-01, FR-02, etc.
   - Non-Functional Requirements (NFRs) - numbered NFR-01, NFR-02, etc.
   - Epics
   - User Stories (US-01, US-02, etc.)

2. **Architecture Document** - `docs/architecture.md`
   - Technical Summary
   - High Level Architecture
   - System Components
   - Tech Stack
   - API Specifications

3. **Epic Documents** - `docs/epics/*.md`
   - Epic title and goal
   - Related stories
   - Success criteria

4. **Story Documents** - `docs/stories/*.md`
   - User story format: "As a... I want... so that..."
   - Acceptance criteria (numbered list)
   - Implementation details

### Common Structural Elements

**YAML Front-Matter**:

```yaml
---
name: 'Product Requirements Document'
version: '4.44.0'
description: 'Scale-adaptive PRD template'
output_file: 'PRD.md'
variables:
  project_name: '{{user_input}}'
  author: '{{from_config}}'
  date: '{{system_date}}'
---
```

**Change Log Table**:

```markdown
| Date | Version | Description | Author |
| :--- | :------ | :---------- | :----- |
```

**Requirement Format**:

- `FR-01: Description` (Functional Requirements)
- `NFR-01: Description` (Non-Functional Requirements)
- `US-01: Story Title` (User Stories)

**Story Format**:

```markdown
As a {{user_type}}, I want {{action}}, so that {{benefit}}.
```

**Acceptance Criteria**:

```markdown
1. Criterion 1
2. Criterion 2
3. Criterion 3
```

---

## Adapter Design

### FormatAdapter Implementation

The BMAD adapter will correctly implement `FormatAdapter` interface from
`packages/adapters/src/base/types.ts`:

```typescript
export class BMADFormatAdapter implements FormatAdapter {
  readonly metadata: AdapterMetadata = {
    name: 'bmad',
    version: '1.0.0',
    displayName: 'BMAD (Breakthrough Method for Agile AI-Driven Development)',
    description: 'BMAD PRD and architecture document adapter',
    formats: ['bmad', 'prd', 'architecture'],
    extensions: ['.md'],
  };

  detect(content: string): DetectionResult;
  parse(content: string, context?: ParseContext): Promise<ParseResult>;
  serialize(plan: APSPlan): Promise<SerializeResult>;
  validate(content: string): Promise<ValidationResult>;
  canImport(format: string): boolean;
  canExport(format: string): boolean;
}
```

### Format Detection Strategy

**Detection Indicators** (confidence scoring):

High confidence (90-100%):

- YAML front-matter with BMAD template metadata
- "Product Requirements Document" or "Architecture Document" in title
- Presence of FR-01, NFR-01, US-01 identifiers
- "As a... I want... so that..." story format
- Change log table with exact column structure

Medium confidence (70-89%):

- Multiple numbered requirements (FR-_, NFR-_, US-\*)
- Acceptance Criteria sections
- Epic/Story terminology

Low confidence (50-69%):

- Generic markdown with "Requirements" section
- User story format without other indicators

**Detection Algorithm**:

```typescript
detect(content: string): DetectionResult {
  let score = 0;
  const indicators = [];

  // YAML front-matter (30 points)
  if (/^---\s*\n.*template:/ms.test(content)) {
    score += 30;
    indicators.push('yaml-frontmatter');
  }

  // Requirement identifiers (25 points)
  const frMatches = content.match(/\bFR-\d{2}/g);
  const nfrMatches = content.match(/\bNFR-\d{2}/g);
  const usMatches = content.match(/\bUS-\d{2}/g);

  if (frMatches || nfrMatches || usMatches) {
    score += 25;
    indicators.push('requirement-identifiers');
  }

  // User story format (20 points)
  if (/As a .+,\s*\nI want .+,\s*\nso that .+\./mi.test(content)) {
    score += 20;
    indicators.push('user-story-format');
  }

  // Change log table (15 points)
  if (/\|\s*Date\s*\|\s*Version\s*\|\s*Description\s*\|\s*Author\s*\|/i.test(content)) {
    score += 15;
    indicators.push('change-log-table');
  }

  // PRD/Architecture title (10 points)
  if (/(Product Requirements|Architecture) Document/i.test(content)) {
    score += 10;
    indicators.push('document-title');
  }

  return createDetection(score >= 50, score, indicators.join(', '));
}
```

### Parse Strategy

**Input**: BMAD markdown document (PRD, Architecture, Epic, or Story)
**Output**: APS plan with converted changes

**Parsing Steps**:

1. **Extract YAML Front-Matter** (if present)
   - Parse metadata (name, version, author, date)
   - Use for APS provenance

2. **Identify Document Type**
   - PRD: Contains FR/NFR sections
   - Architecture: Contains Technical Summary, High Level Architecture
   - Epic: Contains Epic goal and stories
   - Story: Contains "As a... I want... so that..."

3. **Extract Requirements → APS Changes**
   - FR-01 → `file_create` or `file_update` change
   - NFR-01 → `config_change` or validation requirement
   - US-01 → `file_create` with acceptance criteria as description

4. **Extract Intent**
   - PRD: Use Executive Summary or Product Vision
   - Architecture: Use Technical Summary
   - Epic: Use Epic Goal
   - Story: Use Story description

5. **Generate APS Plan**
   ```typescript
   const plan = createPlan({
     id: generatePlanId(),
     intent: extractedIntent,
     provenance: {
       timestamp: yamlDate || new Date().toISOString(),
       author: yamlAuthor || context?.author || 'unknown',
       source: 'cli',
       version: this.metadata.version,
       repository: context?.repositoryPath,
     },
     changes: extractedChanges,
     validations: {
       required_checks: ['lint', 'test', 'coverage'],
       skip_checks: [],
     },
   });
   ```

### Serialize Strategy

**Input**: APS plan **Output**: BMAD markdown document

**Serialization Steps**:

1. **Generate YAML Front-Matter**

   ```yaml
   ---
   name: 'Product Requirements Document'
   version: '1.0.0'
   date: '{{provenance.timestamp}}'
   author: '{{provenance.author}}'
   ---
   ```

2. **Create Document Header**

   ```markdown
   # {{project_name}} - Product Requirements Document

   **Author:** {{provenance.author}} **Date:** {{provenance.timestamp}}
   **Version:** 1.0
   ```

3. **Generate Change Log**

   ```markdown
   ## Change Log

   | Date     | Version | Description     | Author     |
   | :------- | :------ | :-------------- | :--------- |
   | {{date}} | 1.0     | Initial version | {{author}} |
   ```

4. **Convert APS Changes → Requirements**
   - `file_create` → FR-01: Create {{path}} - {{description}}
   - `file_update` → FR-02: Update {{path}} - {{description}}
   - `config_change` → NFR-01: {{description}}

5. **Generate Sections**

   ```markdown
   ## Functional Requirements

   FR-01: {{change.description}} ({{change.path}}) FR-02: {{change.description}}
   ({{change.path}})

   ## Non-Functional Requirements

   NFR-01: {{validation.required_checks}}
   ```

---

## Implementation Plan

### File Structure

```
packages/adapters/src/bmad/
├── format-adapter.ts          # Main FormatAdapter implementation
├── parser.ts                  # Document parsing logic
├── serializer.ts              # Document generation logic
├── types.ts                   # BMAD-specific types
├── utils.ts                   # Helper utilities
└── __tests__/
    ├── bmad-format-adapter.test.ts
    ├── bmad-parser.test.ts
    ├── bmad-serializer.test.ts
    └── fixtures/
        ├── valid-prd.md
        ├── valid-architecture.md
        ├── valid-epic.md
        ├── valid-story.md
        └── invalid-documents/
```

### Test Strategy

**Target**: 50+ tests (match SpecKit coverage)

**Test Categories**:

1. **Format Detection Tests** (15 tests)
   - Valid PRD detection (high confidence)
   - Valid Architecture doc detection
   - User story format detection
   - Edge cases (partial matches, false positives)
   - Confidence scoring accuracy

2. **Parser Tests** (20 tests)
   - Parse PRD with FRs/NFRs
   - Parse Architecture document
   - Parse Epic document
   - Parse Story document
   - YAML front-matter extraction
   - Requirement identifier extraction
   - User story format parsing
   - Change log parsing
   - Invalid document handling
   - Missing sections handling

3. **Serializer Tests** (10 tests)
   - Generate PRD from APS
   - Generate Architecture doc from APS
   - YAML front-matter generation
   - Requirement numbering
   - Change log generation
   - Round-trip fidelity

4. **Integration Tests** (5 tests)
   - canImport/canExport
   - Full parse → serialize → parse cycle
   - Multiple document types
   - Edge cases

### Fixtures Required

**Valid Documents**:

- `valid-prd.md` - Complete PRD with FRs, NFRs, Epics, Stories
- `valid-architecture.md` - Architecture doc with all sections
- `valid-epic.md` - Epic with goal and stories
- `valid-story.md` - User story with acceptance criteria

**Invalid Documents**:

- `invalid-no-requirements.md` - Missing FR/NFR sections
- `invalid-malformed-yaml.md` - Bad YAML front-matter
- `invalid-generic-markdown.md` - Regular markdown

---

## Registration

Update `packages/adapters/src/index.ts`:

```typescript
// Auto-register adapters when module is imported
import { registry as baseRegistry } from './base/index.js';
import { BMADFormatAdapter } from './bmad/index.js';

// Register BMAD adapter
baseRegistry.register(new BMADFormatAdapter());

export { baseRegistry as registry };
```

---

## CLI Integration Testing

Once implemented, test with CLI:

```bash
# Test format detection
anvil validate docs/prd.md
# Expected: ✓ Detected format: bmad (95% confidence)

# Test gate command
anvil gate docs/architecture.md
# Expected: Runs all quality gates on BMAD architecture doc

# Test export
anvil export docs/prd.md --to aps
# Expected: Converts BMAD PRD to APS JSON

# Test reverse (APS → BMAD)
anvil export plan.json --to bmad --output docs/
# Expected: Generates docs/prd.md in BMAD format
```

---

## Success Criteria

- [ ] Implements `FormatAdapter` interface correctly
- [ ] Registered with `AdapterRegistry` on module import
- [ ] `anvil validate docs/prd.md` works with auto-detection
- [ ] `anvil gate docs/architecture.md` works with auto-detection
- [ ] `anvil export docs/prd.md --to=aps` works
- [ ] `anvil export plan.json --to=bmad` works
- [ ] All 50+ tests passing
- [ ] Format detection confidence >80% for valid BMAD docs
- [ ] Round-trip fidelity preserved (PRD → APS → PRD)
- [ ] Serves as reference for SpecKit FormatAdapter migration

---

## Timeline

- **Week 8 Day 1-2**: Implementation (format-adapter.ts, parser.ts,
  serializer.ts)
- **Week 8 Day 3**: Tests and fixtures
- **Week 8 Day 4**: CLI integration testing
- **Week 8 Day 5**: Documentation and cleanup

---

## References

- **Context7**: `/bmad-code-org/bmad-method` - 3001 code snippets
- **GitHub**: https://github.com/bmad-code-org/BMAD-METHOD
- **FormatAdapter Interface**: `packages/adapters/src/base/types.ts`
- **SpecKit Reference**: `packages/adapters/src/speckit/` (69 tests)

---

**Created**: 2025-10-23 **Author**: Anvil Team **Sprint**: Week 8 - BMAD Adapter
Implementation
