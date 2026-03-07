import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';

const verifyTokenMock = vi.fn();
const saveAuthMock = vi.fn();
const loadAuthMock = vi.fn();
const infoMock = vi.fn();
const successMock = vi.fn();
const errorMock = vi.fn();
const promptMock = vi.fn();

vi.mock('../services/auth-client.js', () => ({
  verifyToken: verifyTokenMock,
}));

vi.mock('../services/auth-store.js', () => ({
  saveAuth: saveAuthMock,
  loadAuth: loadAuthMock,
}));

vi.mock('../utils/output.js', () => ({
  info: infoMock,
  success: successMock,
  error: errorMock,
}));

vi.mock('inquirer', () => ({
  default: {
    prompt: promptMock,
  },
}));

describe('login command', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    loadAuthMock.mockReturnValue(null);
    verifyTokenMock.mockResolvedValue({
      valid: true,
      user: { email: 'dev@eddacraft.dev' },
      scopes: ['read', 'write'],
      expiresAt: '2099-01-01T00:00:00.000Z',
    });
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('should create command with correct name, description, and token option', async () => {
    const { createLoginCommand } = await import('./login.js');
    const command = createLoginCommand();
    const tokenOpt = command.options.find((option) => option.long === '--token');

    expect(command.name()).toBe('login');
    expect(command.description()).toContain('beta access token');
    expect(tokenOpt).toBeDefined();
  });

  it('should save auth details when token verification succeeds', async () => {
    const { createLoginCommand } = await import('./login.js');
    const command = createLoginCommand();

    await command.parseAsync(['node', 'test', '--token', 'anvil_beta_123']);

    expect(verifyTokenMock).toHaveBeenCalledWith('anvil_beta_123');
    expect(saveAuthMock).toHaveBeenCalledTimes(1);
    expect(successMock).toHaveBeenCalledWith(expect.stringContaining('Authenticated as'));
  });

  it('should stop when already authenticated and user declines re-authentication', async () => {
    loadAuthMock.mockReturnValue({ user: { email: 'existing@eddacraft.dev' } });
    promptMock.mockResolvedValue({ proceed: false });

    const { createLoginCommand } = await import('./login.js');
    const command = createLoginCommand();

    await command.parseAsync(['node', 'test']);

    expect(promptMock).toHaveBeenCalledTimes(1);
    expect(verifyTokenMock).not.toHaveBeenCalled();
    expect(saveAuthMock).not.toHaveBeenCalled();
  });
});
