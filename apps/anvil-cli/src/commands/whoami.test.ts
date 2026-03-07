import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';

const loadAuthMock = vi.fn();
const errorMock = vi.fn();

vi.mock('../services/auth-store.js', () => ({
  loadAuth: loadAuthMock,
}));

vi.mock('../utils/output.js', () => ({
  error: errorMock,
}));

describe('whoami command', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('should create command with correct name and description', async () => {
    const { createWhoamiCommand } = await import('./whoami.js');
    const command = createWhoamiCommand();

    expect(command.name()).toBe('whoami');
    expect(command.description()).toContain('authentication session info');
  });

  it('should not register command arguments or options', async () => {
    const { createWhoamiCommand } = await import('./whoami.js');
    const command = createWhoamiCommand();

    expect(command.options).toHaveLength(0);
    expect(command.registeredArguments).toHaveLength(0);
  });

  it('should print session details when authenticated', async () => {
    loadAuthMock.mockReturnValue({
      user: { email: 'dev@eddacraft.dev' },
      scopes: ['read', 'write'],
      expiresAt: '2099-01-01T00:00:00.000Z',
      verifiedAt: '2098-01-01T00:00:00.000Z',
    });
    const consoleLogSpy = vi.spyOn(console, 'log').mockImplementation(() => {});

    const { createWhoamiCommand } = await import('./whoami.js');
    const command = createWhoamiCommand();

    await command.parseAsync(['node', 'test']);

    expect(consoleLogSpy).toHaveBeenCalledWith(expect.stringContaining('Session Info'));
    expect(consoleLogSpy).toHaveBeenCalledWith(expect.stringContaining('dev@eddacraft.dev'));
    expect(errorMock).not.toHaveBeenCalled();
  });
});
