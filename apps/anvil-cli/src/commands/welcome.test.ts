import { describe, it, expect, vi, afterEach } from 'vitest';

const mockIsTUIAvailable = vi.hoisted(() => vi.fn(() => false));
const mockRenderTUI = vi.hoisted(() => vi.fn());
const mockMarkFirstRunComplete = vi.hoisted(() => vi.fn());

vi.mock('../tui/utils/tty-detection.js', () => ({
  isTUIAvailable: mockIsTUIAvailable,
}));

vi.mock('../tui/utils/renderer.js', () => ({
  renderTUI: mockRenderTUI,
}));

vi.mock('../services/first-run-detector.js', () => ({
  markFirstRunComplete: mockMarkFirstRunComplete,
}));

vi.mock('../tui/commands/welcome/Welcome.js', () => ({
  Welcome: () => null,
}));

vi.mock('../tui/commands/welcome/content.js', () => ({
  ANVIL_LOGO: 'LOGO',
  VALUE_PROPOSITION: 'VALUE',
  QUICK_START_OPTIONS: [
    { command: 'anvil init', description: 'Set up a project' },
    { command: 'anvil check', description: 'Run checks' },
  ],
}));

vi.mock('chalk', () => ({
  default: {
    cyan: Object.assign((s: string) => s, { bold: (s: string) => s }),
    bold: (s: string) => s,
    dim: (s: string) => s,
  },
}));

import { createStartCommand, showWelcome } from './welcome.js';

afterEach(() => {
  vi.restoreAllMocks();
  mockIsTUIAvailable.mockReset().mockReturnValue(false);
  mockRenderTUI.mockReset();
  mockMarkFirstRunComplete.mockReset();
});

describe('start command', () => {
  it('should create command with correct name and description', () => {
    const command = createStartCommand();

    expect(command.name()).toBe('start');
    expect(command.description()).toContain('getting started');
  });
});

describe('showWelcome', () => {
  it('should show plain welcome when TUI is not available', async () => {
    const consoleSpy = vi.spyOn(console, 'log').mockImplementation(() => {});
    mockIsTUIAvailable.mockReturnValue(false);

    await showWelcome();

    const output = consoleSpy.mock.calls.map((c) => c[0]).join('\n');
    expect(output).toContain('LOGO');
    expect(output).toContain('anvil init');
    expect(mockMarkFirstRunComplete).toHaveBeenCalled();
  });

  it('should attempt TUI when available', async () => {
    mockIsTUIAvailable.mockReturnValue(true);
    // renderTUI returns null (e.g. Ink not available) — should fall back to plain
    mockRenderTUI.mockReturnValue(null);
    vi.spyOn(console, 'log').mockImplementation(() => {});

    await showWelcome();

    expect(mockRenderTUI).toHaveBeenCalled();
    // Falls back to plain, which calls markFirstRunComplete
    expect(mockMarkFirstRunComplete).toHaveBeenCalled();
  });
});
