import { existsSync } from 'node:fs';
import { isAbsolute, join, relative, resolve, sep } from 'node:path';
import { execFile } from 'node:child_process';
import type { Timestamp } from '../contracts/temporal.js';

export interface VersionEntry {
  hash: string;
  message: string;
  author: string;
  timestamp: Timestamp;
}

function parseAuthor(author: string): { name: string; email: string } {
  const openBracket = author.lastIndexOf('<');
  const closeBracket = author.lastIndexOf('>');
  if (openBracket !== -1 && closeBracket > openBracket) {
    const name = author.slice(0, openBracket).trim();
    const email = author.slice(openBracket + 1, closeBracket).trim();
    if (name && email) {
      return { name, email };
    }
  }

  return { name: author, email: `${author}@anvil.local` };
}

export class VersionTracker {
  private readonly storagePath: string;

  constructor(storagePath: string) {
    this.storagePath = storagePath;
  }

  async init(): Promise<void> {
    if (await this.isInitialised()) {
      return;
    }

    await this.runGit(['init']);
  }

  async trackChange(filePaths: string[], message: string, author: string): Promise<string> {
    if (!(await this.isInitialised())) {
      await this.init();
    }

    if (filePaths.length === 0) {
      throw new Error('Cannot track change without file paths');
    }

    const safePaths = filePaths.map((filePath) => this.resolveTrackedPath(filePath));

    await this.runGit(['add', ...safePaths]);
    const { name, email } = parseAuthor(author);
    const formattedAuthor = `${name} <${email}>`;
    await this.runGit([
      '-c',
      `user.name=${name}`,
      '-c',
      `user.email=${email}`,
      'commit',
      '-m',
      message,
      '--author',
      formattedAuthor,
    ]);
    const hash = await this.runGit(['rev-parse', 'HEAD']);
    return hash.trim();
  }

  async getHistory(filePath: string, limit = 20): Promise<VersionEntry[]> {
    if (!(await this.isInitialised())) {
      return [];
    }

    const safePath = this.resolveTrackedPath(filePath);
    const logOutput = await this.runGit([
      'log',
      `-n${Math.max(limit, 1)}`,
      '--format=%H%x1f%s%x1f%an%x1f%aI',
      '--',
      safePath,
    ]);

    if (!logOutput.trim()) {
      return [];
    }

    return logOutput
      .trim()
      .split('\n')
      .map((line) => {
        const [hash, message, author, timestamp] = line.split('\x1f');
        if (!hash || !message || !author || !timestamp) {
          return null;
        }

        return {
          hash,
          message,
          author,
          timestamp: timestamp as Timestamp,
        };
      })
      .filter((entry): entry is NonNullable<typeof entry> => entry !== null);
  }

  async getVersion(filePath: string, commitHash: string): Promise<string> {
    const safePath = this.resolveTrackedPath(filePath);
    return this.runGit(['show', `${commitHash}:${safePath}`]);
  }

  async isInitialised(): Promise<boolean> {
    return existsSync(join(this.storagePath, '.git'));
  }

  private resolveTrackedPath(filePath: string): string {
    if (filePath.trim().length === 0) {
      throw new Error('Git paths must not be empty');
    }

    if (isAbsolute(filePath)) {
      throw new Error(`Git path must be relative to storage root: ${filePath}`);
    }

    const pathSegments = filePath.split(/[\\/]+/);
    if (pathSegments.some((segment) => segment === '..')) {
      throw new Error(`Git path must not contain parent-directory traversal: ${filePath}`);
    }

    const resolvedStoragePath = resolve(this.storagePath);
    const resolvedTargetPath = resolve(resolvedStoragePath, filePath);

    if (
      resolvedTargetPath !== resolvedStoragePath &&
      !resolvedTargetPath.startsWith(resolvedStoragePath + sep)
    ) {
      throw new Error(`Git path escapes storage root: ${filePath}`);
    }

    const relativePath = relative(resolvedStoragePath, resolvedTargetPath);
    if (relativePath.length === 0) {
      throw new Error(`Git path must target a file inside storage root: ${filePath}`);
    }

    return relativePath.split(sep).join('/');
  }

  private runGit(args: string[]): Promise<string> {
    return new Promise((resolve, reject) => {
      execFile(
        'git',
        args,
        { cwd: this.storagePath, encoding: 'utf8', timeout: 30_000 },
        (error, stdout) => {
          if (error) {
            reject(
              new Error(`Git command failed (git ${args.join(' ')}): ${error.message}`, {
                cause: error,
              })
            );
            return;
          }
          resolve(typeof stdout === 'string' ? stdout : String(stdout ?? ''));
        }
      );
    });
  }
}
