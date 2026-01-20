# LSP Implementation Plan: Real-Time Layer on CLI

**Status**: Planning **Created**: 2025-12-28 **Architecture Principle**: LSP
wraps existing validation, CLI remains primary

---

## Design Constraints

1. **CLI-first approach** - LSP is enhancement, not replacement
2. **Zero duplication** - Single source of truth for validation logic
3. **Non-VS Code users** - CLI must work standalone
4. **Minimal maintenance** - Fix bugs once, both benefit

---

## Current Architecture (CLI)

The CLI already has excellent layering:

```
CLI Command (validate.ts)
    ↓
PlanLoader Service
    ├── FormatDetectionService → detect format
    ├── AdapterRegistry → get adapter
    ├── Adapter.parse() → convert to APS
    └── APSValidator → validate APS
        ↓
    @eddacraft/anvil-core (single source of truth)
        ├── APSSchema (Zod)
        ├── verifyHash()
        └── validation logic
```

**Key insight**: The CLI doesn't contain validation logic - it imports from
`@eddacraft/anvil-core`.

---

## Proposed Architecture (CLI + LSP)

```
┌─────────────────────────────────────────────────────────────┐
│                  Single Source of Truth                      │
│                                                              │
│  @eddacraft/anvil-core                                                 │
│  ├── APSSchema (Zod definitions)                            │
│  ├── APSValidator                                           │
│  ├── verifyHash()                                           │
│  └── Gate runner                                            │
│                                                              │
│  @eddacraft/anvil-adapters                                             │
│  ├── AdapterRegistry                                        │
│  ├── SpecKitAdapter                                         │
│  ├── BMADAdapter                                            │
│  └── GenericAdapter                                         │
└──────────────────────┬──────────────────┬────────────────────┘
                       │                  │
                       ↓                  ↓
         ┌─────────────────────┐  ┌─────────────────────┐
         │   CLI Tool          │  │  LSP Server (NEW)   │
         │   (existing)        │  │                     │
         ├─────────────────────┤  ├─────────────────────┤
         │ PlanLoader          │  │ LSPValidator        │
         │ ├─ uses @eddacraft/anvil-core │  │ ├─ uses @eddacraft/anvil-core │
         │ └─ uses adapters    │  │ └─ uses adapters    │
         │                     │  │                     │
         │ Output: CLI format  │  │ Output: LSP format  │
         │ ├─ Colorized        │  │ ├─ Diagnostics[]    │
         │ ├─ JSON             │  │ ├─ CompletionItem[] │
         │ └─ Exit codes       │  │ └─ Hover            │
         └─────────────────────┘  └─────────────────────┘
                 │                          │
                 ↓                          ↓
         ┌─────────────────────┐  ┌─────────────────────┐
         │ Terminal users      │  │ Editor users        │
         │ ├─ CI/CD            │  │ ├─ VS Code          │
         │ ├─ Scripts          │  │ ├─ Neovim           │
         │ └─ Manual runs      │  │ └─ Emacs            │
         └─────────────────────┘  └─────────────────────┘
```

**Critical pattern**: Both CLI and LSP are **thin wrappers** around the same
core libraries.

---

## Implementation Strategy

### Phase 1: Extract Shared Services

**Problem**: CLI has `PlanLoader` service that LSP needs **Solution**: Extract
to shared package

Create: `packages/validation-service/`

