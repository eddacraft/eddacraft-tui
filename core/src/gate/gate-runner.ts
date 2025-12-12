import { Check } from './check.interface.js';
import {
  CheckContext,
  GateConfig,
  GateCheck,
  GateResult,
  GateRunResult,
  PlanData,
} from '../types/gate.types.js';
import { ESLintCheck } from './checks/eslint.check.js';
import { CoverageCheck } from './checks/coverage.check.js';
import { SecretCheck } from './checks/secret.check.js';
import { DependencyCheck } from './checks/dependency.check.js';
import { PolicyCheck } from './checks/policy.check.js';
import type { CacheProvider } from '../cache/types.js';
import { NullCacheProvider } from '../cache/providers/null-cache.js';
import { generateCacheKey, hashCheckConfig, generateInputHash } from '../cache/cache-key.js';

/**
 * Internal type for checks to run
 */
interface CheckToRun {
  checkConfig: GateCheck;
  check: Check;
}

/**
 * Options for running gates
 */
export interface GateRunOptions {
  /** Checks to skip */
  skipChecks?: string[];
  /** Only run these checks */
  onlyChecks?: string[];
  /** Stop on first failure */
  failFast?: boolean;
  /** Cache provider (defaults to NullCacheProvider) */
  cache?: CacheProvider;
  /** Maximum parallel checks (0 = sequential, undefined = all parallel) */
  parallelLimit?: number;
  /** Force bypass cache for this run */
  noCache?: boolean;
}

/**
 * Extended gate result with cache metadata
 */
export interface GateRunResultWithCache extends GateRunResult {
  /** Cache statistics for this run */
  cacheStats?: {
    /** Number of cache hits */
    hits: number;
    /** Number of cache misses */
    misses: number;
    /** Estimated time saved by cache hits (ms) */
    timeSavedMs: number;
  };
  /** Execution timing */
  timing?: {
    /** Total execution time (ms) */
    totalMs: number;
    /** Per-check timing */
    checks: Record<string, number>;
  };
}

export class GateRunner {
  private checks: Map<string, Check> = new Map();
  private defaultCache: CacheProvider = new NullCacheProvider();

  constructor() {
    this.registerDefaultChecks();
  }

  /**
   * Set default cache provider
   */
  setDefaultCache(cache: CacheProvider): void {
    this.defaultCache = cache;
  }

  registerCheck(check: Check): void {
    this.checks.set(check.name, check);
  }

  unregisterCheck(name: string): void {
    this.checks.delete(name);
  }

  getAvailableChecks(): string[] {
    return Array.from(this.checks.keys());
  }

  async runGate(
    plan: PlanData,
    config: GateConfig,
    workspaceRoot: string,
    options?: GateRunOptions
  ): Promise<GateRunResultWithCache> {
    const startTime = Date.now();
    const cache = options?.noCache
      ? new NullCacheProvider()
      : (options?.cache ?? this.defaultCache);
    const parallelLimit = options?.parallelLimit;

    // Collect checks to run
    const checksToRun: CheckToRun[] = [];

    const skippedResults: GateResult[] = [];

    for (const checkConfig of config.checks) {
      // Apply skip/only filters
      if (options?.skipChecks?.includes(checkConfig.name)) {
        skippedResults.push({
          check: checkConfig.name,
          passed: true,
          message: 'Check skipped via --skip-checks',
          skipped: true,
        });
        continue;
      }

      if (options?.onlyChecks && !options.onlyChecks.includes(checkConfig.name)) {
        skippedResults.push({
          check: checkConfig.name,
          passed: true,
          message: 'Check not in --only-checks filter',
          skipped: true,
        });
        continue;
      }

      if (!checkConfig.enabled) {
        skippedResults.push({
          check: checkConfig.name,
          passed: true,
          message: 'Check disabled',
          skipped: true,
        });
        continue;
      }

      const check = this.checks.get(checkConfig.name);
      if (!check) {
        skippedResults.push({
          check: checkConfig.name,
          passed: false,
          message: `Check '${checkConfig.name}' not found`,
          error: 'Unknown check',
        });
        continue;
      }

      checksToRun.push({ checkConfig, check });
    }

    // Execute checks (parallel or sequential)
    const { results, cacheStats, timing } = await this.executeChecks(
      checksToRun,
      plan,
      config,
      workspaceRoot,
      cache,
      parallelLimit,
      options?.failFast
    );

    // Combine skipped and executed results
    const allResults = [...skippedResults, ...results];

    // Calculate scores
    let totalScore = 0;
    let validChecks = 0;

    for (const result of allResults) {
      if (result.score !== undefined && !result.skipped) {
        totalScore += result.score;
        validChecks++;
      }
    }

    const overallScore = validChecks > 0 ? totalScore / validChecks : 100;
    const passed = allResults.every((r) => r.passed || r.skipped);
    const overallPassed =
      passed && (validChecks === 0 || overallScore >= (config.thresholds.overall_score || 80));

    const summary = {
      total: allResults.length,
      passed: allResults.filter((r) => r.passed && !r.skipped).length,
      failed: allResults.filter((r) => !r.passed && !r.skipped).length,
      skipped: allResults.filter((r) => r.skipped).length,
    };

    return {
      overall: overallPassed,
      score: overallScore,
      checks: allResults,
      summary,
      cacheStats,
      timing: {
        totalMs: Date.now() - startTime,
        checks: timing,
      },
    };
  }

