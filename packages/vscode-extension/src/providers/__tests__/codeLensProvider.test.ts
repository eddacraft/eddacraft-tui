import { describe, it, expect, beforeEach, vi } from 'vitest';
import { PlanCodeLensProvider } from '../codeLensProvider.js';
import { AnvilService } from '../../services/anvilService.js';
import * as vscode from 'vscode';

describe('PlanCodeLensProvider', () => {
  let provider: PlanCodeLensProvider;
  let mockAnvilService: AnvilService;
  let mockContext: vscode.ExtensionContext;

  beforeEach(() => {
    vi.clearAllMocks();
    mockContext = {
      subscriptions: [],
      extensionPath: '/test',
      globalState: { get: vi.fn(), update: vi.fn() },
      workspaceState: { get: vi.fn(), update: vi.fn() },
    };
    mockAnvilService = new AnvilService(mockContext);
    provider = new PlanCodeLensProvider(mockAnvilService);
  });

  describe('provideCodeLenses', () => {
    it('should provide CodeLens for plan files', () => {
      const document: vscode.TextDocument = {
        uri: vscode.Uri.file('/test/plan.md'),
        fileName: '/test/plan.md',
        languageId: 'markdown',
        getText: () => '# Test Plan\n## Tasks\n- Task 1',
      };

      const codeLenses = provider.provideCodeLenses(
        document,
        {} as vscode.CancellationToken
      ) as vscode.CodeLens[];

      expect(codeLenses).toBeDefined();
      expect(codeLenses.length).toBeGreaterThan(0);

      // Should have Validate, Run Gates, and Export
      const titles = codeLenses.map((cl) => cl.command?.title);
      expect(titles).toContain('$(check) Validate');
      expect(titles).toContain('$(shield) Run Gates');
      expect(titles).toContain('$(export) Export');
    });

    it('should not provide CodeLens for non-plan files', () => {
      const document: vscode.TextDocument = {
        uri: vscode.Uri.file('/test/readme.md'),
        fileName: '/test/readme.md',
        languageId: 'markdown',
        getText: () => '# README',
      };

      const codeLenses = provider.provideCodeLenses(
        document,
        {} as vscode.CancellationToken
      ) as vscode.CodeLens[];

      expect(codeLenses).toBeDefined();
      expect(codeLenses).toHaveLength(0);
    });

    it('should include last validation result in CodeLens', () => {
      vi.spyOn(mockAnvilService, 'getLastValidationResult').mockReturnValue({
        success: true,
        planId: 'test-123',
        format: 'speckit',
        errors: [],
        warnings: [],
      });

      const document: vscode.TextDocument = {
        uri: vscode.Uri.file('/test/plan.md'),
        fileName: '/test/plan.md',
        languageId: 'markdown',
        getText: () => '# Plan',
      };

      const codeLenses = provider.provideCodeLenses(
        document,
        {} as vscode.CancellationToken
      ) as vscode.CodeLens[];

      const statusLens = codeLenses.find((cl) => cl.command?.title.includes('$(pass)'));
      expect(statusLens).toBeDefined();
      expect(statusLens!.command!.title).toContain('Valid');
    });

    it('should include error status in CodeLens', () => {
      vi.spyOn(mockAnvilService, 'getLastValidationResult').mockReturnValue({
        success: false,
        errors: [{ message: 'Error 1' }, { message: 'Error 2' }],
        warnings: [],
      });

      const document: vscode.TextDocument = {
        uri: vscode.Uri.file('/test/plan.md'),
        fileName: '/test/plan.md',
        languageId: 'markdown',
        getText: () => '# Plan',
      };

      const codeLenses = provider.provideCodeLenses(
        document,
        {} as vscode.CancellationToken
      ) as vscode.CodeLens[];

      const statusLens = codeLenses.find((cl) => cl.command?.title.includes('$(error)'));
      expect(statusLens).toBeDefined();
      expect(statusLens!.command!.title).toContain('2 error(s)');
    });

    it('should include gate results in CodeLens', () => {
      vi.spyOn(mockAnvilService, 'getLastGateResults').mockReturnValue({
        success: true,
        gates: [
          { name: 'lint', status: 'passed' },
          { name: 'test', status: 'passed' },
        ],
        timestamp: new Date().toISOString(),
        duration: 100,
      });

      const document: vscode.TextDocument = {
        uri: vscode.Uri.file('/test/plan.md'),
        fileName: '/test/plan.md',
        languageId: 'markdown',
        getText: () => '# Plan',
      };

      const codeLenses = provider.provideCodeLenses(
        document,
        {} as vscode.CancellationToken
      ) as vscode.CodeLens[];

      const gateLens = codeLenses.find((cl) => cl.command?.title.includes('Gates:'));
      expect(gateLens).toBeDefined();
      expect(gateLens!.command!.title).toContain('2/2');
    });
  });

  describe('file detection', () => {
    it('should recognize .plan.md files', () => {
      const document: vscode.TextDocument = {
        uri: vscode.Uri.file('/test/feature.plan.md'),
        fileName: '/test/feature.plan.md',
        languageId: 'markdown',
        getText: () => '# Plan',
      };

      const codeLenses = provider.provideCodeLenses(
        document,
        {} as vscode.CancellationToken
      ) as vscode.CodeLens[];

      expect(codeLenses.length).toBeGreaterThan(0);
    });

    it('should recognize .spec.md files', () => {
      const document: vscode.TextDocument = {
        uri: vscode.Uri.file('/test/spec.md'),
        fileName: '/test/spec.md',
        languageId: 'markdown',
        getText: () => '# Spec',
      };

      const codeLenses = provider.provideCodeLenses(
        document,
        {} as vscode.CancellationToken
      ) as vscode.CodeLens[];

      expect(codeLenses.length).toBeGreaterThan(0);
    });

    it('should recognize .aps.json files', () => {
      const document: vscode.TextDocument = {
        uri: vscode.Uri.file('/test/plan.aps.json'),
        fileName: '/test/plan.aps.json',
        languageId: 'json',
        getText: () => '{}',
      };

      const codeLenses = provider.provideCodeLenses(
        document,
        {} as vscode.CancellationToken
      ) as vscode.CodeLens[];

      expect(codeLenses.length).toBeGreaterThan(0);
    });

    it('should recognize markdown files with plan markers', () => {
      const document: vscode.TextDocument = {
        uri: vscode.Uri.file('/test/doc.md'),
        fileName: '/test/doc.md',
        languageId: 'markdown',
        getText: () => '# Document\n## Tasks\n- Task 1',
      };

      const codeLenses = provider.provideCodeLenses(
        document,
        {} as vscode.CancellationToken
      ) as vscode.CodeLens[];

      expect(codeLenses.length).toBeGreaterThan(0);
    });
  });

  describe('refresh', () => {
    it('should fire onDidChangeCodeLenses event', () => {
      let eventFired = false;
      provider.onDidChangeCodeLenses(() => {
        eventFired = true;
      });

      provider.refresh();

      expect(eventFired).toBe(true);
    });
  });
});
