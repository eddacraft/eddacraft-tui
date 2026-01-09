import { describe, it, expect, beforeEach, vi } from 'vitest';
import { GateResultsProvider } from '../gateResultsProvider.js';
import { AnvilService } from '../../services/anvilService.js';
import type { GateResults } from '../../services/anvilService.js';
import * as vscode from 'vscode';

describe('GateResultsProvider', () => {
  let provider: GateResultsProvider;
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
    provider = new GateResultsProvider(mockAnvilService);
  });

  describe('updateResults', () => {
    it('should store gate results for a file', async () => {
      const results: GateResults = {
        success: true,
        gates: [
          { name: 'lint', status: 'passed' },
          { name: 'test', status: 'passed' },
        ],
        timestamp: new Date().toISOString(),
        duration: 100,
      };

      provider.updateResults('/test/plan.md', results);

      const children = await provider.getChildren();
      expect(children.length).toBe(1);
    });

    it('should trigger tree refresh on update', () => {
      let refreshCalled = false;
      provider.onDidChangeTreeData(() => {
        refreshCalled = true;
      });

      const results: GateResults = {
        success: true,
        gates: [{ name: 'lint', status: 'passed' }],
        timestamp: new Date().toISOString(),
        duration: 100,
      };

      provider.updateResults('/test/plan.md', results);

      expect(refreshCalled).toBe(true);
    });
  });

  describe('getChildren', () => {
    it('should return file categories at root level', async () => {
      const results: GateResults = {
        success: true,
        gates: [{ name: 'lint', status: 'passed' }],
        timestamp: new Date().toISOString(),
        duration: 100,
      };

      provider.updateResults('/test/plan.md', results);

      const children = await provider.getChildren();
      expect(children.length).toBe(1);
      expect(children[0].label).toBe('plan.md');
    });

    it('should show passed/total count in file category', async () => {
      const results: GateResults = {
        success: false,
        gates: [
          { name: 'lint', status: 'passed' },
          { name: 'test', status: 'failed' },
          { name: 'coverage', status: 'passed' },
        ],
        timestamp: new Date().toISOString(),
        duration: 100,
      };

      provider.updateResults('/test/plan.md', results);

      const children = await provider.getChildren();
      expect(children[0].description).toBe('2/3 passed');
    });

    it('should return gates for a file category', async () => {
      const results: GateResults = {
        success: true,
        gates: [
          { name: 'lint', status: 'passed' },
          { name: 'test', status: 'passed' },
        ],
        timestamp: new Date().toISOString(),
        duration: 100,
      };

      provider.updateResults('/test/plan.md', results);

      const [fileCategory] = await provider.getChildren();
      const gates = await provider.getChildren(fileCategory);

      expect(gates.length).toBe(2);
    });
  });

  describe('clearResults', () => {
    it('should clear results for specific file', async () => {
      const results: GateResults = {
        success: true,
        gates: [{ name: 'lint', status: 'passed' }],
        timestamp: new Date().toISOString(),
        duration: 100,
      };

      provider.updateResults('/test/plan.md', results);
      expect((await provider.getChildren()).length).toBe(1);

      provider.clearResults('/test/plan.md');

      expect((await provider.getChildren()).length).toBe(0);
    });

    it('should clear all results when no file specified', async () => {
      const results: GateResults = {
        success: true,
        gates: [{ name: 'lint', status: 'passed' }],
        timestamp: new Date().toISOString(),
        duration: 100,
      };

      provider.updateResults('/test/plan1.md', results);
      provider.updateResults('/test/plan2.md', results);
      expect((await provider.getChildren()).length).toBe(2);

      provider.clearResults();

      expect((await provider.getChildren()).length).toBe(0);
    });
  });

  describe('refresh', () => {
    it('should fire onDidChangeTreeData event', () => {
      let eventFired = false;
      provider.onDidChangeTreeData(() => {
        eventFired = true;
      });

      provider.refresh();

      expect(eventFired).toBe(true);
    });
  });

  describe('tree item icons', () => {
    it('should use pass icon for successful results', async () => {
      const results: GateResults = {
        success: true,
        gates: [{ name: 'lint', status: 'passed' }],
        timestamp: new Date().toISOString(),
        duration: 100,
      };

      provider.updateResults('/test/plan.md', results);

      const [fileCategory] = await provider.getChildren();
      expect(fileCategory.iconPath).toBeDefined();
    });

    it('should use error icon for failed results', async () => {
      const results: GateResults = {
        success: false,
        gates: [{ name: 'lint', status: 'failed' }],
        timestamp: new Date().toISOString(),
        duration: 100,
      };

      provider.updateResults('/test/plan.md', results);

      const [fileCategory] = await provider.getChildren();
      expect(fileCategory.iconPath).toBeDefined();
    });
  });

  describe('gate status display', () => {
    it('should display passed gates correctly', async () => {
      const results: GateResults = {
        success: true,
        gates: [{ name: 'lint', status: 'passed', message: 'All checks passed' }],
        timestamp: new Date().toISOString(),
        duration: 100,
      };

      provider.updateResults('/test/plan.md', results);

      const [fileCategory] = await provider.getChildren();
      const [gate] = await provider.getChildren(fileCategory);

      expect(gate.label).toBe('lint');
      expect(gate.description).toContain('passed');
    });

    it('should display failed gates correctly', async () => {
      const results: GateResults = {
        success: false,
        gates: [{ name: 'test', status: 'failed', message: 'Tests failed' }],
        timestamp: new Date().toISOString(),
        duration: 100,
      };

      provider.updateResults('/test/plan.md', results);

      const [fileCategory] = await provider.getChildren();
      const [gate] = await provider.getChildren(fileCategory);

      expect(gate.label).toBe('test');
      expect(gate.description).toContain('failed');
    });

    it('should display skipped gates correctly', async () => {
      const results: GateResults = {
        success: true,
        gates: [{ name: 'coverage', status: 'skipped', message: 'Skipped' }],
        timestamp: new Date().toISOString(),
        duration: 100,
      };

      provider.updateResults('/test/plan.md', results);

      const [fileCategory] = await provider.getChildren();
      const [gate] = await provider.getChildren(fileCategory);

      expect(gate.label).toBe('coverage');
      expect(gate.description).toContain('Skipped');
    });
  });
});
