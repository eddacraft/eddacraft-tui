import {
  existsSync,
  mkdirSync,
  readFileSync,
  writeFileSync,
  renameSync,
  unlinkSync,
} from 'node:fs';
import { randomBytes } from 'node:crypto';
import { join } from 'node:path';
import { ProvenanceRecordSchema, ProvenanceIndexSchema } from './types.js';
import type { ProvenanceRecord, ProvenanceIndex } from './types.js';
import { createDebugger } from '../utils/debug.js';
import { sanitizeIdentifier } from '../utils/path-safety.js';

const debug = createDebugger('provenance');

const PROVENANCE_DIR = '.anvil';
const HISTORY_DIR = 'history';
const INDEX_FILE = 'index.json';
const MAX_RECORDS = 1000; // Maximum records to keep in history

/**
 * ProvenanceStore - manages provenance records on disk
 */
export class ProvenanceStore {
  private readonly baseDir: string;
  private readonly historyDir: string;
  private readonly indexPath: string;

  constructor(workspaceRoot: string) {
    this.baseDir = join(workspaceRoot, PROVENANCE_DIR);
    this.historyDir = join(this.baseDir, HISTORY_DIR);
    this.indexPath = join(this.historyDir, INDEX_FILE);
  }

  /**
   * Ensures the provenance directories exist
   */
  private ensureDirectories(): void {
    mkdirSync(this.historyDir, { recursive: true });

    // Create .gitignore in .anvil if it doesn't exist (atomic create-or-skip)
    const gitignorePath = join(this.baseDir, '.gitignore');
    try {
      writeFileSync(
        gitignorePath,
        `# anvil local data
history/
*.local.json
`,
        { flag: 'wx' }
      );
    } catch (err) {
      if ((err as NodeJS.ErrnoException).code !== 'EEXIST') throw err;
    }
  }

  /**
   * Loads the provenance index
   */
  private createEmptyIndex(): ProvenanceIndex {
    return {
      version: 1,
      last_updated: new Date().toISOString(),
      records: [],
      statistics: {
        total_checks: 0,
        total_passed: 0,
        total_failed: 0,
      },
    };
  }

  private loadIndex(): ProvenanceIndex {
    if (!existsSync(this.indexPath)) {
      return this.createEmptyIndex();
    }

    try {
      const content = readFileSync(this.indexPath, 'utf-8');
      const parsed = JSON.parse(content);
      const result = ProvenanceIndexSchema.safeParse(parsed);

      if (!result.success) {
        debug('Index validation failed, starting fresh', result.error);
        return this.createEmptyIndex();
      }

      return result.data;
    } catch (error) {
      debug('Failed to load provenance index', error);
      return this.createEmptyIndex();
    }
  }

  /**
   * Saves the provenance index
   */
  private saveIndex(index: ProvenanceIndex): void {
    this.ensureDirectories();
    index.last_updated = new Date().toISOString();
    const tmpIndex = `${this.indexPath}.${randomBytes(6).toString('hex')}.tmp`;
    try {
      writeFileSync(tmpIndex, JSON.stringify(index, null, 2));
      renameSync(tmpIndex, this.indexPath);
    } catch (err) {
      try {
        unlinkSync(tmpIndex);
      } catch {
        /* tmp already gone */
      }
      throw err;
    }
  }

  /**
   * Saves a provenance record
   */
  save(record: ProvenanceRecord): void {
    this.ensureDirectories();

    // Save the full record
    const safeId = sanitizeIdentifier(record.id);
    const recordPath = join(this.historyDir, `${safeId}.json`);
    const tmpRecord = `${recordPath}.${randomBytes(6).toString('hex')}.tmp`;
    try {
      writeFileSync(tmpRecord, JSON.stringify(record, null, 2));
      renameSync(tmpRecord, recordPath);
    } catch (err) {
      try {
        unlinkSync(tmpRecord);
      } catch {
        /* tmp already gone */
      }
      throw err;
    }

    // Update the index
    const index = this.loadIndex();

    // Add to records (prepend for most recent first)
    index.records.unshift({
      id: record.id,
      timestamp: record.timestamp,
      passed: record.overall_passed,
      scope: record.scope,
      files_count: record.files_count,
      commit: record.git?.commit?.substring(0, 8),
    });

    if (index.records.length > MAX_RECORDS) {
      index.records.splice(MAX_RECORDS);
    }

    // Update statistics
    index.statistics.total_checks++;
    if (record.overall_passed) {
      index.statistics.total_passed++;
      index.statistics.last_pass = record.timestamp;
    } else {
      index.statistics.total_failed++;
      index.statistics.last_fail = record.timestamp;
    }

    this.saveIndex(index);
  }

