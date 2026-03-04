import { describe, it, expect, vi, afterEach } from 'vitest';

const mockLoadAuth = vi.hoisted(() => vi.fn());
const mockSaveAuth = vi.hoisted(() => vi.fn());
const mockVerifyToken = vi.hoisted(() => vi.fn());
const mockPrompt = vi.hoisted(() => vi.fn());
const mockSuccess = vi.hoisted(() => vi.fn());
const mockError = vi.hoisted(() => vi.fn());
const mockInfo = vi.hoisted(() => vi.fn());

vi.mock('../services/auth-store.js', () => ({
  loadAuth: mockLoadAuth,
  saveAuth: mockSaveAuth,
}));

vi.mock('../services/auth-client.js', () => ({
  verifyToken: mockVerifyToken,
}));

vi.mock('inquirer', () => ({
  default: { prompt: mockPrompt },
}));

vi.mock('../utils/output.js', () => ({
  success: mockSuccess,
  error: mockError,
  info: mockInfo,
}));

vi.mock('chalk', () => ({
  default: {
    bold: (s: string) => s,
    yellow: (s: string) => s,
  },
}));

import { createLoginCommand } from './login.js';

afterEach(() => {
  vi.restoreAllMocks();
  mockLoadAuth.mockReset();
  mockSaveAuth.mockReset();
  mockVerifyToken.mockReset();
  mockPrompt.mockReset();
});

describe('login command', () => {
  it('should create command with correct name and options', () => {
    const command = createLoginCommand();

    expect(command.name()).toBe('login');
    expect(command.description()).toContain('beta access token');

    const tokenOpt = command.options.find((o) => o.long === '--token');
    expect(tokenOpt).toBeDefined();
  });

  it('should authenticate with --token flag', async () => {
    mockLoadAuth.mockReturnValue(null);
    mockVerifyToken.mockResolvedValue({
      valid: true,
      user: { email: 'test@example.com' },
      scopes: ['gate:read'],
      expiresAt: '2026-12-31T00:00:00.000Z',
    });

    const command = createLoginCommand();
    await command.parseAsync(['--token', 'anvil_beta_test123'], { from: 'user' });

    expect(mockVerifyToken).toHaveBeenCalledWith('anvil_beta_test123');
    expect(mockSaveAuth).toHaveBeenCalledWith(
      expect.objectContaining({
        token: 'anvil_beta_test123',
        user: { email: 'test@example.com' },
      })
    );
    expect(mockSuccess).toHaveBeenCalledWith(expect.stringContaining('test@example.com'));
  });

  it('should prompt interactively when no --token provided', async () => {
    mockLoadAuth.mockReturnValue(null);
    mockPrompt.mockResolvedValue({ token: 'anvil_beta_interactive' });
    mockVerifyToken.mockResolvedValue({
      valid: true,
      user: { email: 'interactive@example.com' },
      scopes: ['gate:read'],
      expiresAt: '2026-12-31T00:00:00.000Z',
    });

    const command = createLoginCommand();
    await command.parseAsync([], { from: 'user' });

    expect(mockPrompt).toHaveBeenCalled();
    expect(mockVerifyToken).toHaveBeenCalledWith('anvil_beta_interactive');
    expect(mockSaveAuth).toHaveBeenCalled();
  });

  it('should ask to re-authenticate when already logged in', async () => {
    mockLoadAuth.mockReturnValue({ user: { email: 'existing@example.com' } });
    mockPrompt.mockResolvedValue({ proceed: false });

    const command = createLoginCommand();
    await command.parseAsync([], { from: 'user' });

    expect(mockInfo).toHaveBeenCalledWith(expect.stringContaining('existing@example.com'));
    expect(mockVerifyToken).not.toHaveBeenCalled();
  });

  it('should throw on invalid token', async () => {
    mockLoadAuth.mockReturnValue(null);
    mockVerifyToken.mockResolvedValue({ valid: false });

    const command = createLoginCommand();
    await expect(
      command.parseAsync(['--token', 'anvil_beta_bad'], { from: 'user' })
    ).rejects.toThrow('Invalid or expired token');
  });

  it('should throw on verification failure', async () => {
    mockLoadAuth.mockReturnValue(null);
    mockVerifyToken.mockRejectedValue(new Error('Network error'));

    const command = createLoginCommand();
    await expect(
      command.parseAsync(['--token', 'anvil_beta_test'], { from: 'user' })
    ).rejects.toThrow('Token verification failed');
  });
});
