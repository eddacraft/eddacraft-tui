/**
 * File Storage Provider
 *
 * Implements IStorageProvider for file system operations.
 */

import { promises as fs, realpathSync } from 'node:fs';
import { dirname, resolve, sep } from 'node:path';
import type { IStorageProvider } from '@eddacraft/anvil-ports';

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

  private resolvePath(filePath: string): string {
    const resolvedBase = resolve(this.baseDir);
    const resolvedTarget = resolve(resolvedBase, filePath);

    // Lexical check first (catches ../ traversal before the file exists)
    if (resolvedTarget !== resolvedBase && !resolvedTarget.startsWith(resolvedBase + sep)) {
      throw new Error(`Path escapes base directory: ${filePath}`);
    }

    // Symlink check: canonicalise and re-verify to catch symlink escapes
    try {
      const realBase = realpathSync(resolvedBase);
      const realTarget = realpathSync(resolvedTarget);
      if (realTarget !== realBase && !realTarget.startsWith(realBase + sep)) {
        throw new Error(`Path escapes base directory (symlink): ${filePath}`);
      }
    } catch (err) {
      if (
        err instanceof Error &&
        'code' in err &&
        (err as NodeJS.ErrnoException).code === 'ENOENT'
      ) {
        // Target doesn't exist yet (common for writes). Canonicalise the
        // nearest existing parent to catch symlinked intermediate directories.
        const realBase = realpathSync(resolvedBase);
        let parent = resolvedTarget;
        // Walk up until we find an existing directory
        while (parent !== resolvedBase) {
          parent = dirname(parent);
          try {
            const realParent = realpathSync(parent);
            if (realParent !== realBase && !realParent.startsWith(realBase + sep)) {
              throw new Error(`Path escapes base directory (symlink): ${filePath}`);
            }
            break; // found existing parent, it's inside base — OK
          } catch (parentErr) {
            if (
              parentErr instanceof Error &&
              'code' in parentErr &&
              (parentErr as NodeJS.ErrnoException).code === 'ENOENT'
            ) {
              continue; // parent doesn't exist either, keep walking up
            }
            throw parentErr;
          }
        }
        return resolvedTarget;
      }
      throw err;
    }

    return resolvedTarget;
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