```typescript
// packages/validation-service/src/index.ts
import { APSValidator } from '@eddacraft/anvil-core';
import { AdapterRegistry } from '@eddacraft/anvil-adapters';

/**
 * Shared validation service used by both CLI and LSP
 * Single source of truth for all validation logic
 */
export class ValidationService {
  private validator: APSValidator;
  private registry: AdapterRegistry;

  constructor() {
    this.validator = new APSValidator();
    this.registry = AdapterRegistry.getInstance();
  }

  /**
   * Validate content in any format
   * Used by: CLI validate command, LSP diagnostics
   */
  async validateContent(
    content: string,
    options?: {
      format?: string;
      validateHash?: boolean;
      filePath?: string;
    }
  ): Promise<ValidationResult> {
    // 1. Detect format (or use specified)
    const detection = options?.format
      ? this.getSpecificAdapter(options.format)
      : await this.registry.detectBestMatch(content);

    if (!detection) {
      return {
        success: false,
        errors: [{ message: 'Unknown format', severity: 'error' }],
      };
    }

    // 2. Parse to APS
    const parseResult = await detection.adapter.parse(content, {
      repositoryPath: process.cwd(),
      timestamp: new Date().toISOString(),
    });

    if (!parseResult.success) {
      return {
        success: false,
        errors: parseResult.errors.map((e) => ({
          message: e.message,
          severity: 'error',
          line: e.line,
          column: e.column,
        })),
      };
    }

    // 3. Validate APS schema
    const validation = await this.validator.validate(parseResult.data!, {
      validateHash: options?.validateHash ?? false,
    });

    // 4. Return standardized result
    return {
      success: validation.valid,
      plan: validation.data,
      format: detection.format,
      errors:
        validation.issues?.map((issue) => ({
          message: issue.message,
          severity: 'error',
          path: issue.path?.join('.'),
          line: this.getLineFromPath(content, issue.path),
        })) ?? [],
      warnings:
        parseResult.warnings?.map((w) => ({
          message: w.message,
          severity: 'warning',
        })) ?? [],
    };
  }

  /**
   * Quick validation without full parse (for real-time LSP)
   * Returns basic syntax errors only
   */
  async quickValidate(
    content: string,
    format?: string
  ): Promise<QuickValidationResult> {
    const detection = format
      ? this.getSpecificAdapter(format)
      : await this.registry.detectBestMatch(content);

    if (!detection) {
      return { syntaxValid: false, errors: ['Unknown format'] };
    }

    // Use adapter's fast validation path if available
    if (detection.adapter.quickValidate) {
      return detection.adapter.quickValidate(content);
    }

    // Fallback: check basic syntax
    return this.checkSyntax(content, detection.format);
  }

  private getLineFromPath(
    content: string,
    path?: string[]
  ): number | undefined {
    // Implementation: Find line number from JSON path in content
    // This is useful for showing diagnostics at the right location
    return undefined; // Simplified for now
  }

  private checkSyntax(content: string, format: string): QuickValidationResult {
    if (format === 'aps') {
      try {
        JSON.parse(content);
        return { syntaxValid: true };
      } catch (e) {
        return { syntaxValid: false, errors: [(e as Error).message] };
      }
    }
    // Markdown formats are always syntactically valid
    return { syntaxValid: true };
  }
}

export interface ValidationResult {
  success: boolean;
  plan?: APSPlan;
  format?: string;
  errors: ValidationIssue[];
  warnings?: ValidationIssue[];
}

export interface ValidationIssue {
  message: string;
  severity: 'error' | 'warning' | 'info';
  path?: string;
  line?: number;
  column?: number;
}

export interface QuickValidationResult {
  syntaxValid: boolean;
  errors?: string[];
}
```

**Benefits**:

- ✅ Single validation logic shared by CLI and LSP
- ✅ CLI and LSP always produce identical results
- ✅ Fix bugs in one place
- ✅ LSP-specific optimisations (quickValidate) don't affect CLI

---

### Phase 2: Build LSP Server

Create: `packages/language-server/`

