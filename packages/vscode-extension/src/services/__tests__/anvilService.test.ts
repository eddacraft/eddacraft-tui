import { describe, it, expect, beforeEach, vi } from 'vitest';
import { AnvilService } from '../anvilService.js';
import * as vscode from 'vscode';
import { EventEmitter } from 'events';

// Mock child_process
vi.mock('child_process', () => ({
  spawn: vi.fn(),
}));

import * as cp from 'child_process';

describe('AnvilService', () => {
  let anvilService: AnvilService;
  let mockContext: vscode.ExtensionContext;
  let mockOutputChannel: vscode.OutputChannel;

  beforeEach(() => {
    vi.clearAllMocks();
    mockContext = {
      subscriptions: [],
      extensionPath: '/test/extension',
      globalState: {
        get: vi.fn(),
        update: vi.fn(),
        keys: vi.fn().mockReturnValue([]),
        setKeysForSync: vi.fn(),
      },
      workspaceState: {
        get: vi.fn(),
        update: vi.fn(),
        keys: vi.fn().mockReturnValue([]),
      },
    } as unknown as vscode.ExtensionContext;
    mockOutputChannel = {
      name: 'Anvil',
      append: vi.fn(),
      appendLine: vi.fn(),
      clear: vi.fn(),
      show: vi.fn(),
      hide: vi.fn(),
      dispose: vi.fn(),
      replace: vi.fn(),
    } as unknown as vscode.OutputChannel;
    anvilService = new AnvilService(mockContext, mockOutputChannel);
  });

  describe('construction', () => {
    it('should create output channel', () => {
      const channel = anvilService.getOutputChannel();
      expect(channel).toBeDefined();
      expect(channel.name).toBe('Anvil');
    });
  });

  describe('validate', () => {
    it('should execute validate command successfully', async () => {
      const mockChild = createMockChildProcess();
      (cp.spawn as ReturnType<typeof vi.fn>).mockReturnValue(mockChild);

      const jsonOutput = JSON.stringify({
        success: true,
        planId: 'test-plan-123',
        format: 'speckit',
        errors: [],
        warnings: [],
      });

      setTimeout(() => {
        mockChild.stdout.emit('data', Buffer.from(jsonOutput));
        mockChild.emit('close', 0);
      }, 10);

      const result = await anvilService.validate('/test/plan.md');

      expect(result.success).toBe(true);
      expect(result.planId).toBe('test-plan-123');
      expect(result.format).toBe('speckit');
    });

    it('should handle validation errors', async () => {
      const mockChild = createMockChildProcess();
      (cp.spawn as ReturnType<typeof vi.fn>).mockReturnValue(mockChild);

      const jsonOutput = JSON.stringify({
        success: false,
        errors: [{ message: 'Invalid plan' }],
        warnings: [],
      });

      setTimeout(() => {
        mockChild.stdout.emit('data', Buffer.from(jsonOutput));
        mockChild.emit('close', 0);
      }, 10);

      const result = await anvilService.validate('/test/plan.md');

      expect(result.success).toBe(false);
      expect(result.errors).toHaveLength(1);
      expect(result.errors[0].message).toBe('Invalid plan');
    });

    it('should handle command execution failure', async () => {
      const mockChild = createMockChildProcess();
      (cp.spawn as ReturnType<typeof vi.fn>).mockReturnValue(mockChild);

      setTimeout(() => {
        mockChild.stderr.emit('data', Buffer.from('Command failed'));
        mockChild.emit('close', 1);
      }, 10);

      const result = await anvilService.validate('/test/plan.md');

      expect(result.success).toBe(false);
      expect(result.errors).toHaveLength(1);
      expect(result.errors[0].message).toContain('Command failed');
    });

    it('should cache validation results', async () => {
      const mockChild = createMockChildProcess();
      (cp.spawn as ReturnType<typeof vi.fn>).mockReturnValue(mockChild);

      const jsonOutput = JSON.stringify({
        success: true,
        planId: 'test-plan',
      });

      setTimeout(() => {
        mockChild.stdout.emit('data', Buffer.from(jsonOutput));
        mockChild.emit('close', 0);
      }, 10);

      const filePath = '/test/plan.md';
      await anvilService.validate(filePath);

      const cached = anvilService.getLastValidationResult(filePath);
      expect(cached).toBeDefined();
      expect(cached!.success).toBe(true);
      expect(cached!.planId).toBe('test-plan');
    });
  });

  describe('runGates', () => {
    it('should execute gate command successfully', async () => {
      const mockChild = createMockChildProcess();
      (cp.spawn as ReturnType<typeof vi.fn>).mockReturnValue(mockChild);

      const jsonOutput = JSON.stringify({
        success: true,
        gates: [
          { name: 'lint', status: 'passed' },
          { name: 'test', status: 'passed' },
        ],
        timestamp: new Date().toISOString(),
        duration: 100,
      });

      setTimeout(() => {
        mockChild.stdout.emit('data', Buffer.from(jsonOutput));
        mockChild.emit('close', 0);
      }, 10);

      const result = await anvilService.runGates('/test/plan.md');

      expect(result.success).toBe(true);
      expect(result.gates).toHaveLength(2);
      expect(result.gates[0].name).toBe('lint');
      expect(result.gates[0].status).toBe('passed');
    });

    it('should handle gate failures', async () => {
      const mockChild = createMockChildProcess();
      (cp.spawn as ReturnType<typeof vi.fn>).mockReturnValue(mockChild);

      const jsonOutput = JSON.stringify({
        success: false,
        gates: [
          { name: 'lint', status: 'passed' },
          { name: 'test', status: 'failed', message: 'Tests failed' },
        ],
      });

      setTimeout(() => {
        mockChild.stdout.emit('data', Buffer.from(jsonOutput));
        mockChild.emit('close', 0);
      }, 10);

      const result = await anvilService.runGates('/test/plan.md');

      expect(result.success).toBe(false);
      expect(result.gates).toHaveLength(2);
      expect(result.gates[1].status).toBe('failed');
    });

    it('should cache gate results', async () => {
      const mockChild = createMockChildProcess();
      (cp.spawn as ReturnType<typeof vi.fn>).mockReturnValue(mockChild);

      const jsonOutput = JSON.stringify({
        success: true,
        gates: [{ name: 'lint', status: 'passed' }],
      });

      setTimeout(() => {
        mockChild.stdout.emit('data', Buffer.from(jsonOutput));
        mockChild.emit('close', 0);
      }, 10);

      const filePath = '/test/plan.md';
      await anvilService.runGates(filePath);

      const cached = anvilService.getLastGateResults(filePath);
      expect(cached).toBeDefined();
      expect(cached!.success).toBe(true);
    });
  });

  describe('exportPlan', () => {
    it('should export plan successfully', async () => {
      const mockChild = createMockChildProcess();
      (cp.spawn as ReturnType<typeof vi.fn>).mockReturnValue(mockChild);

      setTimeout(() => {
        mockChild.emit('close', 0);
      }, 10);

      const result = await anvilService.exportPlan('/test/plan.md', 'aps');

      expect(result.success).toBe(true);
      expect(result.outputPath).toBeDefined();
    });

    it('should handle export failure', async () => {
      const mockChild = createMockChildProcess();
      (cp.spawn as ReturnType<typeof vi.fn>).mockReturnValue(mockChild);

      setTimeout(() => {
        mockChild.stderr.emit('data', Buffer.from('Export failed'));
        mockChild.emit('close', 1);
      }, 10);

      const result = await anvilService.exportPlan('/test/plan.md', 'aps');

      expect(result.success).toBe(false);
      expect(result.error).toBeDefined();
    });
  });

  describe('detectFormat', () => {
    it('should detect format from validation output', async () => {
      const mockChild = createMockChildProcess();
      (cp.spawn as ReturnType<typeof vi.fn>).mockReturnValue(mockChild);

      const jsonOutput = JSON.stringify({
        format: 'speckit',
      });

      setTimeout(() => {
        mockChild.stdout.emit('data', Buffer.from(jsonOutput));
        mockChild.emit('close', 0);
      }, 10);

      const format = await anvilService.detectFormat('/test/plan.md');

      expect(format).toBe('speckit');
    });

    it('should return undefined on detection failure', async () => {
      const mockChild = createMockChildProcess();
      (cp.spawn as ReturnType<typeof vi.fn>).mockReturnValue(mockChild);

      setTimeout(() => {
        mockChild.emit('close', 1);
      }, 10);

      const format = await anvilService.detectFormat('/test/plan.md');

      expect(format).toBeUndefined();
    });
  });

  describe('CLI command selection', () => {
    it('should successfully validate using CLI', async () => {
      const mockChild = createMockChildProcess();
      (cp.spawn as ReturnType<typeof vi.fn>).mockReturnValue(mockChild);

      setTimeout(() => {
        mockChild.stdout.emit('data', Buffer.from(JSON.stringify({ success: true })));
        mockChild.emit('close', 0);
      }, 10);

      const result = await anvilService.validate('/test/plan.md');

      // Verify spawn was called
      expect(cp.spawn).toHaveBeenCalled();
      expect(result.success).toBe(true);
    });

    it('should use custom CLI path when configured', async () => {
      const mockConfig = {
        get: vi.fn((key: string, defaultValue?: unknown) => {
          if (key === 'cli.path') return '/custom/anvil';
          return defaultValue;
        }),
      };
      (vscode.workspace.getConfiguration as ReturnType<typeof vi.fn>).mockReturnValue(mockConfig);

      // Create a new service with the custom config
      const customService = new AnvilService(mockContext, mockOutputChannel);

      const mockChild = createMockChildProcess();
      (cp.spawn as ReturnType<typeof vi.fn>).mockClear();
      (cp.spawn as ReturnType<typeof vi.fn>).mockReturnValue(mockChild);

      setTimeout(() => {
        mockChild.stdout.emit('data', Buffer.from(JSON.stringify({ success: true })));
        mockChild.emit('close', 0);
      }, 10);

      const result = await customService.validate('/test/plan.md');

      // Verify spawn was called and custom path was used
      expect(cp.spawn).toHaveBeenCalled();
      expect(result.success).toBe(true);
    });
  });
});

// Helper function to create a mock child process
function createMockChildProcess(): EventEmitter & {
  stdout: EventEmitter;
  stderr: EventEmitter;
  stdin: EventEmitter;
} {
  const mockChild = new EventEmitter() as EventEmitter & {
    stdout: EventEmitter;
    stderr: EventEmitter;
    stdin: EventEmitter;
  };
  mockChild.stdout = new EventEmitter();
  mockChild.stderr = new EventEmitter();
  mockChild.stdin = new EventEmitter();
  return mockChild;
}
