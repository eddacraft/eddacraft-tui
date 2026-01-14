import type { Warning } from '@anvil/core';

/**
 * Quick win type categorization
 */
export type QuickWinType =
  | 'test-file'
  | 'type-definition'
  | 'config-file'
  | 'generated-code'
  | 'migration'
  | 'third-party'
  | 'legacy-code';

/**
 * Suppression template for a specific pattern
 */
export interface SuppressionTemplate {
  /** Pattern ID (e.g., AP-003) */
  patternId: string;
  /** Context type */
  context: QuickWinType;
  /** Suggested suppression reason */
  reason: string;
  /** Confidence that suppression is appropriate (0-1) */
  confidence: number;
}

/**
 * A quick win - an issue that can be easily suppressed or fixed
 */
export interface QuickWin {
  /** The original warning */
  warning: Warning;
  /** Type of quick win */
  type: QuickWinType;
  /** Suggested suppression reason */
  suggestedReason: string;
  /** Confidence score (0-1) */
  confidence: number;
  /** Whether this can be batch suppressed with similar issues */
  batchable: boolean;
  /** Batch key for grouping */
  batchKey?: string;
}

/**
 * Batch suppression group
 */
export interface BatchGroup {
  /** Unique key for this batch */
  key: string;
  /** Pattern ID */
  patternId: string;
  /** Type of quick win */
  type: QuickWinType;
  /** Warnings in this batch */
  warnings: Warning[];
  /** Suggested reason for batch suppression */
  suggestedReason: string;
  /** Number of occurrences */
  count: number;
}

/**
 * Analysis result from quick wins identifier
 */
export interface QuickWinsAnalysis {
  /** Individual quick wins */
  quickWins: QuickWin[];
  /** Batch suppression groups */
  batchGroups: BatchGroup[];
  /** Total warnings analyzed */
  totalWarnings: number;
  /** Number of suppressable warnings */
  suppressable: number;
  /** Percentage suppressable */
  suppressablePercent: number;
}

/**
 * Service for identifying easy wins in check results
 */
export class QuickWinsIdentifier {
  /**
   * Analyse warnings to find quick wins
   */
  public analyse(warnings: Warning[]): QuickWinsAnalysis {
    const quickWins: QuickWin[] = [];

    for (const warning of warnings) {
      const quickWin = this.identifyQuickWin(warning);
      if (quickWin) {
        quickWins.push(quickWin);
      }
    }

    const batchGroups = this.createBatchGroups(quickWins);

    return {
      quickWins,
      batchGroups,
      totalWarnings: warnings.length,
      suppressable: quickWins.length,
      suppressablePercent: warnings.length > 0 ? (quickWins.length / warnings.length) * 100 : 0,
    };
  }

  /**
   * Identify if a warning is a quick win
   */
  private identifyQuickWin(warning: Warning): QuickWin | null {
    // Skip already suppressed warnings
    if (warning.suppressed) {
      return null;
    }

    const file = warning.location.file;

    // Check for test files
    if (this.isTestFile(file)) {
      return {
        warning,
        type: 'test-file',
        suggestedReason: this.generateTestFileReason(warning),
        confidence: 0.9,
        batchable: true,
        batchKey: `test-${warning.id}`,
      };
    }

    // Check for type definition files
    if (this.isTypeDefinitionFile(file)) {
      return {
        warning,
        type: 'type-definition',
        suggestedReason: this.generateTypeDefinitionReason(warning),
        confidence: 0.95,
        batchable: true,
        batchKey: `types-${warning.id}`,
      };
    }

    // Check for config files
    if (this.isConfigFile(file)) {
      return {
        warning,
        type: 'config-file',
        suggestedReason: this.generateConfigFileReason(warning),
        confidence: 0.85,
        batchable: true,
        batchKey: `config-${warning.id}`,
      };
    }

    // Check for generated code
    if (this.isGeneratedCode(file)) {
      return {
        warning,
        type: 'generated-code',
        suggestedReason: this.generateGeneratedCodeReason(warning),
        confidence: 0.98,
        batchable: true,
        batchKey: `generated-${warning.id}`,
      };
    }

    // Check for third-party patterns
    if (this.isThirdPartyContext(warning)) {
      return {
        warning,
        type: 'third-party',
        suggestedReason: this.generateThirdPartyReason(warning),
        confidence: 0.8,
        batchable: false,
      };
    }

    // Check for migration indicators
    if (this.isMigrationContext(warning)) {
      return {
        warning,
        type: 'migration',
        suggestedReason: this.generateMigrationReason(warning),
        confidence: 0.7,
        batchable: false,
      };
    }

    return null;
  }

  /**
   * Check if file is a test file
   */
  private isTestFile(file: string): boolean {
    const testPatterns = [
      /\.test\.(ts|tsx|js|jsx)$/,
      /\.spec\.(ts|tsx|js|jsx)$/,
      /__tests__\//,
      /__mocks__\//,
      /\/tests?\//,
      /\/spec\//,
    ];

    return testPatterns.some((pattern) => pattern.test(file));
  }

  /**
   * Check if file is a type definition
   */
  private isTypeDefinitionFile(file: string): boolean {
    return file.endsWith('.d.ts');
  }

  /**
   * Check if file is a config file
   */
  private isConfigFile(file: string): boolean {
    const configPatterns = [
      /\.config\.(ts|js|mjs|cjs)$/,
      /^(webpack|vite|rollup|jest|vitest|babel|tsconfig|eslint)\.config/,
      /^next\.config\./,
      /^tailwind\.config\./,
    ];

    return configPatterns.some((pattern) => pattern.test(file));
  }