```typescript
// packages/language-server/src/server.ts
import {
  createConnection,
  TextDocuments,
  ProposedFeatures,
  TextDocumentSyncKind,
} from 'vscode-languageserver/node';
import { TextDocument } from 'vscode-languageserver-textdocument';
import { ValidationService } from '@eddacraft/anvil-validation-service';

class AnvilLanguageServer {
  private connection = createConnection(ProposedFeatures.all);
  private documents = new TextDocuments(TextDocument);

  // ⭐ Uses same validation service as CLI
  private validationService = new ValidationService();

  private debounceTimers = new Map<string, NodeJS.Timeout>();
  private readonly DEBOUNCE_MS = 300;

  constructor() {
    this.setupHandlers();
  }

  private setupHandlers() {
    // LSP lifecycle
    this.connection.onInitialize(this.handleInitialize.bind(this));

    // Document changes (real-time validation)
    this.documents.onDidChangeContent((change) => {
      this.scheduleValidation(change.document);
    });

    this.documents.onDidSave((change) => {
      this.validateImmediately(change.document);
    });

    // LSP features
    this.connection.onCompletion(this.handleCompletion.bind(this));
    this.connection.onHover(this.handleHover.bind(this));

    this.documents.listen(this.connection);
    this.connection.listen();
  }

  private handleInitialize() {
    return {
      capabilities: {
        textDocumentSync: TextDocumentSyncKind.Incremental,
        completionProvider: { triggerCharacters: ['.', ':', '"'] },
        hoverProvider: true,
      },
    };
  }

  // Debounced validation (as-you-type)
  private scheduleValidation(document: TextDocument) {
    const uri = document.uri;

    // Clear existing timer
    const existing = this.debounceTimers.get(uri);
    if (existing) clearTimeout(existing);

    // Schedule new validation
    const timer = setTimeout(() => {
      this.validateDocument(document);
      this.debounceTimers.delete(uri);
    }, this.DEBOUNCE_MS);

    this.debounceTimers.set(uri, timer);
  }

  // Immediate validation (on save)
  private async validateImmediately(document: TextDocument) {
    // Cancel debounced validation
    const existing = this.debounceTimers.get(document.uri);
    if (existing) {
      clearTimeout(existing);
      this.debounceTimers.delete(document.uri);
    }

    await this.validateDocument(document);
  }

  // ⭐ Core validation: uses shared ValidationService
  private async validateDocument(document: TextDocument) {
    const content = document.getText();

    // Use same validation logic as CLI
    const result = await this.validationService.validateContent(content, {
      filePath: document.uri,
      validateHash: false, // Skip expensive hash validation for real-time
    });

    // Convert to LSP diagnostics
    const diagnostics = [
      ...result.errors.map((e) => this.toDiagnostic(e, 'Error')),
      ...(result.warnings?.map((w) => this.toDiagnostic(w, 'Warning')) ?? []),
    ];

    // Send to client
    this.connection.sendDiagnostics({
      uri: document.uri,
      diagnostics,
    });
  }

  private toDiagnostic(issue: ValidationIssue, severity: string) {
    const line = issue.line ?? 0;
    const col = issue.column ?? 0;

    return {
      severity: severity === 'Error' ? 1 : 2,
      range: {
        start: { line, character: col },
        end: { line, character: col + 100 },
      },
      message: issue.message,
      source: 'anvil',
    };
  }

  private async handleCompletion(params) {
    // TODO: Schema-aware completions
    return null;
  }

  private async handleHover(params) {
    // TODO: Schema documentation
    return null;
  }

  start() {
    this.connection.listen();
  }
}

// Start server
new AnvilLanguageServer().start();
```

**Key properties**:

- ✅ Uses `ValidationService` - same code as CLI
- ✅ Debounced for real-time (300ms)
- ✅ Immediate validation on save
- ✅ Minimal LSP-specific code (just protocol translation)

---

### Phase 3: Update CLI to Use ValidationService

Refactor `PlanLoader` to use shared `ValidationService`:

```typescript
// cli/src/services/plan-loader.ts (refactored)
import { ValidationService } from '@eddacraft/anvil-validation-service';

export class PlanLoader {
  private validationService: ValidationService;

  constructor() {
    // Use shared validation service
    this.validationService = new ValidationService();
  }

  async loadPlan(
    filePath: string,
    options?: LoadPlanOptions
  ): Promise<LoadPlanResult> {
    const content = await readFile(filePath, 'utf-8');

    // Delegate to shared validation service
    const result = await this.validationService.validateContent(content, {
      format: options?.format,
      validateHash: options?.validateHash,
      filePath,
    });

    if (!result.success) {
      throw new PlanLoadError(
        `Validation failed: ${result.errors[0]?.message}`
      );
    }

    return {
      plan: result.plan!,
      validation: { valid: true, data: result.plan },
      sourceFormat: result.format
        ? {
            format: result.format,
            adapter: result.format, // From ValidationService
          }
        : undefined,
      warnings: result.warnings,
    };
  }
}
```

