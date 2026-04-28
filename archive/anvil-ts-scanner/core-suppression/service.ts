import * as fs from 'node:fs/promises';
import * as path from 'node:path';
import { parseSuppressions, findMatchingSuppression } from './parser.js';
import type { ParsedSuppression } from './parser.js';
import { SuppressionStore } from './store.js';
import type { Warning, Suppression } from '../antipattern/types.js';
import { createDebugger } from '../utils/debug.js';

const debug = createDebugger('suppression');

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

  private normalizeToRelative(filePath: string): string {
    if (path.isAbsolute(filePath)) {
      const withSep = this.workspaceRoot.endsWith(path.sep)
        ? this.workspaceRoot
        : this.workspaceRoot + path.sep;
      if (filePath.startsWith(withSep)) {
        return filePath.slice(withSep.length);
      }
      return path.relative(this.workspaceRoot, filePath);
    }
    return filePath;
  }

  async initialize(): Promise<void> {
    debug('initializing SuppressionService', { workspaceRoot: this.workspaceRoot });
    await this.store.load();
  }

  async parseFileSuppressions(filePath: string): Promise<ParsedSuppression[]> {
    const normalizedPath = this.normalizeToRelative(filePath);
    const cached = this.fileCache.get(normalizedPath);
    if (cached) {
      return cached;
    }

    try {
      const absolutePath = path.isAbsolute(filePath)
        ? filePath
        : path.join(this.workspaceRoot, filePath);

      const content = await fs.readFile(absolutePath, 'utf-8');
      const result = parseSuppressions(content, normalizedPath);

      this.fileCache.set(normalizedPath, result.suppressions);
      return result.suppressions;
    } catch {
      return [];
    }
  }

  applyToWarnings(warnings: Warning[], filePath: string, now: Date = new Date()): Warning[] {
    const normalizedPath = this.normalizeToRelative(filePath);
    const suppressions = this.fileCache.get(normalizedPath) ?? [];

    return warnings.map((warning) => {
      if (warning.suppressed) {
        return warning;
      }

      const normalizedWarningFile = this.normalizeToRelative(warning.location.file);
      if (normalizedWarningFile !== normalizedPath) {
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
    debug('processing files for suppressions', { fileCount: files.length });
    const results: FileSuppressions[] = [];

    for (const file of files) {
      const normalizedFile = this.normalizeToRelative(file);
      const suppressions = await this.parseFileSuppressions(file);
      if (suppressions.length > 0) {
        results.push({ file: normalizedFile, suppressions });
      }
    }

    return results;
  }

  applyToAllWarnings(warnings: Warning[], now: Date = new Date()): Warning[] {
    const fileGroups = new Map<string, Warning[]>();

    for (const warning of warnings) {
      const normalizedFile = this.normalizeToRelative(warning.location.file);
      const group = fileGroups.get(normalizedFile) ?? [];
      group.push(warning);
      fileGroups.set(normalizedFile, group);
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
