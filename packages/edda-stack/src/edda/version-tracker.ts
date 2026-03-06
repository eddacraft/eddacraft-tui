import { existsSync } from 'node:fs';
import { join } from 'node:path';
import { execFile } from 'node:child_process';
import { promisify } from 'node:util';
import type { Timestamp } from '../contracts/temporal.js';

const execFileAsync = promisify(execFile);

export interface VersionEntry {
  hash: string;
  message: string;
  author: string;
  timestamp: Timestamp;
}

function normaliseAuthor(author: string): string {
  if (author.includes('<')) {
    return author;
  }

  return `${author} <${author}@anvil.local>`;
}

function parseAuthor(normalised: string): { name: string; email: string } {
  const match = normalised.match(/^(.+?)\s*<(.+?)>$/);
  if (match) {
    return { name: match[1].trim(), email: match[2] };
  }
  return { name: normalised, email: `${normalised}@anvil.local` };
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

    const normalisedAuthor = normaliseAuthor(author);
    const { name, email } = parseAuthor(normalisedAuthor);

    await this.runGit(['add', ...filePaths]);
    await this.runGit([
      '-c',
      `user.name=${name}`,
      '-c',
      `user.email=${email}`,
      'commit',
      '-m',
      message,
      '--author',
      normalisedAuthor,
    ]);
    const hash = await this.runGit(['rev-parse', 'HEAD']);
    return hash.trim();
  }

  async getHistory(filePath: string, limit = 20): Promise<VersionEntry[]> {
    if (!(await this.isInitialised())) {
      return [];
    }

    const logOutput = await this.runGit([
      'log',
      `-n${Math.max(limit, 1)}`,
      '--format=%H%x1f%s%x1f%an%x1f%aI',
      '--',
      filePath,
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
    return this.runGit(['show', `${commitHash}:${filePath}`]);
  }

  async isInitialised(): Promise<boolean> {
    return existsSync(join(this.storagePath, '.git'));
  }

  private async runGit(args: string[]): Promise<string> {
    try {
      const result = await execFileAsync('git', args, {
        cwd: this.storagePath,
        encoding: 'utf8',
      });

      if (typeof result === 'string') {
        return result;
      }

      if (result && typeof result === 'object' && 'stdout' in result) {
        const stdout = result.stdout;
        return typeof stdout === 'string' ? stdout : String(stdout ?? '');
      }

      return String(result ?? '');
    } catch (error) {
      const message = error instanceof Error ? error.message : 'Unknown git error';
      throw new Error(`Git command failed (git ${args.join(' ')}): ${message}`, {
        cause: error,
      });
    }
  }
}
