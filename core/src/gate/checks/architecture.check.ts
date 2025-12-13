/**
 * Architecture Check - Validate architectural constraints using dependency-cruiser
 *
 * Detects:
 * - Circular dependencies
 * - Layer/boundary violations
 * - Orphaned modules (optional)
 */

import { BaseCheck } from '../check.interface.js';
import { CheckContext, GateResult } from '../../types/gate.types.js';
import { existsSync } from 'fs';
import { join } from 'path';

/**
 * Configuration for architecture check
 */
export interface ArchitectureCheckConfig {
  /** Path to dependency-cruiser config (default: .dependency-cruiser.js) */
  config_file?: string;
  /** Scope of analysis: 'affected' | 'full' */
  scope?: 'affected' | 'full';
  /** Minimum severity to fail the check */
  severity_threshold?: 'error' | 'warn' | 'info';
  /** Whether circular dependencies should fail the check */
  fail_on_circular?: boolean;
  /** Whether orphaned modules should fail the check */
  fail_on_orphan?: boolean;
  /** Include patterns (glob) for full scope */
  include_patterns?: string[];
  /** Exclude patterns (glob) */
  exclude_patterns?: string[];
}

/**
 * Violation from dependency-cruiser
 */
interface CruiserViolation {
  from: string;
  to: string;
  rule: {
    name: string;
    severity: 'error' | 'warn' | 'info' | 'ignore';
  };
  cycle?: string[];
  comment?: string;
}

/**
 * Summary from dependency-cruiser
 */
interface CruiserSummary {
  violations: CruiserViolation[];
  error: number;
  warn: number;
  info: number;
  totalCruised: number;
}

/**
 * Cruise result from dependency-cruiser
 */
interface ICruiseResult {
  summary: CruiserSummary;
  modules: unknown[];
}

/**
 * Default configuration
 */
const DEFAULT_CONFIG: Required<ArchitectureCheckConfig> = {
  config_file: '.dependency-cruiser.js',
  scope: 'affected',
  severity_threshold: 'error',
  fail_on_circular: true,
  fail_on_orphan: false,
  include_patterns: ['src/**/*.ts', 'src/**/*.js'],
  exclude_patterns: [
    '**/*.test.ts',
    '**/*.spec.ts',
    '**/__fixtures__/**',
    '**/__tests__/**',
    '**/node_modules/**',
  ],
};

/**
 * Score penalties per severity
 */
const SEVERITY_PENALTIES = {
  error: 15,
  warn: 5,
  info: 1,
  ignore: 0,
};

/**
 * Architecture check that validates project structure using dependency-cruiser
 */
export class ArchitectureCheck extends BaseCheck {
  name = 'architecture';
  description = 'Validate architectural constraints using dependency-cruiser';

  async run(context: CheckContext): Promise<GateResult> {
    const config = this.parseConfig(context.check_config);

    try {
      // Step 1: Check if dependency-cruiser is available
      // We use dynamic import to avoid compile-time dependency
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      let cruise: any;

      try {
        // Dynamic import - dependency-cruiser is an optional peer dependency
        const depCruiser = await (Function('return import("dependency-cruiser")')() as Promise<{
          cruise: (
            fileAndDirectoryArray: string[],
            options?: Record<string, unknown>
          ) => Promise<{ output: ICruiseResult }>;
        }>);
        cruise = depCruiser.cruise;
      } catch {
        return this.createSuccess(
          'dependency-cruiser not installed. Run `npm install -D dependency-cruiser` to enable architecture checks.',
          100,
          { skipped: true, reason: 'dependency-cruiser not available' }
        );
      }

      // Step 2: Load dependency-cruiser config or use defaults
      const configPath = join(context.workspace_root, config.config_file);
      let cruiseOptions: Record<string, unknown> = {};

      if (existsSync(configPath)) {
        try {
          // Dynamic import of the config file
          const configModule = await import(configPath);
          cruiseOptions = configModule.default || configModule;
        } catch (configError) {
          return this.createFailure(
            `Failed to load dependency-cruiser config: ${config.config_file}`,
            configError instanceof Error ? configError.message : 'Unknown error'
          );
        }
      } else {
        // Use default rules for circular dependency and orphan detection
        cruiseOptions = this.getDefaultCruiseOptions();
      }

      // Step 3: Determine files to analyse
      const filesToCruise = this.getFilesToCruise(context, config);

      if (filesToCruise.length === 0) {
        return this.createSuccess('No files to analyse for architecture violations', 100);
      }

      // Step 4: Run dependency-cruiser
      const cruiseResult = await cruise(filesToCruise, {
        ...cruiseOptions,
        outputType: 'json',
      });

      const output = cruiseResult.output as ICruiseResult;

      // Step 5: Process violations
      const violations = output.summary.violations;
      const { score, passed, violationsByType } = this.calculateScore(violations, config);

      const message = this.buildMessage(violations, output.summary.totalCruised, passed);

      return this.createResult(passed, message, score, {
        totalModulesCruised: output.summary.totalCruised,
        violationCount: violations.length,
        errorCount: output.summary.error,
        warnCount: output.summary.warn,
        infoCount: output.summary.info,
        violations: violations.map((v) => ({
          from: v.from,
          to: v.to,
          rule: v.rule.name,
          severity: v.rule.severity,
          cycle: v.cycle,
        })),
        violationsByType,
        configFile: existsSync(configPath) ? config.config_file : 'built-in defaults',
        scope: config.scope,
      });
    } catch (error) {
      return this.createFailure(
        'Architecture check failed unexpectedly',
        error instanceof Error ? error.message : 'Unknown error'
      );
    }
  }

