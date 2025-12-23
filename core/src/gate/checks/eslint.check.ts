import { BaseCheck } from '../check.interface.js';
import { CheckContext, GateResult, getFilesFromContext } from '../../types/gate.types.js';
import { ESLint } from 'eslint';

export class ESLintCheck extends BaseCheck {
  name = 'eslint';
  description = 'Run ESLint code quality checks';

  async run(context: CheckContext): Promise<GateResult> {
    try {
      const eslint = new ESLint({
        cwd: context.workspace_root,
      });

      // Get files to lint - either from plan or full codebase scan
      const files = context.fullScan
        ? await this.getFilesForFullScan(context.workspace_root)
        : this.getFilesFromPlan(context);

      if (files.length === 0) {
        return this.createSuccess('No files to lint', 100);
      }

      const results = await eslint.lintFiles(files);

      const errorCount = results.reduce((acc, result) => acc + result.errorCount, 0);
      const warningCount = results.reduce((acc, result) => acc + result.warningCount, 0);
      const fixableCount = results.reduce(
        (acc, result) => acc + result.fixableErrorCount + result.fixableWarningCount,
        0
      );

      const totalIssues = errorCount + warningCount;
      const score =
        totalIssues === 0 ? 100 : Math.max(0, 100 - (errorCount * 10 + warningCount * 2));

      const minScore =
        typeof context.check_config.min_score === 'number' ? context.check_config.min_score : 80;
      const passed = errorCount === 0 && score >= minScore;

      const message = passed
        ? `ESLint passed: ${errorCount} errors, ${warningCount} warnings`
        : `ESLint failed: ${errorCount} errors, ${warningCount} warnings`;

      return this.createResult(passed, message, score, {
        errorCount,
        warningCount,
        fixableCount,
        results: results.map((r) => ({
          filePath: r.filePath,
          errorCount: r.errorCount,
          warningCount: r.warningCount,
          messages: r.messages,
        })),
      });
    } catch (error) {
      return this.createFailure(
        'ESLint check failed',
        error instanceof Error ? error.message : 'Unknown error'
      );
    }
  }

  private getFilesFromPlan(context: CheckContext): string[] {
    // Use unified helper for both planless and plan-based modes
    // This ensures consistent path normalisation and existence checking
    return getFilesFromContext(context, {
      filter: (f) => this.isLintableFile(f),
    });
  }

  private isLintableFile(filePath: string): boolean {
    const lintableExtensions = ['.js', '.ts', '.jsx', '.tsx', '.mjs', '.cjs'];
    return lintableExtensions.some((ext) => filePath.endsWith(ext));
  }

  /**
   * Get all lintable files for full codebase scan
   */
  private async getFilesForFullScan(workspaceRoot: string): Promise<string[]> {
    // Use ESLint's file resolution - pass the workspace root
    // ESLint will use its own config to determine which files to lint
    return [workspaceRoot];
  }
}