**Benefits**:

- ✅ CLI now uses same `ValidationService` as LSP
- ✅ Guaranteed identical validation results
- ✅ CLI still works standalone (no LSP dependency)

---

### Phase 4: VS Code Integration

Update existing VS Code extension to use LSP client:

```typescript
// packages/vscode-extension/src/extension.ts
import {
  LanguageClient,
  ServerOptions,
  LanguageClientOptions,
} from 'vscode-languageclient/node';

export async function activate(context: vscode.ExtensionContext) {
  // ... existing services (keep these!) ...
  anvilService = new AnvilService(context);
  statusBarManager = new StatusBarManager();
  gateResultsProvider = new GateResultsProvider(anvilService);

  // NEW: Start LSP client for real-time diagnostics
  const serverModule = context.asAbsolutePath(
    path.join('..', 'language-server', 'dist', 'server.js')
  );

  const serverOptions: ServerOptions = {
    run: { module: serverModule, transport: TransportKind.ipc },
    debug: { module: serverModule, transport: TransportKind.ipc },
  };

  const clientOptions: LanguageClientOptions = {
    documentSelector: [
      { scheme: 'file', pattern: '**/*.plan.md' },
      { scheme: 'file', pattern: '**/*.spec.md' },
      { scheme: 'file', pattern: '**/*.aps.json' },
    ],
  };

  const lspClient = new LanguageClient(
    'anvilLSP',
    'Anvil Language Server',
    serverOptions,
    clientOptions
  );

  await lspClient.start();
  context.subscriptions.push(lspClient);

  // Keep existing commands (validate, gate, export) - they still use CLI
  registerCommands(
    context,
    anvilService,
    statusBarManager,
    gateResultsProvider
  );

  // Remove old DiagnosticsManager - LSP handles this now
  // Remove old PlanWatcher - LSP handles this now
}
```

**What changes**:

- ❌ Remove: `DiagnosticsManager` (LSP provides diagnostics)
- ❌ Remove: `PlanWatcher` (LSP handles file watching)
- ✅ Keep: Command palette commands (still use CLI)
- ✅ Keep: Status bar integration
- ✅ Keep: Gate results tree view
- ✅ Add: LSP client for real-time validation

---

## Validation Consistency Guarantee

**Both CLI and LSP use identical validation:**

```
User runs: anvil validate plan.md
    ↓
CLI → ValidationService → APSValidator → Zod Schema
    ↓
Result: "✓ Plan is valid"

User types in VS Code (LSP active)
    ↓
LSP → ValidationService → APSValidator → Zod Schema
    ↓
Result: "✓ No diagnostics"
```

**They ALWAYS agree** because they call the same code.

---

## Performance Strategy

### Multi-Level Validation

LSP uses progressive validation levels:

```typescript
// packages/language-server/src/validators/progressive-validator.ts

export class ProgressiveValidator {
  private validationService: ValidationService;

  async validate(document: TextDocument, trigger: 'type' | 'save') {
    if (trigger === 'type') {
      // Level 1: Quick syntax check only (instant)
      const quickResult = await this.validationService.quickValidate(
        document.getText()
      );
      return this.quickResultToDiagnostics(quickResult);
    }

    if (trigger === 'save') {
      // Level 2: Full validation (slower, but comprehensive)
      const fullResult = await this.validationService.validateContent(
        document.getText(),
        { validateHash: true } // Include hash validation on save
      );
      return this.fullResultToDiagnostics(fullResult);
    }
  }
}
```