  /**
   * Parse and validate check configuration
   */
  private parseConfig(checkConfig: Record<string, unknown>): Required<ArchitectureCheckConfig> {
    return {
      config_file:
        typeof checkConfig.config_file === 'string'
          ? checkConfig.config_file
          : DEFAULT_CONFIG.config_file,
      scope: this.parseScope(checkConfig.scope),
      severity_threshold:
        this.parseSeverity(checkConfig.severity_threshold) || DEFAULT_CONFIG.severity_threshold,
      fail_on_circular: checkConfig.fail_on_circular !== false,
      fail_on_orphan: checkConfig.fail_on_orphan === true,
      include_patterns: Array.isArray(checkConfig.include_patterns)
        ? checkConfig.include_patterns.filter((p): p is string => typeof p === 'string')
        : DEFAULT_CONFIG.include_patterns,
      exclude_patterns: Array.isArray(checkConfig.exclude_patterns)
        ? checkConfig.exclude_patterns.filter((p): p is string => typeof p === 'string')
        : DEFAULT_CONFIG.exclude_patterns,
    };
  }

  /**
   * Parse scope option
   */
  private parseScope(value: unknown): 'affected' | 'full' {
    if (value === 'full' || value === 'affected') {
      return value;
    }
    return DEFAULT_CONFIG.scope;
  }

  /**
   * Parse severity threshold
   */
  private parseSeverity(value: unknown): 'error' | 'warn' | 'info' | undefined {
    if (typeof value !== 'string') return undefined;
    const lower = value.toLowerCase();
    if (lower === 'error' || lower === 'warn' || lower === 'info') {
      return lower;
    }
    return undefined;
  }

  /**
   * Get files to cruise based on scope
   */
  private getFilesToCruise(
    context: CheckContext,
    config: Required<ArchitectureCheckConfig>
  ): string[] {
    if (config.scope === 'full') {
      // Return the include patterns for full scan
      return config.include_patterns;
    }

    // For 'affected', use files from plan
    const files: string[] = [];

    for (const change of context.plan.proposed_changes) {
      const isRelevantChange =
        change.type === 'file_create' ||
        change.type === 'file_update' ||
        change.type === 'file_delete';

      if (isRelevantChange && change.path && this.isAnalysableFile(change.path, config)) {
        const fullPath = join(context.workspace_root, change.path);
        if (existsSync(fullPath) || change.type === 'file_delete') {
          files.push(fullPath);
        }
      }
    }

    // If no files from plan match, fall back to include patterns
    if (files.length === 0) {
      return config.include_patterns;
    }

    return files;
  }

  /**
   * Check if a file should be analysed
   */
  private isAnalysableFile(filePath: string, config: Required<ArchitectureCheckConfig>): boolean {
    const analysableExtensions = ['.js', '.ts', '.jsx', '.tsx', '.mjs', '.cjs'];
    const hasValidExtension = analysableExtensions.some((ext) => filePath.endsWith(ext));

    if (!hasValidExtension) return false;

    // Check exclusions
    for (const pattern of config.exclude_patterns) {
      if (this.matchesGlobPattern(filePath, pattern)) {
        return false;
      }
    }

    return true;
  }

