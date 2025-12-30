import { existsSync, readFileSync, statSync } from 'fs';
import { join } from 'path';
import type {
  StatusData,
  HooksStatus,
  HookInfo,
  HookState,
  RepoProfile,
  CheckConfig,
  RecentResults,
  ValidationResult,
} from '../tui/commands/status/types.js';

const KNOWN_HOOKS = ['pre-commit', 'commit-msg', 'pre-push', 'post-merge', 'post-checkout'];
const HUSKY_DIR = '.husky';
const ANVIL_DIR = '.anvil';
const ANVIL_RC = '.anvilrc';
const CACHE_DIR = 'cache';

function detectHookState(hookPath: string): HookState {
  if (!existsSync(hookPath)) return 'missing';

  try {
    const content = readFileSync(hookPath, 'utf-8');
    if (content.includes('exit 0') && content.split('\n').length <= 3) {
      return 'disabled';
    }
    return 'active';
  } catch {
    return 'missing';
  }
}

function getHookLastRun(hookPath: string): Date | undefined {
  try {
    const stats = statSync(hookPath);
    return stats.mtime;
  } catch {
    return undefined;
  }
}

function isAnvilManagedHook(hookPath: string): boolean {
  try {
    const content = readFileSync(hookPath, 'utf-8');
    return content.includes('anvil') || content.includes('ANVIL');
  } catch {
    return false;
  }
}

export function gatherHooksStatus(projectRoot: string): HooksStatus {
  const huskyDir = join(projectRoot, HUSKY_DIR);
  const huskyInstalled = existsSync(huskyDir);

  const hooks: HookInfo[] = KNOWN_HOOKS.map((name) => {
    const hookPath = join(huskyDir, name);
    const state = huskyInstalled ? detectHookState(hookPath) : 'missing';

    return {
      name,
      state,
      path: existsSync(hookPath) ? hookPath : undefined,
      lastRun: getHookLastRun(hookPath),
      isAnvilManaged: isAnvilManagedHook(hookPath),
    };
  });

  return {
    huskyInstalled,
    hooksDir: huskyDir,
    hooks,
  };
}

export function gatherRepoProfile(projectRoot: string): RepoProfile {
  const configPath = join(projectRoot, ANVIL_RC);
  const hasConfig = existsSync(configPath);

  if (!hasConfig) {
    return {
      hasConfig: false,
      configPath,
      checks: [],
    };
  }

  try {
    const content = readFileSync(configPath, 'utf-8');
    const config = JSON.parse(content) as {
      version?: number;
      checks?: Array<{ name: string; enabled?: boolean; config?: Record<string, unknown> }>;
      planningDir?: string;
      format?: string;
      schemaVersion?: string;
      thresholds?: { coverage?: number };
    };

    const checks: CheckConfig[] = (config.checks ?? []).map((check) => ({
      name: check.name,
      enabled: check.enabled ?? true,
      options: check.config,
    }));

    const coverageCheck = config.checks?.find((c) => c.name === 'coverage');
    const coverageThreshold = coverageCheck?.config?.thresholds
      ? ((coverageCheck.config as { thresholds?: { lines?: number } }).thresholds?.lines as
          | number
          | undefined)
      : undefined;

    return {
      hasConfig: true,
      configPath,
      planningDir: config.planningDir,
      format: config.format,
      checks,
      coverageThreshold,
      schemaVersion: config.schemaVersion ?? '0.1.0',
    };
  } catch {
    return {
      hasConfig: true,
      configPath,
      checks: [],
    };
  }
}

export function gatherRecentResults(projectRoot: string, limit = 5): RecentResults {
  const cacheDir = join(projectRoot, ANVIL_DIR, CACHE_DIR);
  const hasCache = existsSync(cacheDir);

  if (!hasCache) {
    return {
      hasCache: false,
      cacheDir,
      results: [],
    };
  }

  const indexPath = join(cacheDir, 'index.json');
  if (!existsSync(indexPath)) {
    return {
      hasCache: true,
      cacheDir,
      results: [],
    };
  }

  try {
    const indexContent = readFileSync(indexPath, 'utf-8');
    const index = JSON.parse(indexContent) as {
      entries?: Record<string, { file: string; created_at: number }>;
    };

    const entries = Object.entries(index.entries ?? {});
    const results: ValidationResult[] = entries
      .filter(([key]) => key.startsWith('gate:') || key.startsWith('validate:'))
      .map(([key, entry]) => {
        const parts = key.split(':');
        return {
          id: entry.file.replace('.json', ''),
          timestamp: new Date(entry.created_at),
          planPath: parts[parts.length - 1] ?? 'unknown',
          passed: true,
          passedChecks: 1,
          totalChecks: 1,
        };
      })
      .sort((a, b) => b.timestamp.getTime() - a.timestamp.getTime())
      .slice(0, limit);

    return {
      hasCache: true,
      cacheDir,
      results,
    };
  } catch {
    return {
      hasCache: true,
      cacheDir,
      results: [],
    };
  }
}

function getProjectName(projectRoot: string): string | undefined {
  const packageJsonPath = join(projectRoot, 'package.json');
  if (!existsSync(packageJsonPath)) return undefined;

  try {
    const content = readFileSync(packageJsonPath, 'utf-8');
    const pkg = JSON.parse(content) as { name?: string };
    return pkg.name;
  } catch {
    return undefined;
  }
}

export function gatherStatusData(projectRoot: string): StatusData {
  return {
    projectRoot,
    projectName: getProjectName(projectRoot),
    hooks: gatherHooksStatus(projectRoot),
    profile: gatherRepoProfile(projectRoot),
    recent: gatherRecentResults(projectRoot),
    gatheredAt: new Date(),
  };
}
