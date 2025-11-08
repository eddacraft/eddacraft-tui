# Product Requirements Document: CLI Integration with SpecKit Adapter

## Executive Summary

### Overview

This PRD defines the integration of the SpecKit format adapter into Anvil's CLI,
enabling developers using GitHub's spec-kit workflow to validate their plans
without changing their existing format. This feature represents a critical
milestone in Anvil's Act 1 strategy: making AI-governed development accessible
by working with existing workflows rather than forcing new formats.

### Problem Statement

Developers using GitHub's spec-kit format (spec.md, plan.md, tasks.md) want to
benefit from Anvil's validation and governance capabilities, but they don't want
to:

- Learn a new planning format (APS)
- Convert their existing documents manually
- Maintain two separate planning systems
- Change their established workflow

Currently, Anvil only works with native APS format, creating a significant
adoption barrier. The CLI lacks format auto-detection and has no integration
with the completed SpecKit adapter framework.

### Success Criteria

The feature is successful when:

1. **Zero Format Friction**: Users run `anvil gate spec.md` and it works
   immediately without format specification
2. **Transparent Validation**: Format conversion happens invisibly; users get
   validation feedback in their native format
3. **Round-trip Preservation**: Evidence injection preserves document structure
   and formatting
4. **Format Interoperability**: Users can export between formats for
   collaboration (SpecKit ↔ APS)
5. **Adoption Metrics**:
   - 80% of test users successfully validate SpecKit documents on first attempt
   - <30 seconds time-to-first-validation
   - 95% of users report "it just worked" in feedback surveys

### Key Deliverables

1. Format auto-detection in CLI commands
2. Enhanced `validate` command with adapter support
3. Enhanced `gate` command with adapter support
4. New `export` command for format conversion
5. Evidence bundle integration with SpecKit format
6. Comprehensive documentation and examples

### Timeline

- **Week 6 (Current Sprint)**: Core CLI integration (auto-detection, validate,
  gate, export)
- **Week 7**: Evidence bundle integration and polish
- **Week 8**: Documentation, examples, and customer demo

---

## Problem Space

### User Research

**Primary Persona: SpecKit Developer**

**Profile**:

- Name: "Jamie" (composite from GitHub spec-kit users)
- Role: Senior Software Engineer at mid-sized tech company
- Context: Team uses GitHub's spec-kit for feature planning
- Pain Points:
  - AI-generated code lacks validation and governance
  - No automated quality gates for planning documents
  - Manual review process is time-consuming and inconsistent
  - No provenance trail for plan changes
  - Concerned about "vibe-coded" repositories going to production

**Jobs to be Done**:

1. "When I write a feature spec, I want automated validation so I catch issues
   early"
2. "When I submit a plan for review, I want evidence of quality so reviewers
   trust it"
3. "When using AI to help write plans, I want governance so outputs are
   production-ready"
4. "When collaborating with my team, I want to keep our existing format so
   there's no learning curve"

**Current Workflow**:

```bash
# Jamie's current process
1. Create spec.md, plan.md, tasks.md in feature branch
2. Manual review by team lead
3. PR comments back-and-forth
4. Update documents based on feedback
5. Repeat until approved
6. Merge to main
```

**Desired Workflow with Anvil**:

```bash
# Jamie's desired process
1. Create spec.md, plan.md, tasks.md in feature branch
2. Run: anvil gate spec.md
3. Fix issues identified by automated checks
4. Evidence appended to documents automatically
5. PR includes validation evidence
6. Team lead reviews pre-validated plan
7. Merge with confidence
```

**Secondary Persona: CLI Power User**

**Profile**:

- Comfortable with command-line tools
- Values automation and scriptability
- Wants clear, actionable error messages
- Expects commands to follow Unix philosophy (do one thing well)

**Expectations**:

- Fast startup time (<1 second)
- Streaming output for long operations
- Exit codes that reflect success/failure
- JSON output option for scripting
- Comprehensive help text

### Current Pain Points

#### Pain Point 1: Format Adoption Barrier

**Problem**: Anvil only works with APS format, which users have never heard of.

**Impact**:

- Immediate rejection: "Why should I learn another format?"
- Workflow disruption: Users must maintain two planning systems
- Lost opportunity: Can't showcase Anvil's value quickly

**Evidence**:

- ARCHITECTURE.md ADR-001: "Users won't adopt a new planning format"
- TODO.md strategic priority #1: "Interoperability First"

**Quote from Architecture**:

> "APS is internal only. Users never see it unless they want to. Adapters
> convert between user formats and APS."

#### Pain Point 2: Manual Format Conversion

**Problem**: No automated way to convert between SpecKit and APS.

**Impact**:

- Time waste: Manual conversion takes 15-30 minutes per plan
- Errors: Manual conversion introduces mistakes
- Barrier to experimentation: Users can't easily try Anvil

**User Story**:

> "I have 20 existing spec.md files. I'm not going to convert them all manually
> just to try a new tool."

#### Pain Point 3: Evidence Integration Gap

**Problem**: Gate validation produces evidence, but can't inject it back into
SpecKit documents.

**Impact**:

- Lost provenance: Evidence stored separately from source document
- Poor UX: Users must look in multiple places for validation results
- Review friction: PR reviewers don't see validation evidence inline

**Current State**: Gate evidence is appended to APS JSON, but SpecKit users
never see APS.

#### Pain Point 4: No Format Auto-detection

**Problem**: Users must explicitly specify format for every operation.

**Impact**:

- Extra typing: `--format=speckit` on every command
- Cognitive load: Users must remember format identifiers
- Error-prone: Wrong format specification causes cryptic errors

**Desired State**: `anvil gate spec.md` auto-detects format and just works.

### Opportunity

#### Strategic Opportunity: Wedge into Developer Market

