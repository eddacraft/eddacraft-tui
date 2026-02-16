/**
 * Layer Validator - Score calculation and validation for architecture violations
 *
 * Handles scoring, pass/fail determination, and severity-based blocking.
 */

import type { CruiserViolation } from './dependency-analyzer.js';

/**
 * Configuration for architecture check
 */
export interface ArchitectureCheckConfig {
  /** Path to dependency-cruiser config (default: .anvil/dependency-cruiser.js) */
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
 * Default configuration
 */
export const DEFAULT_CONFIG: Required<ArchitectureCheckConfig> = {
  config_file: '.anvil/dependency-cruiser.js',
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
 * Result of score calculation
 */
export interface ScoreResult {
  score: number;
  passed: boolean;
  violationsByType: Record<string, number>;
}

/**
 * Layer validator for architecture constraints
 */
export class LayerValidator {
  /**
   * Parse and validate check configuration
   */
  parseConfig(checkConfig: Record<string, unknown>): Required<ArchitectureCheckConfig> {
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
   * Calculate score based on violations
   */
  calculateScore(
    violations: CruiserViolation[],
    config: Required<ArchitectureCheckConfig>
  ): ScoreResult {
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
   * Check if a file should be analysed
   */
  isAnalysableFile(filePath: string, config: Required<ArchitectureCheckConfig>): boolean {
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
    // Normalise to forward slashes for consistent cross-platform matching
    const normalizedPath = filePath.replace(/\\/g, '/');

    // Convert glob to regex (simplified)
    const regexPattern = pattern
      .replace(/\*\*/g, '{{GLOBSTAR}}')
      .replace(/\*/g, '[^/]*')
      .replace(/{{GLOBSTAR}}/g, '.*')
      .replace(/\?/g, '.');

    const regex = new RegExp(regexPattern);
    return regex.test(normalizedPath);
  }

  /**
   * Build human-readable message
   */
  buildMessage(violations: CruiserViolation[], totalCruised: number, passed: boolean): string {
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
