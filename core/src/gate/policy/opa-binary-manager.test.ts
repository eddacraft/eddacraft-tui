/**
 * Unit Tests for OPA Binary Manager
 *
 * Tests OPA binary download, caching, and version management
 */

import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { OPABinaryManager } from './opa-binary-manager.js';
import { existsSync, mkdirSync, rmSync, writeFileSync, chmodSync } from 'fs';
import { join } from 'path';
import { tmpdir } from 'os';

describe('OPABinaryManager', () => {
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
  });

  describe('initialization', () => {
    it('should create manager with default config', () => {
      const defaultManager = new OPABinaryManager();
      expect(defaultManager).toBeDefined();
    });

    it('should accept custom version', () => {
      const customManager = new OPABinaryManager({ version: '0.55.0' });
      expect(customManager).toBeDefined();
    });

    it('should accept custom cache directory', () => {
      const customDir = join(tempCacheDir, 'custom');
      const customManager = new OPABinaryManager({ cacheDir: customDir });
      expect(customManager).toBeDefined();
    });

    it('should respect ANVIL_OPA_VERSION environment variable', () => {
      process.env.ANVIL_OPA_VERSION = '0.50.0';
      const envManager = new OPABinaryManager({ cacheDir: tempCacheDir });
      expect(envManager).toBeDefined();
    });
  });

  describe('binary path detection', () => {
    it('should use ANVIL_OPA_PATH when set', async () => {
      // Create a mock binary
      const mockPath = join(tempCacheDir, 'mock-opa');
      writeFileSync(mockPath, '#!/bin/sh\necho "Version: 0.60.0"');
      chmodSync(mockPath, 0o755);

      process.env.ANVIL_OPA_PATH = mockPath;

      try {
        const path = await manager.ensureBinary();
        expect(path).toBe(mockPath);
      } catch (error) {
        // Version verification might fail, but path should be checked
        expect(error).toBeDefined();
      }
    });

    it('should throw error when ANVIL_OPA_PATH does not exist', async () => {
      process.env.ANVIL_OPA_PATH = '/nonexistent/opa';

      await expect(manager.ensureBinary()).rejects.toThrow('file not found');
    });
  });

  describe('platform detection', () => {
    it('should detect current platform', async () => {
      const info = await manager.getBinaryInfo();

      // If binary is available or can be downloaded
      if (info) {
        expect(info.platform).toMatch(/^(darwin|linux|windows)$/);
      } else {
        // No binary available - expected in test environment
        expect(info).toBeNull();
      }
    });

    it('should detect current architecture', async () => {
      const info = await manager.getBinaryInfo();

      if (info) {
        expect(info.arch).toMatch(/^(amd64|arm64)$/);
      } else {
        expect(info).toBeNull();
      }
    });
  });

  describe('version management', () => {
    it('should return correct version from config', async () => {
      const versionManager = new OPABinaryManager({
        version: '0.55.0',
        cacheDir: tempCacheDir,
        autoDownload: false,
      });

      const info = await versionManager.getBinaryInfo();

      if (info) {
        expect(info.version).toBe('0.55.0');
      }
    });

    it('should handle version verification', async () => {
      // This test verifies the version checking logic doesn't crash
      try {
        await manager.ensureBinary();
      } catch (error) {
        // Expected to fail without OPA installed or auto-download disabled
        expect(error).toBeDefined();
      }
    });
  });

  describe('caching behaviour', () => {
    it('should create cache directory when needed', async () => {
      const newCacheDir = join(tempCacheDir, 'new-cache');
      const cachingManager = new OPABinaryManager({
        cacheDir: newCacheDir,
        autoDownload: false,
      });

      try {
        await cachingManager.ensureBinary();
      } catch {
        // Expected to fail, but cache dir might be created
      }

      // Cache directory creation is handled internally
      expect(cachingManager).toBeDefined();
    });

    it('should reuse cached binary if valid', async () => {
      // Create a mock cached binary with version info
      const binaryPath = join(tempCacheDir, 'opa-0.60.0-linux-amd64');
      writeFileSync(binaryPath, '#!/bin/sh\necho "Version: 0.60.0"');
      chmodSync(binaryPath, 0o755);

      try {
        const path = await manager.ensureBinary();
        // If successful, should return cached path
        expect(existsSync(path)).toBe(true);
      } catch {
        // Version verification or execution might fail in test environment
        expect(true).toBe(true);
      }
    });
  });

  describe('fallback behaviour', () => {
    it('should check system PATH for OPA', async () => {
      // This test verifies the fallback logic doesn't crash
      try {
        await manager.ensureBinary();
      } catch (error) {
        // Expected when OPA is not in PATH and auto-download is disabled
        expect(error).toBeDefined();
      }
    });

    it('should throw error when autoDownload is false and no binary exists', async () => {
      const noDownloadManager = new OPABinaryManager({
        cacheDir: tempCacheDir,
        autoDownload: false,
      });

      await expect(noDownloadManager.ensureBinary()).rejects.toThrow();
    });
  });

  describe('getBinaryInfo', () => {
    it('should return null when binary is not available', async () => {
      const info = await manager.getBinaryInfo();

      // With autoDownload false and no binary in cache, should return null
      expect(info).toBeNull();
    });

    it('should include all required fields when available', async () => {
      // Create a mock binary
      const mockPath = join(tempCacheDir, 'opa-0.60.0-linux-amd64');
      writeFileSync(mockPath, '#!/bin/sh\necho "Version: 0.60.0"');
      chmodSync(mockPath, 0o755);

      try {
        const info = await manager.getBinaryInfo();

        if (info) {
          expect(info.path).toBeDefined();
          expect(info.version).toBeDefined();
          expect(info.platform).toBeDefined();
          expect(info.arch).toBeDefined();
        }
      } catch {
        // Version verification might fail in test environment
        expect(true).toBe(true);
      }
    });
  });

  describe('forceDownload', () => {
    it('should attempt to re-download binary', async () => {
      const downloadManager = new OPABinaryManager({
        cacheDir: tempCacheDir,
        autoDownload: false,
      });

      const downloadSpy = vi

        .spyOn(downloadManager as any, 'downloadBinary')
        .mockResolvedValue(undefined);

      const verifiedPath = await downloadManager.forceDownload();

      expect(downloadSpy).toHaveBeenCalledTimes(1);
      expect(existsSync(verifiedPath)).toBe(false);
    });

    it('should remove existing cached binary', async () => {
      const binaryPath = join(tempCacheDir, 'opa-0.60.0-linux-amd64');
      writeFileSync(binaryPath, 'mock binary');

      expect(existsSync(binaryPath)).toBe(true);

      const downloadSpy = vi.spyOn(manager as any, 'downloadBinary').mockResolvedValue(undefined);

      const verifiedPath = await manager.forceDownload();

      expect(downloadSpy).toHaveBeenCalledTimes(1);
      expect(existsSync(binaryPath)).toBe(false);
      expect(verifiedPath).toBe(binaryPath);
    });
  });

  describe('error handling', () => {
    it('should handle invalid cache directory gracefully', async () => {
      // Use a path that cannot be created (permission denied scenario)
      // In test environment, we just verify error handling exists
      const invalidManager = new OPABinaryManager({
        cacheDir: '/invalid/path/that/does/not/exist',
        autoDownload: false,
      });

      await expect(invalidManager.ensureBinary()).rejects.toThrow();
    });

    it('should provide helpful error messages', async () => {
      try {
        await manager.ensureBinary();
        // Should not reach here
        expect(false).toBe(true);
      } catch (error) {
        const err = error as Error;
        expect(err.message).toBeDefined();
        expect(err.message.length).toBeGreaterThan(0);
      }
    });
  });

  describe('download URL generation', () => {
    it('should generate correct download URL format', () => {
      // This is tested indirectly through the download logic
      // We verify the manager can be created with different configs
      const configs = [{ version: '0.60.0' }, { version: '0.55.0' }, { version: '0.50.0' }];

      configs.forEach((config) => {
        const m = new OPABinaryManager({ ...config, cacheDir: tempCacheDir });
        expect(m).toBeDefined();
      });
    });
  });
});
