# quick-context

Quickly understand a file's purpose and context in the project.

## Parameters
- **file_path**: Path to the file to analyze

## Tasks

1. **Show File Location**
   - Display the file's location in the project structure
   - Identify which package/module it belongs to
   - Show relative path from project root

2. **Extract Purpose**
   - Read the file and extract main purpose from:
     - Header comments and JSDoc
     - Module/class descriptions
     - README references
   - Summarize in 1-2 sentences

3. **List Key Exports**
   - Identify all exported functions, classes, types, interfaces
   - Briefly describe what each export does
   - Note any deprecated or internal APIs

4. **Dependency Analysis**
   - **Imports**: Show what this file imports and depends on
   - **Dependents**: Search for files that import from this file
   - Highlight circular dependencies if any

5. **Find Related Tests**
   - Look for test files: `*.test.ts`, `*.spec.ts`, `__tests__/`
   - Show test coverage if available
   - Note if tests are missing

6. **Recent Changes**
   - Run `git log -5 --oneline -- <file_path>` to show recent commits
   - Highlight if file was recently modified
   - Note any TODO/FIXME comments

7. **Context Summary**
   Provide a brief summary including:
   - File role in the project
   - Key responsibilities
   - Important patterns or conventions used
   - Any gotchas or special considerations

## Example Usage

When called with `file_path: "packages/adapters/src/speckit/parser.ts"`:

```
📁 Location: packages/adapters/src/speckit/parser.ts
   Package: @anvil/adapters
   Module: SpecKit adapter

📝 Purpose:
   Core parser for GitHub spec-kit format. Handles both V1 (simple) and V2 (official)
   formats, extracting metadata, requirements, and implementation details.

🔧 Key Exports:
   - parseSpecKitV2(): Parses official spec-kit format with full metadata
   - parseSpecKitV1(): Legacy parser for simple format
   - SpecKitMetadata: Type definition for parsed metadata

📦 Dependencies:
   Imports:
   - @anvil/core/schema (APS types)
   - ./parsers/metadata (metadata extraction)
   - ./parsers/requirements (requirement parsing)

   Used by:
   - speckit/import.ts (V1 importer)
   - speckit/import-v2.ts (V2 importer)
   - speckit/export.ts (reverse transformation)

🧪 Tests: parser.test.ts (18 tests, 100% coverage)

📅 Recent Changes:
   - abc1234 fix: Improve metadata extraction regex
   - def5678 feat: Add multiline requirement support
```

## Anvil Project Specifics

- Focus on adapter patterns in `packages/adapters/`
- Highlight APS schema usage from `@anvil/core`
- Note CLI command integrations
- Show package dependencies via `workspace:*` protocol
