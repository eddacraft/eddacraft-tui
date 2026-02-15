import * as fs from 'node:fs/promises';
import * as path from 'node:path';
import { execFile } from 'node:child_process';
import { promisify } from 'node:util';
import { createDebugger } from '../utils/debug.js';
import {
  type DriftSnapshot,
  type SnapshotViolation,
  type SnapshotAntiPattern,
  type SnapshotSuppression,
  type SnapshotMetrics,
  type AntiPatternBreakdown,
  type Hotspot,
  SNAPSHOT_SCHEMA_VERSION,
} from './snapshot-schema.js';
import {
  loadBaseline,
  type ArchitectureBaseline,
  type BaselineViolation,
} from '../architecture/index.js';
import { scanFiles, type ScanResult } from '../antipattern/index.js';
import { SuppressionService, type FileSuppressions } from '../suppression/index.js';
import { generateHash } from '../crypto/index.js';

const debug = createDebugger('drift');
const execFileAsync = promisify(execFile);

export interface CaptureOptions {
  name?: string;
  includeOptInPatterns?: boolean;
  sourcePatterns?: string[];
}

export interface CaptureContext {
  workspaceRoot: string;
  files: string[];
  baseline: ArchitectureBaseline | null;
  scanResults: ScanResult[];
  suppressions: FileSuppressions[];
  gitRef?: string;
}

async function getGitRef(workspaceRoot: string): Promise<string | undefined> {
  try {
    const { stdout } = await execFileAsync('git', ['rev-parse', 'HEAD'], { cwd: workspaceRoot });
    return stdout.trim();
  } catch (err) {
    debug('failed to get git ref for workspace', err);
    return undefined;
  }
}

function baselineViolationToSnapshot(violation: BaselineViolation): SnapshotViolation {
  return {
    id: violation.id,
    type: 'boundary',
    from_file: violation.from_file,
    to_file: violation.to_file,
    from_layer: violation.from_layer,
    to_layer: violation.to_layer,
    line: violation.import_line,
    rule: violation.rule,
  };
}

function extractAntiPatterns(scanResults: ScanResult[]): SnapshotAntiPattern[] {
  const antipatterns: SnapshotAntiPattern[] = [];

  for (const result of scanResults) {
    for (const warning of result.warnings) {
      if (warning.category !== 'anti-pattern') continue;
      if (warning.suppressed) continue;

      antipatterns.push({
        id: warning.id,
        file: warning.location.file,
        line: warning.location.line,
        pattern: warning.pattern ?? warning.id,
        severity: warning.severity,
      });
    }
  }

  return antipatterns;
}

function extractSuppressions(
  fileSuppressions: FileSuppressions[],
  now: Date = new Date()
): SnapshotSuppression[] {
  const suppressions: SnapshotSuppression[] = [];

  for (const { file, suppressions: fileSupprs } of fileSuppressions) {
    for (const s of fileSupprs) {
      const isExpired = s.expiresAt ? s.expiresAt < now : false;

      suppressions.push({
        id: `${file}:${s.line}:${s.warningId}`,
        pattern_id: s.warningId,
        file,
        line: s.line,
        reason: s.reason,
        scope: s.scope,
        expires_at: s.expiresAt?.toISOString(),
        is_expired: isExpired,
      });
    }
  }

  return suppressions;
}

function calculateAntiPatternBreakdown(antipatterns: SnapshotAntiPattern[]): AntiPatternBreakdown {
  const breakdown: AntiPatternBreakdown = {};

  for (const ap of antipatterns) {
    breakdown[ap.id] = (breakdown[ap.id] ?? 0) + 1;
  }

  return breakdown;
}

function calculateHotspots(
  violations: SnapshotViolation[],
  antipatterns: SnapshotAntiPattern[],
  topN: number = 5
): Hotspot[] {
  const pathCounts = new Map<string, { count: number; types: Set<string> }>();

  for (const v of violations) {
    const dir = path.dirname(v.from_file);
    const existing = pathCounts.get(dir) ?? { count: 0, types: new Set() };
    existing.count++;
    existing.types.add('boundary');
    pathCounts.set(dir, existing);
  }

  for (const ap of antipatterns) {
    const dir = path.dirname(ap.file);
    const existing = pathCounts.get(dir) ?? { count: 0, types: new Set() };
    existing.count++;
    existing.types.add(ap.id);
    pathCounts.set(dir, existing);
  }

  return Array.from(pathCounts.entries())
    .filter(([_, data]) => data.count > 1)
    .sort((a, b) => b[1].count - a[1].count)
    .slice(0, topN)
    .map(([dirPath, data]) => ({
      path: dirPath,
      violation_count: data.count,
      types: Array.from(data.types),
    }));
}