**Benefit**: As-you-type is fast (syntax only), on-save is thorough (full
validation).

---

## Package Structure

```
anvil/
├── core/                           # Validation logic (single source of truth)
│   ├── src/schema/aps.schema.ts   # Zod schemas
│   ├── src/validation/            # APSValidator
│   └── src/crypto/hash.ts         # Hash verification
│
├── packages/adapters/              # Format detection & parsing
│   ├── src/base/registry.ts       # AdapterRegistry
│   ├── src/speckit/               # SpecKit adapter
│   └── src/bmad/                  # BMAD adapter
│
├── packages/validation-service/    # ⭐ NEW: Shared validation service
│   ├── src/index.ts               # ValidationService (used by CLI & LSP)
│   ├── src/types.ts               # Shared types
│   └── package.json
│
├── packages/language-server/       # ⭐ NEW: LSP server
│   ├── src/server.ts              # LSP protocol handler
│   ├── src/validators/            # LSP-specific validators
│   ├── src/providers/             # Completion, hover, etc.
│   └── package.json
│
├── cli/                            # CLI tool (refactored to use ValidationService)
│   ├── src/commands/validate.ts   # Uses ValidationService
│   ├── src/services/plan-loader.ts # Uses ValidationService
│   └── package.json
│
└── packages/vscode-extension/      # VS Code extension (adds LSP client)
    ├── src/extension.ts           # Integrates LSP client
    ├── src/services/              # Keep existing services (gates, status)
    └── package.json
```

---

## Migration Path

### Extract ValidationService

- [ ] Create `packages/validation-service/`
- [ ] Implement `ValidationService` using existing `@eddacraft/anvil-core`
- [ ] Write tests (use existing test fixtures)
- [ ] Verify: ValidationService produces same results as current CLI

### Build LSP Server

- [ ] Create `packages/language-server/`
- [ ] Implement basic LSP server (sync, diagnostics only)
- [ ] Use `ValidationService` for validation
- [ ] Test with VS Code extension
- [ ] Verify: LSP diagnostics match CLI output

### Refactor CLI

- [ ] Update `PlanLoader` to use `ValidationService`
- [ ] Test all CLI commands still work
- [ ] Verify: CLI output unchanged

### VS Code Integration

- [ ] Add LSP client to VS Code extension
- [ ] Remove old `DiagnosticsManager` and `PlanWatcher`
- [ ] Keep command palette integration (still uses CLI)
- [ ] Test real-time validation
- [ ] Document LSP features

### Polish & Advanced Features

- [ ] Add schema-aware completions
- [ ] Add hover documentation
- [ ] Add code actions (quick fixes)
- [ ] Performance tuning (caching, debouncing)
- [ ] Multi-editor testing (Neovim, Emacs configs)

---

## Testing Strategy

### Consistency Testing

**Critical test**: CLI and LSP must produce identical results

```typescript
// packages/validation-service/test/consistency.test.ts

describe('CLI and LSP consistency', () => {
  const testCases = [
    'valid-speckit-plan.md',
    'invalid-missing-intent.md',
    'valid-aps.json',
    'invalid-hash-mismatch.json',
  ];

  for (const testFile of testCases) {
    test(`${testFile}: CLI and LSP agree`, async () => {
      const content = await readFile(testFile, 'utf-8');

      // Validate via ValidationService (used by both CLI and LSP)
      const service = new ValidationService();
      const result = await service.validateContent(content);

      // Simulate CLI
      const cliOutput = formatAsCLI(result);

      // Simulate LSP
      const lspDiagnostics = formatAsLSPDiagnostics(result);

      // They must report the same issues
      expect(result.success).toBe(cliShouldPass);
      expect(lspDiagnostics.length).toBe(cliOutput.errorCount);
    });
  }
});
```

### Performance Testing

```typescript
describe('LSP performance', () => {
  test('validates large plan in <300ms', async () => {
    const largePlan = generateLargePlan(1000); // 1000 steps

    const start = Date.now();
    const result = await service.quickValidate(largePlan);
    const duration = Date.now() - start;

    expect(duration).toBeLessThan(300);
  });
});
```

