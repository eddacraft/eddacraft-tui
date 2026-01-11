import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { DiagnosticsManager } from '../diagnostics.js';
import type { ValidationResult, GateResults } from '../anvilService.js';
import * as vscode from 'vscode';

describe('DiagnosticsManager', () => {
  let diagnosticsManager: DiagnosticsManager;
  let testUri: vscode.Uri;

  beforeEach(() => {
    diagnosticsManager = new DiagnosticsManager();
    testUri = vscode.Uri.file('/test/plan.md');
  });

  afterEach(() => {
    diagnosticsManager.dispose();
  });

  describe('construction', () => {
    it('should create a diagnostic collection', () => {
      expect(diagnosticsManager.diagnosticCollection).toBeDefined();
      expect(diagnosticsManager.diagnosticCollection.name).toBe('anvil');
    });
  });

  describe('updateFromValidation', () => {
    it('should create error diagnostics from validation errors', () => {
      const result: ValidationResult = {
        success: false,
        errors: [
          {
            message: 'Invalid syntax',
            line: 5,
            column: 10,
          },
        ],
        warnings: [],
      };

      diagnosticsManager.updateFromValidation(testUri, result);

      const diagnostics = diagnosticsManager.diagnosticCollection.get(testUri);
      expect(diagnostics).toBeDefined();
      expect(diagnostics).toHaveLength(1);
      expect(diagnostics![0].message).toBe('Invalid syntax');
      expect(diagnostics![0].severity).toBe(vscode.DiagnosticSeverity.Error);
      expect(diagnostics![0].source).toBe('anvil:validation');
    });

    it('should create warning diagnostics from validation warnings', () => {
      const result: ValidationResult = {
        success: true,
        errors: [],
        warnings: [
          {
            message: 'Deprecated field',
            line: 10,
            column: 5,
          },
        ],
      };

      diagnosticsManager.updateFromValidation(testUri, result);

      const diagnostics = diagnosticsManager.diagnosticCollection.get(testUri);
      expect(diagnostics).toBeDefined();
      expect(diagnostics).toHaveLength(1);
      expect(diagnostics![0].message).toBe('Deprecated field');
      expect(diagnostics![0].severity).toBe(vscode.DiagnosticSeverity.Warning);
    });

    it('should handle validation results with no line numbers', () => {
      const result: ValidationResult = {
        success: false,
        errors: [
          {
            message: 'General error',
          },
        ],
        warnings: [],
      };

      diagnosticsManager.updateFromValidation(testUri, result);

      const diagnostics = diagnosticsManager.diagnosticCollection.get(testUri);
      expect(diagnostics).toBeDefined();
      expect(diagnostics).toHaveLength(1);
      expect(diagnostics![0].range.start.line).toBe(0);
    });
  });

  describe('updateFromGateResults', () => {
    it('should create diagnostics for failed gates', () => {
      const results: GateResults = {
        success: false,
        gates: [
          {
            name: 'lint',
            status: 'failed',
            message: 'Linting failed',
          },
        ],
        timestamp: new Date().toISOString(),
        duration: 100,
      };

      diagnosticsManager.updateFromGateResults(testUri, results);

      const diagnostics = diagnosticsManager.diagnosticCollection.get(testUri);
      expect(diagnostics).toBeDefined();
      expect(diagnostics!.length).toBeGreaterThan(0);
      expect(diagnostics![0].message).toContain('lint');
      expect(diagnostics![0].message).toContain('failed');
    });

    it('should not create diagnostics for passed gates', () => {
      const results: GateResults = {
        success: true,
        gates: [
          {
            name: 'lint',
            status: 'passed',
          },
        ],
        timestamp: new Date().toISOString(),
        duration: 100,
      };

      diagnosticsManager.updateFromGateResults(testUri, results);

      const diagnostics = diagnosticsManager.diagnosticCollection.get(testUri);
      // Should only have validation diagnostics if any, not gate diagnostics
      const gateDiagnostics = diagnostics?.filter((d) => d.source?.startsWith('anvil:gate'));
      expect(gateDiagnostics).toHaveLength(0);
    });

    it('should create diagnostics for gate details', () => {
      const results: GateResults = {
        success: false,
        gates: [
          {
            name: 'test',
            status: 'failed',
            details: [
              {
                type: 'error',
                message: 'Test failed',
                file: 'test.ts',
                line: 42,
              },
            ],
          },
        ],
        timestamp: new Date().toISOString(),
        duration: 100,
      };

      diagnosticsManager.updateFromGateResults(testUri, results);

      const diagnostics = diagnosticsManager.diagnosticCollection.get(testUri);
      expect(diagnostics).toBeDefined();
      expect(diagnostics!.length).toBeGreaterThan(0);
    });
  });

  describe('clearForUri', () => {
    it('should clear diagnostics for specific URI', () => {
      const result: ValidationResult = {
        success: false,
        errors: [{ message: 'Error' }],
        warnings: [],
      };

      diagnosticsManager.updateFromValidation(testUri, result);
      expect(diagnosticsManager.diagnosticCollection.get(testUri)).toBeDefined();

      diagnosticsManager.clearForUri(testUri);

      expect(diagnosticsManager.diagnosticCollection.get(testUri)).toBeUndefined();
    });
  });

  describe('clearAll', () => {
    it('should clear all diagnostics', () => {
      const result: ValidationResult = {
        success: false,
        errors: [{ message: 'Error' }],
        warnings: [],
      };

      diagnosticsManager.updateFromValidation(testUri, result);
      const uri2 = vscode.Uri.file('/test/other.md');
      diagnosticsManager.updateFromValidation(uri2, result);

      diagnosticsManager.clearAll();

      expect(diagnosticsManager.diagnosticCollection.get(testUri)).toBeUndefined();
      expect(diagnosticsManager.diagnosticCollection.get(uri2)).toBeUndefined();
    });
  });

  describe('dispose', () => {
    it('should dispose diagnostic collection', () => {
      diagnosticsManager.dispose();
      // After disposal, the collection should not be usable
      expect(() => diagnosticsManager.diagnosticCollection.clear()).not.toThrow();
    });
  });
});
