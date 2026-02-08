import { describe, it, expect, vi, afterEach } from 'vitest';

// Mock TUI/renderer dependencies to avoid Ink import issues in non-TTY test env
vi.mock('../../tui/utils/tty-detection.js', () => ({
  isTUIAvailable: () => false,
}));

vi.mock('../../tui/utils/renderer.js', () => ({
  renderTUIAndWait: vi.fn(),
}));

vi.mock('../../utils/file-io.js', () => ({
  getWorkspaceRoot: () => '/mock/workspace',
}));

import { createTutorialCommand } from '../tutorial.js';

afterEach(() => {
  vi.restoreAllMocks();
});

describe('tutorial --list', () => {
  it('exports createTutorialCommand', () => {
    expect(typeof createTutorialCommand).toBe('function');
  });

  it('creates a command with correct name', () => {
    const command = createTutorialCommand();
    expect(command.name()).toBe('tutorial');
  });

  it('has --list option', () => {
    const command = createTutorialCommand();
    const listOption = command.options.find((opt) => opt.long === '--list');
    expect(listOption).toBeDefined();
  });

  it('has --reset option', () => {
    const command = createTutorialCommand();
    const resetOption = command.options.find((opt) => opt.long === '--reset');
    expect(resetOption).toBeDefined();
  });

  it('has topic argument', () => {
    const command = createTutorialCommand();
    // Commander stores arguments in _args
    const args = (command as unknown as { _args: Array<{ _name: string }> })._args;
    expect(args).toHaveLength(1);
    expect(args[0]._name).toBe('topic');
  });

  it('lists all expected tutorials when --list is used', async () => {
    const consoleSpy = vi.spyOn(console, 'log').mockImplementation(() => {});

    const command = createTutorialCommand();
    await command.parseAsync(['--list'], { from: 'user' });

    const allOutput = consoleSpy.mock.calls.map((c) => c[0]).join('\n');

    // Core tutorial
    expect(allOutput).toContain('anvil tutorial');
    expect(allOutput).toContain('Core tutorial');

    // Feature tutorials
    expect(allOutput).toContain('policies');
    expect(allOutput).toContain('architecture');
    expect(allOutput).toContain('drift');
    expect(allOutput).toContain('ci');

    consoleSpy.mockRestore();
  });

  it('shows descriptions for each tutorial', async () => {
    const consoleSpy = vi.spyOn(console, 'log').mockImplementation(() => {});

    const command = createTutorialCommand();
    await command.parseAsync(['--list'], { from: 'user' });

    const allOutput = consoleSpy.mock.calls.map((c) => c[0]).join('\n');

    expect(allOutput).toContain('OPA/Rego');
    expect(allOutput).toContain('boundaries');
    expect(allOutput).toContain('drift');
    expect(allOutput).toContain('CI');

    consoleSpy.mockRestore();
  });

  it('handles unknown topic gracefully', async () => {
    const consoleSpy = vi.spyOn(console, 'log').mockImplementation(() => {});

    const command = createTutorialCommand();
    await command.parseAsync(['nonexistent'], { from: 'user' });

    const allOutput = consoleSpy.mock.calls.map((c) => c[0]).join('\n');

    expect(allOutput).toContain('Unknown tutorial topic');
    expect(allOutput).toContain('nonexistent');

    consoleSpy.mockRestore();
  });
});

describe('tutorial --reset with topic', () => {
  it('resets a known topic and confirms', async () => {
    const consoleSpy = vi.spyOn(console, 'log').mockImplementation(() => {});

    const command = createTutorialCommand();
    await command.parseAsync(['architecture', '--reset'], { from: 'user' });

    const allOutput = consoleSpy.mock.calls.map((c) => c[0]).join('\n');

    expect(allOutput).toContain("Tutorial 'architecture' reset");
    expect(allOutput).toContain('anvil tutorial architecture');

    consoleSpy.mockRestore();
  });

  it('resets policies topic and mentions cleanup', async () => {
    const consoleSpy = vi.spyOn(console, 'log').mockImplementation(() => {});

    const command = createTutorialCommand();
    await command.parseAsync(['policies', '--reset'], { from: 'user' });

    const allOutput = consoleSpy.mock.calls.map((c) => c[0]).join('\n');

    expect(allOutput).toContain("Tutorial 'policies' reset");
    expect(allOutput).toContain('anvil tutorial policies');

    consoleSpy.mockRestore();
  });

  it('resets drift topic', async () => {
    const consoleSpy = vi.spyOn(console, 'log').mockImplementation(() => {});

    const command = createTutorialCommand();
    await command.parseAsync(['drift', '--reset'], { from: 'user' });

    const allOutput = consoleSpy.mock.calls.map((c) => c[0]).join('\n');

    expect(allOutput).toContain("Tutorial 'drift' reset");

    consoleSpy.mockRestore();
  });

  it('resets ci topic', async () => {
    const consoleSpy = vi.spyOn(console, 'log').mockImplementation(() => {});

    const command = createTutorialCommand();
    await command.parseAsync(['ci', '--reset'], { from: 'user' });

    const allOutput = consoleSpy.mock.calls.map((c) => c[0]).join('\n');

    expect(allOutput).toContain("Tutorial 'ci' reset");

    consoleSpy.mockRestore();
  });

  it('rejects --reset with unknown topic', async () => {
    const consoleSpy = vi.spyOn(console, 'log').mockImplementation(() => {});

    const command = createTutorialCommand();
    await command.parseAsync(['nonexistent', '--reset'], { from: 'user' });

    const allOutput = consoleSpy.mock.calls.map((c) => c[0]).join('\n');

    expect(allOutput).toContain('Unknown tutorial topic');
    expect(allOutput).toContain('nonexistent');

    consoleSpy.mockRestore();
  });
});
