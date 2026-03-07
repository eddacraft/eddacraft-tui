import { glob } from 'glob';
import { createDebugger } from '@eddacraft/anvil-core';
import { GateRunner, createCacheProvider, type AnalyzeResult } from '@eddacraft/anvil-runtime';
import { ProjectDetector, type ProjectContext } from './project-detector.js';
import { HistoricalAnalyser, type HistoricalAnalysis } from './historical-analyser.js';

const log = createDebugger('service');

const ANALYSABLE_EXTENSIONS = ['.ts', '.tsx', '.js', '.jsx', '.mjs', '.cjs'];

/**
 * Current issues found in the codebase
 */
export interface CurrentIssues {
  /** Total files scanned */
  filesScanned: number;
  /** Total warnings found */
  totalWarnings: number;
  /** Warnings by severity */
  bySeverity: {
    errors: number;
    warnings: number;
    info: number;
  };
  /** Warnings by category */
  byCategory: Record<string, number>;
  /** Top issues (most common patterns) */
  topIssues: Array<{
    id: string;
    title: string;
    count: number;
    severity: string;
  }>;
  /** Execution time in ms */
  executionTimeMs: number;
  /** Checks that were run */
  checksRun: string[];
  /** Has blocking warnings (errors) */
  hasBlockingWarnings: boolean;
  /** Raw analysis result for detailed access */
  rawResult: AnalyzeResult;
}

/**
 * Complete repo scan result
 */
export interface RepoScanResult {
  /** Project context */
  project: ProjectContext;
  /** Current issues in the codebase */
  currentIssues: CurrentIssues;
  /** Historical analysis (what Anvil would have caught) */
  historical: HistoricalAnalysis;
  /** Scan timestamp */
  timestamp: Date;
  /** Total scan duration in ms */
  totalDurationMs: number;
}

/** Check types supported by the scanner */
type CheckType = 'architecture' | 'antipattern';

/**
 * Options for repo scanning
 */
export interface RepoScanOptions {
  /** Days to look back for historical analysis */
  historicalDaysBack?: number;
  /** Max commits to analyse for historical analysis */
  historicalMaxCommits?: number;
  /** Use cache for analysis */
  useCache?: boolean;
  /** Checks to run (defaults to architecture + antipattern) */
  checks?: CheckType[];
  /** Max files to scan (0 = unlimited) */
  maxFiles?: number;
  /** Skip historical analysis */
  skipHistorical?: boolean;
  /** Progress callback */
  onProgress?: (stage: string, detail?: string) => void;
}

/**
 * Service for performing comprehensive repository scans
 *
 * This service combines:
 * - Project context detection
 * - Full codebase analysis (architecture + anti-pattern checks)
 * - Historical git analysis (what Anvil would have caught)
 *
 * Used by both `anvil scan` command and `anvil init` for first-run analysis.
 */
export class RepoScanner {
  constructor(private readonly projectRoot: string) {}

  /**
   * Perform a full repository scan
   */
  async scan(options: RepoScanOptions = {}): Promise<RepoScanResult> {
    log(`RepoScanner.scan: root=${this.projectRoot}`);
    const startTime = Date.now();
    const {
      historicalDaysBack = 30,
      historicalMaxCommits = 100,
      useCache = true,
      checks = ['architecture', 'antipattern'] as CheckType[],
      maxFiles = 0,
      skipHistorical = false,
      onProgress,
    } = options;

    // Step 1: Detect project context
    onProgress?.('project', 'Detecting project context...');
    const projectDetector = new ProjectDetector(this.projectRoot);
    const project = projectDetector.detect();

    // Step 2: Gather source files
    onProgress?.('files', 'Gathering source files...');
    let sourceFiles = await this.getSourceFiles();

    if (maxFiles > 0 && sourceFiles.length > maxFiles) {
      sourceFiles = sourceFiles.slice(0, maxFiles);
    }

    // Step 3: Run analysis on all files
    onProgress?.('analysis', `Analysing ${sourceFiles.length} files...`);
    const currentIssues = await this.analyseFiles(sourceFiles, { useCache, checks });

    // Step 4: Run historical analysis (if not skipped)
    let historical: HistoricalAnalysis;
    if (skipHistorical) {
      historical = this.createEmptyHistoricalAnalysis();
    } else {
      onProgress?.('historical', 'Analysing git history...');
      const historicalAnalyser = new HistoricalAnalyser(this.projectRoot);
      historical = await historicalAnalyser.analyse({
        daysBack: historicalDaysBack,
        maxCommits: historicalMaxCommits,
      });
    }

    const totalDurationMs = Date.now() - startTime;
    log('RepoScanner.scan complete', {
      files: currentIssues.filesScanned,
      warnings: currentIssues.totalWarnings,
      historical: historical.totalViolations,
      duration: totalDurationMs,
    });

    return {
      project,
      currentIssues,
      historical,
      timestamp: new Date(),
      totalDurationMs,
    };
  }

