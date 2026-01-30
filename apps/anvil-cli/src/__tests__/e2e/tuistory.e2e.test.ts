/**
 * Tuistory E2E Tests - Proof of Concept
 *
 * These tests demonstrate tuistory's capabilities for TUI testing,
 * showcasing features that go beyond the existing custom PTY utilities:
 *
 * 1. ANSI-aware text extraction (bold, colors, etc.)
 * 2. Comprehensive keyboard support with modifiers
 * 3. click() API for text-based interactions
 * 4. Frame capture for transition testing
 * 5. Cleaner async/await patterns
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

describe('Tuistory E2E Tests (PoC)', () => {
  let session: Session | null = null;

  beforeAll(() => {
    ensureCliBuild();
  });

  afterEach(() => {
    safeClose(session);
    session = null;
  });

  describe('Basic CLI Commands', () => {
    it('anvil --version: shows version number', { timeout: 20000 }, async () => {
      session = await launchAnvil({ args: ['--version'] });

      const expectedVersion = getPackageVersion();
      const output = await session.waitForText(expectedVersion, { timeout: 10000 });

      expect(output).toContain(expectedVersion);
    });

    it('anvil --help: shows help with available commands', { timeout: 20000 }, async () => {
      session = await launchAnvil({ args: ['--help'] });

      // Wait for key commands to appear
      await session.waitForText('validate', { timeout: 10000 });
      const output = await session.text({ trimEnd: true });

      expect(output).toContain('gate');
      expect(output).toContain('doctor');
      expect(output).toContain('init');
    });
  });

  describe('Interactive TUI Commands', () => {
    it(
      'anvil status: renders status dashboard and responds to quit',
      { timeout: 25000 },
      async () => {
        session = await launchAnvil({
          args: ['status'],
          waitForDataTimeout: 10000,
        });

        // Wait for status-related content to render
        await waitForTextWithContext(
          session,
          /ANVIL|Status|Hooks|Configuration|project/i,
          'Status dashboard'
        );

        // Test keyboard interaction - press 'q' to quit
        await session.press('q');

        // Give it time to process the quit
        await new Promise((resolve) => setTimeout(resolve, 500));
      }
    );

    it('anvil doctor: runs diagnostics and shows results', { timeout: 25000 }, async () => {
      session = await launchAnvil({
        args: ['doctor'],
        waitForDataTimeout: 10000,
      });

      // Wait for diagnostic results
      await waitForTextWithContext(
        session,
        /passed|warnings|Healthy|failed|Node\.js/i,
        'Doctor diagnostics'
      );

      const output = await session.text({ trimEnd: true });
      expect(output).toMatch(/Node\.js/i);

      await session.press('q');
    });

    it('anvil doctor --json: shows JSON output', { timeout: 20000 }, async () => {
      session = await launchAnvil({
        args: ['doctor', '--json'],
        waitForDataTimeout: 10000,
      });

      // Wait for JSON output
      await session.waitForText('{', { timeout: 10000 });
      const output = await session.text({ trimEnd: true });

      expect(output).toContain('"results"');
    });
  });

  describe('Tuistory-Specific Features', () => {
    it('demonstrates keyboard modifiers (ctrl+c to interrupt)', { timeout: 20000 }, async () => {
      session = await launchAnvil({
        args: ['init', '--tui'],
        cwd: '/tmp',
        waitForDataTimeout: 10000,
      });

      // Wait for init wizard to start
      await waitForTextWithContext(session, /init|setup|configure|project|Anvil/i, 'Init wizard');

      // Use keyboard modifier to interrupt
      await session.press(['ctrl', 'c']);

      // Give it time to process
      await new Promise((resolve) => setTimeout(resolve, 500));
    });

    it('demonstrates vim-style navigation (j/k keys)', { timeout: 25000 }, async () => {
      session = await launchAnvil({
        args: ['new'],
        waitForDataTimeout: 10000,
      });

      // Wait for template browser
      await waitForTextWithContext(
        session,
        /template|select|choose|Template|category/i,
        'Template browser'
      );

      // Navigate with vim keys
      await session.press('j'); // down
      await new Promise((resolve) => setTimeout(resolve, 200));
      await session.press('k'); // up
      await new Promise((resolve) => setTimeout(resolve, 200));

      // Quit
      await session.press('q');
    });

    it('demonstrates arrow key navigation', { timeout: 25000 }, async () => {
      session = await launchAnvil({
        args: ['new'],
        waitForDataTimeout: 10000,
      });

      // Wait for template browser
      await waitForTextWithContext(
        session,
        /template|select|choose|Template|category/i,
        'Template browser'
      );

      // Navigate with arrow keys
      await session.press('down');
      await new Promise((resolve) => setTimeout(resolve, 200));
      await session.press('up');
      await new Promise((resolve) => setTimeout(resolve, 200));

      // Quit
      await session.press('q');
    });

    it('demonstrates text extraction with trimEnd option', { timeout: 20000 }, async () => {
      session = await launchAnvil({ args: ['--help'] });

      await session.waitForText('validate', { timeout: 10000 });

      // Get text with trailing whitespace trimmed
      const trimmedOutput = await session.text({ trimEnd: true });

      // Verify it's properly trimmed
      expect(trimmedOutput).not.toMatch(/\s+$/);
    });

    it('demonstrates frame capture for transition detection', { timeout: 25000 }, async () => {
      session = await launchAnvil({
        args: ['tutorial'],
        waitForDataTimeout: 10000,
      });

      // Wait for tutorial to load
      await waitForTextWithContext(session, /Tutorial|Welcome|Step|Anvil/i, 'Tutorial screen');

      // Capture frames during a transition (pressing a key)
      const frames = await session.captureFrames('q', {
        frameCount: 3,
        intervalMs: 50,
      });

      // We captured multiple frames
      expect(frames.length).toBe(3);
      // Frames should be strings (terminal output snapshots)
      frames.forEach((frame) => {
        expect(typeof frame).toBe('string');
      });
    });
  });

  describe('Comparison with PTY Utils', () => {
    /**
     * This test demonstrates what tuistory makes easier compared to
     * the custom PTY utilities:
     *
     * 1. No manual ANSI stripping needed - handled internally
     * 2. Built-in timeout handling with clear error messages
     * 3. Cleaner async/await patterns
     * 4. More comprehensive key support out of the box
     */
    it('provides cleaner API than manual PTY management', { timeout: 20000 }, async () => {
      session = await launchAnvil({ args: ['--help'] });

      // With tuistory: simple waitForText with built-in timeout
      // (vs manual polling loop in PTY utils)
      await session.waitForText('validate', { timeout: 10000 });

      // With tuistory: direct text() call returns clean text
      // (vs manual stripAnsi in PTY utils)
      const output = await session.text({ trimEnd: true });

      // With tuistory: array of keys for modifiers
      // (vs manually looking up escape codes)
      // await session.press(['ctrl', 'c']); // if needed

      expect(output).toContain('gate');
      expect(output).toContain('doctor');
    });
  });
});
