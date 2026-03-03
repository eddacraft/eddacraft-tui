import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { existsSync, readFileSync, mkdirSync, rmSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { tmpdir } from 'node:os';

// Create a unique temp directory for each test
function createTempDir(): string {
  const tempDir = join(
    tmpdir(),
    `anvil-hooks-test-${Date.now()}-${Math.random().toString(36).slice(2)}`
  );
  mkdirSync(tempDir, { recursive: true });
  return tempDir;
}

// Clean up temp directory
function cleanupTempDir(dir: string): void {
  if (existsSync(dir)) {
    rmSync(dir, { recursive: true, force: true });
  }
}

// Initialise a git repo in the temp directory
function initGitRepo(dir: string): void {
  const gitDir = join(dir, '.git');
  const hooksDir = join(gitDir, 'hooks');
  mkdirSync(hooksDir, { recursive: true });
}

describe('hooks command', () => {
  let tempDir: string;
  let originalCwd: string;

  beforeEach(() => {
    tempDir = createTempDir();
    originalCwd = process.cwd();

    // Create package.json so getWorkspaceRoot() works
    writeFileSync(join(tempDir, 'package.json'), JSON.stringify({ name: 'test' }), 'utf-8');

    // Initialise git repo
    initGitRepo(tempDir);

    process.chdir(tempDir);
  });

  afterEach(() => {
    process.chdir(originalCwd);
    cleanupTempDir(tempDir);
    vi.restoreAllMocks();
  });

  describe('install', () => {
    it('should create pre-commit hook in .git/hooks', { timeout: 10000 }, async () => {
      const { createHooksCommand } = await import('./hooks.js');
      const command = createHooksCommand();

      // Find and execute the install subcommand
      const installCmd = command.commands.find((c) => c.name() === 'install');
      expect(installCmd).toBeDefined();

      // Execute the action with options
      await installCmd!.parseAsync(['node', 'test', '--pre-commit-only']);

      const hookPath = join(tempDir, '.git', 'hooks', 'pre-commit');
      expect(existsSync(hookPath)).toBe(true);

      const content = readFileSync(hookPath, 'utf-8');
      expect(content).toContain('# Anvil-managed hook');
      expect(content).toContain('anvil validate');
    });

    it('should create pre-push hook in .git/hooks', async () => {
      const { createHooksCommand } = await import('./hooks.js');
      const command = createHooksCommand();

      const installCmd = command.commands.find((c) => c.name() === 'install');
      await installCmd!.parseAsync(['node', 'test', '--pre-push-only']);

      const hookPath = join(tempDir, '.git', 'hooks', 'pre-push');
      expect(existsSync(hookPath)).toBe(true);

      const content = readFileSync(hookPath, 'utf-8');
      expect(content).toContain('# Anvil-managed hook');
      expect(content).toContain('anvil gate');
      expect(content).toContain('ANVIL_SKIP_HOOKS');
    });

    it('should create both hooks by default', async () => {
      const { createHooksCommand } = await import('./hooks.js');
      const command = createHooksCommand();

      const installCmd = command.commands.find((c) => c.name() === 'install');
      await installCmd!.parseAsync(['node', 'test']);

      expect(existsSync(join(tempDir, '.git', 'hooks', 'pre-commit'))).toBe(true);
      expect(existsSync(join(tempDir, '.git', 'hooks', 'pre-push'))).toBe(true);
    });

    it('should skip existing non-Anvil hooks without --force', async () => {
      // Create an existing hook
      const hooksDir = join(tempDir, '.git', 'hooks');
      mkdirSync(hooksDir, { recursive: true });
      writeFileSync(join(hooksDir, 'pre-commit'), '#!/bin/sh\necho "existing hook"', 'utf-8');

      const { createHooksCommand } = await import('./hooks.js');
      const command = createHooksCommand();

      const installCmd = command.commands.find((c) => c.name() === 'install');
      await installCmd!.parseAsync(['node', 'test', '--pre-commit-only']);

      // Hook should not be overwritten
      const content = readFileSync(join(hooksDir, 'pre-commit'), 'utf-8');
      expect(content).not.toContain('# Anvil-managed hook');
      expect(content).toContain('existing hook');
    });

    it('should overwrite existing hooks with --force and create backup', async () => {
      // Create an existing hook
      const hooksDir = join(tempDir, '.git', 'hooks');
      mkdirSync(hooksDir, { recursive: true });
      writeFileSync(join(hooksDir, 'pre-commit'), '#!/bin/sh\necho "existing hook"', 'utf-8');

      const { createHooksCommand } = await import('./hooks.js');
      const command = createHooksCommand();

      const installCmd = command.commands.find((c) => c.name() === 'install');
      await installCmd!.parseAsync(['node', 'test', '--pre-commit-only', '--force']);

      // Hook should be overwritten
      const content = readFileSync(join(hooksDir, 'pre-commit'), 'utf-8');
      expect(content).toContain('# Anvil-managed hook');

      // Backup should exist
      const backupPath = join(hooksDir, 'pre-commit.anvil-backup');
      expect(existsSync(backupPath)).toBe(true);
      const backupContent = readFileSync(backupPath, 'utf-8');
      expect(backupContent).toContain('existing hook');
    });

    it('should update existing Anvil hooks without backup', async () => {
      // Create an existing Anvil hook
      const hooksDir = join(tempDir, '.git', 'hooks');
      mkdirSync(hooksDir, { recursive: true });
      writeFileSync(
        join(hooksDir, 'pre-commit'),
        '# Anvil-managed hook\n#!/bin/sh\necho "old anvil hook"',
        'utf-8'
      );

      const { createHooksCommand } = await import('./hooks.js');
      const command = createHooksCommand();

      const installCmd = command.commands.find((c) => c.name() === 'install');
      await installCmd!.parseAsync(['node', 'test', '--pre-commit-only']);

      // Hook should be updated
      const content = readFileSync(join(hooksDir, 'pre-commit'), 'utf-8');
      expect(content).toContain('# Anvil-managed hook');
      expect(content).toContain('anvil validate');
      expect(content).not.toContain('old anvil hook');

      // No backup should be created for Anvil hooks
      const backupPath = join(hooksDir, 'pre-commit.anvil-backup');
      expect(existsSync(backupPath)).toBe(false);
    });

    it('should install in .husky directory with --husky flag', async () => {
      const { createHooksCommand } = await import('./hooks.js');
      const command = createHooksCommand();

      const installCmd = command.commands.find((c) => c.name() === 'install');
      await installCmd!.parseAsync(['node', 'test', '--husky', '--pre-commit-only']);

      const hookPath = join(tempDir, '.husky', 'pre-commit');
      expect(existsSync(hookPath)).toBe(true);

      const content = readFileSync(hookPath, 'utf-8');
      expect(content).toContain('# Anvil-managed hook');
    });

    it('should detect Husky and use .husky directory automatically', async () => {
      // Create .husky directory to simulate existing Husky setup
      mkdirSync(join(tempDir, '.husky'), { recursive: true });

      const { createHooksCommand } = await import('./hooks.js');
      const command = createHooksCommand();

      const installCmd = command.commands.find((c) => c.name() === 'install');
      await installCmd!.parseAsync(['node', 'test', '--pre-commit-only']);

      // Should install in .husky, not .git/hooks
      expect(existsSync(join(tempDir, '.husky', 'pre-commit'))).toBe(true);
      expect(existsSync(join(tempDir, '.git', 'hooks', 'pre-commit'))).toBe(false);
    });
  });

  describe('uninstall', () => {
    it('should remove Anvil-managed hooks', async () => {
      // First install the hook
      const hooksDir = join(tempDir, '.git', 'hooks');
      mkdirSync(hooksDir, { recursive: true });
      writeFileSync(
        join(hooksDir, 'pre-commit'),
        '# Anvil-managed hook\n#!/bin/sh\necho "anvil hook"',
        'utf-8'
      );

      const { createHooksCommand } = await import('./hooks.js');
      const command = createHooksCommand();

      const uninstallCmd = command.commands.find((c) => c.name() === 'uninstall');
      await uninstallCmd!.parseAsync(['node', 'test', '--pre-commit-only']);

      // Hook should be removed
      expect(existsSync(join(hooksDir, 'pre-commit'))).toBe(false);
    });

    it('should not remove non-Anvil hooks', async () => {
      const hooksDir = join(tempDir, '.git', 'hooks');
      mkdirSync(hooksDir, { recursive: true });
      writeFileSync(join(hooksDir, 'pre-commit'), '#!/bin/sh\necho "custom hook"', 'utf-8');

      const { createHooksCommand } = await import('./hooks.js');
      const command = createHooksCommand();

      const uninstallCmd = command.commands.find((c) => c.name() === 'uninstall');
      await uninstallCmd!.parseAsync(['node', 'test', '--pre-commit-only']);

      // Hook should still exist
      expect(existsSync(join(hooksDir, 'pre-commit'))).toBe(true);
      const content = readFileSync(join(hooksDir, 'pre-commit'), 'utf-8');
      expect(content).toContain('custom hook');
    });

    it('should restore backup when uninstalling', async () => {
      const hooksDir = join(tempDir, '.git', 'hooks');
      mkdirSync(hooksDir, { recursive: true });

      // Create Anvil hook with backup
      writeFileSync(
        join(hooksDir, 'pre-commit'),
        '# Anvil-managed hook\n#!/bin/sh\necho "anvil hook"',
        'utf-8'
      );
      writeFileSync(
        join(hooksDir, 'pre-commit.anvil-backup'),
        '#!/bin/sh\necho "original hook"',
        'utf-8'
      );

      const { createHooksCommand } = await import('./hooks.js');
      const command = createHooksCommand();

      const uninstallCmd = command.commands.find((c) => c.name() === 'uninstall');
      await uninstallCmd!.parseAsync(['node', 'test', '--pre-commit-only']);

      // Original hook should be restored
      expect(existsSync(join(hooksDir, 'pre-commit'))).toBe(true);
      const content = readFileSync(join(hooksDir, 'pre-commit'), 'utf-8');
      expect(content).toContain('original hook');

      // Backup should be removed
      expect(existsSync(join(hooksDir, 'pre-commit.anvil-backup'))).toBe(false);
    });
  });

  describe('status', () => {
    it('should show hook status', async () => {
      const hooksDir = join(tempDir, '.git', 'hooks');
      mkdirSync(hooksDir, { recursive: true });
      writeFileSync(
        join(hooksDir, 'pre-commit'),
        '# Anvil-managed hook\n#!/bin/sh\necho "anvil hook"',
        'utf-8'
      );
      writeFileSync(join(hooksDir, 'pre-push'), '#!/bin/sh\necho "custom hook"', 'utf-8');

      const { createHooksCommand } = await import('./hooks.js');
      const command = createHooksCommand();

      const consoleSpy = vi.spyOn(console, 'error').mockImplementation(() => {});

      const statusCmd = command.commands.find((c) => c.name() === 'status');
      await statusCmd!.parseAsync(['node', 'test']);

      // Check that status was output (human output goes to stderr)
      const calls = consoleSpy.mock.calls.map((call) => call[0]);
      const output = calls.join('\n');

      expect(output).toContain('.git/hooks');

      consoleSpy.mockRestore();
    });
  });
});

describe('hook script content', () => {
  it('pre-commit hook should check for plan files', async () => {
    // The hook content is defined as a constant, we can test it directly
    const { createHooksCommand } = await import('./hooks.js');

    // We can't easily access the constant, but we can verify via installation
    const tempDir = createTempDir();
    const originalCwd = process.cwd();

    try {
      writeFileSync(join(tempDir, 'package.json'), JSON.stringify({ name: 'test' }), 'utf-8');
      initGitRepo(tempDir);
      process.chdir(tempDir);

      const command = createHooksCommand();

      const installCmd = command.commands.find((c) => c.name() === 'install');
      await installCmd!.parseAsync(['node', 'test', '--pre-commit-only']);

      const hookPath = join(tempDir, '.git', 'hooks', 'pre-commit');
      const content = readFileSync(hookPath, 'utf-8');

      // Verify hook content
      expect(content).toContain('git diff --cached');
      expect(content).toContain('anvil validate');
      expect(content).toContain('md|yaml|yml|json'); // Check for file extension pattern
    } finally {
      process.chdir(originalCwd);
      cleanupTempDir(tempDir);
    }
  });

  it('pre-push hook should support ANVIL_SKIP_HOOKS', async () => {
    const tempDir = createTempDir();
    const originalCwd = process.cwd();

    try {
      writeFileSync(join(tempDir, 'package.json'), JSON.stringify({ name: 'test' }), 'utf-8');
      initGitRepo(tempDir);
      process.chdir(tempDir);

      const { createHooksCommand } = await import('./hooks.js');
      const command = createHooksCommand();

      const installCmd = command.commands.find((c) => c.name() === 'install');
      await installCmd!.parseAsync(['node', 'test', '--pre-push-only']);

      const hookPath = join(tempDir, '.git', 'hooks', 'pre-push');
      const content = readFileSync(hookPath, 'utf-8');

      // Verify skip hooks support
      expect(content).toContain('ANVIL_SKIP_HOOKS');
      expect(content).toContain('anvil gate');
    } finally {
      process.chdir(originalCwd);
      cleanupTempDir(tempDir);
    }
  });
});
