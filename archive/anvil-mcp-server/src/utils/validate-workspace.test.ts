import { describe, it, expect, afterEach } from 'vitest';
import { mkdirSync, mkdtempSync, symlinkSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { tmpdir } from 'node:os';
import { safeCleanup } from '../../../../tools/test-utils/safe-cleanup.js';
import { validateWorkspaceRoot, validateWorkspaceRootAgainstServer } from './validate-workspace.js';

describe('validateWorkspaceRoot', () => {
  let tmpDir: string;

  afterEach(async () => {
    if (tmpDir) {
      await safeCleanup(tmpDir);
    }
  });

  it('accepts an absolute path to an existing directory', () => {
    tmpDir = mkdtempSync(join(tmpdir(), 'anvil-vw-'));
    const result = validateWorkspaceRoot(tmpDir);
    expect(result).toBe(tmpDir);
  });

  it('rejects a relative path', () => {
    expect(() => validateWorkspaceRoot('relative/path')).toThrow(
      'workspaceRoot must be an absolute path'
    );
  });

  it('rejects a non-existent path', () => {
    expect(() => validateWorkspaceRoot('/nonexistent/path/12345')).toThrow(
      'workspaceRoot does not exist'
    );
  });

  it('rejects a file path (not a directory)', () => {
    tmpDir = mkdtempSync(join(tmpdir(), 'anvil-vw-'));
    const filePath = join(tmpDir, 'file.txt');
    writeFileSync(filePath, 'content');
    expect(() => validateWorkspaceRoot(filePath)).toThrow('workspaceRoot is not a directory');
  });
});

describe('validateWorkspaceRootAgainstServer', () => {
  let tmpDir: string;

  afterEach(async () => {
    if (tmpDir) {
      await safeCleanup(tmpDir);
    }
  });

  it('passes through when serverRoot is undefined (stdio transport)', () => {
    tmpDir = mkdtempSync(join(tmpdir(), 'anvil-vw-'));
    const result = validateWorkspaceRootAgainstServer(tmpDir, undefined);
    expect(result).toBe(tmpDir);
  });

  it('accepts client path that equals the server root', () => {
    tmpDir = mkdtempSync(join(tmpdir(), 'anvil-vw-'));
    const result = validateWorkspaceRootAgainstServer(tmpDir, tmpDir);
    expect(result).toBe(tmpDir);
  });

  it('accepts a subdirectory of the server root', () => {
    tmpDir = mkdtempSync(join(tmpdir(), 'anvil-vw-'));
    const sub = join(tmpDir, 'project');
    mkdirSync(sub);
    const result = validateWorkspaceRootAgainstServer(sub, tmpDir);
    expect(result).toBe(sub);
  });

  it('rejects a path outside the server root', async () => {
    tmpDir = mkdtempSync(join(tmpdir(), 'anvil-vw-'));
    const outside = mkdtempSync(join(tmpdir(), 'anvil-vw-outside-'));
    try {
      expect(() => validateWorkspaceRootAgainstServer(outside, tmpDir)).toThrow(
        "outside the server's allowed root"
      );
    } finally {
      await safeCleanup(outside);
    }
  });

  it('rejects a path that uses .. to escape the server root', () => {
    tmpDir = mkdtempSync(join(tmpdir(), 'anvil-vw-'));
    const sub = join(tmpDir, 'a');
    mkdirSync(sub);
    // Try to reference tmpDir's parent via sub/../..
    const escapePath = join(sub, '..', '..');
    // This resolves to tmpDir's parent, which should be rejected.
    // validateWorkspaceRoot will resolve it to an absolute path.
    expect(() => validateWorkspaceRootAgainstServer(escapePath, tmpDir)).toThrow(
      "outside the server's allowed root"
    );
  });

  it('rejects symlink escape attempts', () => {
    tmpDir = mkdtempSync(join(tmpdir(), 'anvil-vw-'));
    const serverDir = join(tmpDir, 'server');
    const outsideDir = join(tmpDir, 'outside');
    const symlinkDir = join(serverDir, 'escape');

    mkdirSync(serverDir);
    mkdirSync(outsideDir);
    symlinkSync(outsideDir, symlinkDir);

    expect(() => validateWorkspaceRootAgainstServer(symlinkDir, serverDir)).toThrow(
      "outside the server's allowed root"
    );
  });
});
