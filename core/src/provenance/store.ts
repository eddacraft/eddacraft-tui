import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'fs';
import { join } from 'path';
import type { ProvenanceRecord, ProvenanceIndex } from './types.js';

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
    if (!existsSync(this.baseDir)) {
      mkdirSync(this.baseDir, { recursive: true });
    }
    if (!existsSync(this.historyDir)) {
      mkdirSync(this.historyDir, { recursive: true });
    }

    // Create .gitignore in .anvil if it doesn't exist
    const gitignorePath = join(this.baseDir, '.gitignore');
    if (!existsSync(gitignorePath)) {
      writeFileSync(
        gitignorePath,
        `# Anvil local data
history/
*.local.json
`
      );
    }
  }

  /**
   * Loads the provenance index
   */
  private loadIndex(): ProvenanceIndex {
    if (!existsSync(this.indexPath)) {
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

    try {
      const content = readFileSync(this.indexPath, 'utf-8');
      return JSON.parse(content) as ProvenanceIndex;
    } catch (error) {
      // Corrupted or missing index, start fresh
      console.warn('[ProvenanceStore] Failed to load index, starting fresh:', error);
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
  }

  /**
   * Saves the provenance index
   */
  private saveIndex(index: ProvenanceIndex): void {
    this.ensureDirectories();
    index.last_updated = new Date().toISOString();
    writeFileSync(this.indexPath, JSON.stringify(index, null, 2));
  }

  /**
   * Saves a provenance record
   */
  save(record: ProvenanceRecord): void {
    this.ensureDirectories();

    // Save the full record
    const recordPath = join(this.historyDir, `${record.id}.json`);
    writeFileSync(recordPath, JSON.stringify(record, null, 2));

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

    // Trim old records if over limit
    if (index.records.length > MAX_RECORDS) {
      const removedRecords = index.records.splice(MAX_RECORDS);
      // Optionally delete old record files
      for (const removed of removedRecords) {
        const oldPath = join(this.historyDir, `${removed.id}.json`);
        try {
          if (existsSync(oldPath)) {
            // We'll keep files but remove from index for now
            // Could add cleanup option later
          }
        } catch (error) {
          console.error('[ProvenanceStore] Cleanup error for old record:', error);
        }
      }
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
    const recordPath = join(this.historyDir, `${id}.json`);
    if (!existsSync(recordPath)) {
      return null;
    }

    try {
      const content = readFileSync(recordPath, 'utf-8');
      return JSON.parse(content) as ProvenanceRecord;
    } catch (error) {
      console.error(`[ProvenanceStore] Failed to load record ${id}:`, error);
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

    // Delete all record files
    for (const record of index.records) {
      const recordPath = join(this.historyDir, `${record.id}.json`);
      try {
        if (existsSync(recordPath)) {
          // Using unlinkSync would be cleaner but keeping simple
          writeFileSync(recordPath, ''); // Truncate
        }
      } catch (error) {
        console.error(`[ProvenanceStore] Failed to clear record ${record.id}:`, error);
      }
    }

    // Reset index
    this.saveIndex({
      version: 1,
      last_updated: new Date().toISOString(),
      records: [],
      statistics: {
        total_checks: 0,
        total_passed: 0,
        total_failed: 0,
      },
    });
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