function calculateMetrics(
  violations: SnapshotViolation[],
  antipatterns: SnapshotAntiPattern[],
  suppressions: SnapshotSuppression[],
  filesAnalysed: number
): SnapshotMetrics {
  const expiredSuppressions = suppressions.filter((s) => s.is_expired).length;

  return {
    boundary_violations: violations.length,
    antipattern_count: antipatterns.length,
    suppression_count: suppressions.filter((s) => !s.is_expired).length,
    expired_suppressions: expiredSuppressions,
    files_analysed: filesAnalysed,
  };
}

function generateBaselineHash(baseline: ArchitectureBaseline | null): string | undefined {
  if (!baseline) return undefined;
  return generateHash(JSON.stringify(baseline));
}

export async function captureSnapshot(
  context: CaptureContext,
  options: CaptureOptions = {}
): Promise<DriftSnapshot> {
  const { files, baseline, scanResults, suppressions, gitRef } = context;

  const violations = baseline
    ? baseline.baseline_snapshot.violations.map(baselineViolationToSnapshot)
    : [];

  const antipatterns = extractAntiPatterns(scanResults);
  const snapshotSuppressions = extractSuppressions(suppressions);
  const metrics = calculateMetrics(violations, antipatterns, snapshotSuppressions, files.length);
  const breakdown = calculateAntiPatternBreakdown(antipatterns);
  const hotspots = calculateHotspots(violations, antipatterns);

  return {
    schema_version: SNAPSHOT_SCHEMA_VERSION,
    created_at: new Date().toISOString(),
    name: options.name,
    metrics,
    antipattern_breakdown: Object.keys(breakdown).length > 0 ? breakdown : undefined,
    hotspots: hotspots.length > 0 ? hotspots : undefined,
    violations,
    antipatterns,
    suppressions: snapshotSuppressions,
    baseline_hash: generateBaselineHash(baseline),
    git_ref: gitRef,
  };
}

export class SnapshotCaptureService {
  private workspaceRoot: string;

  constructor(workspaceRoot: string) {
    this.workspaceRoot = workspaceRoot;
  }

  async capture(files: string[], options: CaptureOptions = {}): Promise<DriftSnapshot> {
    const baseline = loadBaseline(this.workspaceRoot);

    const filesWithContent = await Promise.all(
      files.map(async (filePath) => {
        const fullPath = path.isAbsolute(filePath)
          ? filePath
          : path.join(this.workspaceRoot, filePath);
        try {
          const content = await fs.readFile(fullPath, 'utf-8');
          return { path: filePath, content };
        } catch (err) {
          debug('failed to read file for snapshot capture: %s', err);
          return null;
        }
      })
    );

    const validFiles = filesWithContent.filter(
      (f): f is { path: string; content: string } => f !== null
    );

    const scanResults = scanFiles(validFiles, { includeOptIn: options.includeOptInPatterns });
    const suppressionService = new SuppressionService(this.workspaceRoot);
    await suppressionService.initialize();
    const suppressions = await suppressionService.processFiles(files);
    const gitRef = await getGitRef(this.workspaceRoot);

    const context: CaptureContext = {
      workspaceRoot: this.workspaceRoot,
      files,
      baseline,
      scanResults,
      suppressions,
      gitRef,
    };

    return captureSnapshot(context, options);
  }

  async captureWithContext(
    context: Partial<CaptureContext>,
    options: CaptureOptions = {}
  ): Promise<DriftSnapshot> {
    const fullContext: CaptureContext = {
      workspaceRoot: context.workspaceRoot ?? this.workspaceRoot,
      files: context.files ?? [],
      baseline: context.baseline ?? null,
      scanResults: context.scanResults ?? [],
      suppressions: context.suppressions ?? [],
      gitRef: context.gitRef,
    };

    return captureSnapshot(fullContext, options);
  }
}

export function createSnapshotCaptureService(workspaceRoot: string): SnapshotCaptureService {
  return new SnapshotCaptureService(workspaceRoot);
}
