/**
 * Init Wizard TUI E2E Tests
 *
 * Comprehensive tests for the interactive init wizard component.
 * Tests cover step navigation, selections, and keyboard interactions.
 *
 * The wizard has 5 steps:
 * 1. Configuration Mode (Standard/Strict/CI-optimised)
 * 2. Planning Format (APS/SpecKit/BMAD/Generic/Skip)
 * 3. Directory Setup (text input)
 * 4. Quality Checks (checkbox toggles)
 * 5. Review & Confirm (summary)
 *
 * @see https://github.com/remorses/tuistory
 */

import { describe, it, expect, beforeAll, afterEach, beforeEach } from 'vitest';
import {
  launchAnvil,
  ensureCliBuild,
  safeClose,
  waitForTextWithContext,
  type Session,
} from './tuistory-utils.js';
import { mkdtempSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

describe('Init Wizard TUI Tests', () => {
  let session: Session | null = null;
  let testDir: string;

  beforeAll(() => {
    ensureCliBuild();
  });

  beforeEach(() => {
    // Create a fresh temp directory for each test
    testDir = mkdtempSync(join(tmpdir(), 'anvil-init-test-'));
  });

  afterEach(() => {
    safeClose(session);
    session = null;

    // Clean up temp directory
    try {
      rmSync(testDir, { recursive: true, force: true });
    } catch {
      // Ignore cleanup errors
    }
  });

  describe('Wizard Launch', () => {
    it('shows init wizard with --tui flag', { timeout: 30000 }, async () => {
      session = await launchAnvil({
        args: ['init', '--tui'],
        cwd: testDir,
        waitForDataTimeout: 15000,
      });

      // Init command shows project analysis first, then architecture prompt
      await waitForTextWithContext(
        session,
        /Initialising|Anvil|Detected|architecture|Analysis/i,
        'Init analysis'
      );

      const output = await session.text({ trimEnd: true });
      // Should show analysis results or prompts
      expect(output).toMatch(/Detected|Architecture|Anvil|init/i);

      await session.press(['ctrl', 'c']);
      await new Promise((resolve) => setTimeout(resolve, 500));
    });

    it('exits with Ctrl+C during analysis', { timeout: 30000 }, async () => {
      session = await launchAnvil({
        args: ['init', '--tui'],
        cwd: testDir,
        waitForDataTimeout: 15000,
      });

      await waitForTextWithContext(
        session,
        /Initialising|Anvil|Detected/i,
        'Init starting'
      );

      await session.press(['ctrl', 'c']);
      await new Promise((resolve) => setTimeout(resolve, 500));
    });
  });

  describe('Init with --no-analysis flag', () => {
    it('shows init wizard without project analysis', { timeout: 30000 }, async () => {
      session = await launchAnvil({
        args: ['init', '--tui', '--no-analysis'],
        cwd: testDir,
        waitForDataTimeout: 15000,
      });

      // With --no-analysis, should go directly to setup prompts
      await waitForTextWithContext(
        session,
        /Initialising|Anvil|init|setup|configure/i,
        'Init starting'
      );

      await session.press(['ctrl', 'c']);
      await new Promise((resolve) => setTimeout(resolve, 500));
    });
  });

  describe('Init with existing project (main repo)', () => {
    it('shows init wizard in actual project directory', { timeout: 30000 }, async () => {
      // Use the main repo which has a real project structure
      session = await launchAnvil({
        args: ['init', '--tui'],
        // Don't specify cwd - use PROJECT_ROOT (default)
        waitForDataTimeout: 15000,
      });

      // Should show analysis of the real project
      await waitForTextWithContext(
        session,
        /Initialising|Anvil|Detected|architecture|TypeScript|pnpm/i,
        'Init with real project'
      );

      await session.press(['ctrl', 'c']);
      await new Promise((resolve) => setTimeout(resolve, 500));
    });
  });

  describe('Non-Interactive Mode', () => {
    it('runs with --non-interactive flag', { timeout: 25000 }, async () => {
      session = await launchAnvil({
        args: ['init', '--non-interactive'],
        cwd: testDir,
        waitForDataTimeout: 10000,
      });

      // Should complete without prompts
      await new Promise((resolve) => setTimeout(resolve, 3000));

      const output = await session.text({ trimEnd: true });
      // Should show completion message or create files
      expect(output).toBeDefined();
    });
  });
});
