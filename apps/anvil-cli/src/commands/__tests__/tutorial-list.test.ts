import { describe, it, expect, vi, afterEach } from 'vitest';

const mockFs = vi.hoisted(() => ({
  forceExistsSync: false,
  rmSyncCalls: [] as Array<unknown[]>,
}));

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

vi.mock(import('node:fs'), async (importOriginal) => {
  const actual = await importOriginal();
  // eslint-disable-next-line @typescript-eslint/no-require-imports
  const realFs = require('node:fs') as typeof import('node:fs');
  const result = Object.create(null);
  for (const key of Reflect.ownKeys(actual)) {
    result[key] = (actual as Record<string | symbol, unknown>)[key];
  }
  result.existsSync = (path: string) => {
    if (mockFs.forceExistsSync) return true;
    return realFs.existsSync(path);
  };
  result.rmSync = (...args: unknown[]) => {
    if (mockFs.forceExistsSync) {
      mockFs.rmSyncCalls.push(args);
      return;
    }
    return (realFs.rmSync as (...a: unknown[]) => void)(...args);
  };
  if (!('default' in result)) {
    result.default = result;
  }
  return result;
});

import { createTutorialCommand } from '../tutorial.js';

afterEach(() => {
  vi.restoreAllMocks();
  mockFs.forceExistsSync = false;
  mockFs.rmSyncCalls = [];
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
  });

  it('handles unknown topic gracefully', async () => {
    const consoleSpy = vi.spyOn(console, 'log').mockImplementation(() => {});

    const command = createTutorialCommand();
    await command.parseAsync(['nonexistent'], { from: 'user' });

    const allOutput = consoleSpy.mock.calls.map((c) => c[0]).join('\n');

    expect(allOutput).toContain('Unknown tutorial topic');
    expect(allOutput).toContain('nonexistent');
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
  });

  it('resets policies topic and confirms reset', async () => {
    const consoleSpy = vi.spyOn(console, 'log').mockImplementation(() => {});

    const command = createTutorialCommand();
    await command.parseAsync(['policies', '--reset'], { from: 'user' });

    const allOutput = consoleSpy.mock.calls.map((c) => c[0]).join('\n');

    expect(allOutput).toContain("Tutorial 'policies' reset");
    expect(allOutput).toContain('anvil tutorial policies');
  });

  it('removes the policy file and logs cleanup when it exists', async () => {
    mockFs.forceExistsSync = true;
    const consoleSpy = vi.spyOn(console, 'log').mockImplementation(() => {});

    const command = createTutorialCommand();
    await command.parseAsync(['policies', '--reset'], { from: 'user' });

    const policyRmCall = mockFs.rmSyncCalls.find(
      (args) => typeof args[0] === 'string' && (args[0] as string).includes('max_file_length.rego')
    );
    expect(policyRmCall).toBeDefined();

    const allOutput = consoleSpy.mock.calls.map((c) => c[0]).join('\n');
    expect(allOutput).toContain('Removed tutorial policy file');
  });

  it('resets drift topic', async () => {
    const consoleSpy = vi.spyOn(console, 'log').mockImplementation(() => {});

    const command = createTutorialCommand();
    await command.parseAsync(['drift', '--reset'], { from: 'user' });

    const allOutput = consoleSpy.mock.calls.map((c) => c[0]).join('\n');

    expect(allOutput).toContain("Tutorial 'drift' reset");
  });

  it('resets ci topic', async () => {
    const consoleSpy = vi.spyOn(console, 'log').mockImplementation(() => {});

    const command = createTutorialCommand();
    await command.parseAsync(['ci', '--reset'], { from: 'user' });

    const allOutput = consoleSpy.mock.calls.map((c) => c[0]).join('\n');

    expect(allOutput).toContain("Tutorial 'ci' reset");
  });

  it('rejects --reset with unknown topic', async () => {
    const consoleSpy = vi.spyOn(console, 'log').mockImplementation(() => {});

    const command = createTutorialCommand();
    await command.parseAsync(['nonexistent', '--reset'], { from: 'user' });

    const allOutput = consoleSpy.mock.calls.map((c) => c[0]).join('\n');

    expect(allOutput).toContain('Unknown tutorial topic');
    expect(allOutput).toContain('nonexistent');
  });
});
