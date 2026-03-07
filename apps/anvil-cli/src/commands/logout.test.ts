import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';

const clearAuthMock = vi.fn();
const loadAuthMock = vi.fn();
const successMock = vi.fn();
const infoMock = vi.fn();

vi.mock('../services/auth-store.js', () => ({
  clearAuth: clearAuthMock,
  loadAuth: loadAuthMock,
}));

vi.mock('../utils/output.js', () => ({
  success: successMock,
  info: infoMock,
}));

describe('logout command', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('should create command with correct name and description', async () => {
    const { createLogoutCommand } = await import('./logout.js');
    const command = createLogoutCommand();

    expect(command.name()).toBe('logout');
    expect(command.description()).toContain('stored beta access credentials');
  });

  it('should not register command options', async () => {
    const { createLogoutCommand } = await import('./logout.js');
    const command = createLogoutCommand();

    expect(command.options).toHaveLength(0);
  });

  it('should clear auth and show success when an active session exists', async () => {
    loadAuthMock.mockReturnValue({ user: { email: 'dev@eddacraft.dev' } });

    const { createLogoutCommand } = await import('./logout.js');
    const command = createLogoutCommand();

    await command.parseAsync(['node', 'test']);

    expect(clearAuthMock).toHaveBeenCalledTimes(1);
    expect(successMock).toHaveBeenCalledWith('Logged out (was dev@eddacraft.dev)');
    expect(infoMock).not.toHaveBeenCalled();
  });
});
