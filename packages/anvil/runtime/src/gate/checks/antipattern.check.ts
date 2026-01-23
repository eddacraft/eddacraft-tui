/**
 * Anti-pattern Check - Detect AI escape hatches and code quality issues
 *
 * Detects:
 * - ESLint disable comments (broad and rule-specific)
 * - Type escapes (any, @ts-ignore, @ts-expect-error)
 * - Empty catch blocks
 * - Console statements in production code
 */

import { readFileSync } from 'fs';
import * as path from 'node:path';
import { BaseCheck } from '../check.interface.js';
import { CheckContext, GateResult, getFilesFromContext } from '../../types/gate.types.js';
import { scanFile, type ScanOptions, type ScanResult } from '@eddacraft/anvil-core/antipattern';
import {
  createWarningResult,
  type Warning,
  type WarningResult,
} from '@eddacraft/anvil-core/antipattern';
import { parseSeverity } from '@eddacraft/anvil-core';

export interface AntipatternCheckConfig {
  patterns?: string[];
  includeOptIn?: boolean;
  extensions?: string[];
  severityThreshold?: 'error' | 'warning' | 'info';
}

const DEFAULT_CONFIG: Required<AntipatternCheckConfig> = {
  patterns: [],
  includeOptIn: false,
  extensions: ['.ts', '.tsx', '.js', '.jsx', '.mjs', '.cjs'],
  severityThreshold: 'error',
};

const SEVERITY_PENALTIES = {
  error: 15,
  warning: 5,
  info: 1,
};

export class AntipatternCheck extends BaseCheck {
  name = 'antipattern';
  description = 'Detect AI escape hatches and code quality anti-patterns';

  async run(context: CheckContext): Promise<GateResult> {
    const config = this.parseConfig(context.check_config);

    try {
      const files = getFilesFromContext(context, {
        filter: (f) => this.isScannableFile(f, config),
      });

      if (files.length === 0) {
        return this.createSuccess('No files to scan for anti-patterns', 100, {
          warnings: createWarningResult([], []),
          filesScanned: 0,
        });
      }

      const scanOptions: ScanOptions = {
        patterns: config.patterns.length > 0 ? config.patterns : undefined,
        includeOptIn: config.includeOptIn,
      };

      const allWarnings: Warning[] = [];
      const allPatternsChecked: Set<string> = new Set();
      let filesScanned = 0;

      for (const filePath of files) {
        let content: string;
        try {
          content = readFileSync(filePath, 'utf-8');
        } catch {
          continue;
        }

        const relativePath = path.isAbsolute(filePath)
          ? path.relative(context.workspace_root, filePath)
          : filePath;
        const result: ScanResult = scanFile(relativePath, content, scanOptions);

        allWarnings.push(...result.warnings);
        result.patternsChecked.forEach((p) => allPatternsChecked.add(p));
        filesScanned++;
      }

      const warningResult = createWarningResult(allWarnings, Array.from(allPatternsChecked));
      const { score, passed } = this.calculateScore(allWarnings, config);
      const message = this.buildMessage(warningResult, filesScanned, passed);

      return this.createResult(passed, message, score, {
        warnings: warningResult,
        filesScanned,
        patternsChecked: Array.from(allPatternsChecked),
      });
    } catch (error) {
      return this.createFailure(
        'Anti-pattern check failed unexpectedly',
        error instanceof Error ? error.message : 'Unknown error'
      );
    }
  }

  private parseConfig(checkConfig: Record<string, unknown>): Required<AntipatternCheckConfig> {
    return {
      patterns: Array.isArray(checkConfig.patterns)
        ? checkConfig.patterns.filter((p): p is string => typeof p === 'string')
        : DEFAULT_CONFIG.patterns,
      includeOptIn: checkConfig.includeOptIn === true,
      extensions: Array.isArray(checkConfig.extensions)
        ? checkConfig.extensions.filter((e): e is string => typeof e === 'string')
        : DEFAULT_CONFIG.extensions,
      severityThreshold: parseSeverity(
        checkConfig.severityThreshold,
        DEFAULT_CONFIG.severityThreshold
      ) as 'error' | 'warning' | 'info',
    };
  }

  private isScannableFile(filePath: string, config: Required<AntipatternCheckConfig>): boolean {
    return config.extensions.some((ext) => filePath.endsWith(ext));
  }

  private calculateScore(
    warnings: Warning[],
    config: Required<AntipatternCheckConfig>
  ): { score: number; passed: boolean } {
    const severityLevels = { error: 3, warning: 2, info: 1 };
    const threshold = severityLevels[config.severityThreshold];

    let totalPenalty = 0;
    let hasBlockingWarning = false;

    for (const warning of warnings) {
      if (warning.suppressed) continue;

      const warningSeverity = severityLevels[warning.severity];
      if (warningSeverity >= threshold) {
        hasBlockingWarning = true;
      }

      totalPenalty += SEVERITY_PENALTIES[warning.severity] || 0;
    }

    const score = Math.max(0, 100 - totalPenalty);
    const passed = !hasBlockingWarning;

    return { score, passed };
  }

  private buildMessage(result: WarningResult, filesScanned: number, passed: boolean): string {
    if (result.warnings.length === 0) {
      return `Anti-pattern check passed: ${filesScanned} files scanned, no issues found`;
    }

    const parts: string[] = [];
    if (result.summary.errors > 0) {
      parts.push(`${result.summary.errors} error${result.summary.errors > 1 ? 's' : ''}`);
    }
    if (result.summary.warnings > 0) {
      parts.push(`${result.summary.warnings} warning${result.summary.warnings > 1 ? 's' : ''}`);
    }
    if (result.summary.info > 0) {
      parts.push(`${result.summary.info} info`);
    }
    if (result.summary.suppressed > 0) {
      parts.push(`${result.summary.suppressed} suppressed`);
    }

    const status = passed ? 'passed with issues' : 'failed';
    return `Anti-pattern check ${status}: ${parts.join(', ')} (${filesScanned} files scanned)`;
  }
}
