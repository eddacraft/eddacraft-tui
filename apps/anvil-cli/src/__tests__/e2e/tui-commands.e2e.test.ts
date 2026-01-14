import { describe, it, expect, beforeAll, afterEach } from 'vitest';
import { spawnAnvilPTY, stripAnsi, typeWithDelay, type PTYSession } from './pty-utils.js';
import { resolve } from 'node:path';
import { existsSync } from 'node:fs';

const CLI_PATH = resolve(__dirname, '../../..', 'dist', 'index.js');
const PROJECT_ROOT = resolve(__dirname, '../../../..');

describe('TUI E2E Tests', () => {
  let session: PTYSession | null = null;

  beforeAll(() => {
    if (!existsSync(CLI_PATH)) {
      throw new Error(`CLI not built. Run 'pnpm build' first. Expected: ${CLI_PATH}`);
    }
  });

  afterEach(() => {
    if (session?.isRunning()) {
      session.kill();
    }
    session = null;
  });

  describe('anvil --version', () => {
    it('shows version number', { timeout: 20000, retry: 2 }, async () => {
      session = spawnAnvilPTY({
        args: ['--version'],
        timeout: 15000,
      });

      await session.waitFor('0.0.0', 10000);
      const code = await session.waitForExit(10000);

      expect(code).toBe(0);
    });
  });

  describe('anvil --help', () => {
    it('shows help with available commands', { timeout: 20000, retry: 2 }, async () => {
      session = spawnAnvilPTY({
        args: ['--help'],
        timeout: 15000,
      });

      await session.waitFor('validate', 10000);
      await session.waitFor('gate', 10000);
      await session.waitFor('doctor', 10000);

      const code = await session.waitForExit(10000);
      expect(code).toBe(0);
    });
  });

  describe('anvil status', () => {
    it('renders status output', { timeout: 20000, retry: 2 }, async () => {
      session = spawnAnvilPTY({
        args: ['status'],
        cwd: PROJECT_ROOT,
        timeout: 15000,
      });

      await session.waitForMatch(/ANVIL|Status|Hooks|Configuration|project/i, 10000);

      session.sendKey('q');
      const code = await session.waitForExit(5000).catch(() => {
        session?.kill();
        return 0;
      });
      expect([0, 1]).toContain(code);
    });
  });

  describe('anvil doctor', () => {
    it('runs diagnostics and shows results', async () => {
      session = spawnAnvilPTY({
        args: ['doctor'],
        cwd: PROJECT_ROOT,
        timeout: 15000,
      });

      await session.waitForMatch(/passed|warnings|Healthy|failed/i, 10000);

      const output = stripAnsi(session.getOutput());
      expect(output).toMatch(/Node\.js/i);

      session.sendKey('q');
      const code = await session.waitForExit(5000).catch(() => {
        session?.kill();
        return 0;
      });
      expect([0, 1]).toContain(code);
    }, 20000);

    it('shows JSON output with --json flag', async () => {
      session = spawnAnvilPTY({
        args: ['doctor', '--json'],
        cwd: PROJECT_ROOT,
        timeout: 15000,
      });

      await session.waitFor('{', 10000);
      const code = await session.waitForExit(10000);

      const output = stripAnsi(session.getOutput());
      expect(output).toContain('"results"');
      expect([0, 1]).toContain(code);
    }, 20000);
  });

  describe('anvil tutorial', () => {
    it('starts tutorial and shows content', async () => {
      session = spawnAnvilPTY({
        args: ['tutorial'],
        cwd: PROJECT_ROOT,
        timeout: 15000,
      });

      await session.waitForMatch(/Tutorial|Welcome|Step|Anvil/i, 8000);

      session.sendKey('q');
      const code = await session.waitForExit(3000).catch(() => {
        session?.kill();
        return 0;
      });
      expect([0, 1]).toContain(code);
    }, 20000);
  });

  describe('anvil new', () => {
    it('shows template browser', async () => {
      session = spawnAnvilPTY({
        args: ['new'],
        cwd: PROJECT_ROOT,
        timeout: 15000,
      });

      await session.waitForMatch(/template|select|choose|Template|API|auth/i, 8000);

      session.sendKey('q');
      const code = await session.waitForExit(3000).catch(() => {
        session?.kill();
        return 0;
      });
      expect([0, 1]).toContain(code);
    }, 20000);

    it('handles keyboard navigation', async () => {
      session = spawnAnvilPTY({
        args: ['new'],
        cwd: PROJECT_ROOT,
        timeout: 15000,
      });

      await session.waitForMatch(/category|template|Template/i, 8000);

      await typeWithDelay(session, ['down', 'up'], 200);

      session.sendKey('q');
      const code = await session.waitForExit(3000).catch(() => {
        session?.kill();
        return 0;
      });
      expect([0, 1]).toContain(code);
    }, 20000);
  });

  describe('anvil init --tui', () => {
    it('shows init wizard interface', async () => {
      session = spawnAnvilPTY({
        args: ['init', '--tui'],
        cwd: '/tmp',
        timeout: 15000,
      });

      await session.waitForMatch(/init|setup|configure|project|Anvil/i, 8000);

      session.sendKey('ctrl+c');
      await session.waitForExit(3000).catch(() => session?.kill());
    }, 20000);
  });
});