**Thesis**: By supporting SpecKit (GitHub's official format), we eliminate the
primary adoption barrier and gain access to a large, influential user base.

**Market Data**:

- GitHub has 100M+ developers
- Spec-kit promoted as best practice for AI-assisted development
- Growing concern about "vibe-coded" repositories

**Competitive Advantage**: No other tool validates SpecKit documents with
governance and provenance.

#### Technical Opportunity: Showcase Adapter Architecture

**What We've Built**:

- Adapter framework (586 LOC, 22 tests, 100% passing)
- SpecKit adapter (2,469 LOC, 51 tests, 96% passing)
- Round-trip conversion with fidelity preservation

**What's Missing**: CLI integration to make it accessible to users.

**Unlock**: This feature makes all the adapter investment immediately useful.

#### Business Opportunity: Reference Customer

**Goal**: First 5 SpecKit teams using Anvil = proof of product-market fit.

**Path**:

1. Week 6: Ship CLI integration
2. Week 7: Demo to Customer #1 (SpecKit user)
3. Week 8: Onboard and gather feedback
4. Week 9: Case study and testimonial

**Impact**: Reference customers enable Act 1 fundraising narrative.

---

## Solution Overview

### High-Level Approach

**Core Principle**: Make Anvil work seamlessly with SpecKit without users
knowing about APS.

**Architecture**:

```
User Command: anvil gate spec.md
       ↓
[CLI] Detect format from content
       ↓
[Adapter] Parse SpecKit → APS
       ↓
[Core] Validate APS schema
       ↓
[Gate] Run quality checks
       ↓
[Adapter] Inject evidence → SpecKit
       ↓
[CLI] Display results
```

**Key Insight**: Format conversion is transparent. Users only interact with
their preferred format.

### Key Features

#### Feature 1: Format Auto-detection

**What**: CLI automatically detects document format by analyzing content.

**Why**: Eliminates need to specify `--format` flag.

**How**: Use AdapterRegistry.detectAdapter() with confidence scoring.

**User Experience**:

```bash
# Before (manual format specification)
$ anvil gate spec.md --format=speckit

# After (auto-detection)
$ anvil gate spec.md
✓ Detected format: SpecKit (95% confidence)
```

#### Feature 2: Enhanced Validate Command

**What**: `anvil validate` works with any supported format.

**Why**: Enables fast validation feedback loop.

**How**:

1. Load file
2. Auto-detect or use --format flag
3. Parse to APS via adapter
4. Validate APS schema
5. Return actionable feedback

**User Experience**:

```bash
$ anvil validate spec.md

✓ Detected: SpecKit v2.0.0 (98% confidence)
✓ Schema validation passed
✓ Hash verification passed

Plan Details:
  Format:        SpecKit
  User Scenarios: 3
  Requirements:   12
  Tasks:          24
  Success Criteria: 5

✓ Document is valid
```

#### Feature 3: Enhanced Gate Command

**What**: `anvil gate` runs quality checks on any supported format.

**Why**: Core value proposition - automated validation.

**How**:

1. Load and parse document
2. Convert to APS
3. Run gate checks (lint, test, coverage, secrets)
4. Collect evidence
5. Inject evidence back into source format
6. Display results

**User Experience**:

```bash
$ anvil gate spec.md

✓ Detected: SpecKit v2.0.0 (98% confidence)
✓ Plan loaded and validated

Running quality gates...
  ✓ Lint check passed (0 issues)
  ✓ Test check passed (24/24 tests)
  ✓ Coverage check passed (87% coverage, threshold: 80%)
  ✓ Secret scan passed (0 secrets detected)

Evidence updated in spec.md

✓ All quality gates passed!
```

#### Feature 4: Export Command

**What**: Convert between supported formats.

**Why**: Enable collaboration across teams using different formats.

**How**:

1. Load source document
2. Parse via source adapter
3. Convert to APS
4. Serialize via target adapter
5. Write output file

**User Experience**:

```bash
$ anvil export spec.md --to=aps --output=plan.json
✓ Converted SpecKit → APS
✓ Written to plan.json

$ anvil export plan.json --to=speckit --output=exported/
✓ Converted APS → SpecKit
✓ Written 3 files:
  - exported/spec.md
  - exported/plan.md
  - exported/tasks.md
```

#### Feature 5: Evidence Bundle Integration

**What**: Validation evidence injected into SpecKit documents as markdown
comments.

**Why**: Keep evidence with source document for provenance.

**How**:

1. Gate produces evidence bundle
2. SpecKit adapter formats evidence as markdown
3. Evidence appended as HTML comment at end of document
4. Preserves document structure and formatting

**User Experience**:

spec.md after gate validation:

```markdown
# Feature: User Authentication

## User Scenarios

...

<!-- ANVIL VALIDATION EVIDENCE
Gate Run: 2025-10-20T14:30:00Z
Status: PASSED
Checks:
  - Lint: ✓ Passed (0 issues)
  - Tests: ✓ Passed (24/24)
  - Coverage: ✓ Passed (87%)
  - Secrets: ✓ Passed (0 detected)
Plan Hash: sha256:a1b2c3d4...
-->
```

### User Experience Flow

#### Flow 1: First-Time User Validates SpecKit Document

**Scenario**: Jamie has never used Anvil before, wants to try it on existing
spec.md

**Steps**:

1. Install Anvil: `npm install -g @anvil/cli`
2. Navigate to project: `cd my-project`
3. Run validation: `anvil validate spec.md`
4. See results immediately
5. Fix any issues
6. Run gate: `anvil gate spec.md`
7. See evidence appended to document
8. Commit and push

**Time**: <2 minutes from install to validated plan

**Outcome**: "That was easier than I expected!"

#### Flow 2: Power User Integrates into CI/CD

**Scenario**: Taylor wants automated gate checks in GitHub Actions

**Steps**:

1. Create workflow file: `.github/workflows/anvil-gate.yml`
2. Add step:
   ```yaml
   - name: Validate Plans
     run: anvil gate spec.md
   ```
3. Push to repository
4. PR automatically runs gate
5. Status check blocks merge on failure

**Time**: <5 minutes to integrate

**Outcome**: Automated governance without manual intervention

#### Flow 3: Team Lead Reviews PR with Evidence

**Scenario**: Alex reviews Jamie's PR with validation evidence

**Steps**:

1. Open PR on GitHub
2. See gate status check (✓ passed)
3. View spec.md in PR
4. Scroll to evidence section
5. See all checks passed with timestamps
6. Review changes knowing they're validated
7. Approve with confidence

**Time**: Review time reduced 50% vs manual validation

**Outcome**: Faster reviews with higher confidence

---

## User Stories & Acceptance Criteria

### Epic: Format Auto-detection

#### Story 1.1: Detect SpecKit Format

**As a** SpecKit user **I want** Anvil to automatically detect my spec.md format
**So that** I don't have to specify --format on every command

**Acceptance Criteria**:

- **Given** a valid spec.md file with SpecKit format markers
- **When** I run `anvil validate spec.md` without --format flag
- **Then** CLI detects SpecKit with >90% confidence
- **And** processes file as SpecKit format
- **And** displays detected format in output

**Additional Criteria**:

- Detection works for spec.md, plan.md, and tasks.md
- Detection confidence shown in verbose mode
- User can override with --format flag
- Ambiguous detection prompts user for confirmation

#### Story 1.2: Handle Unknown Formats

**As a** user with a non-standard format **I want** clear error messages when
format can't be detected **So that** I understand what to do next

**Acceptance Criteria**:

- **Given** a file that doesn't match any adapter pattern
- **When** I run `anvil validate unknown.md`
- **Then** CLI displays error: "Could not detect format"
- **And** suggests: "Supported formats: speckit, aps"
- **And** suggests: "Use --format flag to specify manually"
- **And** exits with code 1

#### Story 1.3: Handle Format Conflicts

**As a** user with ambiguous content **I want** to be prompted when multiple
formats are possible **So that** I can choose the correct one

**Acceptance Criteria**:

- **Given** content that matches multiple adapters with similar confidence
- **When** I run `anvil validate ambiguous.md`
- **Then** CLI displays detected formats with confidence scores
- **And** prompts: "Multiple formats detected. Which format is this?"
- **And** allows me to select from options
- **And** remembers choice for this file (optional .anvilrc cache)

### Epic: Validate Command Enhancement

#### Story 2.1: Validate SpecKit Document

**As a** SpecKit user **I want** to validate my spec.md file **So that** I know
it's properly formatted before submission

**Acceptance Criteria**:

- **Given** a valid spec.md file
- **When** I run `anvil validate spec.md`
- **Then** document is parsed to APS successfully
- **And** APS schema validation passes
- **And** hash verification passes
- **And** success message displays with plan details
- **And** exit code is 0

**Additional Criteria**:

- Displays format detection result
- Shows user scenarios count
- Shows requirements count
- Shows tasks count
- Verbose mode shows full parse tree

#### Story 2.2: Validate with Errors

**As a** SpecKit user **I want** clear error messages when validation fails **So
that** I can fix issues quickly

**Acceptance Criteria**:

- **Given** an invalid spec.md file (missing required sections)
- **When** I run `anvil validate spec.md`
- **Then** validation fails with clear error messages
- **And** each error includes:
  - What's wrong
  - Where in document (line number if available)
  - How to fix it
- **And** exit code is 1

**Example Output**:

```
✗ Validation failed

Errors found:
  1. Missing required section: "User Scenarios & Testing"
     Location: spec.md
     Fix: Add ## User Scenarios & Testing section

  2. Invalid requirement format: "Login feature"
     Location: spec.md, line 42
     Fix: Requirements must start with "REQ-" or "FR-"

See documentation: https://anvil.dev/docs/speckit-format
```

#### Story 2.3: Validate with Format Conversion

**As a** user **I want** to see how my SpecKit document converts to APS **So
that** I understand the internal representation

**Acceptance Criteria**:

- **Given** a valid spec.md file
- **When** I run `anvil validate spec.md --show-aps`
- **Then** validation succeeds
- **And** displays converted APS JSON
- **And** shows mapping (SpecKit sections → APS fields)

### Epic: Gate Command Enhancement

#### Story 3.1: Run Gate on SpecKit

**As a** SpecKit user **I want** to run quality gates on my spec.md **So that**
my plan meets quality standards

**Acceptance Criteria**:

- **Given** a valid spec.md and passing gate checks
- **When** I run `anvil gate spec.md`
- **Then** all checks execute (lint, test, coverage, secrets)
- **And** results displayed in formatted table
- **And** evidence injected into spec.md
- **And** exit code is 0

**Output Format**:

```
✓ Detected: SpecKit v2.0.0 (98% confidence)
✓ Plan loaded and validated

Running quality gates...
┌──────────┬────────┬──────────────────┐
│ Check    │ Status │ Details          │
├──────────┼────────┼──────────────────┤
│ Lint     │ ✓ Pass │ 0 issues         │
│ Tests    │ ✓ Pass │ 24/24 tests      │
│ Coverage │ ✓ Pass │ 87% (≥80%)       │
│ Secrets  │ ✓ Pass │ 0 detected       │
└──────────┴────────┴──────────────────┘

Evidence updated in spec.md

✓ All quality gates passed!
```

#### Story 3.2: Gate with Failures

**As a** SpecKit user **I want** to see which gates failed and why **So that** I
can fix issues

**Acceptance Criteria**:

- **Given** a spec.md with failing lint checks
- **When** I run `anvil gate spec.md`
- **Then** gate runs all checks (don't fail fast)
- **And** displays failed checks with details
- **And** evidence includes failure information
- **And** exit code is 1

**Output Format**:

```
✓ Detected: SpecKit v2.0.0
✓ Plan loaded

Running quality gates...
┌──────────┬────────┬─────────────────────────────┐
│ Check    │ Status │ Details                     │
├──────────┼────────┼─────────────────────────────┤
│ Lint     │ ✗ Fail │ 3 errors, 2 warnings        │
│ Tests    │ ✓ Pass │ 24/24 tests                 │
│ Coverage │ ✗ Fail │ 72% (expected ≥80%)         │
│ Secrets  │ ✓ Pass │ 0 detected                  │
└──────────┴────────┴─────────────────────────────┘

Lint errors:
  1. src/auth.ts:42 - Unused variable 'token'
  2. src/api.ts:15 - Missing return type
  3. src/db.ts:88 - Prefer const over let

Coverage details:
  Overall: 72% (target: 80%)
  Files below threshold:
    - src/auth.ts: 65%
    - src/api.ts: 58%

Evidence updated in spec.md

✗ Quality gates failed
```

#### Story 3.3: Gate with Evidence Injection

**As a** SpecKit user **I want** evidence appended to my spec.md **So that** PR
reviewers see validation results

**Acceptance Criteria**:

- **Given** a spec.md file
- **When** I run `anvil gate spec.md` successfully
- **Then** evidence appended as HTML comment at end
- **And** evidence includes:
  - Timestamp
  - Overall status
  - Individual check results
  - Plan hash
  - Anvil version
- **And** original document structure preserved
- **And** running gate again replaces old evidence

**Evidence Format**:

```markdown
<!-- ANVIL VALIDATION EVIDENCE
Generated: 2025-10-20T14:30:00Z
Anvil Version: 1.0.0
Format: SpecKit v2.0.0

Status: PASSED

Quality Gates:
  ✓ Lint:     Passed (0 issues)
  ✓ Tests:    Passed (24/24 tests)
  ✓ Coverage: Passed (87%, threshold 80%)
  ✓ Secrets:  Passed (0 detected)

Plan Hash: sha256:a1b2c3d4e5f6789...
Evidence Hash: sha256:1a2b3c4d5e6f789...

Full details: .anvil/evidence/aps-a1b2c3d4/gate-001.json
-->
```

### Epic: Export Command

#### Story 4.1: Export SpecKit to APS

**As a** power user **I want** to export my spec.md to APS format **So that** I
can see the internal representation

**Acceptance Criteria**:

- **Given** a valid spec.md file
- **When** I run `anvil export spec.md --to=aps --output=plan.json`
- **Then** SpecKit parsed to APS
- **And** APS JSON written to plan.json
- **And** success message displays
- **And** exit code is 0

**Additional Criteria**:

- --output defaults to <filename>.aps.json
- --to=json and --to=yaml both work
- Pretty-printed JSON by default
- --compact flag for minified JSON

#### Story 4.2: Export APS to SpecKit

**As a** user with APS format **I want** to export to SpecKit **So that** I can
share with SpecKit users

**Acceptance Criteria**:

- **Given** a valid APS plan.json
- **When** I run `anvil export plan.json --to=speckit --output=exported/`
- **Then** APS serialized to SpecKit format
- **And** writes 3 files:
  - exported/spec.md
  - exported/plan.md
  - exported/tasks.md
- **And** files contain all APS data
- **And** success message lists created files
- **And** exit code is 0

**Additional Criteria**:

- --output defaults to current directory
- Can specify single file: --output=spec.md (creates just spec.md)
- Warns if data loss during conversion

#### Story 4.3: Export with Format Auto-detection

**As a** user **I want** source format auto-detected during export **So that** I
only need to specify target format

**Acceptance Criteria**:

- **Given** a spec.md file
- **When** I run `anvil export spec.md --to=aps`
- **Then** source format auto-detected as SpecKit
- **And** converts SpecKit → APS
- **And** writes output file
- **And** displays: "Detected source: SpecKit"

### Epic: Evidence Bundle Integration

#### Story 5.1: Preserve Document Structure

**As a** SpecKit user **I want** evidence injection to preserve my formatting
**So that** my document doesn't get corrupted

**Acceptance Criteria**:

- **Given** a spec.md with custom formatting and comments
- **When** I run `anvil gate spec.md`
- **Then** evidence appended as HTML comment
- **And** all existing content preserved exactly
- **And** no whitespace changes in original content
- **And** custom comments preserved
- **And** diff shows only evidence addition

#### Story 5.2: Update Existing Evidence

**As a** SpecKit user **I want** old evidence replaced when I run gate again
**So that** I don't accumulate stale evidence

**Acceptance Criteria**:

- **Given** a spec.md with existing Anvil evidence
- **When** I run `anvil gate spec.md`
- **Then** old evidence removed
- **And** new evidence appended
- **And** only one evidence block exists
- **And** evidence block is always at end of file

#### Story 5.3: Link to Detailed Evidence

**As a** power user **I want** evidence comment to link to full details **So
that** I can access complete audit trail

**Acceptance Criteria**:

- **Given** evidence appended to spec.md
- **When** I view the evidence comment
- **Then** it includes path to full evidence JSON
- **And** path is: `.anvil/evidence/[plan-id]/gate-[number].json`
- **And** full evidence includes:
  - Complete check outputs
  - Timestamps
  - Provenance
  - Plan snapshots

---

## Functional Requirements

### FR-1: Format Detection

**Priority**: P0 (Critical)

**Requirements**:

1. **FR-1.1**: CLI MUST auto-detect document format before processing
2. **FR-1.2**: Detection MUST use AdapterRegistry.detectAdapter() with content
   analysis
3. **FR-1.3**: Detection MUST return confidence score (0-100)
4. **FR-1.4**: CLI MUST accept formats with confidence ≥50%
5. **FR-1.5**: CLI MUST allow --format flag to override detection
6. **FR-1.6**: CLI MUST display detected format in output (verbose mode)
7. **FR-1.7**: CLI MUST handle detection failures gracefully with helpful errors

**Implementation Notes**:

- Use registry.detectAdapter(content, minConfidence=50)
- Cache detection results per file (optional optimisation)
- Support file extension hints for ambiguous content

### FR-2: Validate Command

**Priority**: P0 (Critical)

**Requirements**:

1. **FR-2.1**: validate MUST accept file path or plan ID as argument
2. **FR-2.2**: validate MUST auto-detect format or use --format flag
3. **FR-2.3**: validate MUST parse content via appropriate adapter
4. **FR-2.4**: validate MUST run APS schema validation
5. **FR-2.5**: validate MUST verify plan hash integrity
6. **FR-2.6**: validate MUST display validation results clearly
7. **FR-2.7**: validate MUST exit with code 0 on success, 1 on failure
8. **FR-2.8**: validate MUST support --verbose flag for detailed output
9. **FR-2.9**: validate MUST support --show-aps flag to display APS conversion

**Error Handling**:

- File not found: Clear error with path
- Parse failure: Show adapter errors with line numbers
- Schema failure: Show Zod errors formatted for CLI
- Hash failure: Warn about potential tampering

### FR-3: Gate Command

**Priority**: P0 (Critical)

**Requirements**:

1. **FR-3.1**: gate MUST accept file path or plan ID as argument
2. **FR-3.2**: gate MUST auto-detect format or use --format flag
3. **FR-3.3**: gate MUST parse and validate plan before running checks
4. **FR-3.4**: gate MUST run all configured checks (lint, test, coverage,
   secrets)
5. **FR-3.5**: gate MUST collect evidence from all checks
6. **FR-3.6**: gate MUST inject evidence into source document via adapter
7. **FR-3.7**: gate MUST display results in formatted table
8. **FR-3.8**: gate MUST exit with code 0 if all pass, 1 if any fail
9. **FR-3.9**: gate MUST support --checks flag to select specific checks
10. **FR-3.10**: gate MUST support --skip flag to exclude checks

**Evidence Injection**:

- Use adapter.injectEvidence() method
- Preserve original formatting
- Replace old evidence if present
- Write updated content back to file

### FR-4: Export Command

**Priority**: P0 (Critical)

**Requirements**:

1. **FR-4.1**: export MUST accept source file path as argument
2. **FR-4.2**: export MUST require --to flag specifying target format
3. **FR-4.3**: export MUST support --output flag for destination path
4. **FR-4.4**: export MUST auto-detect source format or use --from flag
5. **FR-4.5**: export MUST parse via source adapter
6. **FR-4.6**: export MUST serialize via target adapter
7. **FR-4.7**: export MUST write output file(s)
8. **FR-4.8**: export MUST display success message with file paths
9. **FR-4.9**: export MUST exit with code 0 on success, 1 on failure
10. **FR-4.10**: export MUST warn if conversion may lose data

**Supported Conversions**:

- SpecKit → APS (JSON/YAML)
- APS → SpecKit (multi-file)
- SpecKit → SpecKit (validation and formatting)

### FR-5: Evidence Bundle Format

**Priority**: P0 (Critical)

**Requirements**:

1. **FR-5.1**: Evidence MUST be appended as HTML comment in SpecKit
2. **FR-5.2**: Evidence MUST include timestamp in ISO 8601 format
3. **FR-5.3**: Evidence MUST include Anvil version
4. **FR-5.4**: Evidence MUST include overall status (PASSED/FAILED)
5. **FR-5.5**: Evidence MUST include individual check results
6. **FR-5.6**: Evidence MUST include plan hash for verification
7. **FR-5.7**: Evidence MUST include link to full evidence JSON
8. **FR-5.8**: Evidence MUST be parseable by future Anvil versions
9. **FR-5.9**: Evidence MUST not break document rendering
10. **FR-5.10**: Old evidence MUST be replaced on subsequent gate runs

**Format Specification**:

```
<!-- ANVIL VALIDATION EVIDENCE
[key]: [value]
...
-->
```

### FR-6: Configuration

**Priority**: P1 (High)

**Requirements**:

1. **FR-6.1**: CLI MUST support .anvilrc configuration file
2. **FR-6.2**: Configuration MUST support default format preference
3. **FR-6.3**: Configuration MUST support gate check settings
4. **FR-6.4**: Configuration MUST support adapter-specific options
5. **FR-6.5**: Configuration MUST be validated on load
6. **FR-6.6**: Command-line flags MUST override configuration

**Configuration Format** (.anvilrc):

```json
{
  "defaultFormat": "speckit",
  "gate": {
    "checks": ["lint", "test", "coverage", "secrets"],
    "coverage": {
      "threshold": 80
    }
  },
  "adapters": {
    "speckit": {
      "preserveComments": true,
      "version": "2.0.0"
    }
  }
}
```

### FR-7: Help and Documentation

**Priority**: P1 (High)

**Requirements**:

1. **FR-7.1**: All commands MUST have --help flag
2. **FR-7.2**: Help text MUST include examples
3. **FR-7.3**: Help text MUST list all flags and options
4. **FR-7.4**: Error messages MUST suggest fixes
5. **FR-7.5**: CLI MUST include --version flag
6. **FR-7.6**: CLI MUST link to online documentation

**Help Text Example**:

```
anvil gate <plan>

Run quality gates on a plan

Arguments:
  plan                  Path to plan file or plan ID

Options:
  -f, --format <format>   Specify format (auto-detected by default)
  -c, --config <path>     Custom config file path
  --checks <checks>       Comma-separated list of checks to run
  --skip <checks>         Comma-separated list of checks to skip
  -v, --verbose           Verbose output
  -h, --help              Show this help

Examples:
  # Auto-detect format and run all checks
  $ anvil gate spec.md

  # Run only lint and test checks
  $ anvil gate spec.md --checks=lint,test

  # Use custom config
  $ anvil gate spec.md --config=.anvil/custom.json

Supported formats: speckit, aps

Documentation: https://anvil.dev/docs/cli/gate
```

---

## Non-Functional Requirements

### NFR-1: Performance

**Requirements**:

1. **NFR-1.1**: CLI startup time MUST be <1 second (cold start)
2. **NFR-1.2**: Format detection MUST complete in <100ms for typical files
   (<100KB)
3. **NFR-1.3**: Validate command MUST complete in <2 seconds for typical plans
4. **NFR-1.4**: Gate command MUST complete in <2 minutes for typical
   repositories
5. **NFR-1.5**: Export command MUST process at <1 second per 10KB of content
6. **NFR-1.6**: Memory usage MUST stay under 200MB for typical operations
7. **NFR-1.7**: CLI MUST support files up to 10MB

**Measurement**: Use performance benchmarks in CI with sample files.

### NFR-2: Reliability

**Requirements**:

1. **NFR-2.1**: CLI MUST NOT corrupt user files under any circumstance
2. **NFR-2.2**: CLI MUST create backup before modifying files (optional:
   --no-backup)
3. **NFR-2.3**: CLI MUST handle interrupts gracefully (Ctrl+C)
4. **NFR-2.4**: CLI MUST validate all outputs before writing
5. **NFR-2.5**: CLI MUST provide atomic file operations (write to temp, then
   rename)
6. **NFR-2.6**: Failed operations MUST leave system in consistent state
7. **NFR-2.7**: CLI MUST log errors for debugging

**Error Recovery**:

- Write to temporary file first
- Verify content before replacing original
- On error, preserve original file
- Provide clear rollback instructions

### NFR-3: Usability

**Requirements**:

1. **NFR-3.1**: Error messages MUST be clear and actionable
2. **NFR-3.2**: Success messages MUST confirm what happened
3. **NFR-3.3**: Progress indicators MUST show for operations >2 seconds
4. **NFR-3.4**: Output MUST use colour and formatting for readability
5. **NFR-3.5**: CLI MUST support NO_COLOR environment variable
6. **NFR-3.6**: CLI MUST support --json flag for machine-readable output
7. **NFR-3.7**: Commands MUST follow standard Unix conventions

**Design Principles**:

- Colorize success (green), errors (red), warnings (yellow)
- Use spinners for long operations
- Display tables for structured data
- Provide progress bars for multi-step operations

### NFR-4: Compatibility

**Requirements**:

1. **NFR-4.1**: CLI MUST work on Node.js ≥18.0.0
2. **NFR-4.2**: CLI MUST work on Linux, macOS, and Windows
3. **NFR-4.3**: CLI MUST handle different line endings (LF, CRLF)
4. **NFR-4.4**: CLI MUST support UTF-8 encoding
5. **NFR-4.5**: CLI MUST work in CI environments (non-interactive)
6. **NFR-4.6**: CLI MUST work with different shells (bash, zsh, fish,
   PowerShell)

**Testing**: Run integration tests on all platforms in CI.

### NFR-5: Security

**Requirements**:

1. **NFR-5.1**: CLI MUST validate all file paths to prevent path traversal
2. **NFR-5.2**: CLI MUST NOT execute arbitrary code from plans
3. **NFR-5.3**: CLI MUST sanitize all user inputs
4. **NFR-5.4**: CLI MUST NOT log sensitive data (secrets, tokens)
5. **NFR-5.5**: Evidence injection MUST escape all content properly
6. **NFR-5.6**: CLI MUST verify file permissions before writing

**Security Review**: Required before first release.

### NFR-6: Maintainability

**Requirements**:

1. **NFR-6.1**: Code coverage MUST be ≥90% for CLI integration
2. **NFR-6.2**: All commands MUST have integration tests
3. **NFR-6.3**: All adapters MUST have round-trip tests
4. **NFR-6.4**: Documentation MUST be updated with code changes
5. **NFR-6.5**: Error paths MUST have explicit tests
6. **NFR-6.6**: TypeScript strict mode MUST be enabled

**Code Quality**: Enforced by existing Anvil standards (ESLint, Prettier,
Husky).

---

## User Experience

### Command-Line Workflows

#### Workflow 1: Quick Validation

**Goal**: Validate a SpecKit document quickly

**Commands**:

```bash
$ anvil validate spec.md
```

**Output**:

```
✓ Detected: SpecKit v2.0.0 (98% confidence)
✓ Schema validation passed
✓ Hash verification passed

Plan Details:
  Format:        SpecKit
  User Scenarios: 3
  Requirements:   12
  Tasks:          24

✓ Document is valid

Time: 0.8s
```

**User Reaction**: "That was fast and clear!"

#### Workflow 2: Quality Gate with Fixes

**Goal**: Run gate, see failures, fix issues, re-run

**Commands**:

```bash
$ anvil gate spec.md
```

**Output (first run)**:

```
✓ Detected: SpecKit v2.0.0
✓ Plan loaded

Running quality gates...
┌──────────┬────────┬─────────────────────────────┐
│ Check    │ Status │ Details                     │
├──────────┼────────┼─────────────────────────────┤
│ Lint     │ ✗ Fail │ 3 errors                    │
│ Tests    │ ✓ Pass │ 24/24 tests                 │
│ Coverage │ ✗ Fail │ 72% (expected ≥80%)         │
│ Secrets  │ ✓ Pass │ 0 detected                  │
└──────────┴────────┴─────────────────────────────┘

Lint errors:
  src/auth.ts:42 - Unused variable 'token'
  src/api.ts:15 - Missing return type
  src/db.ts:88 - Prefer const over let

Coverage details:
  src/auth.ts: 65% (15% below threshold)
  src/api.ts: 58% (22% below threshold)

✗ Quality gates failed

Fix issues and re-run: anvil gate spec.md
```

**User Action**: Fix lint errors, add tests

**Commands**:

```bash
$ # Fix issues...
$ anvil gate spec.md
```

**Output (second run)**:

```
✓ Detected: SpecKit v2.0.0
✓ Plan loaded

Running quality gates...
┌──────────┬────────┬─────────────────────────────┐
│ Check    │ Status │ Details                     │
├──────────┼────────┼─────────────────────────────┤
│ Lint     │ ✓ Pass │ 0 issues                    │
│ Tests    │ ✓ Pass │ 26/26 tests (+2 new)        │
│ Coverage │ ✓ Pass │ 84% (+12%)                  │
│ Secrets  │ ✓ Pass │ 0 detected                  │
└──────────┴────────┴─────────────────────────────┘

Evidence updated in spec.md

✓ All quality gates passed!

Time: 1m 23s
```

**User Reaction**: "Clear feedback helped me fix issues quickly!"

#### Workflow 3: Format Conversion

**Goal**: Convert SpecKit to APS for inspection

**Commands**:

```bash
$ anvil export spec.md --to=aps --output=plan.json
$ cat plan.json
```

**Output**:

```
✓ Detected: SpecKit v2.0.0 (98% confidence)
✓ Parsing spec.md, plan.md, tasks.md
✓ Converted to APS
✓ Written to plan.json

View with: cat plan.json
Validate with: anvil validate plan.json
```

**User Reaction**: "Easy to see internal representation!"

#### Workflow 4: CI Integration

**Goal**: Set up automated gate checks in GitHub Actions

**File**: `.github/workflows/anvil-gate.yml`

```yaml
name: Anvil Quality Gates

on:
  pull_request:
    paths:
      - 'spec.md'
      - 'plan.md'
      - 'tasks.md'

jobs:
  validate:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3

      - name: Install Anvil
        run: npm install -g @anvil/cli

      - name: Validate Plan
        run: anvil validate spec.md

      - name: Run Quality Gates
        run: anvil gate spec.md

      - name: Upload Evidence
        if: always()
        uses: actions/upload-artifact@v3
        with:
          name: gate-evidence
          path: .anvil/evidence/
```

**User Reaction**: "Set up once, automatic validation forever!"

### Error Handling

#### Error Scenario 1: File Not Found

**Command**:

```bash
$ anvil validate missing.md
```

**Output**:

```
✗ Error: File not found

  Could not find: missing.md

  Did you mean one of these?
    - spec.md
    - plan.md

  See help: anvil validate --help
```

**Design**: Helpful suggestions, not just "file not found"

#### Error Scenario 2: Parse Failure

**Command**:

```bash
$ anvil validate malformed.md
```

**Output**:

```
✗ Error: Failed to parse document

  Format: SpecKit (detected with 85% confidence)

  Issues found:
    1. Missing required section: "User Scenarios & Testing"
       Location: malformed.md
       Expected: ## User Scenarios & Testing

    2. Invalid requirement format on line 42
       Found: "Login feature"
       Expected: "FR-001: Login feature" (must start with FR-)

  Documentation: https://anvil.dev/docs/speckit-format
  Need help? https://anvil.dev/support
```

**Design**: Specific errors with line numbers and fix suggestions

#### Error Scenario 3: Gate Failures

**Command**:

```bash
$ anvil gate spec.md
```

**Output**:

```
✓ Detected: SpecKit v2.0.0
✓ Plan loaded

Running quality gates...
┌──────────┬────────┬─────────────────────────────┐
│ Check    │ Status │ Details                     │
├──────────┼────────┼─────────────────────────────┤
│ Lint     │ ✓ Pass │ 0 issues                    │
│ Tests    │ ✗ Fail │ 2/24 failed                 │
│ Coverage │ ⚠ Warn │ 79% (target 80%)            │
│ Secrets  │ ✓ Pass │ 0 detected                  │
└──────────┴────────┴─────────────────────────────┘

Failed tests:
  1. src/auth.test.ts: should validate token expiry
     Error: Expected 401, got 200

  2. src/api.test.ts: should handle rate limiting
     Error: Timeout after 5000ms

Coverage summary:
  Overall: 79% (1% below threshold)
  Almost there! Add 1-2 tests to reach 80%

✗ Quality gates failed

Fix failing tests and re-run: anvil gate spec.md
Documentation: https://anvil.dev/docs/gate-checks
```

**Design**: Actionable feedback with next steps

### Output Formatting

#### Success Output

**Colors**:

- Green: Success messages, checkmarks
- Cyan: Plan details, metadata
- White: Normal text

**Example**:

```
✓ Document is valid  [GREEN]

Plan Details:
  Format:        SpecKit  [CYAN]
  User Scenarios: 3  [CYAN]
```

#### Error Output

**Colors**:

- Red: Error messages, failures
- Yellow: Warnings
- Gray: Context and suggestions

**Example**:

```
✗ Quality gates failed  [RED]

Failed tests:  [RED]
  1. src/auth.test.ts  [YELLOW]
     Error: Expected 401, got 200  [GRAY]
```

#### Progress Indicators

**Spinners** (for operations >2 seconds):

```
⠋ Loading plan...
⠙ Running quality gates...
⠹ Analyzing coverage...
```

**Progress Bars** (for multi-step operations):

```
Running checks [████████████░░░░░░░░] 60% (3/5 checks)
```

---

## Success Metrics

### User Adoption Metrics

**Primary Metrics**:

1. **Time to First Validation**: <30 seconds from install to successful
   `anvil validate`
   - Target: 80% of users validate on first attempt
   - Measure: Telemetry from first-run experience

2. **Command Success Rate**: Successful command executions / total attempts
   - Target: >95% success rate for valid inputs
   - Measure: Exit code tracking in telemetry

3. **Format Detection Accuracy**: Correct format detected / total detections
   - Target: >98% accuracy for SpecKit documents
   - Measure: Detection confidence logs

**Secondary Metrics**:

4. **Retry Rate**: Users re-running commands after failures
   - Target: <2 retries average to success
   - Measure: Session tracking

5. **Help Access Rate**: --help flag usage / total command runs
   - Target: <10% (indicates clear UX)
   - Measure: Command flag tracking

6. **Error Recovery Rate**: Successful runs after errors / total errors
   - Target: >80% (errors lead to fixes)
   - Measure: Session success after failure

### Business Metrics

**Activation Metrics**:

1. **Weekly Active Users**: Users running any Anvil command per week
   - Target Week 8: 20 users
   - Target Week 12: 50 users

2. **Retention Rate**: Users active in Week N who return in Week N+1
   - Target: >60% week-over-week retention

3. **Feature Adoption**: % of users using each command
   - validate: Target >90%
   - gate: Target >70%
   - export: Target >30%

**Quality Metrics**:

4. **Gate Pass Rate**: Gate runs that pass / total gate runs
   - Baseline: Track to understand typical workflows
   - Should increase over time as users improve plans

5. **Evidence Adoption**: Gate runs with evidence injection / total gate runs
   - Target: 100% (should always inject evidence)

6. **Format Distribution**: Usage by format
   - SpecKit: Target >60% of commands
   - APS: Target <40% (mostly power users)

### Technical Metrics

**Performance Metrics**:

1. **P95 Latency**:
   - validate: <2 seconds
   - gate: <2 minutes
   - export: <5 seconds

2. **Error Rate**: Commands with errors / total commands
   - Target: <5% error rate

3. **Crash Rate**: Unexpected exits / total command runs
   - Target: <0.1% crash rate

**Quality Metrics**:

4. **Test Coverage**: Lines covered / total lines
   - Target: >90% for CLI integration code

5. **Bug Density**: Bugs reported / 1000 lines of code
   - Target: <1 bug per 1000 LOC

### User Satisfaction Metrics

**Qualitative Metrics**:

1. **Net Promoter Score (NPS)**: "Would you recommend Anvil?"
   - Target: NPS >50
   - Measure: Post-validation survey

2. **User Satisfaction Score**: "How satisfied are you with this feature?"
   - Target: >4.0/5.0 average
   - Measure: In-app feedback

3. **Time Saved**: "How much time did Anvil save you?"
   - Target: >30 minutes saved per week (self-reported)
   - Measure: User surveys

**Feedback Metrics**:

4. **Issue Reports**: Bug reports and feature requests
   - Track: Volume and severity
   - Target: <5 critical bugs in first month

5. **Documentation Quality**: "Did documentation help you?"
   - Target: >80% say "yes"
   - Measure: Doc feedback form

---

## Out of Scope

### Explicitly NOT Included in This Release

#### 1. BMAD Adapter Integration

**Rationale**: BMAD adapter planned for weeks 7-8. This PRD focuses on SpecKit
only.

**Future Work**: Separate PRD for BMAD integration, using same patterns
established here.

#### 2. Policy Engine (OPA/Rego) Integration

**Rationale**: Policy engine is Phase 5 work. This release uses basic gate
checks only.

**Future Work**: Policy-based validation will be added after CLI integration is
stable.

#### 3. Web Dashboard / UI

**Rationale**: CLI-first approach. Web UI is post-MVP.

**Future Work**: React dashboard for plan approval and visualization (Act 1
expansion).

#### 4. Advanced Evidence Features

**Not Included**:

- Cryptographic signatures on evidence
- Evidence encryption
- Evidence tamper detection (beyond hash verification)
- Multi-user evidence trails

**Rationale**: Core evidence injection is sufficient for MVP. Advanced features
come later.

#### 5. GitHub Action Integration

**Rationale**: Separate feature, separate PRD. Will build on this CLI
integration.

**Future Work**: Week 9-10, after CLI stabilizes.

#### 6. Interactive Plan Creation

**Not Included**:

- `anvil plan --interactive` with prompts
- AI-assisted plan generation
- Template-based plan creation

**Rationale**: Read-only operations (validate, gate, export) establish value
first.

**Future Work**: Plan creation features in next sprint.

#### 7. Advanced Format Detection

**Not Included**:

- Machine learning-based format detection
- Format detection from partial files
- Detection confidence learning/improvement

**Rationale**: Rule-based detection sufficient for v1.

**Future Work**: ML-based detection if needed based on user feedback.

#### 8. Multi-File Plan Support

**Not Included**:

- Plans spanning multiple repositories
- Monorepo support with multiple plans
- Plan dependencies and composition

**Rationale**: Single-plan workflows cover 90% of use cases.

**Future Work**: Enterprise features in Act 2.

#### 9. Rollback/Apply Integration with Adapters

**Not Included**:

- Apply changes from SpecKit documents
- Rollback operations
- Dry-run preview

**Rationale**: These features depend on Sidecar (Phase 6-7), which isn't ready
yet.

**Future Work**: Weeks 9-11, sidecar development.

#### 10. Advanced CLI Features

**Not Included**:

- Shell completion (bash/zsh/fish)
- CLI plugins/extensions
- Custom check development
- Anvil CLI scripting API

**Rationale**: Core commands provide foundation. Advanced CLI features come
after adoption.

**Future Work**: Based on user requests and use cases.

---

## Dependencies & Risks

### Dependencies

#### Critical Dependencies (Blockers)

**DEP-1: Adapter Framework Complete** ✅ DONE

- **Status**: Complete (586 LOC, 22 tests, 100% passing)
- **Owner**: Core team
- **Risk**: None (already done)

**DEP-2: SpecKit Adapter Complete** ✅ DONE

- **Status**: Complete (2,469 LOC, 51 tests, 49 passing, 2 minor fixes pending)
- **Owner**: Core team
- **Risk**: Low (98% complete, only minor fixes needed)
- **Mitigation**: Fix 2 failing tests in week 6 sprint

**DEP-3: APS Core Complete** ✅ DONE

- **Status**: Complete (validation, hashing, schema all done)
- **Owner**: Core team
- **Risk**: None (already done)

**DEP-4: Gate v1 Complete** ✅ DONE

- **Status**: Complete (lint, test, coverage, secrets checks)
- **Owner**: Core team
- **Risk**: None (already done)

#### High-Priority Dependencies

**DEP-5: CLI Infrastructure**

- **Status**: Partially complete (Commander.js setup, basic commands)
- **What's Needed**: Adapter integration, enhanced error handling
- **Timeline**: Week 6 (current sprint)
- **Risk**: Low (infrastructure exists, needs enhancement)

**DEP-6: File I/O Utilities**

- **Status**: Basic utilities exist (loadPlan, findPlanById)
- **What's Needed**: Support for multi-file SpecKit loads
- **Timeline**: Week 6
- **Risk**: Low (straightforward implementation)

**DEP-7: Output Formatting**

- **Status**: Basic output utilities exist
- **What's Needed**: Table formatting, evidence display
- **Timeline**: Week 6
- **Risk**: Low (libraries available: cli-table3, chalk)

#### Nice-to-Have Dependencies

**DEP-8: Configuration System**

- **Status**: Basic config exists (.anvilrc)
- **What's Needed**: Adapter-specific configuration
- **Timeline**: Week 7 (can ship without it)
- **Risk**: None (optional enhancement)

**DEP-9: Telemetry**

- **Status**: Not implemented
- **What's Needed**: Usage tracking for metrics
- **Timeline**: Week 8
- **Risk**: None (ship without telemetry, add later)

### Risks

#### High-Risk Items

**RISK-1: SpecKit Adapter Stability**

**Risk**: 2 tests still failing in SpecKit adapter, may indicate deeper issues

**Impact**:

- HIGH: Can't ship CLI integration if adapter is broken
- Blocks entire feature

**Probability**: Low (tests are minor edge cases)

**Mitigation**:

- Week 6 Day 1: Fix failing tests
- Add additional integration tests
- Manual testing with real SpecKit documents
- Customer validation with actual files

**Owner**: Adapter team lead

**Status**: In progress

**RISK-2: Evidence Injection Corrupts Documents**

**Risk**: Injecting evidence might break SpecKit document structure or rendering

**Impact**:

- CRITICAL: User documents corrupted = loss of trust
- Potential data loss

**Probability**: Medium (complex string manipulation)

**Mitigation**:

- Extensive testing with diverse SpecKit files
- Backup files before modification (--no-backup to disable)
- Atomic file writes (write to temp, verify, rename)
- Round-trip tests (inject evidence, parse again, verify no corruption)
- Manual review of evidence-injected files

**Owner**: CLI team lead

**Backup Plan**: Ship without evidence injection initially, add in week 7

**RISK-3: Format Detection False Positives**

**Risk**: Auto-detection identifies wrong format, causes confusing errors

**Impact**:

- MEDIUM: Poor user experience, increased support burden
- Users lose trust in "smart" detection

**Probability**: Medium (heuristics can fail on edge cases)

**Mitigation**:

- Conservative detection (require >90% confidence)
- Show detected format in output for user verification
- Allow --format override
- Prompt for confirmation if confidence <90%
- Log detection decisions for debugging
- Build corpus of test files (positive and negative cases)

**Owner**: Adapter framework team

**Backup Plan**: Require --format flag by default, make auto-detection opt-in

**RISK-4: Performance Degradation**

**Risk**: Gate checks take too long on large repositories

**Impact**:

- MEDIUM: Poor user experience
- Users abandon tool if too slow

**Probability**: Low (existing gate is fast enough)

**Mitigation**:

- Performance benchmarks in CI
- Optimize check execution (parallel where possible)
- Add progress indicators for long operations
- Document performance expectations
- Provide --checks flag to run subset of checks

**Owner**: Gate team

**Metrics**:

- P95 gate time <2 minutes for typical repos
- Warn users if operation will take >30 seconds

#### Medium-Risk Items

**RISK-5: Cross-Platform Compatibility Issues**

**Risk**: CLI works on macOS but fails on Windows or Linux

**Impact**:

- MEDIUM: Limits user base
- Support burden from platform-specific bugs

**Probability**: Low (Node.js handles most platform differences)

**Mitigation**:

- CI testing on Linux, macOS, Windows
- Handle path separators correctly (use path.join, path.resolve)
- Handle line endings (LF vs CRLF)
- Test in WSL and native Windows
- Use cross-platform libraries (cross-spawn, etc.)

**Owner**: CLI team

**Testing**: Integration tests on all platforms required

**RISK-6: Breaking Changes to SpecKit Format**

**Risk**: GitHub updates spec-kit format, breaks our adapter

**Impact**:

- MEDIUM: Existing users can't upgrade
- Need emergency adapter update

**Probability**: Low (spec-kit is stable)

**Mitigation**:

- Version detection in adapter (v1, v2, etc.)
- Monitor GitHub spec-kit repository for changes
- Deprecation warnings if old format detected
- Support multiple versions simultaneously
- Clear upgrade path documentation

**Owner**: Adapter team

**Monitoring**: Watch github.com/github/spec-kit for releases

**RISK-7: Unclear Error Messages**

**Risk**: Users get cryptic errors, don't know how to fix

**Impact**:

- MEDIUM: Poor UX, high support burden
- Users give up on tool

**Probability**: Medium (error messaging is hard)

**Mitigation**:

- User testing with real users (non-team members)
- Error message review in PRs
- Include suggestions in every error
- Link to documentation
- Add examples to help text
- Collect feedback on confusing errors

**Owner**: UX lead

**Testing**: User testing sessions in weeks 6-7

#### Low-Risk Items

**RISK-8: Documentation Gaps**

**Risk**: Users can't figure out how to use features

**Impact**:

- LOW: Support burden, slower adoption
- Mitigated by good help text

**Probability**: Medium (documentation always incomplete)

**Mitigation**:

- Help text in every command
- Examples in documentation
- Video walkthrough (optional)
- User feedback on docs

**Owner**: Documentation lead

**Timeline**: Week 7

**RISK-9: Dependency Vulnerabilities**

**Risk**: Security vulnerabilities in npm dependencies

**Impact**:

- LOW: Security scanner alerts
- Need to update dependencies

**Probability**: Medium (npm ecosystem)

**Mitigation**:

- Dependabot alerts enabled
- Regular dependency updates
- npm audit in CI
- Minimal dependencies

**Owner**: Security team

**Monitoring**: Weekly Dependabot checks

---

## Contract-first Outputs

### TypeScript API Surface

```typescript
// CLI Command Types
export interface CommandOptions {
  format?: string;
  config?: string;
  verbose?: boolean;
}

export interface ValidateOptions extends CommandOptions {
  showAps?: boolean;
}

export interface GateOptions extends CommandOptions {
  checks?: string;
  skip?: string;
}

export interface ExportOptions extends CommandOptions {
  to: string;
  output?: string;
  from?: string;
  compact?: boolean;
}

// Format Detection
export interface FormatDetectionResult {
  format: string;
  confidence: number;
  adapter: FormatAdapter;
}

export async function detectFormat(
  content: string,
  filename?: string
): Promise<FormatDetectionResult | null>;

// Command Handlers
export async function validateCommand(
  planPath: string,
  options: ValidateOptions
): Promise<ValidationCommandResult>;

export async function gateCommand(
  planPath: string,
  options: GateOptions
): Promise<GateCommandResult>;

export async function exportCommand(
  sourcePath: string,
  options: ExportOptions
): Promise<ExportCommandResult>;

// Results
export interface ValidationCommandResult {
  success: boolean;
  plan?: APSPlan;
  errors?: ValidationError[];
  format?: string;
}

export interface GateCommandResult {
  success: boolean;
  overall: boolean;
  checks: CheckResult[];
  evidence?: Evidence;
  format?: string;
}

export interface ExportCommandResult {
  success: boolean;
  outputPaths: string[];
  sourceFormat: string;
  targetFormat: string;
  warnings?: string[];
}
```

### Zod Schemas

```typescript
import { z } from 'zod';

// Evidence Bundle Schema (for SpecKit injection)
export const EvidenceBundleSchema = z.object({
  generated: z.string().datetime(),
  anvilVersion: z.string(),
  format: z.string(),
  formatVersion: z.string().optional(),
  status: z.enum(['PASSED', 'FAILED', 'PARTIAL']),
  checks: z.array(
    z.object({
      name: z.string(),
      status: z.enum(['pass', 'fail', 'warn', 'skip']),
      details: z.string(),
      duration: z.number().optional(),
    })
  ),
  planHash: z.string(),
  evidenceHash: z.string(),
  fullDetailsPath: z.string().optional(),
});

export type EvidenceBundle = z.infer<typeof EvidenceBundleSchema>;

// CLI Configuration Schema
export const CLIConfigSchema = z.object({
  defaultFormat: z.string().optional(),
  gate: z
    .object({
      checks: z.array(z.string()).optional(),
      coverage: z
        .object({
          threshold: z.number().min(0).max(100),
        })
        .optional(),
      failFast: z.boolean().optional(),
    })
    .optional(),
  adapters: z
    .record(
      z.string(),
      z.object({
        preserveComments: z.boolean().optional(),
        preserveMetadata: z.boolean().optional(),
        version: z.string().optional(),
      })
    )
    .optional(),
});

export type CLIConfig = z.infer<typeof CLIConfigSchema>;

// Format Detection Schema
export const FormatDetectionSchema = z.object({
  format: z.string(),
  confidence: z.number().min(0).max(100),
  version: z.string().optional(),
  reason: z.string().optional(),
});

export type FormatDetection = z.infer<typeof FormatDetectionSchema>;
```

### CLI Output Formats

```typescript
// JSON Output (--json flag)
export interface JSONOutput {
  success: boolean;
  command: string;
  timestamp: string;
  data?: unknown;
  errors?: Array<{
    code: string;
    message: string;
    path?: string;
  }>;
  warnings?: Array<{
    code: string;
    message: string;
  }>;
}

// Table Output (default for gate)
export interface TableRow {
  check: string;
  status: '✓ Pass' | '✗ Fail' | '⚠ Warn' | '○ Skip';
  details: string;
}

// Evidence Comment Format (injected into SpecKit)
export const EVIDENCE_COMMENT_FORMAT = `
<!-- ANVIL VALIDATION EVIDENCE
Generated: {timestamp}
Anvil Version: {version}
Format: {format} v{formatVersion}

Status: {status}

Quality Gates:
{checks}

Plan Hash: {planHash}
Evidence Hash: {evidenceHash}

Full details: {detailsPath}
-->
`.trim();
```

---

## Telemetry

### Event Tracking

**EVENT-1: command_executed**

- **Properties**:
  - `command`: string (validate | gate | export)
  - `format_detected`: string | null
  - `format_specified`: string | null
  - `detection_confidence`: number | null
  - `success`: boolean
  - `duration_ms`: number
  - `error_code`: string | null
- **Success signal**: `success: true`
- **Failure signal**: `success: false`, `error_code` present

**EVENT-2: format_detection**

- **Properties**:
  - `filename`: string (hashed for privacy)
  - `detected_format`: string
  - `confidence`: number
  - `correct`: boolean | null (if user overrides, assume detection wrong)
- **Success signal**: `confidence >= 90`
- **Failure signal**: `confidence < 50` or user override

**EVENT-3: validation_completed**

- **Properties**:
  - `format`: string
  - `plan_size_kb`: number
  - `schema_valid`: boolean
  - `hash_valid`: boolean
  - `error_count`: number
  - `warning_count`: number
  - `duration_ms`: number
- **Success signal**: `schema_valid && hash_valid && error_count == 0`
- **Failure signal**: `error_count > 0`

**EVENT-4: gate_executed**

- **Properties**:
  - `format`: string
  - `checks_run`: string[] (e.g., ["lint", "test", "coverage"])
  - `checks_passed`: number
  - `checks_failed`: number
  - `overall_pass`: boolean
  - `duration_ms`: number
  - `evidence_injected`: boolean
- **Success signal**: `overall_pass: true`
- **Failure signal**: `checks_failed > 0`

**EVENT-5: export_completed**

- **Properties**:
  - `source_format`: string
  - `target_format`: string
  - `source_size_kb`: number
  - `output_files`: number
  - `success`: boolean
  - `warnings`: number
  - `duration_ms`: number
- **Success signal**: `success: true`
- **Failure signal**: `success: false`

**EVENT-6: evidence_injection**

- **Properties**:
  - `format`: string
  - `file_size_before_kb`: number
  - `file_size_after_kb`: number
  - `injection_success`: boolean
  - `backup_created`: boolean
- **Success signal**: `injection_success: true`
- **Failure signal**: `injection_success: false`

**EVENT-7: error_occurred**

- **Properties**:
  - `command`: string
  - `error_code`: string
  - `error_message`: string (sanitized, no PII)
  - `error_type`: string (file_not_found | parse_error | validation_error |
    etc.)
  - `recoverable`: boolean
- **Success signal**: N/A (error event)
- **Failure signal**: Always a failure

### Privacy & Data Collection

**Privacy Policy**:

- No PII collected (no usernames, file contents, paths)
- File sizes and counts only (no content)
- Error messages sanitized (no stack traces with paths)
- Format and command usage only
- Opt-out available via `ANVIL_TELEMETRY=0`

**Data Retention**: 90 days

**Usage**: Product improvement, bug prioritisation, performance optimisation

---

## Risks & Open Questions

### Risks (Summary from Dependencies section)

1. **HIGH**: SpecKit adapter stability (2 failing tests)
   - **Mitigation**: Fix in week 6 day 1

2. **CRITICAL**: Evidence injection might corrupt documents
   - **Mitigation**: Extensive testing, atomic writes, backups

3. **MEDIUM**: Format detection false positives
   - **Mitigation**: Conservative detection, user confirmation

4. **MEDIUM**: Performance on large repositories
   - **Mitigation**: Benchmarks, optimization, progress indicators

5. **MEDIUM**: Cross-platform compatibility
   - **Mitigation**: CI testing on all platforms

### Open Questions

**Q1: Should evidence injection be default behavior?**

**Options**:

- A) Always inject evidence (default)
- B) Require --inject-evidence flag
- C) Prompt user first time, remember preference

