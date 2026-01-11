/**
 * Architecture Check - Validate architectural constraints using dependency-cruiser
 *
 * Detects:
 * - Circular dependencies
 * - Layer/boundary violations
 * - Orphaned modules (optional)
 */

import { BaseCheck } from '../check.interface.js';
import { CheckContext, GateResult, getFilesFromContext } from '../../types/gate.types.js';
import { existsSync } from 'fs';
import { join } from 'path';
import { minimatch } from 'minimatch';
import { loadBaseline } from '../../architecture/baseline.js';
import type { ArchitectureBaseline } from '../../architecture/types.js';
import {
  createContextBuilder,
  type ArchitectureContext,
  type ArchViolation,
} from '../../architecture/context.js';
import {
  createWarningResult,
  createWarningFingerprint,
  type Warning,
  type WarningResult,
  type WarningSeverity,
} from '../../antipattern/types.js';
import { DependencyAnalyzer, type CruiserViolation } from './architecture/dependency-analyzer.js';
import { CircularDetector } from './architecture/circular-detector.js';
import { LayerValidator, type ArchitectureCheckConfig } from './architecture/layer-validator.js';

// Re-export configuration type for backwards compatibility
export type { ArchitectureCheckConfig } from './architecture/layer-validator.js';

const ARCH_VIOLATION_IDS = {
  circular: 'ARCH-001',
  orphan: 'ARCH-002',
  layer: 'ARCH-003',
  other: 'ARCH-004',
} as const;

const ARCH_VIOLATION_TITLES = {
  'ARCH-001': 'Circular dependency detected',
  'ARCH-002': 'Orphaned module',
  'ARCH-003': 'Layer/boundary violation',
  'ARCH-004': 'Architecture violation',
} as const;

const ARCH_VIOLATION_EXPLANATIONS = {
  'ARCH-001':
    'Circular dependencies make code harder to understand and can cause issues with module loading.',
  'ARCH-002':
    'This module has no dependents or dependencies, which may indicate dead code or missing integration.',
  'ARCH-003': 'This import crosses an architectural boundary that should not be crossed.',
  'ARCH-004': 'This import violates a configured architecture rule.',
} as const;

const ARCH_VIOLATION_SUGGESTIONS = {
  'ARCH-001':
    'Break the cycle by extracting shared code into a separate module or using dependency injection.',
  'ARCH-002': 'Either connect this module to your application or remove it if unused.',
  'ARCH-003': 'Move the import target to an appropriate layer or adjust the boundary definition.',
  'ARCH-004': 'Review the dependency-cruiser rule and adjust your code or configuration.',
} as const;

/**
 * Architecture check that validates project structure using dependency-cruiser
 */
export class ArchitectureCheck extends BaseCheck {
  name = 'architecture';
  description = 'Validate architectural constraints using dependency-cruiser';

  private analyzer = new DependencyAnalyzer();
  private detector = new CircularDetector();
  private validator = new LayerValidator();

  async run(context: CheckContext): Promise<GateResult> {
    const config = this.validator.parseConfig(context.check_config);

    try {
      // Step 1: Load dependency-cruiser
      const loadResult = await this.analyzer.loadCruiser();
      if (!loadResult.success) {
        return this.createSuccess(
          'dependency-cruiser not installed. Run `npm install -D dependency-cruiser` to enable architecture checks.',
          100,
          {
            skipped: true,
            reason: 'dependency-cruiser not available',
            warnings: createWarningResult([], []),
          }
        );
      }

      // Step 2: Load dependency-cruiser config or use defaults
      const configPath = join(context.workspace_root, config.config_file);
      let cruiseOptions: Record<string, unknown> | null = await this.analyzer.loadConfig(
        context.workspace_root,
        config.config_file
      );

      if (!cruiseOptions) {
        if (existsSync(configPath)) {
          return this.createFailure(
            `Failed to load dependency-cruiser config: ${config.config_file}`,
            'Config file exists but could not be loaded'
          );
        }
        // Use default rules for circular dependency and orphan detection
        cruiseOptions = this.analyzer.getDefaultCruiseOptions();
      }

      // Step 3: Determine files to analyse
      const effectiveConfig = context.fullScan ? { ...config, scope: 'full' as const } : config;
      const filesToCruise = this.getFilesToCruise(context, effectiveConfig);

      if (filesToCruise.length === 0) {
        return this.createSuccess('No files to analyse for architecture violations', 100, {
          warnings: createWarningResult([], []),
        });
      }

      // Step 4: Run dependency analysis
      const analysisResult = await this.analyzer.analyze(filesToCruise, cruiseOptions);

      if (!analysisResult.success || !analysisResult.result) {
        if (analysisResult.skipped) {
          return this.createSuccess(analysisResult.reason || 'Analysis skipped', 100, {
            skipped: true,
            reason: analysisResult.reason,
            warnings: createWarningResult([], []),
          });
        }
        return this.createFailure(
          'Dependency analysis failed',
          analysisResult.error || 'Unknown error'
        );
      }

      const output = analysisResult.result;

      // Step 5: Load baseline if exists (for new-only mode)
      const baseline = loadBaseline(context.workspace_root);

      // Step 6: Convert ALL violations to Warning format with drift info
      const allViolations = output.summary.violations;
      const allWarnings = allViolations.map((v) => this.convertViolationToWarning(v, baseline));

      // Step 7: In new-only mode (when baseline exists), filter to NEW violations only
      const warnings = baseline ? allWarnings.filter((w) => w.drift?.isNew !== false) : allWarnings;

      // Step 8: Calculate score/passed from the effective warnings
      const effectiveViolations = this.detector.filterNewViolations(allViolations, baseline);
      const { score, passed, violationsByType } = this.validator.calculateScore(
        effectiveViolations,
        config
      );

      const warningResult = this.createArchWarningResult(warnings, violationsByType);
      const message = this.validator.buildMessage(
        effectiveViolations,
        output.summary.totalCruised,
        passed
      );

      const architectureContext = this.buildArchitectureContext(
        allViolations,
        effectiveViolations,
        output.summary,
        violationsByType,
        baseline,
        config
      );

      return this.createResult(passed, message, score, {
        warnings: warningResult,
        totalModulesCruised: output.summary.totalCruised,
        violationCount: allViolations.length,
        newViolationCount: effectiveViolations.length,
        errorCount: output.summary.error,
        warnCount: output.summary.warn,
        infoCount: output.summary.info,
        violations: allViolations.map((v) => ({
          from: v.from,
          to: v.to,
          rule: v.rule.name,
          severity: v.rule.severity,
          cycle: v.cycle,
        })),
        violationsByType,
        configFile: existsSync(configPath) ? config.config_file : 'built-in defaults',
        scope: config.scope,
        baselineLoaded: baseline !== null,
        architectureContext,
      });
    } catch (error) {
      return this.createFailure(
        'Architecture check failed unexpectedly',
        error instanceof Error ? error.message : 'Unknown error'
      );
    }
  }

