import { existsSync, readFileSync, statSync } from 'fs';
import { join } from 'path';
import { z } from 'zod';
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
import { createDebugger } from '@anvil/core';

const debug = createDebugger('validation');

/**
 * Zod schemas for runtime validation of JSON.parse results
 */
const AnvilRcCheckSchema = z.object({
  name: z.string(),
  enabled: z.boolean().optional(),
  config: z.record(z.string(), z.unknown()).optional(),
});

const AnvilRcSchema = z.object({
  version: z.number().optional(),
  checks: z.array(AnvilRcCheckSchema).optional(),
  planningDir: z.string().optional(),
  format: z.string().optional(),
  schemaVersion: z.string().optional(),
  thresholds: z.object({ coverage: z.number().optional() }).optional(),
});

const CacheIndexEntrySchema = z.object({
  file: z.string(),
  created_at: z.number(),
});

type CacheIndexEntry = z.infer<typeof CacheIndexEntrySchema>;

const CacheIndexSchema = z.object({
  entries: z.record(z.string(), CacheIndexEntrySchema).optional(),
});

const PackageJsonSchema = z.object({
  name: z.string().optional(),
});

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
  } catch (error) {
    debug('Failed to detect hook state', error);
    return 'missing';
  }
}

function getHookLastRun(hookPath: string): Date | undefined {
  try {
    const stats = statSync(hookPath);
    return stats.mtime;
  } catch (error) {
    debug('Failed to get hook last run time', error);
    return undefined;
  }
}

function isAnvilManagedHook(hookPath: string): boolean {
  try {
    const content = readFileSync(hookPath, 'utf-8');
    return content.includes('anvil') || content.includes('ANVIL');
  } catch (error) {
    debug('Failed to check if hook is Anvil-managed', error);
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
    const parseResult = AnvilRcSchema.safeParse(JSON.parse(content));
    if (!parseResult.success) {
      debug('Invalid .anvilrc schema', parseResult.error);
      return {
        hasConfig: true,
        configPath,
        checks: [],
      };
    }
    const config = parseResult.data;

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
  } catch (error) {
    debug('Failed to parse .anvilrc configuration', error);
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
    const parseResult = CacheIndexSchema.safeParse(JSON.parse(indexContent));
    if (!parseResult.success) {
      debug('Invalid cache index schema', parseResult.error);
      return {
        hasCache: true,
        cacheDir,
        results: [],
      };
    }
    const index = parseResult.data;

    const entries = Object.entries(index.entries ?? {}) as [string, CacheIndexEntry][];
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
  } catch (error) {
    debug('Failed to parse cache index', error);
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
    const parseResult = PackageJsonSchema.safeParse(JSON.parse(content));
    if (!parseResult.success) {
      debug('Invalid package.json schema', parseResult.error);
      return undefined;
    }
    return parseResult.data.name;
  } catch (error) {
    debug('Failed to parse package.json for project name', error);
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