**Recommendation**: A (always inject)

**Rationale**: Core value prop is provenance. Evidence must be persistent.

**Resolution deadline**: Week 6 day 1

**Owner**: Product lead

---

**Q2: How should we handle SpecKit format version conflicts?**

**Scenario**: User has v1 SpecKit, we support v2. What happens?

**Options**:

- A) Auto-upgrade v1 → v2
- B) Support both, let user choose
- C) Warn about old version, suggest upgrade
- D) Reject v1, require v2

**Recommendation**: B (support both)

**Rationale**: Backwards compatibility critical for adoption. Both versions
implemented.

**Resolution deadline**: Week 6 (already resolved by implementation)

**Owner**: Adapter team

---

**Q3: Should format detection cache results?**

**Scenario**: Running `anvil gate spec.md` multiple times. Re-detect each time?

**Options**:

- A) Always re-detect (no cache)
- B) Cache in .anvilrc per-file
- C) Cache in memory for session only

**Recommendation**: A for v1, B for v2

**Rationale**: Simplicity first. Caching is premature optimization.

**Resolution deadline**: Week 7 (v2 feature)

**Owner**: Performance team

---

**Q4: How verbose should default output be?**

**Scenario**: Balance between helpful and overwhelming

**Options**:

- A) Minimal (success/fail only)
- B) Moderate (summary of checks)
- C) Verbose (full details)

