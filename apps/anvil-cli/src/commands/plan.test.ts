import { describe, it, expect, vi, afterEach } from 'vitest';
import { Command } from 'commander';

const mockOra = vi.hoisted(() => {
  const spinnerInstance = {
    start: vi.fn().mockReturnThis(),
    stop: vi.fn(),
    succeed: vi.fn(),
    fail: vi.fn(),
    text: '',
  };
  const oraFn = vi.fn(() => spinnerInstance);
  return { oraFn, spinnerInstance };
});

const mockSavePlan = vi.hoisted(() => vi.fn());

vi.mock('ora', () => ({ default: mockOra.oraFn }));

vi.mock('chalk', () => ({
  default: {
    bold: (s: string) => s,
    green: (s: string) => s,
    red: (s: string) => s,
    cyan: (s: string) => s,
    white: (s: string) => s,
    gray: (s: string) => s,
  },
}));

vi.mock('@eddacraft/anvil-core', () => ({
  generatePlanId: () => 'plan-test-123',
  generateHash: () => 'hash-abc-def-ghi',
  APS_SCHEMA_VERSION: '1.0.0',
  createDebugger: () => () => {},
  validatePathWithinRoot: (p: string) => p,
}));

vi.mock('../utils/file-io.js', () => ({
  savePlan: mockSavePlan,
  getWorkspaceRoot: () => '/mock/workspace',
}));

vi.mock('node:child_process', () => ({
  execFileSync: vi.fn(() => 'main'),
}));

vi.mock('./plan/index.js', () => ({
  createValidateSubcommand: () => new Command('validate').description('Validate APS doc'),
  createLoadSubcommand: () => new Command('load').description('Load APS doc'),
  createLockSubcommand: () => new Command('lock').argument('<task>').description('Lock a task'),
  createUnlockSubcommand: () =>
    new Command('unlock').argument('<task>').description('Unlock a task'),
  createStatusSubcommand: () => new Command('status').description('Show status'),
}));

import { createPlanCommand } from './plan.js';

afterEach(() => {
  vi.restoreAllMocks();
  mockOra.oraFn.mockClear();
  mockOra.spinnerInstance.succeed.mockClear();
  mockOra.spinnerInstance.fail.mockClear();
  mockSavePlan.mockReset();
});

describe('plan command', () => {
  it('should create command with correct name and subcommands', () => {
    const command = createPlanCommand();

    expect(command.name()).toBe('plan');
    expect(command.description()).toContain('APS');

    const subcommandNames = command.commands.map((c) => c.name());
    expect(subcommandNames).toContain('validate');
    expect(subcommandNames).toContain('load');
    expect(subcommandNames).toContain('lock');
    expect(subcommandNames).toContain('unlock');
    expect(subcommandNames).toContain('status');
    expect(subcommandNames).toContain('create');
  });

  describe('create subcommand', () => {
    it('should create a plan with valid intent', async () => {
      vi.spyOn(console, 'log').mockImplementation(() => {});

      const command = createPlanCommand();
      await command.parseAsync(['create', 'Add authentication middleware to protect API routes'], {
        from: 'user',
      });

      expect(mockSavePlan).toHaveBeenCalledWith(
        expect.objectContaining({
          id: 'plan-test-123',
          intent: 'Add authentication middleware to protect API routes',
          hash: 'hash-abc-def-ghi',
        }),
        expect.any(String)
      );
      expect(mockOra.spinnerInstance.succeed).toHaveBeenCalled();
    });

    it('should reject intent shorter than 10 characters', async () => {
      vi.spyOn(console, 'log').mockImplementation(() => {});
      vi.spyOn(console, 'error').mockImplementation(() => {});

      const command = createPlanCommand();
      await expect(command.parseAsync(['create', 'Too short'], { from: 'user' })).rejects.toThrow(
        'at least 10 characters'
      );
    });

    it('should output JSON with --json flag without saving', async () => {
      const consoleSpy = vi.spyOn(console, 'log').mockImplementation(() => {});

      const command = createPlanCommand();
      await command.parseAsync(
        ['create', '--json', 'Generate a plan and output as JSON to stdout'],
        { from: 'user' }
      );

      expect(mockSavePlan).not.toHaveBeenCalled();

      const jsonCall = consoleSpy.mock.calls.find((c) => {
        try {
          const parsed = JSON.parse(c[0]);
          return parsed.id === 'plan-test-123';
        } catch {
          return false;
        }
      });
      expect(jsonCall).toBeDefined();
    });
  });
});
