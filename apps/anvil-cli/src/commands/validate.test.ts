import { Command } from 'commander';
import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { CliError } from '../utils/cli-error.js';

const verifyHashMock = vi.fn();
const loadPlanMock = vi.fn();
const resolvePlanPathOrIdMock = vi.fn();

const spinner = {
  text: '',
  start: vi.fn(),
  succeed: vi.fn(),
  fail: vi.fn(),
};

spinner.start.mockReturnValue(spinner);

vi.mock('ora', () => ({
  default: vi.fn(() => spinner),
}));

vi.mock('@eddacraft/anvil-core', () => ({
  verifyHash: verifyHashMock,
  createDebugger: vi.fn(() => vi.fn()),
}));

vi.mock('../utils/file-io.js', () => ({
  loadPlan: loadPlanMock,
}));

vi.mock('../utils/plan-resolution.js', () => ({
  resolvePlanPathOrId: resolvePlanPathOrIdMock,
}));

vi.mock('../services/plan-loader.js', () => ({
  PlanLoader: vi.fn().mockImplementation(() => ({
    loadPlan: vi.fn(),
  })),
}));

describe('validate command', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    resolvePlanPathOrIdMock.mockReturnValue({ path: '/tmp/workspace/plan.json' });
    loadPlanMock.mockResolvedValue({
      id: 'PLAN-001',
      schema_version: '1.0.0',
      hash: 'abcdef0123456789abcdef0123456789',
      intent: 'Validate plan behaviour',
      proposed_changes: [],
      evidence: [],
      provenance: {
        author: 'dev',
        timestamp: '2026-01-01T00:00:00.000Z',
      },
      validations: {
        required_checks: ['lint'],
      },
    });
    verifyHashMock.mockReturnValue(true);
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('should create command with correct name, description, and required plan argument', async () => {
    const { createValidateCommand } = await import('./validate.js');
    const command = createValidateCommand();

    expect(command.name()).toBe('validate');
    expect(command.description()).toContain('Validate an Anvil plan');
    expect(command.registeredArguments[0]?.name()).toBe('plan');
    expect(command.registeredArguments[0]?.required).toBe(true);
  });

  it('should register verbose, format, native, and validate-hash options', async () => {
    const { createValidateCommand } = await import('./validate.js');
    const command = createValidateCommand();

    expect(command.options.find((option) => option.long === '--verbose')).toBeDefined();
    expect(command.options.find((option) => option.long === '--format')).toBeDefined();
    expect(command.options.find((option) => option.long === '--native')).toBeDefined();
    expect(command.options.find((option) => option.long === '--validate-hash')).toBeDefined();
  });

  it('should validate native plan and print success on happy path', async () => {
    const consoleErrorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});

    const { createValidateCommand } = await import('./validate.js');
    const command = createValidateCommand();

    await command.parseAsync(['node', 'test', 'PLAN-001', '--native']);

    expect(resolvePlanPathOrIdMock).toHaveBeenCalledWith('PLAN-001');
    expect(loadPlanMock).toHaveBeenCalledWith('/tmp/workspace/plan.json');
    expect(verifyHashMock).toHaveBeenCalledTimes(1);
    expect(consoleErrorSpy).toHaveBeenCalledWith(expect.stringContaining('Plan Details:'));
  });

  describe('planPathOrId guard', () => {
    async function runValidate(args: string[]): Promise<void> {
      const { createValidateCommand } = await import('./validate.js');
      const program = new Command();
      program.exitOverride();
      program.addCommand(createValidateCommand());
      await program.parseAsync(['node', 'test', 'validate', ...args]);
    }

    it('throws CliError when planPathOrId is an empty string', async () => {
      await expect(runValidate([''])).rejects.toThrow(CliError);
    });

    it('throws CliError when planPathOrId is blank whitespace', async () => {
      await expect(runValidate(['   '])).rejects.toThrow(CliError);
    });

    it('includes helpful message in the error', async () => {
      await expect(runValidate([''])).rejects.toThrow(
        'Plan argument is required'
      );
    });
  });
});
