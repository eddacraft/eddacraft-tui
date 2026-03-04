import { describe, it, expect, vi, afterEach } from 'vitest';

const mockLoadAuth = vi.hoisted(() => vi.fn());
const mockClearAuth = vi.hoisted(() => vi.fn());
const mockSuccess = vi.hoisted(() => vi.fn());
const mockInfo = vi.hoisted(() => vi.fn());

vi.mock('../services/auth-store.js', () => ({
  loadAuth: mockLoadAuth,
  clearAuth: mockClearAuth,
}));

vi.mock('../utils/output.js', () => ({
  success: mockSuccess,
  info: mockInfo,
}));

import { createLogoutCommand } from './logout.js';

afterEach(() => {
  vi.restoreAllMocks();
});

describe('logout command', () => {
  it('should create command with correct name and description', () => {
    const command = createLogoutCommand();

    expect(command.name()).toBe('logout');
    expect(command.description()).toContain('credentials');
  });

  it('should clear auth and show email when session exists', async () => {
    mockLoadAuth.mockReturnValue({ user: { email: 'test@example.com' } });

    const command = createLogoutCommand();
    await command.parseAsync([], { from: 'user' });

    expect(mockClearAuth).toHaveBeenCalled();
    expect(mockSuccess).toHaveBeenCalledWith(expect.stringContaining('test@example.com'));
  });

  it('should clear auth and show info when no session exists', async () => {
    mockLoadAuth.mockReturnValue(null);

    const command = createLogoutCommand();
    await command.parseAsync([], { from: 'user' });

    expect(mockClearAuth).toHaveBeenCalled();
    expect(mockInfo).toHaveBeenCalledWith('No active session');
  });
});
