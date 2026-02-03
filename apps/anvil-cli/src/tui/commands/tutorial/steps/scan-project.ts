import { glob } from 'glob';
import { GateRunner, createCacheProvider } from '@eddacraft/anvil-runtime';
import type { Warning } from '@eddacraft/anvil-core/antipattern';
import type { ScanResults, ScanWarning } from '../types.js';

const ANALYSABLE_EXTENSIONS = ['.ts', '.tsx', '.js', '.jsx'];
const IGNORE_PATTERNS = ['**/node_modules/**', '**/dist/**', '**/.git/**'];
const MAX_TOP_WARNINGS = 3;

/**
 * Scan the project for warnings using GateRunner's analyzeFiles.
 *
 * Extracted into its own module so tests can mock this function
 * without needing to mock GateRunner internals.
 */
export async function scanProject(workspaceRoot: string): Promise<ScanResults> {
  const patterns = ANALYSABLE_EXTENSIONS.map((ext) => `**/*${ext}`);

  const files: string[] = [];
  for (const pattern of patterns) {
    const matches = await glob(pattern, {
      cwd: workspaceRoot,
      ignore: IGNORE_PATTERNS,
      nodir: true,
    });
    files.push(...matches);
  }

  const uniqueFiles = [...new Set(files)].sort();

  if (uniqueFiles.length === 0) {
    return {
      warningCount: 0,
      fileCount: 0,
      executionTimeMs: 0,
      topWarnings: [],
    };
  }

  const runner = new GateRunner();
  const cache = createCacheProvider({ type: 'memory' });

  const result = await runner.analyzeFiles(uniqueFiles, workspaceRoot, {
    cache,
    checks: ['architecture', 'antipattern'],
  });

  const warnings = result.warnings.warnings.filter((w: Warning) => !w.suppressed);
  const affectedFiles = new Set(warnings.map((w: Warning) => w.location.file));

  const topWarnings: ScanWarning[] = warnings.slice(0, MAX_TOP_WARNINGS).map((w: Warning) => ({
    id: w.id,
    title: w.title,
    file: w.location.file,
    line: w.location.line,
    message: w.message,
    suggestion: w.suggestion,
  }));

  return {
    warningCount: warnings.length,
    fileCount: affectedFiles.size,
    executionTimeMs: result.executionTimeMs,
    topWarnings,
  };
}