---

## Non-VS Code Users

Users without VS Code still have full functionality:

### CLI Standalone Usage

```bash
# All CLI commands work without LSP
anvil validate plan.md           # Full validation
anvil gate plan.md               # Run quality gates
anvil export plan.md --to aps    # Format conversion
anvil watch                      # File watching (CLI-based)
```

### Multi-Editor LSP Support

Users in other editors get LSP features:

**Neovim**:

```lua
-- Install Anvil LSP globally
-- $ npm install -g @eddacraft/anvil-language-server

require('lspconfig').anvil_lsp.setup{}
```

**Emacs**:

```elisp
(use-package lsp-mode
  :config
  (lsp-register-client
   (make-lsp-client
    :new-connection (lsp-stdio-connection '("anvil-lsp" "--stdio"))
    :activation-fn (lsp-activate-on "markdown")
    :server-id 'anvil-lsp)))
```

---

## Maintenance Benefits

### Single Bug Fix Location

**Before** (hypothetical duplication):

```
Bug: Hash validation has off-by-one error

Fix in: CLI validation logic
Fix in: LSP validation logic  (duplication!)
Test: Both implementations
```

**After** (shared ValidationService):

```
Bug: Hash validation has off-by-one error

Fix in: @eddacraft/anvil-core/crypto/hash.ts  (one place!)
Test: ValidationService
Result: CLI and LSP both fixed automatically
```

### Feature Addition

**Example**: Add new gate type "architecture"

```typescript
// 1. Add to @eddacraft/anvil-core (one place)
export const GateType = z.enum([
  'lint',
  'test',
  'coverage',
  'secrets',
  'dependencies',
  'architecture', // ← new gate
]);

// 2. CLI automatically supports it (uses @eddacraft/anvil-core)
anvil gate plan.md  // ✓ Runs architecture gate

// 3. LSP automatically supports it (uses @eddacraft/anvil-core)
// VS Code shows diagnostic if architecture gate fails
```

---

## Success Metrics

### Performance

- **Real-time validation**: <300ms after typing stops
- **Save validation**: <500ms (includes hash validation)
- **Large plans**: <1s for 1000+ steps

### Consistency

- **CLI ↔ LSP agreement**: 100% (they use same code)
- **Regression tests**: All existing CLI tests pass after refactor

### Adoption

- **VS Code users**: LSP features enabled by default
- **CLI users**: Unchanged experience, no regression
- **Multi-editor**: Neovim/Emacs configs documented

---

## Risks & Mitigations

| Risk                               | Impact | Mitigation                                       |
| ---------------------------------- | ------ | ------------------------------------------------ |
| LSP adds complexity                | Medium | Keep LSP layer thin, all logic in shared service |
| CLI and LSP diverge                | High   | Shared `ValidationService`, consistency tests    |
| Performance regression             | Medium | Progressive validation levels, caching           |
| Breaking CLI for non-VS Code users | High   | CLI has zero LSP dependencies                    |

---

## Dependencies

### New Packages

- `vscode-languageserver` (^9.0.0) - LSP protocol implementation
- `vscode-languageserver-textdocument` (^1.0.0) - Document management

### Existing Packages (no changes)

- `@eddacraft/anvil-core` - Validation logic (single source of truth)
- `@eddacraft/anvil-adapters` - Format detection and parsing
- All existing CLI dependencies

---

## Conclusion

This architecture achieves all constraints:

✅ **CLI-first**: CLI works standalone, LSP is enhancement ✅ **Zero
duplication**: `ValidationService` is single source of truth ✅ **Non-VS Code
users**: CLI fully functional, multi-editor LSP support ✅ **Minimal
maintenance**: Fix bugs once in `@eddacraft/anvil-core`

**Next Steps**:

1. Review and approve this plan
2. Create `packages/validation-service/` stub
3. Begin Phase 1: Extract ValidationService

---
