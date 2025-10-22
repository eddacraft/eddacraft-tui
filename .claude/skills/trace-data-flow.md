# trace-data-flow

Trace how data flows through the system from input to output. Useful for
understanding complex transformations.

## Parameters

- **entry_point**: Where data enters (e.g., "CLI validate command", "SpecKit
  import", "gate runner")
- **data_type**: Type of data being traced (e.g., "spec.md file", "APS
  document", "gate evidence")

## Tasks

1. **Identify Entry Point**
   - Locate the entry point file and function:
     - **CLI commands**: `cli/src/commands/<command>.ts`
     - **Adapters**: `packages/adapters/src/<format>/import.ts`
     - **API endpoints**: Look for route handlers
     - **Event handlers**: Search for listeners
   - Read the entry point to understand:
     - Input parameters and types
     - Initial validation
     - First transformation step

2. **Trace Function Call Chain** For each function in the flow:
   - Record function name and file location
   - Note input type
   - Note output type
   - Identify key transformations
   - Track error handling paths
   - Follow the call to the next function

3. **Identify Validation Points** Find where data is validated:
   - **Schema validation**: Zod schema parsing
   - **Type guards**: Runtime type checking
   - **Business rules**: Custom validation logic
   - **Format verification**: Adapter canHandle checks

   For each validation:
   - What is being validated
   - What happens on validation failure
   - What errors can be thrown

4. **Map Data Transformations** Document each transformation:

   ```
   Input → [Function] → Output

   FormatSource → [parseSpecKit] → ParsedSpec
   ParsedSpec → [toAPS] → APS
   APS → [hashArtifact] → HashedAPS
   HashedAPS → [validateAPS] → ValidatedAPS
   ```

5. **Highlight Key Types** List important types in the flow:
   - Input type (what enters the system)
   - Intermediate types (transformations)
   - Output type (final result)
   - Error types (what can go wrong)

   Show type definitions:

   ```typescript
   type FormatSource = { fileName: string; content: string };
   type APS = { metadata: Metadata; requirements: Requirement[] };
   ```

6. **Identify Side Effects** Note operations that aren't pure transformations:
   - **File I/O**: Reading, writing files
   - **Network**: API calls, fetching data
   - **Logging**: Console output, log files
   - **State**: Mutations, cache updates
   - **Database**: Queries, inserts, updates

7. **Show Error Paths** Trace what happens when errors occur:

   ```
   Entry → Validation → [FAIL] → Error handler → Format error → Return to user
                     ↓
                   [PASS]
                     ↓
                 Transform → [FAIL] → Catch block → Log → Rethrow
                           ↓
                         [PASS]
                           ↓
                        Success
   ```

8. **Create Visual Flow Diagram** Generate ASCII diagram showing the flow:

   ```
   User Input (spec.md)
        ↓
   [CLI validate command]
        ↓
   [FormatDetectionService.detect()]
        ↓
   [SpecKitImportAdapter.canHandle()] ✓
        ↓
   [SpecKitImportAdapter.convert()]
        ↓
   [parseSpecKitV2()]
        ├→ extractMetadata()
        ├→ parseRequirements()
        └→ parseImplementation()
        ↓
   [toAPS()] - Transform to APS format
        ↓
   [apsSchema.parse()] - Validate against schema
        ↓
   [hashArtifact()] - Calculate deterministic hash
        ↓
   Output: ValidatedAPS with hash
        ↓
   [Display to user]
   ```

9. **Identify Optimization Opportunities** Look for:
   - Redundant transformations
   - Unnecessary copying
   - Multiple passes over same data
   - Opportunities for streaming
   - Expensive operations that could be cached

10. **Document Flow** Create summary:
    - Entry point and trigger
    - High-level flow (3-5 steps)
    - Key transformations
    - Final output
    - Error handling strategy
    - Performance characteristics

## Data Flow Patterns in Anvil

### CLI Command Flow

```
User command
  ↓
CLI argument parsing (yargs)
  ↓
Command handler
  ↓
Load/detect format
  ↓
Adapter conversion
  ↓
Validation
  ↓
Output formatting
  ↓
Display to user
```

### Adapter Import Flow

```
Format file (spec.md)
  ↓
FormatSource { fileName, content }
  ↓
Adapter.canHandle() - Format detection
  ↓
Adapter.convert()
  ↓
  ├→ Parse file (extract sections)
  ├→ Transform to APS structure
  └→ Validate against APS schema
  ↓
APS document
```

### Gate Execution Flow

```
Plan (APS)
  ↓
Gate runner
  ↓
For each gate:
  ├→ Resolve executable
  ├→ Execute check
  ├→ Collect evidence
  └→ Determine pass/fail
  ↓
Evidence bundle
  ↓
Summary report
```

### Hash Calculation Flow

```
APS document
  ↓
Normalize structure
  ↓
Extract hashable fields (exclude metadata.hash)
  ↓
Serialize to canonical JSON
  ↓
SHA-256 hash
  ↓
Hash string (hex)
```

## Example Output

```
🔍 Data Flow Trace: CLI validate command → spec.md → APS

Entry Point: cli/src/commands/validate.ts:45
Data Type: spec.md file content

Flow:
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

1. User Input
   Type: string (file path)
   Location: CLI argument
   ↓
2. File Loading (cli/src/services/plan-loader.ts:23)
   Input: string (path)
   Output: FormatSource { fileName, content }
   Side effect: fs.readFile()
   ↓
3. Format Detection (cli/src/services/format-detection.ts:15)
   Input: FormatSource
   Output: FormatAdapter (SpecKitImportAdapter)
   Validation: Try each adapter's canHandle()
   ↓
4. Parsing (packages/adapters/src/speckit/parser.ts:78)
   Input: string (content)
   Output: ParsedSpec
   Transforms:
     - Extract metadata via regex
     - Parse requirements section
     - Parse implementation details
   Validation: Check required sections exist
   ↓
5. APS Transformation (packages/adapters/src/speckit/import-v2.ts:112)
   Input: ParsedSpec
   Output: APS (unvalidated)
   Transforms:
     - Map metadata fields
     - Convert requirements to APS format
     - Structure implementation as artifacts
   ↓
6. Schema Validation (packages/core/src/schema/aps.ts:45)
   Input: unknown
   Output: APS (validated)
   Validation: apsSchema.parse() - Zod schema
   Errors: ZodError with detailed issues
   ↓
7. Hashing (packages/core/src/hash/artifact.ts:12)
   Input: APS
   Output: APS with hash
   Transform: Calculate SHA-256 of normalized structure
   ↓
8. Output Formatting (cli/src/commands/validate.ts:89)
   Input: APS
   Output: Formatted string
   Display: Console.log with colors

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

Key Types:
  string → FormatSource → ParsedSpec → APS (unvalidated) → APS (validated) → string

Validation Points:
  ✓ File exists (step 2)
  ✓ Format detected (step 3)
  ✓ Required sections present (step 4)
  ✓ APS schema compliance (step 6)

Side Effects:
  📄 File read (step 2)
  🖥️  Console output (step 8)

Error Paths:
  - File not found → Exit with error
  - Unknown format → Exit with "No adapter found"
  - Parse error → Exit with parse details
  - Schema validation → Exit with validation errors

Performance:
  - I/O: 1 file read (~1-10ms)
  - Parsing: ~5-20ms for typical spec
  - Total: ~50ms for end-to-end
```

## Anvil Project Specifics

- Main data structure: APS (Artifact Planning Specification)
- Adapters transform various formats → APS
- Validation uses Zod schemas throughout
- Hashing ensures artifact integrity
- Evidence collection for gate checks
- Deterministic operations (no random IDs, timestamps in hashes)
