# Known Issues

## SpecKit Adapter Format Detection Not Working (Week 7)

**GitHub Issue**: [#28](https://github.com/EddaCraft/anvil-001/issues/28)

**Issue**: CLI format auto-detection cannot find SpecKit adapter

**Root Cause**: SpecKit adapters (`SpecKitImportAdapter`,
`SpecKitExportAdapter`) were implemented using the legacy `BaseAdapter`
interface from `packages/adapters/src/common/types.ts`. The CLI's
`FormatDetectionService` and `PlanLoader` require adapters that implement the
new `FormatAdapter` interface from `packages/adapters/src/base/types.ts`.

**Impact**:

- `anvil validate spec.md` fails with "Unable to detect plan format"
- `anvil gate spec.md` fails with format detection error
- `anvil export spec.md --to=aps` fails
- Users cannot use CLI with SpecKit documents via auto-detection

**Workarounds**:

1. Use native APS format (`anvil validate plan.json`)
2. Manual conversion using old adapter API (not exposed via CLI)

**Migration Required**: SpecKit adapters need to be refactored to implement
`FormatAdapter`:

1. Add `metadata: AdapterMetadata` property
2. Implement `detect(content: string): DetectionResult`
3. Implement
   `parse(content: string, context?: ParseContext): Promise<ParseResult>`
4. Implement `serialize(plan: APSPlan): Promise<SerializeResult>`
5. Implement `validate(content: string): Promise<ValidationResult>`
6. Implement `canImport(format: string): boolean`
7. Implement `canExport(format: string): boolean`

**Estimated Effort**: 2-3 days

**Status**: Deferred to after BMAD adapter complete (Week 9-10)

**Rationale**: BMAD adapter will be implemented correctly with `FormatAdapter`
from the start, serving as reference for SpecKit migration.

---

## Hash Validation Failure in CLI (Week 7) - ✅ RESOLVED

**GitHub Issue**: [#29](https://github.com/EddaCraft/anvil-001/issues/29)

**Issue**: Plans with valid hashes (generated via `generateHash()`) fail hash
verification in CLI

**Root Cause**: Hash validation was being applied to external formats (SpecKit,
BMAD) where hashes are generated during parsing, not stored in source documents.

**Resolution**: Modified `validate.ts` to only validate hashes for native APS
plans. External formats skip hash validation since their hashes are generated
fresh during parsing.

**Fix Details**:

- Hash validation now checks if `sourceFormat` is present
- If format is external (SpecKit, BMAD, Generic), hash validation is skipped
- Only native APS JSON/YAML plans with stored hashes are validated
- Added clear comments explaining the logic

**Status**: ✅ Resolved (November 17, 2025)

**Commit**: TBD

---

## Test Coverage Gaps (Week 7)

**Issue**: CLI integration tests exist but were not discoverable due to
directory confusion

**Resolution**: Tests run fine from project root: `pnpm test cli/src` (36 tests
passing)

**Status**: Resolved

---

## Suppression Fallback Files Not Parsed (December 2025)

**Severity**: Low

**Issue**: `analyzeFiles()` only parses suppressions for the input files. If a
check scans additional files (e.g., architecture check fallback to include
patterns), those suppressions won't be parsed or applied.

**Location**: `core/src/gate/gate-runner.ts:277`,
`core/src/gate/checks/architecture.check.ts:391`

**Impact**: Suppressions in files not explicitly passed to `analyzeFiles()` will
not be applied. This only affects edge cases where:

1. Architecture check falls back to include patterns (no explicit files)
2. Those fallback files contain `@anvil-ignore` comments

**Workaround**: Explicitly pass all relevant files to `analyzeFiles()`.

**Recommended Fix**: Add a separate suppression parsing step in architecture
check when using fallback patterns, or expose a method to parse additional files
after initial analysis.

**Status**: Deferred (low priority edge case)

---

## Architecture Baseline Ignores Import Line (December 2025)

**Severity**: Low

**Issue**: `isNewViolation()` compares `from_file`, `to_file`, and `rule` but
ignores `import_line`. A new violating import between the same files with the
same rule can be treated as "existing" if only the line number changed.

**Location**: `core/src/gate/checks/architecture.check.ts:321`

**Impact**: If a developer fixes a violation by moving an import to a different
line (without actually fixing it), the baseline will still match and suppress
the warning. However, this is arguably correct behaviour — same from/to/rule is
the same logical violation regardless of line number.

**Rationale for Deferral**: The current behaviour may actually be desirable.
Adding line number comparison would cause violations to "reappear" when code is
reformatted or imports are reordered, creating noise.

**Status**: Deferred (behaviour may be intentional)

---

**Last Updated**: 2026-01-04

---

## Security Vulnerabilities Fixed (January 2026)

**GitHub Issues**: #83-88

**Resolved**:

1. **Timing Attack (AP-083)** - Hash comparison now uses `timingSafeEqual`
2. **Shell Injection (AP-085)** - OPA executor uses `spawn()` with argument
   arrays
3. **Path Traversal (AP-086)** - VS Code CLI validates paths before execution
4. **Binary Downloads (AP-084)** - OPA binary download enforces HTTPS and
   checksum
5. **ReDoS (AP-088)** - Vulnerable regex patterns replaced with safe
   alternatives

**Status**: Resolved (January 2026)

---

## Architecture & Type Safety Improvements (January 2026)

**GitHub Issues**: #89-93

**Resolved**:

1. **Non-deterministic IDs (AP-089)** - Plan IDs now use content SHA-256 hash
2. **Unsafe z.any() (AP-090)** - Replaced with `z.unknown()` throughout
3. **Weak Provenance IDs (AP-092)** - Uses `crypto.randomUUID()`
4. **YAML Export (AP-093)** - Now throws clear error instead of silent JSON
   fallback
5. **Hash Validation (AP-091)** - Added `hashValidated` field to
   ValidationResult

**Status**: Resolved (January 2026)

---

## VS Code Extension Improvements (January 2026)

**GitHub Issues**: #103-104

**Resolved**:

1. **Resource Disposal** - Added proper `dispose()` methods to providers
2. **Command Timeout** - Commands now have configurable timeouts (60s default)
3. **Cache Invalidation** - Added `clearCacheForUri()` for file deletion cleanup

**Status**: Resolved (January 2026)

---

## Code Quality Improvements (January 2026)

**GitHub Issues**: #98, #100, #102

**Resolved**:

1. **Magic Numbers (#100)** - Extracted to named constants
2. **Silent Error Swallowing (#98)** - Added debug logging utility with
   namespace support (`ANVIL_DEBUG=1` or `DEBUG=anvil:*` to enable)
3. **Unsafe Type Assertions (#102)** - Added Zod runtime validation for
   JSON.parse results in provenance store and tutorial progress

**Files Added**:

- `core/src/utils/debug.ts` - Debug logging utility
- `core/src/utils/index.ts` - Utils barrel export

**Status**: Resolved (January 2026)
