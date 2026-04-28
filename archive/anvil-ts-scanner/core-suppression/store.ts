import { z } from 'zod';
import * as fs from 'node:fs/promises';
import * as path from 'node:path';
import { randomBytes } from 'node:crypto';
import { SuppressionRecordSchema } from '../antipattern/types.js';
import type { ParsedSuppression } from './parser.js';
import { createDebugger } from '../utils/debug.js';

const debug = createDebugger('suppression');

export type SuppressionRecord = z.infer<typeof SuppressionRecordSchema>;

export const SuppressionStoreDataSchema = z.object({
  version: z.literal(1),
  suppressions: z.array(SuppressionRecordSchema),
  lastUpdated: z.string().datetime(),
});

export type SuppressionStoreData = z.infer<typeof SuppressionStoreDataSchema>;

export interface SuppressionMatch {
  record: SuppressionRecord;
  isExpired: boolean;
}

function generateSuppressionId(file: string, line: number, patternId: string): string {
  return `${file}:${line}:${patternId}`;
}

export class SuppressionStore {
  private data: SuppressionStoreData;
  private filePath: string;
  private loaded = false;

  constructor(anvilDir: string) {
    this.filePath = path.join(anvilDir, 'suppressions.json');
    this.data = {
      version: 1,
      suppressions: [],
      lastUpdated: new Date().toISOString(),
    };
  }

  async load(): Promise<void> {
    debug('loading suppression store from', this.filePath);
    try {
      const content = await fs.readFile(this.filePath, 'utf-8');
      const parsed = JSON.parse(content);
      const result = SuppressionStoreDataSchema.safeParse(parsed);

      if (result.success) {
        debug('suppression store loaded', { count: result.data.suppressions.length });
        this.data = result.data;
      } else {
        this.data = {
          version: 1,
          suppressions: [],
          lastUpdated: new Date().toISOString(),
        };
      }
    } catch {
      this.data = {
        version: 1,
        suppressions: [],
        lastUpdated: new Date().toISOString(),
      };
    }

    this.loaded = true;
  }

  async save(): Promise<void> {
    debug('saving suppression store', { count: this.data.suppressions.length });
    this.data.lastUpdated = new Date().toISOString();

    const dir = path.dirname(this.filePath);
    await fs.mkdir(dir, { recursive: true });
    const tmpPath = `${this.filePath}.${randomBytes(6).toString('hex')}.tmp`;
    try {
      await fs.writeFile(tmpPath, JSON.stringify(this.data, null, 2), 'utf-8');
      await fs.rename(tmpPath, this.filePath);
    } catch (err) {
      try {
        await fs.unlink(tmpPath);
      } catch {
        /* tmp already gone */
      }
      throw err;
    }
  }

  add(record: SuppressionRecord): void {
    const existingIndex = this.data.suppressions.findIndex((s) => s.id === record.id);

    if (existingIndex >= 0) {
      this.data.suppressions[existingIndex] = record;
    } else {
      this.data.suppressions.push(record);
    }
  }

  remove(id: string): boolean {
    const index = this.data.suppressions.findIndex((s) => s.id === id);
    if (index >= 0) {
      this.data.suppressions.splice(index, 1);
      return true;
    }
    return false;
  }

  /**
   * Check if a warning is suppressed at the given location.
   * Returns null if no matching suppression OR if the matching suppression is expired.
   * Use findSuppressionMatch() if you need to know about expired matches.
   */
  isSuppressed(
    warningId: string,
    file: string,
    line: number,
    now: Date = new Date()
  ): SuppressionMatch | null {
    const match = this.findSuppressionMatch(warningId, file, line, now);

    // Per acceptance criteria: expired suppressions should NOT suppress warnings
    if (match && match.isExpired) {
      return null;
    }

    return match;
  }

  /**
   * Find a suppression match, including expired ones.
   * Use this when you need to report on expired suppressions.
   */
  findSuppressionMatch(
    warningId: string,
    file: string,
    line: number,
    now: Date = new Date()
  ): SuppressionMatch | null {
    for (const record of this.data.suppressions) {
      if (record.file !== file) continue;
      if (record.pattern_id !== warningId) continue;

      const isExpired = this.isRecordExpired(record, now);
      const matches = this.matchesScope(record, line);

      if (matches) {
        return { record, isExpired };
      }
    }

    return null;
  }

  private matchesScope(record: SuppressionRecord, warningLine: number): boolean {
    switch (record.scope) {
      case 'file':
        return true;
      case 'line':
        return record.line === warningLine;
      case 'statement':
        return warningLine === record.line + 1;
      case 'import':
        return warningLine === record.line + 1;
      default:
        return false;
    }
  }

  private isRecordExpired(record: SuppressionRecord, now: Date): boolean {
    if (!record.expires_at) {
      return false;
    }

    const expiresAt = new Date(record.expires_at);
    return expiresAt < now;
  }

  getAll(): SuppressionRecord[] {
    return [...this.data.suppressions];
  }

  getByFile(file: string): SuppressionRecord[] {
    return this.data.suppressions.filter((s) => s.file === file);
  }

  getExpired(now: Date = new Date()): SuppressionRecord[] {
    return this.data.suppressions.filter((record) => this.isRecordExpired(record, now));
  }

  pruneExpired(now: Date = new Date()): number {
    const before = this.data.suppressions.length;
    this.data.suppressions = this.data.suppressions.filter(
      (record) => !this.isRecordExpired(record, now)
    );
    return before - this.data.suppressions.length;
  }

  createRecordFromParsed(
    parsed: ParsedSuppression,
    file: string,
    gitCommit?: string
  ): SuppressionRecord {
    const id = generateSuppressionId(file, parsed.line, parsed.warningId);

    const record: SuppressionRecord = {
      id,
      pattern_id: parsed.warningId,
      file,
      line: parsed.line,
      reason: parsed.reason,
      timestamp: new Date().toISOString(),
      scope: parsed.scope,
    };

    if (gitCommit) {
      record.commit = gitCommit;
    }

    if (parsed.expiresAt) {
      record.expires_at = parsed.expiresAt.toISOString();
    }

    return record;
  }

  get isLoaded(): boolean {
    return this.loaded;
  }

  get count(): number {
    return this.data.suppressions.length;
  }
}