  /**
   * Gets a specific provenance record by ID
   */
  get(id: string): ProvenanceRecord | null {
    const safeId = sanitizeIdentifier(id);
    const recordPath = join(this.historyDir, `${safeId}.json`);
    if (!existsSync(recordPath)) {
      return null;
    }

    try {
      const content = readFileSync(recordPath, 'utf-8');
      const parsed = JSON.parse(content);
      const result = ProvenanceRecordSchema.safeParse(parsed);

      if (!result.success) {
        debug(`Record ${id} validation failed`, result.error);
        return null;
      }

      return result.data;
    } catch (error) {
      debug(`Failed to load provenance record ${id}`, error);
      return null;
    }
  }

  /**
   * Gets the most recent provenance record
   */
  getLatest(): ProvenanceRecord | null {
    const index = this.loadIndex();
    if (index.records.length === 0) {
      return null;
    }
    return this.get(index.records[0].id);
  }

  /**
   * Gets the provenance index with statistics
   */
  getIndex(): ProvenanceIndex {
    return this.loadIndex();
  }

  /**
   * Lists recent provenance records
   */
  list(options?: { limit?: number; passed?: boolean; since?: Date }): ProvenanceRecord[] {
    const index = this.loadIndex();
    let records = index.records;

    // Filter by passed/failed
    if (options?.passed !== undefined) {
      records = records.filter((r) => r.passed === options.passed);
    }

    // Filter by date
    if (options?.since) {
      const sinceTime = options.since.getTime();
      records = records.filter((r) => new Date(r.timestamp).getTime() >= sinceTime);
    }

    // Apply limit
    const limit = options?.limit || 10;
    records = records.slice(0, limit);

    // Load full records
    return records.map((r) => this.get(r.id)).filter((r): r is ProvenanceRecord => r !== null);
  }

  /**
   * Finds records by git commit
   */
  findByCommit(commitPrefix: string): ProvenanceRecord[] {
    const index = this.loadIndex();
    const matching = index.records.filter((r) => r.commit?.startsWith(commitPrefix));
    return matching.map((r) => this.get(r.id)).filter((r): r is ProvenanceRecord => r !== null);
  }

  /**
   * Gets statistics about provenance history
   */
  getStatistics(): {
    total: number;
    passed: number;
    failed: number;
    passRate: number;
    lastCheck: string | null;
    lastPass: string | null;
    lastFail: string | null;
  } {
    const index = this.loadIndex();
    const stats = index.statistics;
    const lastRecord = index.records[0];

    return {
      total: stats.total_checks,
      passed: stats.total_passed,
      failed: stats.total_failed,
      passRate: stats.total_checks > 0 ? (stats.total_passed / stats.total_checks) * 100 : 0,
      lastCheck: lastRecord?.timestamp || null,
      lastPass: stats.last_pass || null,
      lastFail: stats.last_fail || null,
    };
  }

  /**
   * Exports provenance history as JSON
   */
  export(options?: { since?: Date; limit?: number }): string {
    const records = this.list(options);
    const stats = this.getStatistics();

    return JSON.stringify(
      {
        exported_at: new Date().toISOString(),
        statistics: stats,
        records,
      },
      null,
      2
    );
  }

  /**
   * Clears all provenance history
   */
  clear(): void {
    const index = this.loadIndex();

    for (const record of index.records) {
      const safeId = sanitizeIdentifier(record.id);
      const recordPath = join(this.historyDir, `${safeId}.json`);
      try {
        if (existsSync(recordPath)) {
          unlinkSync(recordPath);
        }
      } catch (error) {
        debug(`Failed to delete record file ${record.id}`, error);
      }
    }

    this.saveIndex(this.createEmptyIndex());
  }

  /**
   * Checks if provenance tracking is initialised
   */
  isInitialised(): boolean {
    return existsSync(this.historyDir);
  }
}

/**
 * Creates a provenance store for the given workspace
 */
export function createProvenanceStore(workspaceRoot: string): ProvenanceStore {
  return new ProvenanceStore(workspaceRoot);
}
