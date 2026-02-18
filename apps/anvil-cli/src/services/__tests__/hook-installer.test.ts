/**
 * Tests for HookInstaller service
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { existsSync, readFileSync, writeFileSync, mkdirSync, statSync } from 'node:fs';
import { join } from 'node:path';
import { mkdtempSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { HookInstaller, ANVIL_MARKER, AVAILABLE_HOOKS } from '../hook-installer.js';

describe('HookInstaller', () => {
  let tempDir: string;
  let hooksDir: string;
  let installer: HookInstaller;

  beforeEach(() => {
    tempDir = mkdtempSync(join(tmpdir(), 'hook-installer-test-'));
    hooksDir = '.git/hooks';
    installer = new HookInstaller();
  });

  afterEach(() => {
    rmSync(tempDir, { recursive: true, force: true });
  });

  describe('AVAILABLE_HOOKS', () => {
    it('should define pre-commit and pre-push hooks', () => {
      expect(AVAILABLE_HOOKS).toHaveLength(2);
      expect(AVAILABLE_HOOKS.map((h) => h.name)).toEqual(['pre-commit', 'pre-push']);
    });

    it('should have script paths for all hooks', () => {
      for (const hook of AVAILABLE_HOOKS) {
        expect(hook.scriptPath).toBeTruthy();
        expect(hook.description).toBeTruthy();
      }
    });
  });

  describe('ANVIL_MARKER', () => {
    it('should be a comment string', () => {
      expect(ANVIL_MARKER).toBe('# Anvil-managed hook');
    });
  });

  describe('installHook', () => {
    it('should create hook file with Anvil marker after shebang', () => {
      installer.installHook(tempDir, 'pre-commit', hooksDir);
      const hookPath = join(tempDir, hooksDir, 'pre-commit');

      expect(existsSync(hookPath)).toBe(true);
      const content = readFileSync(hookPath, 'utf-8');
      expect(content).toContain(ANVIL_MARKER);
      // Shebang must be on line 1, marker on line 2
      const lines = content.split('\n');
      expect(lines[0]).toMatch(/^#!/);
      expect(lines[1]).toBe(ANVIL_MARKER);
    });

    it('should create hooks directory if it does not exist', () => {
      const customHooksDir = '.custom-hooks';
      expect(existsSync(join(tempDir, customHooksDir))).toBe(false);
      installer.installHook(tempDir, 'pre-commit', customHooksDir);
      expect(existsSync(join(tempDir, customHooksDir))).toBe(true);
    });

    it.skipIf(process.platform === 'win32')(
      'should set executable permissions on hook file',
      () => {
        installer.installHook(tempDir, 'pre-commit', hooksDir);
        const hookPath = join(tempDir, hooksDir, 'pre-commit');
        const stat = statSync(hookPath);
        // Check owner execute bit (0o100)
        expect(stat.mode & 0o111).toBeGreaterThan(0);
      }
    );

    it('should install pre-push hook', () => {
      installer.installHook(tempDir, 'pre-push', hooksDir);
      const hookPath = join(tempDir, hooksDir, 'pre-push');

      expect(existsSync(hookPath)).toBe(true);
      const content = readFileSync(hookPath, 'utf-8');
      expect(content).toContain(ANVIL_MARKER);
      expect(content).toContain('#!/bin/sh');
    });
  });

  describe('loadHookScript', () => {
    it('should return script content for pre-commit', () => {
      const content = installer.loadHookScript('pre-commit');
      expect(content).toContain('#!/bin/sh');
      expect(content).toContain('pre-commit');
    });

    it('should return script content for pre-push', () => {
      const content = installer.loadHookScript('pre-push');
      expect(content).toContain('#!/bin/sh');
      expect(content).toContain('pre-push');
    });

    it('should throw for unknown hook names', () => {
      // loadHookScript falls through to getEmbeddedScript which throws
      expect(() => installer.loadHookScript('post-merge')).toThrow(
        /No embedded script for hook: post-merge/
      );
    });
  });

  describe('isAnvilManagedHook', () => {
    it('should return true for Anvil-managed hook', () => {
      const hookPath = join(tempDir, 'hook');
      writeFileSync(hookPath, `${ANVIL_MARKER}\n#!/bin/sh\necho "test"`);
      expect(installer.isAnvilManagedHook(hookPath)).toBe(true);
    });

    it('should return false for non-Anvil hook', () => {
      const hookPath = join(tempDir, 'hook');
      writeFileSync(hookPath, '#!/bin/sh\necho "custom hook"');
      expect(installer.isAnvilManagedHook(hookPath)).toBe(false);
    });

    it('should return false for non-existent file', () => {
      expect(installer.isAnvilManagedHook(join(tempDir, 'nonexistent'))).toBe(false);
    });

    it('should detect marker anywhere in content', () => {
      const hookPath = join(tempDir, 'hook');
      writeFileSync(hookPath, `#!/bin/sh\n# Some comment\n${ANVIL_MARKER}\necho test`);
      expect(installer.isAnvilManagedHook(hookPath)).toBe(true);
    });
  });

  describe('uninstallHook', () => {
    it('should remove Anvil-managed hook', () => {
      // Install first
      installer.installHook(tempDir, 'pre-commit', hooksDir);
      const hookPath = join(tempDir, hooksDir, 'pre-commit');
      expect(existsSync(hookPath)).toBe(true);

      // Uninstall
      const result = installer.uninstallHook(tempDir, 'pre-commit', hooksDir);
      expect(result).toBe(true);
      expect(existsSync(hookPath)).toBe(false);
    });

    it('should return false for non-existent hook', () => {
      const result = installer.uninstallHook(tempDir, 'pre-commit', hooksDir);
      expect(result).toBe(false);
    });

    it('should throw when trying to remove non-Anvil hook', () => {
      const hookPath = join(tempDir, hooksDir, 'pre-commit');
      mkdirSync(join(tempDir, hooksDir), { recursive: true });
      writeFileSync(hookPath, '#!/bin/sh\necho "custom"');

      expect(() => installer.uninstallHook(tempDir, 'pre-commit', hooksDir)).toThrow(
        /not managed by Anvil/
      );
      // File should still exist
      expect(existsSync(hookPath)).toBe(true);
    });
  });

  describe('isHookInstalled', () => {
    it('should return true when Anvil hook is installed', () => {
      installer.installHook(tempDir, 'pre-commit', hooksDir);
      expect(installer.isHookInstalled(tempDir, 'pre-commit', hooksDir)).toBe(true);
    });

    it('should return false when no hook exists', () => {
      expect(installer.isHookInstalled(tempDir, 'pre-commit', hooksDir)).toBe(false);
    });

    it('should return false when non-Anvil hook exists', () => {
      mkdirSync(join(tempDir, hooksDir), { recursive: true });
      writeFileSync(join(tempDir, hooksDir, 'pre-commit'), '#!/bin/sh\necho "custom"');
      expect(installer.isHookInstalled(tempDir, 'pre-commit', hooksDir)).toBe(false);
    });
  });

  describe('backupExistingHook', () => {
    it('should backup non-Anvil hook to .backup file', () => {
      mkdirSync(join(tempDir, hooksDir), { recursive: true });
      const hookPath = join(tempDir, hooksDir, 'pre-commit');
      writeFileSync(hookPath, '#!/bin/sh\necho "custom hook"');

      installer.backupExistingHook(tempDir, 'pre-commit', hooksDir);

      const backupPath = `${hookPath}.backup`;
      expect(existsSync(backupPath)).toBe(true);
      expect(readFileSync(backupPath, 'utf-8')).toBe('#!/bin/sh\necho "custom hook"');
    });

    it('should not backup Anvil-managed hook', () => {
      installer.installHook(tempDir, 'pre-commit', hooksDir);

      installer.backupExistingHook(tempDir, 'pre-commit', hooksDir);

      const backupPath = join(tempDir, hooksDir, 'pre-commit.backup');
      expect(existsSync(backupPath)).toBe(false);
    });

    it('should not fail when no hook exists', () => {
      expect(() => {
        installer.backupExistingHook(tempDir, 'pre-commit', hooksDir);
      }).not.toThrow();
    });
  });
});
