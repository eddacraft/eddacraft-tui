import { BaseCheck } from '../check.interface.js';
import { CheckContext, GateResult } from '../../types/gate.types.js';
import { readFileSync, existsSync } from 'fs';
import { join } from 'path';

interface CoverageMetric {
  total: number;
  covered: number;
  skipped: number;
  pct: number;
}

interface CoverageData {
  total: {
    lines: CoverageMetric;
    functions: CoverageMetric;
    branches: CoverageMetric;
    statements: CoverageMetric;
    [key: string]: CoverageMetric;
  };
}

interface CoverageAnalysisResult extends Record<string, unknown> {
  overall: number;
  details: Record<
    string,
    {
      coverage: number;
      threshold: number;
      passed: boolean;
    }
  >;
  passed: boolean;
  failed_checks: string[];
}

export class CoverageCheck extends BaseCheck {
  name = 'coverage';
  description = 'Check test coverage thresholds';

  async run(context: CheckContext): Promise<GateResult> {
    try {
      const coveragePath = join(context.workspace_root, 'coverage', 'coverage-summary.json');

      if (!existsSync(coveragePath)) {
        return this.createFailure('Coverage report not found', 'Run tests with coverage first');
      }

      const coverageData: CoverageData = JSON.parse(readFileSync(coveragePath, 'utf-8'));
      const defaultThresholds: Record<string, number> = {
        lines: 80,
        functions: 80,
        branches: 80,
        statements: 80,
      };
      const thresholds =
        typeof context.check_config.thresholds === 'object' &&
        context.check_config.thresholds !== null
          ? (context.check_config.thresholds as Record<string, number>)
          : defaultThresholds;

      const results = this.analyzeCoverage(coverageData, thresholds);
      const overallScore = results.overall;
      const minScore =
        typeof context.check_config.min_score === 'number' ? context.check_config.min_score : 80;
      const passed = overallScore >= minScore;

      const message = passed
        ? `Coverage passed: ${overallScore.toFixed(1)}% overall`
        : `Coverage failed: ${overallScore.toFixed(1)}% overall (required: ${minScore}%)`;

      return this.createResult(passed, message, overallScore, results);
    } catch (error) {
      return this.createFailure(
        'Coverage check failed',
        error instanceof Error ? error.message : 'Unknown error'
      );
    }
  }

  private analyzeCoverage(
    coverageData: CoverageData,
    thresholds: Record<string, number>
  ): CoverageAnalysisResult {
    const totals = coverageData.total;
    const results: CoverageAnalysisResult = {
      overall: 0,
      details: {},
      passed: true,
      failed_checks: [],
    };

    // Calculate overall coverage as average of all metrics
    const metrics = ['lines', 'functions', 'branches', 'statements'];
    let totalCoverage = 0;
    let validMetrics = 0;

    for (const metric of metrics) {
      if (totals[metric]) {
        const coverage = totals[metric].pct;
        const threshold = thresholds[metric] || 0;

        results.details[metric] = {
          coverage,
          threshold,
          passed: coverage >= threshold,
        };

        totalCoverage += coverage;
        validMetrics++;

        if (coverage < threshold) {
          results.passed = false;
          results.failed_checks.push(`${metric}: ${coverage}% < ${threshold}%`);
        }
      }
    }

    results.overall = validMetrics > 0 ? totalCoverage / validMetrics : 0;
    return results;
  }
}
