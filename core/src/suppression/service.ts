import * as fs from 'node:fs/promises';
import * as path from 'node:path';
import { parseSuppressions, findMatchingSuppression } from './parser.js';
import type { ParsedSuppression } from './parser.js';
import { SuppressionStore } from './store.js';
import type { Warning, Suppression } from '../antipattern/types.js';

export interface SuppressionStats {
  total: number;
  active: number;
  expired: number;
  appliedThisRun: number;
}

export interface FileSuppressions {
  file: string;
  suppressions: ParsedSuppression[];
}

export class SuppressionService {
  private store: SuppressionStore;
  private fileCache: Map<string, ParsedSuppression[]> = new Map();
  private workspaceRoot: string;

  constructor(workspaceRoot: string, store?: SuppressionStore) {
    this.workspaceRoot = workspaceRoot;
    const anvilDir = path.join(workspaceRoot, '.anvil');
    this.store = store ?? new SuppressionStore(anvilDir);
  }

  async initialize(): Promise<void> {
    await this.store.load();
  }

  async parseFileSuppressions(filePath: string): Promise<ParsedSuppression[]> {
    const cached = this.fileCache.get(filePath);
    if (cached) {
      return cached;
    }

    try {
      const absolutePath = path.isAbsolute(filePath)
        ? filePath
        : path.join(this.workspaceRoot, filePath);

      const content = await fs.readFile(absolutePath, 'utf-8');
      const result = parseSuppressions(content, filePath);

      this.fileCache.set(filePath, result.suppressions);
      return result.suppressions;
    } catch {
      return [];
    }
  }

  applyToWarnings(warnings: Warning[], filePath: string, now: Date = new Date()): Warning[] {
    const suppressions = this.fileCache.get(filePath) ?? [];

    return warnings.map((warning) => {
      if (warning.suppressed) {
        return warning;
      }

      if (warning.location.file !== filePath) {
        return warning;
      }

      const match = findMatchingSuppression(suppressions, warning.id, warning.location.line, now);

      if (!match) {
        return warning;
      }

      const suppression: Suppression = {
        reason: match.reason,
        scope: match.scope,
      };

      return {
        ...warning,
        suppressed: suppression,
      };
    });
  }

  async processFiles(files: string[]): Promise<FileSuppressions[]> {
    const results: FileSuppressions[] = [];

    for (const file of files) {
      const suppressions = await this.parseFileSuppressions(file);
      if (suppressions.length > 0) {
        results.push({ file, suppressions });
      }
    }

    return results;
  }

  applyToAllWarnings(warnings: Warning[], now: Date = new Date()): Warning[] {
    const fileGroups = new Map<string, Warning[]>();

    for (const warning of warnings) {
      const file = warning.location.file;
      const group = fileGroups.get(file) ?? [];
      group.push(warning);
      fileGroups.set(file, group);
    }

    const result: Warning[] = [];

    for (const [file, fileWarnings] of fileGroups) {
      const processed = this.applyToWarnings(fileWarnings, file, now);
      result.push(...processed);
    }

    return result;
  }

  getStats(warnings: Warning[], now: Date = new Date()): SuppressionStats {
    const allSuppressions: ParsedSuppression[] = [];
    for (const suppressions of this.fileCache.values()) {
      allSuppressions.push(...suppressions);
    }

    let expired = 0;
    let active = 0;

    for (const s of allSuppressions) {
      if (s.expiresAt && s.expiresAt < now) {
        expired++;
      } else {
        active++;
      }
    }

    const appliedThisRun = warnings.filter((w) => w.suppressed).length;

    return {
      total: allSuppressions.length,
      active,
      expired,
      appliedThisRun,
    };
  }

  clearCache(): void {
    this.fileCache.clear();
  }

  getStore(): SuppressionStore {
    return this.store;
  }
}