  private async executeChecks(
    checksToRun: CheckToRun[],
    plan: PlanData,
    config: GateConfig,
    workspaceRoot: string,
    cache: CacheProvider,
    parallelLimit?: number,
    failFast?: boolean
  ): Promise<{
    results: GateResult[];
    cacheStats: { hits: number; misses: number; timeSavedMs: number };
    timing: Record<string, number>;
  }> {
    const results: GateResult[] = [];
    const cacheStats = { hits: 0, misses: 0, timeSavedMs: 0 };
    const timing: Record<string, number> = {};

    // Sequential execution (failFast or parallelLimit === 0)
    if (failFast || parallelLimit === 0) {
      for (const { checkConfig, check } of checksToRun) {
        const { result, cached, executionTimeMs } = await this.runCheckWithCache(
          check,
          checkConfig,
          plan,
          config,
          workspaceRoot,
          cache
        );

        results.push(result);
        timing[checkConfig.name] = executionTimeMs;

        if (cached) {
          cacheStats.hits++;
          cacheStats.timeSavedMs += executionTimeMs;
        } else {
          cacheStats.misses++;
        }

        // Fail-fast: stop on first failure
        if (failFast && !result.passed && !result.skipped) {
          break;
        }
      }
      return { results, cacheStats, timing };
    }

    // Parallel execution
    const runCheck = async (item: CheckToRun) => {
      const { result, cached, executionTimeMs } = await this.runCheckWithCache(
        item.check,
        item.checkConfig,
        plan,
        config,
        workspaceRoot,
        cache
      );
      return { checkConfig: item.checkConfig, result, cached, executionTimeMs };
    };

    if (parallelLimit === undefined || parallelLimit >= checksToRun.length) {
      // Fully parallel
      const checkResults = await Promise.all(checksToRun.map(runCheck));

      for (const { checkConfig, result, cached, executionTimeMs } of checkResults) {
        results.push(result);
        timing[checkConfig.name] = executionTimeMs;

        if (cached) {
          cacheStats.hits++;
          cacheStats.timeSavedMs += executionTimeMs;
        } else {
          cacheStats.misses++;
        }
      }
    } else {
      // Limited parallelism using batches
      for (let i = 0; i < checksToRun.length; i += parallelLimit) {
        const batch = checksToRun.slice(i, i + parallelLimit);
        const batchResults = await Promise.all(batch.map(runCheck));

        for (const { checkConfig, result, cached, executionTimeMs } of batchResults) {
          results.push(result);
          timing[checkConfig.name] = executionTimeMs;

          if (cached) {
            cacheStats.hits++;
            cacheStats.timeSavedMs += executionTimeMs;
          } else {
            cacheStats.misses++;
          }
        }
      }
    }

    return { results, cacheStats, timing };
  }

  private async runCheckWithCache(
    check: Check,
    checkConfig: { name: string; config?: Record<string, unknown> },
    plan: PlanData,
    gateConfig: GateConfig,
    workspaceRoot: string,
    cache: CacheProvider
  ): Promise<{ result: GateResult; cached: boolean; executionTimeMs: number }> {
    const startTime = Date.now();

    // Generate cache key
    const cacheKeyInput = {
      check_name: checkConfig.name,
      plan_hash: plan.hash || 'no-hash',
      config_hash: hashCheckConfig(checkConfig.config || {}),
      workspace_root: workspaceRoot,
    };
    const cacheKey = generateCacheKey(cacheKeyInput);
    const inputHash = generateInputHash(cacheKeyInput);

    // Try to get from cache
    try {
      const cached = await cache.get<GateResult>(cacheKey);
      if (cached && cached.input_hash === inputHash) {
        return {
          result: {
            ...cached.value,
            details: {
              ...cached.value.details,
              cached: true,
              cached_at: cached.created_at,
            },
          },
          cached: true,
          executionTimeMs: Date.now() - startTime,
        };
      }
    } catch {
      // Cache read failed, continue without cache
    }

    // Run the check
    try {
      const context: CheckContext = {
        plan,
        workspace_root: workspaceRoot,
        config: gateConfig,
        check_config: checkConfig.config || {},
      };

      const result = await check.run(context);
      const executionTimeMs = Date.now() - startTime;

      // Cache the result (only cache successful or failed results, not errors)
      if (!result.error) {
        try {
          await cache.set(cacheKey, result, { input_hash: inputHash });
        } catch {
          // Cache write failed, continue without caching
        }
      }

      return { result, cached: false, executionTimeMs };
    } catch (error) {
      const errorResult: GateResult = {
        check: checkConfig.name,
        passed: false,
        message: `Check '${checkConfig.name}' failed with error`,
        error: error instanceof Error ? error.message : 'Unknown error',
      };
      return {
        result: errorResult,
        cached: false,
        executionTimeMs: Date.now() - startTime,
      };
    }
  }

  private registerDefaultChecks(): void {
    this.registerCheck(new ESLintCheck());
    this.registerCheck(new CoverageCheck());
    this.registerCheck(new SecretCheck());
    this.registerCheck(new DependencyCheck());
    this.registerCheck(new PolicyCheck());
  }
}
