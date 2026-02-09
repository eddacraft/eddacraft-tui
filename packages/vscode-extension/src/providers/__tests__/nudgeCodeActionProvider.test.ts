import { describe, it, expect, vi, beforeEach } from 'vitest';
import * as vscode from 'vscode';
import { NudgeCodeActionProvider } from '../nudgeCodeActionProvider.js';

// Mock the embedded analysis service
vi.mock('../../services/embeddedAnalysis.js', () => {
  const mockPatterns: Record<string, { nudge?: string }> = {
    'AP-001': {
      nudge:
        "Don't disable all linting rules. Identify which specific rule is failing and " +
        'either fix the underlying issue or disable only that one rule with ' +
        '`/* eslint-disable specific-rule */`. Blanket disables hide real problems.',
    },
    'AP-003': {
      nudge:
        "Don't use `any` here. Think about what type this value actually holds and " +
        'declare it explicitly.',
    },
    'AP-004': {
      nudge:
        "Don't suppress this TypeScript error — fix it. If you must suppress, use " +
        '`@ts-expect-error` instead.',
    },
  };

  return {
    getEmbeddedAnalysisService: () => ({
      getPatternInfo: (id: string) => mockPatterns[id],
    }),
  };
});

function createDiagnostic(
  patternId: string,
  line: number = 0,
  message: string = 'test'
): vscode.Diagnostic {
  const range = new vscode.Range(new vscode.Position(line, 0), new vscode.Position(line, 20));
  const diagnostic = new vscode.Diagnostic(range, message, vscode.DiagnosticSeverity.Warning);
  diagnostic.source = 'anvil:antipattern';
  diagnostic.code = { value: patternId, target: vscode.Uri.parse(`https://example.com`) };
  return diagnostic;
}

function createMockDocument(content: string = ''): vscode.TextDocument {
  return {
    uri: vscode.Uri.file('/test/file.ts'),
    fileName: '/test/file.ts',
    languageId: 'typescript',
    getText: vi.fn().mockReturnValue(content),
  } as unknown as vscode.TextDocument;
}

describe('NudgeCodeActionProvider', () => {
  let provider: NudgeCodeActionProvider;

  beforeEach(() => {
    provider = new NudgeCodeActionProvider();
  });

  it('should provide a Rethink action for an antipattern diagnostic', () => {
    const diagnostic = createDiagnostic('AP-003');
    const document = createMockDocument();
    const range = new vscode.Range(new vscode.Position(0, 0), new vscode.Position(0, 20));
    const context = { diagnostics: [diagnostic] } as vscode.CodeActionContext;

    const actions = provider.provideCodeActions(document, range, context);

    expect(actions.length).toBeGreaterThanOrEqual(1);
    const rethink = actions.find((a) => a.title.startsWith('Anvil: Rethink'));
    expect(rethink).toBeDefined();
    expect(rethink!.title).toContain("Don't use `any` here");
    expect(rethink!.command?.command).toBe('anvil.showNudge');
  });

  it('should not provide actions for non-anvil diagnostics', () => {
    const range = new vscode.Range(new vscode.Position(0, 0), new vscode.Position(0, 20));
    const diagnostic = new vscode.Diagnostic(range, 'test', vscode.DiagnosticSeverity.Warning);
    diagnostic.source = 'eslint';
    diagnostic.code = { value: 'no-console', target: vscode.Uri.parse('https://example.com') };

    const document = createMockDocument();
    const context = { diagnostics: [diagnostic] } as vscode.CodeActionContext;

    const actions = provider.provideCodeActions(document, range, context);

    expect(actions).toHaveLength(0);
  });

  it('should provide a deterministic fix for AP-004 (@ts-ignore)', () => {
    const diagnostic = createDiagnostic('AP-004');
    const document = createMockDocument('// @ts-ignore');
    const range = new vscode.Range(new vscode.Position(0, 0), new vscode.Position(0, 20));
    const context = { diagnostics: [diagnostic] } as vscode.CodeActionContext;

    const actions = provider.provideCodeActions(document, range, context);

    const fix = actions.find((a) => a.title.includes('Anvil: Fix'));
    expect(fix).toBeDefined();
    expect(fix!.title).toContain('@ts-ignore');
    expect(fix!.title).toContain('@ts-expect-error');
    expect(fix!.isPreferred).toBe(true);
    expect(fix!.edit).toBeDefined();
  });

  it('should not provide a fix action for patterns without deterministic fixes', () => {
    const diagnostic = createDiagnostic('AP-003');
    const document = createMockDocument('const x: any = 1;');
    const range = new vscode.Range(new vscode.Position(0, 0), new vscode.Position(0, 20));
    const context = { diagnostics: [diagnostic] } as vscode.CodeActionContext;

    const actions = provider.provideCodeActions(document, range, context);

    const fix = actions.find((a) => a.title.includes('Anvil: Fix'));
    expect(fix).toBeUndefined();
  });

  it('should truncate long nudge text in action title', () => {
    const diagnostic = createDiagnostic('AP-001');
    const document = createMockDocument();
    const range = new vscode.Range(new vscode.Position(0, 0), new vscode.Position(0, 20));
    const context = { diagnostics: [diagnostic] } as vscode.CodeActionContext;

    const actions = provider.provideCodeActions(document, range, context);

    const rethink = actions.find((a) => a.title.startsWith('Anvil: Rethink'));
    expect(rethink).toBeDefined();
    // Title should be capped (prefix + truncated nudge)
    expect(rethink!.title.length).toBeLessThanOrEqual(120);
  });

  it('should handle multiple diagnostics in context', () => {
    const diag1 = createDiagnostic('AP-003', 0);
    const diag2 = createDiagnostic('AP-004', 5);
    const document = createMockDocument('// @ts-ignore');
    const range = new vscode.Range(new vscode.Position(0, 0), new vscode.Position(10, 0));
    const context = { diagnostics: [diag1, diag2] } as vscode.CodeActionContext;

    const actions = provider.provideCodeActions(document, range, context);

    const rethinks = actions.filter((a) => a.title.startsWith('Anvil: Rethink'));
    expect(rethinks).toHaveLength(2);

    const fixes = actions.filter((a) => a.title.includes('Anvil: Fix'));
    expect(fixes).toHaveLength(1); // Only AP-004 has a fix
  });

  it('should advertise QuickFix code action kind', () => {
    expect(NudgeCodeActionProvider.providedCodeActionKinds).toContain(
      vscode.CodeActionKind.QuickFix
    );
  });
});
