/**
 * TUI E2E Tests - Migrated to Tuistory
 *
 * This file contains the migrated tests from tui-commands.e2e.test.ts
 * using tuistory for improved terminal testing capabilities.
 *
 * @see https://github.com/remorses/tuistory
 */

import { describe, it, expect, beforeAll, afterEach } from 'vitest';
import {
  launchAnvil,
  ensureCliBuild,
  getPackageVersion,
  safeClose,
  waitForTextWithContext,
  type Session,
} from './tuistory-utils.js';

describe('TUI E2E Tests (Tuistory)', () => {
  let session: Session | null = null;

  beforeAll(() => {
    ensureCliBuild();
  });

  afterEach(() => {
    safeClose(session);
    session = null;
  });

  describe('anvil --version', () => {
    it('shows version number', { timeout: 20000 }, async () => {
      session = await launchAnvil({ args: ['--version'] });

      const expectedVersion = getPackageVersion();
      const output = await session.waitForText(expectedVersion, { timeout: 10000 });

      expect(output).toContain(expectedVersion);
    });
  });

  describe('anvil --help', () => {
    it('shows help with available commands', { timeout: 20000 }, async () => {
      session = await launchAnvil({ args: ['--help'] });

      await session.waitForText('validate', { timeout: 10000 });
      await session.waitForText('gate', { timeout: 5000 });
      await session.waitForText('doctor', { timeout: 5000 });

      const output = await session.text({ trimEnd: true });
      expect(output).toContain('validate');
      expect(output).toContain('gate');
      expect(output).toContain('doctor');
    });
  });

  describe('anvil status', () => {
    it('renders status output', { timeout: 20000 }, async () => {
      session = await launchAnvil({
        args: ['status'],
        waitForDataTimeout: 10000,
      });

      await waitForTextWithContext(
        session,
        /ANVIL|Status|Hooks|Configuration|project/i,
        'Status output'
      );

      await session.press('q');
      await new Promise((resolve) => setTimeout(resolve, 500));
    });
  });

  describe('anvil doctor', () => {
    it('runs diagnostics and shows results', { timeout: 20000 }, async () => {
      session = await launchAnvil({
        args: ['doctor'],
        waitForDataTimeout: 10000,
      });

      await waitForTextWithContext(session, /passed|warnings|Healthy|failed/i, 'Doctor results');

      const output = await session.text({ trimEnd: true });
      expect(output).toMatch(/Node\.js/i);

      await session.press('q');
      await new Promise((resolve) => setTimeout(resolve, 500));
    });

    it('shows JSON output with --json flag', { timeout: 20000 }, async () => {
      session = await launchAnvil({
        args: ['doctor', '--json'],
        waitForDataTimeout: 10000,
      });

      await session.waitForText('{', { timeout: 10000 });
      const output = await session.text({ trimEnd: true });

      expect(output).toContain('"results"');
    });
  });

  describe('anvil tutorial', () => {
    it('starts tutorial and shows content', { timeout: 20000 }, async () => {
      session = await launchAnvil({
        args: ['tutorial'],
        waitForDataTimeout: 10000,
      });

      await waitForTextWithContext(session, /Tutorial|Welcome|Step|Anvil/i, 'Tutorial content');

      await session.press('q');
      await new Promise((resolve) => setTimeout(resolve, 500));
    });
  });

  describe('anvil new', () => {
    it('shows template browser', { timeout: 20000 }, async () => {
      session = await launchAnvil({
        args: ['new'],
        waitForDataTimeout: 10000,
      });

      await waitForTextWithContext(
        session,
        /template|select|choose|Template|API|auth/i,
        'Template browser'
      );

      await session.press('q');
      await new Promise((resolve) => setTimeout(resolve, 500));
    });

    it('handles keyboard navigation', { timeout: 20000 }, async () => {
      session = await launchAnvil({
        args: ['new'],
        waitForDataTimeout: 10000,
      });

      await waitForTextWithContext(session, /category|template|Template/i, 'Template browser');

      // Navigate with arrow keys
      await session.press('down');
      await new Promise((resolve) => setTimeout(resolve, 200));
      await session.press('up');
      await new Promise((resolve) => setTimeout(resolve, 200));

      await session.press('q');
      await new Promise((resolve) => setTimeout(resolve, 500));
    });
  });

  describe('anvil init --tui', () => {
    it('shows init wizard interface', { timeout: 20000 }, async () => {
      session = await launchAnvil({
        args: ['init', '--tui'],
        cwd: '/tmp',
        waitForDataTimeout: 10000,
      });

      await waitForTextWithContext(session, /init|setup|configure|project|Anvil/i, 'Init wizard');

      await session.press(['ctrl', 'c']);
      await new Promise((resolve) => setTimeout(resolve, 500));
    });
  });
});
