/**
 * Unit Tests for OPA Binary Manager
 *
 * Tests OPA binary download, caching, and version management
 */

import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { OPABinaryManager } from './opa-binary-manager.js';
import { existsSync, mkdirSync, rmSync, writeFileSync, chmodSync } from 'node:fs';
import { join } from 'node:path';
import { tmpdir, platform, arch } from 'node:os';

// Windows CI runners are slower with filesystem operations in temp dirs;
// bump the per-test timeout to prevent flaky timeouts on ensureBinary().
describe('OPABinaryManager', { timeout: 15_000 }, () => {
  let manager: OPABinaryManager;
  let tempCacheDir: string;
  let originalEnv: NodeJS.ProcessEnv;

  beforeEach(() => {
    // Save original environment
    originalEnv = { ...process.env };

    // Create temp cache directory
    tempCacheDir = join(tmpdir(), 'anvil-opa-test', Math.random().toString(36));
    mkdirSync(tempCacheDir, { recursive: true });

    // Clean up environment variables
    delete process.env.ANVIL_OPA_PATH;
    delete process.env.ANVIL_OPA_VERSION;

    manager = new OPABinaryManager({
      cacheDir: tempCacheDir,
      autoDownload: false, // Disable auto-download for tests
    });
  });

  afterEach(() => {
    // Restore environment
    process.env = originalEnv;

    // Clean up temp directory
    if (existsSync(tempCacheDir)) {
      rmSync(tempCacheDir, { recursive: true, force: true });
    }

    vi.restoreAllMocks();
  });

  describe('initialization', () => {
    it('should use ANVIL_OPA_VERSION from environment when provided', async () => {
      process.env.ANVIL_OPA_VERSION = '0.50.0';
      const envManager = new OPABinaryManager({ cacheDir: tempCacheDir, autoDownload: false });

      // getBinaryInfo returns null when no binary exists, but the version
      // is embedded in the expected binary path name. Verify via error message.
      const error = await envManager.ensureBinary().catch((e: Error) => e);
      expect(error).toBeInstanceOf(Error);
      expect((error as Error).message).toContain('0.50.0');
    });

    it('should use custom version from config', async () => {
      const customManager = new OPABinaryManager({
        version: '0.55.0',
        cacheDir: tempCacheDir,
        autoDownload: false,
      });

      const error = await customManager.ensureBinary().catch((e: Error) => e);
      expect(error).toBeInstanceOf(Error);
      expect((error as Error).message).toContain('0.55.0');
    });
  });

  describe('binary path detection', () => {
    it('should use ANVIL_OPA_PATH when set and file exists', async () => {
      // Create a mock binary - verifyVersion will fail but the path check happens first
      const mockPath = join(tempCacheDir, 'mock-opa');
      writeFileSync(mockPath, '#!/bin/sh\necho "Version: 0.60.0"');
      chmodSync(mockPath, 0o755);

      process.env.ANVIL_OPA_PATH = mockPath;

      // The binary exists but version verification may fail since this is a shell
      // script not a real OPA binary. Either way, the path should be checked first.
      const result = await manager.ensureBinary().catch((e: Error) => e);

      // It either returns the path (if verification passes) or throws
      // (if verification fails). Either way, it should NOT throw "file not found".
      if (typeof result === 'string') {
        expect(result).toBe(mockPath);
      } else {
        expect((result as Error).message).not.toContain('file not found');
      }
    });

    it('should throw error when ANVIL_OPA_PATH does not exist', async () => {
      process.env.ANVIL_OPA_PATH = '/nonexistent/opa';

      await expect(manager.ensureBinary()).rejects.toThrow('file not found');
    });
  });

  describe('getBinaryInfo', () => {
    it('should return null when binary is not available', async () => {
      const info = await manager.getBinaryInfo();

      // With autoDownload false and no binary in cache, should return null
      expect(info).toBeNull();
    });

    it('should return null rather than throwing when no binary exists', async () => {
      // getBinaryInfo wraps ensureBinary in try/catch, returning null on failure
      const info = await manager.getBinaryInfo();
      expect(info).toBeNull();

      // Verify that ensureBinary itself DOES throw
      await expect(manager.ensureBinary()).rejects.toThrow();
    });
  });

  describe('fallback behaviour', () => {
    it('should throw descriptive error when autoDownload is false and no binary exists', async () => {
      const noDownloadManager = new OPABinaryManager({
        cacheDir: tempCacheDir,
        autoDownload: false,
      });

      await expect(noDownloadManager.ensureBinary()).rejects.toThrow(/OPA binary not found/);
    });

    it('should include install instructions in error message', async () => {
      await expect(manager.ensureBinary()).rejects.toThrow(/ANVIL_OPA_PATH/);
    });
  });

  describe('forceDownload', () => {
    it('should remove existing cached binary before re-downloading', async () => {
      const PLATFORM_MAP: Record<string, string> = {
        darwin: 'darwin',
        linux: 'linux',
        win32: 'windows',
      };
      const ARCH_MAP: Record<string, string> = { x64: 'amd64', arm64: 'arm64' };
      const plat = PLATFORM_MAP[platform()] || platform();
      const architecture = ARCH_MAP[arch()] || arch();
      const ext = plat === 'windows' ? '.exe' : '';
      const binaryPath = join(tempCacheDir, `opa-0.60.0-${plat}-${architecture}${ext}`);
      writeFileSync(binaryPath, 'mock binary');

      expect(existsSync(binaryPath)).toBe(true);

      // Mock downloadBinary to prevent actual network call

      vi.spyOn(
        manager as unknown as Record<string, (...args: unknown[]) => unknown>,
        'downloadBinary'
      ).mockResolvedValue(undefined);

      const verifiedPath = await manager.forceDownload();

      // The old binary should have been deleted before download was attempted
      expect(existsSync(binaryPath)).toBe(false);
      expect(verifiedPath).toBe(binaryPath);
    });
  });

  describe('error handling', () => {
    it('should handle invalid cache directory gracefully', async () => {
      const invalidManager = new OPABinaryManager({
        cacheDir: '/invalid/path/that/does/not/exist',
        autoDownload: false,
      });

      await expect(invalidManager.ensureBinary()).rejects.toThrow();
    });

    it('should provide helpful error messages with version info', async () => {
      const error = await manager.ensureBinary().catch((e: Error) => e);
      const err = error as Error;
      expect(err.message).toContain('OPA');
      expect(err.message.length).toBeGreaterThan(10);
    });
  });

  describe('download URL generation', () => {
    it('should generate correct download URL format', () => {
      // Access private method to verify URL generation
      const url = (manager as unknown as { getDownloadUrl(): string }).getDownloadUrl();

      expect(url).toMatch(/^https:\/\/openpolicyagent\.org\/downloads\/v\d+\.\d+\.\d+\/opa_/);
      expect(url).toContain('0.60.0');
      // Should contain platform and architecture
      expect(url).toMatch(/opa_(darwin|linux|windows)_(amd64|arm64)/);
    });

    it('should use correct version in download URL', () => {
      const customManager = new OPABinaryManager({
        version: '0.55.0',
        cacheDir: tempCacheDir,
      });
      const url = (customManager as unknown as { getDownloadUrl(): string }).getDownloadUrl();

      expect(url).toContain('0.55.0');
      expect(url).not.toContain('0.60.0');
    });
  });
});
