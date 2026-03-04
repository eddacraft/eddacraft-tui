import { describe, it, expect, vi, afterEach } from 'vitest';

const mockLoadAuth = vi.hoisted(() => vi.fn());
const mockError = vi.hoisted(() => vi.fn());

vi.mock('../services/auth-store.js', () => ({
  loadAuth: mockLoadAuth,
}));

vi.mock('../utils/output.js', () => ({
  error: mockError,
}));

vi.mock('chalk', () => ({
  default: {
    bold: (s: string) => s,
    cyan: (s: string) => s,
  },
}));

import { createWhoamiCommand } from './whoami.js';

afterEach(() => {
  vi.restoreAllMocks();
});

describe('whoami command', () => {
  it('should create command with correct name and description', () => {
    const command = createWhoamiCommand();

    expect(command.name()).toBe('whoami');
    expect(command.description()).toContain('authentication');
  });

  it('should display session info when authenticated', async () => {
    const consoleSpy = vi.spyOn(console, 'log').mockImplementation(() => {});

    mockLoadAuth.mockReturnValue({
      user: { email: 'test@example.com' },
      scopes: ['gate:read', 'gate:write'],
      expiresAt: '2026-12-31T00:00:00.000Z',
      verifiedAt: '2026-01-01T00:00:00.000Z',
    });

    const command = createWhoamiCommand();
    await command.parseAsync([], { from: 'user' });

    const output = consoleSpy.mock.calls.map((c) => c[0]).join('\n');
    expect(output).toContain('test@example.com');
    expect(output).toContain('gate:read, gate:write');
  });

  it('should throw CliError when not authenticated', async () => {
    mockLoadAuth.mockReturnValue(null);

    const command = createWhoamiCommand();
    await expect(command.parseAsync([], { from: 'user' })).rejects.toThrow('Not authenticated');
    expect(mockError).toHaveBeenCalled();
  });
});