  /**
   * Check if file is generated code
   */
  private isGeneratedCode(file: string): boolean {
    const generatedPatterns = [
      /\.generated\./,
      /\/generated\//,
      /__generated__\//,
      /\.g\.(ts|js)$/,
      /\/dist\//,
      /\/build\//,
    ];

    return generatedPatterns.some((pattern) => pattern.test(file));
  }

  /**
   * Check if warning is in third-party context
   */
  private isThirdPartyContext(warning: Warning): boolean {
    // Check message/explanation for third-party indicators
    const thirdPartyIndicators = [
      'third-party',
      'external library',
      'npm package',
      'dependency',
      'vendor',
      'sdk',
      'api client',
    ];

    const text = `${warning.message} ${warning.explanation} ${warning.location.file}`.toLowerCase();

    return thirdPartyIndicators.some((indicator) => text.includes(indicator));
  }

  /**
   * Check if warning is in migration context
   */
  private isMigrationContext(warning: Warning): boolean {
    const migrationIndicators = [
      'migration',
      'legacy',
      'deprecated',
      'old code',
      'refactor',
      'temporary',
    ];

    const text = `${warning.message} ${warning.explanation} ${warning.location.file}`.toLowerCase();

    return migrationIndicators.some((indicator) => text.includes(indicator));
  }

  /**
   * Generate suppression reason for test file
   */
  private generateTestFileReason(warning: Warning): string {
    const patternMap: Record<string, string> = {
      'AP-001': 'Test file requires broad ESLint disable for test utilities',
      'AP-003': 'Test mocks require any type for flexibility',
      'AP-004': 'Test fixtures use ts-ignore for intentional type violations',
      'AP-006': 'Test error handling intentionally uses empty catch',
    };

    return patternMap[warning.id] || 'Test file - relaxed rules for testing purposes';
  }

  /**
   * Generate suppression reason for type definition
   */
  private generateTypeDefinitionReason(warning: Warning): string {
    const patternMap: Record<string, string> = {
      'AP-003': 'Type definition requires any for generic compatibility',
      'AP-004': 'Type definition uses ts-ignore for complex type inference',
    };

    return patternMap[warning.id] || 'Type definition file - external type compatibility';
  }

  /**
   * Generate suppression reason for config file
   */
  private generateConfigFileReason(warning: Warning): string {
    const fileName = warning.location.file.split('/').pop() || '';

    return `Configuration file (${fileName}) - framework requirements`;
  }

  /**
   * Generate suppression reason for generated code
   */
  private generateGeneratedCodeReason(warning: Warning): string {
    return 'Generated code - auto-generated by tooling';
  }

  /**
   * Generate suppression reason for third-party context
   */
  private generateThirdPartyReason(warning: Warning): string {
    return 'Third-party library integration requires type flexibility';
  }

  /**
   * Generate suppression reason for migration context
   */
  private generateMigrationReason(warning: Warning): string {
    return 'Legacy code - planned for refactoring (track in issue tracker)';
  }

  /**
   * Create batch suppression groups
   */
  private createBatchGroups(quickWins: QuickWin[]): BatchGroup[] {
    const groups = new Map<string, BatchGroup>();

    for (const quickWin of quickWins) {
      if (!quickWin.batchable || !quickWin.batchKey) {
        continue;
      }

      if (!groups.has(quickWin.batchKey)) {
        groups.set(quickWin.batchKey, {
          key: quickWin.batchKey,
          patternId: quickWin.warning.id,
          type: quickWin.type,
          warnings: [],
          suggestedReason: quickWin.suggestedReason,
          count: 0,
        });
      }

      const group = groups.get(quickWin.batchKey)!;
      group.warnings.push(quickWin.warning);
      group.count++;
    }

    return Array.from(groups.values())
      .filter((group) => group.count > 1) // Only include batches with 2+ items
      .sort((a, b) => b.count - a.count); // Sort by count descending
  }

  /**
   * Generate suppression comment for a quick win
   */
  public generateSuppressionComment(quickWin: QuickWin): string {
    return `// @anvil-ignore ${quickWin.warning.id}: ${quickWin.suggestedReason}`;
  }

  /**
   * Generate batch suppression summary
   */
  public generateBatchSuppressionSummary(batch: BatchGroup): string {
    const typeLabels: Record<QuickWinType, string> = {
      'test-file': 'test files',
      'type-definition': 'type definition files',
      'config-file': 'configuration files',
      'generated-code': 'generated files',
      migration: 'legacy code',
      'third-party': 'third-party integrations',
      'legacy-code': 'legacy code',
    };

    const label = typeLabels[batch.type] || 'files';

    return `${batch.count} occurrences of ${batch.patternId} in ${label}`;
  }

  /**
   * Get statistics about quick wins
   */
  public getStatistics(analysis: QuickWinsAnalysis): {
    byType: Record<QuickWinType, number>;
    byPattern: Record<string, number>;
    batchableCount: number;
    individualCount: number;
  } {
    const byType: Record<QuickWinType, number> = {
      'test-file': 0,
      'type-definition': 0,
      'config-file': 0,
      'generated-code': 0,
      migration: 0,
      'third-party': 0,
      'legacy-code': 0,
    };

    const byPattern: Record<string, number> = {};

    let batchableCount = 0;
    let individualCount = 0;

    for (const quickWin of analysis.quickWins) {
      byType[quickWin.type]++;

      const patternId = quickWin.warning.id;
      byPattern[patternId] = (byPattern[patternId] || 0) + 1;

      if (quickWin.batchable) {
        batchableCount++;
      } else {
        individualCount++;
      }
    }

    return {
      byType,
      byPattern,
      batchableCount,
      individualCount,
    };
  }
}
