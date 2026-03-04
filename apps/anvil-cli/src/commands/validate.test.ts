import { describe, it, expect, vi, afterEach } from 'vitest';

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

const mockResolvePlanPathOrId = vi.hoisted(() => vi.fn());
const mockLoadPlan = vi.hoisted(() => vi.fn());
const mockPlanLoaderLoadPlan = vi.hoisted(() => vi.fn());

vi.mock('ora', () => ({ default: mockOra.oraFn }));

vi.mock('chalk', () => ({
  default: {
    bold: (s: string) => s,
    green: (s: string) => s,
    red: (s: string) => s,
    yellow: (s: string) => s,
    cyan: (s: string) => s,
    white: (s: string) => s,
    gray: (s: string) => s,
  },
}));

vi.mock('../utils/plan-resolution.js', () => ({
  resolvePlanPathOrId: mockResolvePlanPathOrId,
}));

vi.mock('../utils/file-io.js', () => ({
  loadPlan: mockLoadPlan,
}));

vi.mock('../services/plan-loader.js', () => ({
  PlanLoader: class {
    loadPlan = mockPlanLoaderLoadPlan;
  },
}));

vi.mock('@eddacraft/anvil-core', () => ({
  verifyHash: vi.fn(() => true),
  createDebugger: () => () => {},
  validatePathWithinRoot: (p: string) => p,
}));

import { createValidateCommand } from './validate.js';

afterEach(() => {
  vi.restoreAllMocks();
  mockOra.oraFn.mockClear();
  mockOra.spinnerInstance.start.mockClear();
  mockOra.spinnerInstance.succeed.mockClear();
  mockOra.spinnerInstance.fail.mockClear();
  mockResolvePlanPathOrId.mockReset();
  mockPlanLoaderLoadPlan.mockReset();
});

describe('validate command', () => {
  it('should create command with correct name and options', () => {
    const command = createValidateCommand();

    expect(command.name()).toBe('validate');
    expect(command.description()).toContain('Validate');

    expect(command.registeredArguments).toHaveLength(1);
    expect(command.registeredArguments[0].name()).toBe('plan');

    const verboseOpt = command.options.find((o) => o.long === '--verbose');
    expect(verboseOpt).toBeDefined();

    const formatOpt = command.options.find((o) => o.long === '--format');
    expect(formatOpt).toBeDefined();

    const nativeOpt = command.options.find((o) => o.long === '--native');
    expect(nativeOpt).toBeDefined();
  });

  it('should validate a plan via PlanLoader', async () => {
    vi.spyOn(console, 'log').mockImplementation(() => {});

    mockResolvePlanPathOrId.mockReturnValue({ path: '/mock/plan.json' });
    mockPlanLoaderLoadPlan.mockResolvedValue({
      plan: {
        id: 'test-id',
        schema_version: '1.0.0',
        hash: 'abc123def456',
        intent: 'Test plan',
        proposed_changes: [],
        evidence: [],
      },
      validation: { valid: true },
      sourceFormat: { format: 'speckit', confidence: 95, adapter: 'speckit-adapter' },
      warnings: [],
    });

    const command = createValidateCommand();
    await command.parseAsync(['test-plan.md'], { from: 'user' });

    expect(mockOra.spinnerInstance.succeed).toHaveBeenCalled();
  });

  it('should fail when validation returns invalid', async () => {
    vi.spyOn(console, 'log').mockImplementation(() => {});
    vi.spyOn(console, 'error').mockImplementation(() => {});

    mockResolvePlanPathOrId.mockReturnValue({ path: '/mock/plan.json' });
    mockPlanLoaderLoadPlan.mockResolvedValue({
      plan: {},
      validation: { valid: false, issues: [{ message: 'Missing intent' }] },
      warnings: [],
    });

    const command = createValidateCommand();
    await expect(command.parseAsync(['bad-plan.md'], { from: 'user' })).rejects.toThrow(
      'Plan validation failed'
    );

    expect(mockOra.spinnerInstance.fail).toHaveBeenCalled();
  });
});
