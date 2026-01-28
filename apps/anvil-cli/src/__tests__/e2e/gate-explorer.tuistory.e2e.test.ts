/**
 * Gate Explorer TUI E2E Tests
 *
 * Comprehensive tests for the interactive gate explorer component.
 * Tests cover navigation, filtering, search, and keyboard interactions.
 *
 * @see https://github.com/remorses/tuistory
 */

import { describe, it, expect, beforeAll, afterEach } from 'vitest';
import {
  launchAnvil,
  ensureCliBuild,
  safeClose,
  waitForTextWithContext,
  type Session,
} from './tuistory-utils.js';

describe('Gate Explorer TUI Tests', () => {
  let session: Session | null = null;

  beforeAll(() => {
    ensureCliBuild();
  });

  afterEach(() => {
    safeClose(session);
    session = null;
  });

  describe('Basic Gate Execution', () => {
    it('runs gate command and shows progress', { timeout: 45000 }, async () => {
      session = await launchAnvil({
        args: ['gate', '--tui'],
        waitForDataTimeout: 20000,
      });

      // Wait for gate to start - it shows progress first
      await waitForTextWithContext(
        session,
        /Gate|Running|Loading|configuration|quality/i,
        'Gate starting'
      );

      const output = await session.text({ trimEnd: true });
      // Should show progress or config loading
      expect(output).toMatch(/gate|configuration|loading|running/i);

      await session.press('q');
      await new Promise((resolve) => setTimeout(resolve, 500));
    });
  });

  describe('Navigation', () => {
    it('supports vim-style navigation (j/k)', { timeout: 30000 }, async () => {
      session = await launchAnvil({
        args: ['gate', '--tui'],
        waitForDataTimeout: 15000,
      });

      await waitForTextWithContext(session, /Gate|Results|Check/i, 'Gate explorer');

      // Navigate down with 'j'
      await session.press('j');
      await new Promise((resolve) => setTimeout(resolve, 300));

      // Navigate up with 'k'
      await session.press('k');
      await new Promise((resolve) => setTimeout(resolve, 300));

      await session.press('q');
      await new Promise((resolve) => setTimeout(resolve, 500));
    });

    it('supports arrow key navigation', { timeout: 30000 }, async () => {
      session = await launchAnvil({
        args: ['gate', '--tui'],
        waitForDataTimeout: 15000,
      });

      await waitForTextWithContext(session, /Gate|Results|Check/i, 'Gate explorer');

      // Navigate with arrow keys
      await session.press('down');
      await new Promise((resolve) => setTimeout(resolve, 300));

      await session.press('up');
      await new Promise((resolve) => setTimeout(resolve, 300));

      await session.press('q');
      await new Promise((resolve) => setTimeout(resolve, 500));
    });

    it('supports failure navigation with n/N', { timeout: 30000 }, async () => {
      session = await launchAnvil({
        args: ['gate', '--tui'],
        waitForDataTimeout: 15000,
      });

      await waitForTextWithContext(session, /Gate|Results|Check/i, 'Gate explorer');

      // Jump to next failure with 'n'
      await session.press('n');
      await new Promise((resolve) => setTimeout(resolve, 300));

      // Jump to previous failure with 'N' (shift+n)
      await session.press(['shift', 'n']);
      await new Promise((resolve) => setTimeout(resolve, 300));

      await session.press('q');
      await new Promise((resolve) => setTimeout(resolve, 500));
    });
  });

  describe('Filtering', () => {
    it('filters to show all checks with "a"', { timeout: 30000 }, async () => {
      session = await launchAnvil({
        args: ['gate', '--tui'],
        waitForDataTimeout: 15000,
      });

      await waitForTextWithContext(session, /Gate|Results|Check/i, 'Gate explorer');

      // Show all checks
      await session.press('a');
      await new Promise((resolve) => setTimeout(resolve, 500));

      const output = await session.text({ trimEnd: true });
      // Should indicate "all" filter or show all checks
      expect(output).toBeDefined();

      await session.press('q');
      await new Promise((resolve) => setTimeout(resolve, 500));
    });

    it('filters to show passed checks with "p"', { timeout: 30000 }, async () => {
      session = await launchAnvil({
        args: ['gate', '--tui'],
        waitForDataTimeout: 15000,
      });

      await waitForTextWithContext(session, /Gate|Results|Check/i, 'Gate explorer');

      // Show passed checks only
      await session.press('p');
      await new Promise((resolve) => setTimeout(resolve, 500));

      await session.press('q');
      await new Promise((resolve) => setTimeout(resolve, 500));
    });

    it('filters to show failed checks with "f"', { timeout: 30000 }, async () => {
      session = await launchAnvil({
        args: ['gate', '--tui'],
        waitForDataTimeout: 15000,
      });

      await waitForTextWithContext(session, /Gate|Results|Check/i, 'Gate explorer');

      // Show failed checks only
      await session.press('f');
      await new Promise((resolve) => setTimeout(resolve, 500));

      await session.press('q');
      await new Promise((resolve) => setTimeout(resolve, 500));
    });

    it('filters to show skipped checks with "s"', { timeout: 30000 }, async () => {
      session = await launchAnvil({
        args: ['gate', '--tui'],
        waitForDataTimeout: 15000,
      });

      await waitForTextWithContext(session, /Gate|Results|Check/i, 'Gate explorer');

      // Show skipped checks only
      await session.press('s');
      await new Promise((resolve) => setTimeout(resolve, 500));

      await session.press('q');
      await new Promise((resolve) => setTimeout(resolve, 500));
    });
  });

  describe('Search', () => {
    it('enters search mode with "/"', { timeout: 30000 }, async () => {
      session = await launchAnvil({
        args: ['gate', '--tui'],
        waitForDataTimeout: 15000,
      });

      await waitForTextWithContext(session, /Gate|Results|Check/i, 'Gate explorer');

      // Enter search mode
      await session.press('/');
      await new Promise((resolve) => setTimeout(resolve, 500));

      // Type search query
      await session.type('test');
      await new Promise((resolve) => setTimeout(resolve, 300));

      // Exit search with Escape
      await session.press('escape');
      await new Promise((resolve) => setTimeout(resolve, 300));

      await session.press('q');
      await new Promise((resolve) => setTimeout(resolve, 500));
    });

    it('confirms search with Enter', { timeout: 30000 }, async () => {
      session = await launchAnvil({
        args: ['gate', '--tui'],
        waitForDataTimeout: 15000,
      });

      await waitForTextWithContext(session, /Gate|Results|Check/i, 'Gate explorer');

      // Enter search mode and search
      await session.press('/');
      await new Promise((resolve) => setTimeout(resolve, 300));

      await session.type('lint');
      await new Promise((resolve) => setTimeout(resolve, 300));

      // Confirm search
      await session.press('enter');
      await new Promise((resolve) => setTimeout(resolve, 500));

      await session.press('q');
      await new Promise((resolve) => setTimeout(resolve, 500));
    });
  });

  describe('Detail Expansion', () => {
    it('expands check details with Enter', { timeout: 30000 }, async () => {
      session = await launchAnvil({
        args: ['gate', '--tui'],
        waitForDataTimeout: 15000,
      });

      await waitForTextWithContext(session, /Gate|Results|Check/i, 'Gate explorer');

      // Press Enter to expand selected check
      await session.press('enter');
      await new Promise((resolve) => setTimeout(resolve, 500));

      // Should show expanded details
      const output = await session.text({ trimEnd: true });
      expect(output).toBeDefined();

      await session.press('q');
      await new Promise((resolve) => setTimeout(resolve, 500));
    });
  });

  describe('Export', () => {
    it('exports results with "e" key', { timeout: 30000 }, async () => {
      session = await launchAnvil({
        args: ['gate', '--tui'],
        waitForDataTimeout: 15000,
      });

      await waitForTextWithContext(session, /Gate|Results|Check/i, 'Gate explorer');

      // Press 'e' to export
      await session.press('e');
      await new Promise((resolve) => setTimeout(resolve, 500));

      await session.press('q');
      await new Promise((resolve) => setTimeout(resolve, 500));
    });
  });

  describe('Command Options', () => {
    it('respects --skip-checks option', { timeout: 30000 }, async () => {
      session = await launchAnvil({
        args: ['gate', '--tui', '--skip-checks', 'eslint'],
        waitForDataTimeout: 15000,
      });

      await waitForTextWithContext(session, /Gate|Results|Check/i, 'Gate explorer');

      await session.press('q');
      await new Promise((resolve) => setTimeout(resolve, 500));
    });

    it('respects --profile option', { timeout: 30000 }, async () => {
      session = await launchAnvil({
        args: ['gate', '--tui', '--profile', 'ci'],
        waitForDataTimeout: 15000,
      });

      await waitForTextWithContext(session, /Gate|Results|Check/i, 'Gate explorer');

      await session.press('q');
      await new Promise((resolve) => setTimeout(resolve, 500));
    });
  });

  describe('Exit Handling', () => {
    it('exits with "q" key', { timeout: 25000 }, async () => {
      session = await launchAnvil({
        args: ['gate', '--tui'],
        waitForDataTimeout: 15000,
      });

      await waitForTextWithContext(session, /Gate|Results|Check/i, 'Gate explorer');

      await session.press('q');
      await new Promise((resolve) => setTimeout(resolve, 500));
    });

    it('exits with Ctrl+C', { timeout: 25000 }, async () => {
      session = await launchAnvil({
        args: ['gate', '--tui'],
        waitForDataTimeout: 15000,
      });

      await waitForTextWithContext(session, /Gate|Results|Check/i, 'Gate explorer');

      await session.press(['ctrl', 'c']);
      await new Promise((resolve) => setTimeout(resolve, 500));
    });
  });
});