  private convertViolationToWarning(
    violation: CruiserViolation,
    baseline: ArchitectureBaseline | null
  ): Warning {
    const violationType = this.detector.categoriseViolation(violation);
    const id = ARCH_VIOLATION_IDS[violationType];
    const severity = this.mapCruiserSeverity(violation.rule.severity);

    const warning: Warning = {
      id,
      category: 'architecture',
      severity,
      confidence: 'high',
      title: ARCH_VIOLATION_TITLES[id],
      message: `${violation.from} → ${violation.to} (${violation.rule.name})`,
      explanation: ARCH_VIOLATION_EXPLANATIONS[id],
      suggestion: ARCH_VIOLATION_SUGGESTIONS[id],
      location: {
        file: violation.from,
        line: 1,
      },
      pattern: violation.rule.name,
    };

    warning.fingerprint = createWarningFingerprint(warning);

    if (baseline) {
      const isNew = this.detector.isNewViolation(violation, baseline);
      warning.drift = {
        isNew,
        existingCount: baseline.baseline_snapshot.violations.length,
      };
    }

    return warning;
  }

  private mapCruiserSeverity(severity: 'error' | 'warn' | 'info' | 'ignore'): WarningSeverity {
    if (severity === 'error') return 'error';
    if (severity === 'warn') return 'warning';
    return 'info';
  }

  private createArchWarningResult(
    warnings: Warning[],
    violationsByType: Record<string, number>
  ): WarningResult {
    const patternsChecked = Object.keys(violationsByType).map(
      (type) => ARCH_VIOLATION_IDS[type as keyof typeof ARCH_VIOLATION_IDS]
    );
    return createWarningResult(warnings, patternsChecked);
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

    // Use unified helper for both planless and plan-based modes
    const files = getFilesFromContext(context, {
      filter: (f) => this.validator.isAnalysableFile(f, config),
    });

    // If no files match, fall back to include patterns
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

  private matchesGlobPattern(filePath: string, pattern: string): boolean {
    return minimatch(filePath, pattern, { dot: true });
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

  private buildArchitectureContext(
    allViolations: CruiserViolation[],
    effectiveViolations: CruiserViolation[],
    summary: { totalCruised: number; error: number; warn: number; info: number },
    violationsByType: Record<string, number>,
    baseline: ArchitectureBaseline | null,
    config: Required<ArchitectureCheckConfig>
  ): ArchitectureContext {
    const builder = createContextBuilder();

    const archViolations: ArchViolation[] = allViolations.map((v) => ({
      from: v.from,
      to: v.to,
      rule: v.rule.name,
      severity: v.rule.severity,
      is_circular: v.cycle !== undefined && v.cycle.length > 0,
      cycle: v.cycle,
      is_new: baseline ? this.detector.isNewViolation(v, baseline) : true,
      from_layer: null,
      to_layer: null,
    }));

    builder.setViolations(archViolations);

    builder.setSummary({
      total_modules: summary.totalCruised,
      total_violations: allViolations.length,
      new_violations: effectiveViolations.length,
      error_count: summary.error,
      warn_count: summary.warn,
      info_count: summary.info,
      circular_count: violationsByType.circular ?? 0,
      orphan_count: violationsByType.orphan ?? 0,
      layer_violation_count: violationsByType.layer ?? 0,
      baseline_loaded: baseline !== null,
    });

    builder.setConfig({
      config_file: config.config_file,
      scope: config.scope,
      severity_threshold: config.severity_threshold,
    });

    return builder.build();
  }
}