  /**
   * Get all analysable source files
   */
  private async getSourceFiles(): Promise<string[]> {
    const patterns = ANALYSABLE_EXTENSIONS.map((ext) => `**/*${ext}`);
    const ignorePatterns = [
      '**/node_modules/**',
      '**/dist/**',
      '**/build/**',
      '**/.git/**',
      '**/*.test.*',
      '**/*.spec.*',
      '**/__tests__/**',
      '**/__mocks__/**',
    ];

    const files: string[] = [];
    for (const pattern of patterns) {
      const matches = await glob(pattern, {
        cwd: this.projectRoot,
        ignore: ignorePatterns,
        nodir: true,
      });
      files.push(...matches);
    }

    return [...new Set(files)].sort();
  }

  /**
   * Analyse files for issues
   */
  private async analyseFiles(
    files: string[],
    options: { useCache: boolean; checks: CheckType[] }
  ): Promise<CurrentIssues> {
    const gateRunner = new GateRunner();

    const cache = createCacheProvider({
      type: options.useCache ? 'file' : 'null',
      workspaceRoot: this.projectRoot,
      disabled: !options.useCache,
    });

    const result = await gateRunner.analyzeFiles(files, this.projectRoot, {
      cache,
      noCache: !options.useCache,
      checks: options.checks,
    });

    // Process warnings into summary
    const warnings = result.warnings.warnings.filter((w) => !w.suppressed);

    // Count by severity
    const bySeverity = {
      errors: warnings.filter((w) => w.severity === 'error').length,
      warnings: warnings.filter((w) => w.severity === 'warning').length,
      info: warnings.filter((w) => w.severity === 'info').length,
    };

    // Count by category
    const byCategory: Record<string, number> = {};
    for (const w of warnings) {
      byCategory[w.category] = (byCategory[w.category] || 0) + 1;
    }

    // Find top issues (most common warning IDs)
    const issueCount = new Map<
      string,
      { id: string; title: string; count: number; severity: string }
    >();
    for (const w of warnings) {
      const existing = issueCount.get(w.id);
      if (existing) {
        existing.count++;
      } else {
        issueCount.set(w.id, {
          id: w.id,
          title: w.title,
          count: 1,
          severity: w.severity,
        });
      }
    }

    const topIssues = Array.from(issueCount.values())
      .sort((a, b) => b.count - a.count)
      .slice(0, 10);

    return {
      filesScanned: files.length,
      totalWarnings: warnings.length,
      bySeverity,
      byCategory,
      topIssues,
      executionTimeMs: result.executionTimeMs,
      checksRun: result.checksRun,
      hasBlockingWarnings: result.hasBlockingWarnings,
      rawResult: result,
    };
  }

  /**
   * Create empty historical analysis when skipped or unavailable
   */
  private createEmptyHistoricalAnalysis(): HistoricalAnalysis {
    return {
      commits: [],
      totalCommits: 0,
      totalViolations: 0,
      avgViolationsPerCommit: 0,
      patternOccurrences: [],
      timeline: [],
      dateRange: {
        from: new Date(),
        to: new Date(),
      },
    };
  }

  /**
   * Generate a human-readable summary of the scan results
   */
  generateSummary(result: RepoScanResult): string {
    const lines: string[] = [];

    lines.push(`Repository Scan Complete`);
    lines.push(`========================`);
    lines.push('');

    // Project info
    lines.push(`Project: ${result.project.framework} (${result.project.size})`);
    lines.push(`Files: ${result.project.fileCount.toLocaleString()}`);
    if (result.project.monorepo !== 'none') {
      lines.push(
        `Monorepo: ${result.project.monorepo} (${result.project.workspacePackages.length} packages)`
      );
    }
    lines.push('');

    // Current issues
    lines.push(`Current Issues`);
    lines.push(`--------------`);
    lines.push(`Files scanned: ${result.currentIssues.filesScanned}`);
    lines.push(`Total warnings: ${result.currentIssues.totalWarnings}`);

    if (result.currentIssues.totalWarnings > 0) {
      lines.push(`  Errors: ${result.currentIssues.bySeverity.errors}`);
      lines.push(`  Warnings: ${result.currentIssues.bySeverity.warnings}`);
      lines.push(`  Info: ${result.currentIssues.bySeverity.info}`);
      lines.push('');

      if (result.currentIssues.topIssues.length > 0) {
        lines.push(`Top issues:`);
        for (const issue of result.currentIssues.topIssues.slice(0, 5)) {
          lines.push(`  [${issue.id}] ${issue.title}: ${issue.count} occurrences`);
        }
      }
    } else {
      lines.push(`  No issues found!`);
    }
    lines.push('');

    // Historical analysis
    if (result.historical.totalCommits > 0) {
      lines.push(`Historical Analysis (last 30 days)`);
      lines.push(`----------------------------------`);
      lines.push(`Commits analysed: ${result.historical.totalCommits}`);
      lines.push(`Anvil would have caught: ${result.historical.totalViolations} issues`);
      lines.push(`Average per commit: ${result.historical.avgViolationsPerCommit.toFixed(1)}`);

      if (result.historical.patternOccurrences.length > 0) {
        lines.push('');
        lines.push(`Most common patterns:`);
        for (const pattern of result.historical.patternOccurrences.slice(0, 5)) {
          lines.push(`  ${pattern.patternName}: ${pattern.count} occurrences`);
        }
      }
    }
    lines.push('');

    lines.push(`Scan completed in ${result.totalDurationMs}ms`);

    return lines.join('\n');
  }
}
