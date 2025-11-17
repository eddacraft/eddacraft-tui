import { Check } from './check.interface.js';
import {
  CheckContext,
  GateConfig,
  GateResult,
  GateRunResult,
  PlanData,
} from '../types/gate.types.js';
import { ESLintCheck } from './checks/eslint.check.js';
import { CoverageCheck } from './checks/coverage.check.js';
import { SecretCheck } from './checks/secret.check.js';

export class GateRunner {
  private checks: Map<string, Check> = new Map();

  constructor() {
    this.registerDefaultChecks();
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
    options?: {
      skipChecks?: string[];
      onlyChecks?: string[];
      failFast?: boolean;
    }
  ): Promise<GateRunResult> {
    const results: GateResult[] = [];
    let totalScore = 0;
    let validChecks = 0;

    for (const checkConfig of config.checks) {
      // Apply skip/only filters
      if (options?.skipChecks?.includes(checkConfig.name)) {
        results.push({
          check: checkConfig.name,
          passed: true,
          message: 'Check skipped via --skip-checks',
          skipped: true,
        });
        continue;
      }

      if (options?.onlyChecks && !options.onlyChecks.includes(checkConfig.name)) {
        results.push({
          check: checkConfig.name,
          passed: true,
          message: 'Check not in --only-checks filter',
          skipped: true,
        });
        continue;
      }

      if (!checkConfig.enabled) {
        results.push({
          check: checkConfig.name,
          passed: true,
          message: 'Check disabled',
          skipped: true,
        });
        continue;
      }

      const check = this.checks.get(checkConfig.name);
      if (!check) {
        results.push({
          check: checkConfig.name,
          passed: false,
          message: `Check '${checkConfig.name}' not found`,
          error: 'Unknown check',
        });
        continue;
      }

      try {
        const context: CheckContext = {
          plan,
          workspace_root: workspaceRoot,
          config,
          check_config: checkConfig.config || {},
        };

        const result = await check.run(context);
        results.push(result);

        if (result.score !== undefined) {
          totalScore += result.score;
          validChecks++;
        }

        // Fail-fast: stop on first failure
        if (options?.failFast && !result.passed && !result.skipped) {
          break;
        }
      } catch (error) {
        const errorResult: GateResult = {
          check: checkConfig.name,
          passed: false,
          message: `Check '${checkConfig.name}' failed with error`,
          error: error instanceof Error ? error.message : 'Unknown error',
        };
        results.push(errorResult);

        // Fail-fast: stop on error
        if (options?.failFast) {
          break;
        }
      }
    }

    const overallScore = validChecks > 0 ? totalScore / validChecks : 100; // Default to 100 if no checks ran
    const passed = results.every((r) => r.passed || r.skipped);
    const overallPassed =
      passed && (validChecks === 0 || overallScore >= (config.thresholds.overall_score || 80));

    const summary = {
      total: results.length,
      passed: results.filter((r) => r.passed && !r.skipped).length,
      failed: results.filter((r) => !r.passed && !r.skipped).length,
      skipped: results.filter((r) => r.skipped).length,
    };

    return {
      overall: overallPassed,
      score: overallScore,
      checks: results,
      summary,
    };
  }

  private registerDefaultChecks(): void {
    this.registerCheck(new ESLintCheck());
    this.registerCheck(new CoverageCheck());
    this.registerCheck(new SecretCheck());
  }
}