**Recommendation**: B (moderate)

**Rationale**: Users want to see what happened, but not overwhelmed. --verbose
for details.

**Resolution deadline**: Week 6 (UX testing)

**Owner**: UX lead

---

**Q5: Should we support partial SpecKit documents?**

**Scenario**: User only has spec.md, no plan.md or tasks.md

**Options**:

- A) Require all three files
- B) Support any combination
- C) Require spec.md, others optional

**Recommendation**: C (spec.md required, others optional)

**Rationale**: Real-world workflows vary. Be flexible.

**Resolution deadline**: Week 6 (already implemented)

**Owner**: Adapter team

---

**Q6: How should we handle concurrent gate executions?**

**Scenario**: User runs `anvil gate spec.md` twice simultaneously

**Options**:

- A) Allow parallel execution
- B) Lock file during execution
- C) Detect concurrent run, warn user

**Recommendation**: C (detect and warn)

**Rationale**: Parallel execution could corrupt evidence. Locking is complex.
Warning is sufficient.

**Resolution deadline**: Week 7

**Owner**: CLI team

---

## Implementation Plan

### Week 6: Core CLI Integration

**Goal**: Users can run `anvil validate spec.md` and `anvil gate spec.md`
successfully

**Tasks**:

1. Fix 2 failing SpecKit adapter tests
2. Implement format auto-detection in CLI
3. Enhance validate command with adapter support
4. Enhance gate command with adapter support
5. Implement export command
6. Add evidence injection to SpecKit adapter
7. Integration tests for all commands
8. Documentation updates