  /**
   * Simple glob pattern matching (for common patterns)
   */
  private matchesGlobPattern(filePath: string, pattern: string): boolean {
    // Convert glob to regex (simplified)
    const regexPattern = pattern
      .replace(/\*\*/g, '{{GLOBSTAR}}')
      .replace(/\*/g, '[^/]*')
      .replace(/{{GLOBSTAR}}/g, '.*')
      .replace(/\?/g, '.');

    const regex = new RegExp(regexPattern);
    return regex.test(filePath);
  }

  /**
   * Get default cruise options when no config file exists
   */
  private getDefaultCruiseOptions(): Record<string, unknown> {
    return {
      validate: true,
      ruleSet: {
        forbidden: [
          {
            name: 'no-circular',
            severity: 'error',
            comment: 'Circular dependencies are not allowed',
            from: {},
            to: {
              circular: true,
            },
          },
          {
            name: 'no-orphans',
            severity: 'warn',
            comment: 'Modules without dependents or dependencies',
            from: {
              orphan: true,
              pathNot: [
                '\\.d\\.ts$',
                '\\.test\\.(ts|js)$',
                '\\.spec\\.(ts|js)$',
                'index\\.(ts|js)$',
              ],
            },
            to: {},
          },
        ],
      },
    };
  }

  /**
   * Calculate score based on violations
   */
  private calculateScore(
    violations: CruiserViolation[],
    config: Required<ArchitectureCheckConfig>
  ): {
    score: number;
    passed: boolean;
    violationsByType: Record<string, number>;
  } {
    const violationsByType: Record<string, number> = {
      circular: 0,
      orphan: 0,
      layer: 0,
      other: 0,
    };

    let totalPenalty = 0;
    let hasBlockingViolation = false;

    for (const v of violations) {
      // Categorise violation
      if (v.cycle && v.cycle.length > 0) {
        violationsByType.circular++;
        if (
          config.fail_on_circular &&
          this.isBlockingSeverity(v.rule.severity, config.severity_threshold)
        ) {
          hasBlockingViolation = true;
        }
      } else if (v.rule.name.includes('orphan')) {
        violationsByType.orphan++;
        if (
          config.fail_on_orphan &&
          this.isBlockingSeverity(v.rule.severity, config.severity_threshold)
        ) {
          hasBlockingViolation = true;
        }
      } else if (v.rule.name.includes('layer') || v.rule.name.includes('boundary')) {
        violationsByType.layer++;
        if (this.isBlockingSeverity(v.rule.severity, config.severity_threshold)) {
          hasBlockingViolation = true;
        }
      } else {
        violationsByType.other++;
        if (this.isBlockingSeverity(v.rule.severity, config.severity_threshold)) {
          hasBlockingViolation = true;
        }
      }

      // Calculate penalty
      totalPenalty += SEVERITY_PENALTIES[v.rule.severity] || 0;
    }

    const score = Math.max(0, 100 - totalPenalty);
    const passed = !hasBlockingViolation;

    return { score, passed, violationsByType };
  }

  /**
   * Check if a severity level should block the check
   */
  private isBlockingSeverity(
    severity: 'error' | 'warn' | 'info' | 'ignore',
    threshold: 'error' | 'warn' | 'info'
  ): boolean {
    const levels = { error: 3, warn: 2, info: 1, ignore: 0 };
    return levels[severity] >= levels[threshold];
  }

  /**
   * Build human-readable message
   */
  private buildMessage(
    violations: CruiserViolation[],
    totalCruised: number,
    passed: boolean
  ): string {
    if (violations.length === 0) {
      return `Architecture check passed: ${totalCruised} modules analysed, no violations`;
    }

    const errorCount = violations.filter((v) => v.rule.severity === 'error').length;
    const warnCount = violations.filter((v) => v.rule.severity === 'warn').length;
    const infoCount = violations.filter((v) => v.rule.severity === 'info').length;

    const parts: string[] = [];
    if (errorCount > 0) parts.push(`${errorCount} error${errorCount > 1 ? 's' : ''}`);
    if (warnCount > 0) parts.push(`${warnCount} warning${warnCount > 1 ? 's' : ''}`);
    if (infoCount > 0) parts.push(`${infoCount} info`);

    const status = passed ? 'passed with issues' : 'failed';
    return `Architecture check ${status}: ${parts.join(', ')} (${totalCruised} modules analysed)`;
  }
}
