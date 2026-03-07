import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';

const isTUIAvailableMock = vi.fn();
const renderTUIMock = vi.fn();
const markFirstRunCompleteMock = vi.fn();

vi.mock('../tui/utils/tty-detection.js', () => ({
  isTUIAvailable: isTUIAvailableMock,
}));

vi.mock('../tui/utils/renderer.js', () => ({
  renderTUI: renderTUIMock,
}));

vi.mock('../services/first-run-detector.js', () => ({
  markFirstRunComplete: markFirstRunCompleteMock,
}));

vi.mock('../tui/commands/welcome/Welcome.js', () => ({
  Welcome: () => null,
}));

vi.mock('../tui/commands/welcome/content.js', () => ({
  ANVIL_LOGO: 'ANVIL',
  VALUE_PROPOSITION: 'Deterministic development automation',
  QUICK_START_OPTIONS: [
    { command: 'anvil login', description: 'Authenticate with your token' },
    { command: 'anvil tutorial', description: 'Run the tutorial' },
  ],
}));

describe('welcome command', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    isTUIAvailableMock.mockReturnValue(false);
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('should create start command with correct name and description', async () => {
    const { createStartCommand } = await import('./welcome.js');
    const command = createStartCommand();

    expect(command.name()).toBe('start');
    expect(command.description()).toContain('getting started options');
  });

  it('should not register options on start command', async () => {
    const { createStartCommand } = await import('./welcome.js');
    const command = createStartCommand();

    expect(command.options).toHaveLength(0);
  });

  it('should render plain welcome output and mark first run complete on happy path', async () => {
    const consoleErrorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});

    const { createStartCommand } = await import('./welcome.js');
    const command = createStartCommand();

    await command.parseAsync(['node', 'test']);

    expect(isTUIAvailableMock).toHaveBeenCalledTimes(1);
    expect(consoleErrorSpy).toHaveBeenCalledWith(expect.stringContaining('ANVIL'));
    expect(markFirstRunCompleteMock).toHaveBeenCalledTimes(1);
    expect(renderTUIMock).not.toHaveBeenCalled();
  });
});
