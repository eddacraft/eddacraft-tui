/**
 * Tests for FileStorage - security-critical path traversal validation
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { promises as fs } from 'node:fs';
import { join } from 'node:path';
import { mkdtempSync, rmSync, writeFileSync, mkdirSync, symlinkSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { FileStorage, createFileStorage } from './file-storage.js';

describe('FileStorage', () => {
  let tempDir: string;
  let storage: FileStorage;

  beforeEach(() => {
    tempDir = mkdtempSync(join(tmpdir(), 'filestorage-test-'));
    storage = new FileStorage(tempDir);
  });

  afterEach(() => {
    rmSync(tempDir, { recursive: true, force: true });
  });

  describe('path traversal prevention (security-critical)', () => {
    it('should reject ../../../etc/passwd traversal', async () => {
      await expect(storage.read('../../../etc/passwd')).rejects.toThrow(
        /Path escapes base directory/
      );
    });

    it('should reject simple ../ traversal', async () => {
      await expect(storage.read('../secret')).rejects.toThrow(/Path escapes base directory/);
    });

    it('should reject deeply nested traversal', async () => {
      await expect(storage.read('a/b/../../../../etc/passwd')).rejects.toThrow(
        /Path escapes base directory/
      );
    });

    it('should reject absolute paths outside base directory', async () => {
      await expect(storage.read('/etc/passwd')).rejects.toThrow(/Path escapes base directory/);
    });

    it('should handle Windows-style backslash paths', async () => {
      // On POSIX, backslashes are literal filename characters (not separators),
      // so '..\\..\\' does NOT traverse directories — it's just a weird filename.
      // On Windows, path.resolve would interpret backslashes as separators and
      // the traversal check would correctly reject it.
      if (process.platform === 'win32') {
        await expect(storage.read('..\\..\\etc\\passwd')).rejects.toThrow(
          /Path escapes base directory/
        );
      } else {
        // On POSIX, this resolves within the base dir (as a literal filename)
        // so it throws ENOENT, not a path-escape error
        await expect(storage.read('..\\..\\etc\\passwd')).rejects.toThrow();
      }
    });

    it('should reject traversal via write', async () => {
      await expect(storage.write('../escape.txt', 'data')).rejects.toThrow(
        /Path escapes base directory/
      );
    });

    it('should reject traversal via delete', async () => {
      await expect(storage.delete('../escape.txt')).rejects.toThrow(/Path escapes base directory/);
    });

    it('should reject traversal via exists', async () => {
      // Note: exists() wraps resolvePath() in a try-catch and returns false
      // on any error, including path traversal. This means traversal attempts
      // return false rather than throwing. The path is still blocked from
      // reaching the filesystem.
      const result = await storage.exists('../escape.txt');
      expect(result).toBe(false);
    });

    it('should reject traversal via list', async () => {
      await expect(storage.list('../')).rejects.toThrow(/Path escapes base directory/);
    });

    it('should reject traversal via mkdir', async () => {
      await expect(storage.mkdir('../escape')).rejects.toThrow(/Path escapes base directory/);
    });

    it('should reject traversal via readBuffer', async () => {
      await expect(storage.readBuffer('../secret')).rejects.toThrow(/Path escapes base directory/);
    });

    it('should allow the base directory itself as a path', async () => {
      // Listing the base directory should work (empty path resolves to base)
      const files = await storage.list('.');
      expect(Array.isArray(files)).toBe(true);
    });
  });

  describe.skipIf(process.platform === 'win32')('symlink escape prevention', () => {
    let outsideDir: string;

    beforeEach(() => {
      outsideDir = mkdtempSync(join(tmpdir(), 'filestorage-outside-'));
      writeFileSync(join(outsideDir, 'secret.txt'), 'sensitive data');
    });

    afterEach(() => {
      rmSync(outsideDir, { recursive: true, force: true });
    });

    it('should reject read through symlink file pointing outside base', async () => {
      symlinkSync(join(outsideDir, 'secret.txt'), join(tempDir, 'escape-link'));

      await expect(storage.read('escape-link')).rejects.toThrow(/Path escapes base directory/);
    });

    it('should reject read through symlink with absolute target outside base', async () => {
      symlinkSync(outsideDir, join(tempDir, 'outside-dir-link'));

      await expect(storage.read('outside-dir-link/secret.txt')).rejects.toThrow(
        /Path escapes base directory/
      );
    });

    it('should reject read through symlinked intermediate directory', async () => {
      mkdirSync(join(tempDir, 'legit'));
      symlinkSync(outsideDir, join(tempDir, 'legit', 'escape'));

      await expect(storage.read('legit/escape/secret.txt')).rejects.toThrow(
        /Path escapes base directory/
      );
    });

    it('should reject write through symlink pointing outside base', async () => {
      symlinkSync(join(outsideDir, 'secret.txt'), join(tempDir, 'write-link'));

      await expect(storage.write('write-link', 'overwritten')).rejects.toThrow(
        /Path escapes base directory/
      );
      // Verify original file was not modified
      const content = await fs.readFile(join(outsideDir, 'secret.txt'), 'utf-8');
      expect(content).toBe('sensitive data');
    });

    it('should reject delete through symlink pointing outside base', async () => {
      symlinkSync(join(outsideDir, 'secret.txt'), join(tempDir, 'delete-link'));

      await expect(storage.delete('delete-link')).rejects.toThrow(/Path escapes base directory/);
    });

    it('should allow symlink that points within the base directory', async () => {
      await fs.writeFile(join(tempDir, 'real-file.txt'), 'safe content');
      symlinkSync(join(tempDir, 'real-file.txt'), join(tempDir, 'internal-link'));

      const content = await storage.read('internal-link');
      expect(content).toBe('safe content');
    });
  });

  describe('read', () => {
    it('should read file content as string', async () => {
      await fs.writeFile(join(tempDir, 'test.txt'), 'hello world');
      const content = await storage.read('test.txt');
      expect(content).toBe('hello world');
    });

    it('should read file in subdirectory', async () => {
      await fs.mkdir(join(tempDir, 'sub'), { recursive: true });
      await fs.writeFile(join(tempDir, 'sub', 'file.txt'), 'nested');
      const content = await storage.read('sub/file.txt');
      expect(content).toBe('nested');
    });

    it('should throw on non-existent file', async () => {
      await expect(storage.read('nonexistent.txt')).rejects.toThrow();
    });
  });

  describe('readBuffer', () => {
    it('should read file as Buffer', async () => {
      const data = Buffer.from([0x00, 0x01, 0x02, 0xff]);
      await fs.writeFile(join(tempDir, 'binary.dat'), data);
      const buffer = await storage.readBuffer('binary.dat');
      expect(Buffer.isBuffer(buffer)).toBe(true);
      expect(buffer).toEqual(data);
    });
  });

  describe('write', () => {
    it('should write string content to file', async () => {
      await storage.write('output.txt', 'test content');
      const content = await fs.readFile(join(tempDir, 'output.txt'), 'utf-8');
      expect(content).toBe('test content');
    });

    it('should write Buffer content to file', async () => {
      const data = Buffer.from('binary content');
      await storage.write('binary.dat', data);
      const content = await fs.readFile(join(tempDir, 'binary.dat'));
      expect(content).toEqual(data);
    });

    it('should create parent directories automatically', async () => {
      await storage.write('deep/nested/dir/file.txt', 'deep');
      const content = await fs.readFile(join(tempDir, 'deep/nested/dir/file.txt'), 'utf-8');
      expect(content).toBe('deep');
    });

    it('should overwrite existing file', async () => {
      await storage.write('file.txt', 'first');
      await storage.write('file.txt', 'second');
      const content = await fs.readFile(join(tempDir, 'file.txt'), 'utf-8');
      expect(content).toBe('second');
    });
  });

  describe('exists', () => {
    it('should return true for existing file', async () => {
      await fs.writeFile(join(tempDir, 'exists.txt'), '');
      expect(await storage.exists('exists.txt')).toBe(true);
    });

    it('should return false for non-existent file', async () => {
      expect(await storage.exists('nope.txt')).toBe(false);
    });

    it('should return true for existing directory', async () => {
      await fs.mkdir(join(tempDir, 'subdir'));
      expect(await storage.exists('subdir')).toBe(true);
    });
  });

  describe('delete', () => {
    it('should delete existing file', async () => {
      await fs.writeFile(join(tempDir, 'delete-me.txt'), 'bye');
      await storage.delete('delete-me.txt');
      await expect(fs.access(join(tempDir, 'delete-me.txt'))).rejects.toThrow();
    });

    it('should throw when deleting non-existent file', async () => {
      await expect(storage.delete('nonexistent.txt')).rejects.toThrow();
    });
  });

  describe('list', () => {
    it('should list files in directory', async () => {
      await fs.writeFile(join(tempDir, 'a.txt'), '');
      await fs.writeFile(join(tempDir, 'b.txt'), '');
      const files = await storage.list('.');
      expect(files).toContain('a.txt');
      expect(files).toContain('b.txt');
    });

    it('should list files in subdirectory', async () => {
      await fs.mkdir(join(tempDir, 'sub'));
      await fs.writeFile(join(tempDir, 'sub', 'c.txt'), '');
      const files = await storage.list('sub');
      expect(files).toEqual(['c.txt']);
    });

    it('should return empty array for empty directory', async () => {
      await fs.mkdir(join(tempDir, 'empty'));
      const files = await storage.list('empty');
      expect(files).toEqual([]);
    });
  });

  describe('mkdir', () => {
    it('should create directory', async () => {
      await storage.mkdir('newdir');
      const stat = await fs.stat(join(tempDir, 'newdir'));
      expect(stat.isDirectory()).toBe(true);
    });

    it('should create nested directories recursively', async () => {
      await storage.mkdir('a/b/c');
      const stat = await fs.stat(join(tempDir, 'a/b/c'));
      expect(stat.isDirectory()).toBe(true);
    });

    it('should not throw if directory already exists', async () => {
      await storage.mkdir('existing');
      await expect(storage.mkdir('existing')).resolves.not.toThrow();
    });
  });

  describe('createFileStorage factory', () => {
    it('should create a FileStorage instance', () => {
      const fs = createFileStorage(tempDir);
      expect(fs).toBeInstanceOf(FileStorage);
    });

    it('should default to cwd when no baseDir provided', () => {
      const fs = createFileStorage();
      expect(fs).toBeInstanceOf(FileStorage);
    });
  });
});