**Deliverables**:

- All commands working with SpecKit
- 90% test coverage
- Basic documentation

**Demo**: Show `anvil gate spec.md` end-to-end

---

### Week 7: Polish & Evidence

**Goal**: Evidence integration works flawlessly, UX polished

**Tasks**:

1. Extensive testing of evidence injection
2. Round-trip tests (inject evidence, parse, verify no corruption)
3. Error message improvements
4. Performance optimization
5. User testing sessions
6. Documentation polish

**Deliverables**:

- Evidence injection battle-tested
- Improved error messages
- User testing feedback incorporated
- Complete documentation

**Demo**: Show evidence in PR review workflow

---

### Week 8: Documentation & Customer Demo

**Goal**: Ready for first customer onboarding

**Tasks**:

1. Comprehensive documentation (getting started, CLI reference, examples)
2. Video walkthrough
3. Customer demo preparation
4. Customer #1 onboarding
5. Feedback collection
6. Bug fixes from customer feedback

**Deliverables**:

- Complete documentation
- Video tutorial
- First customer using Anvil
- Feedback and testimonial

**Milestone**: Product-market fit validation with reference customer

---

## Conclusion

This PRD defines the integration of SpecKit adapter with Anvil CLI, removing the
primary adoption barrier for Act 1 (developer wedge). By supporting GitHub's
official spec-kit format, we enable developers to benefit from Anvil's
validation and governance without changing their workflow.

**Key Success Factors**:

1. Format auto-detection "just works"
2. Evidence injection preserves document integrity
3. Clear, actionable error messages
4. Fast performance (<2 seconds for validate, <2 minutes for gate)
5. Comprehensive documentation and examples

**Expected Outcomes**:

- First 5 SpecKit teams using Anvil by Week 10
- Reference customers for Act 1 fundraising
- Proof of product-market fit
- Foundation for BMAD and additional format adapters

**Next Steps**:

1. Week 6: Implement CLI integration
2. Week 7: Polish and evidence integration
3. Week 8: Documentation and customer demo
4. Week 9+: BMAD adapter, GitHub Action integration

---

**Document Version**: 1.0 **Author**: Product Team **Date**: 2025-10-20
**Status**: Ready for Review
