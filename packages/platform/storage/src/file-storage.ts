/**
 * File Storage Provider
 *
 * Implements IStorageProvider for file system operations.
 */

import { promises as fs } from 'node:fs';
import { dirname } from 'node:path';
import type { IStorageProvider } from '@anvil/ports';

/**
 * File system storage provider
 */
export class FileStorage implements IStorageProvider {
  constructor(private readonly baseDir: string = process.cwd()) {}

  async read(path: string): Promise<string> {
    return fs.readFile(this.resolvePath(path), 'utf-8');
  }

  async readBuffer(path: string): Promise<Buffer> {
    return fs.readFile(this.resolvePath(path));
  }

  async write(path: string, content: string | Buffer): Promise<void> {
    const fullPath = this.resolvePath(path);
    await this.ensureDir(dirname(fullPath));
    await fs.writeFile(fullPath, content);
  }

  async exists(path: string): Promise<boolean> {
    try {
      await fs.access(this.resolvePath(path));
      return true;
    } catch {
      return false;
    }
  }

  async delete(path: string): Promise<void> {
    await fs.unlink(this.resolvePath(path));
  }

  async list(directory: string): Promise<string[]> {
    return fs.readdir(this.resolvePath(directory));
  }

  async mkdir(path: string, recursive = true): Promise<void> {
    await fs.mkdir(this.resolvePath(path), { recursive });
  }

  private resolvePath(path: string): string {
    if (path.startsWith('/')) {
      return path;
    }
    return `${this.baseDir}/${path}`;
  }

  private async ensureDir(dir: string): Promise<void> {
    await fs.mkdir(dir, { recursive: true });
  }
}

/**
 * Create a file storage provider
 */
export function createFileStorage(baseDir?: string): FileStorage {
  return new FileStorage(baseDir);
}
