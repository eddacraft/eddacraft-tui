/**
 * Dependency Analyzer - Wrapper for dependency-cruiser
 *
 * Handles loading and executing dependency-cruiser for architectural analysis.
 */

import { existsSync } from 'node:fs';
import { join } from 'node:path';

/**
 * Violation from dependency-cruiser
 */
export interface CruiserViolation {
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
export interface CruiserSummary {
  violations: CruiserViolation[];
  error: number;
  warn: number;
  info: number;
  totalCruised: number;
}

/**
 * Cruise result from dependency-cruiser
 */
export interface ICruiseResult {
  summary: CruiserSummary;
  modules: unknown[];
}

/**
 * Cruise function type
 */
export type CruiseFn = (
  fileAndDirectoryArray: string[],
  options?: Record<string, unknown>
) => Promise<{ output: ICruiseResult }>;

/**
 * Result of dependency analysis
 */
export interface AnalysisResult {
  success: boolean;
  result?: ICruiseResult;
  error?: string;
  skipped?: boolean;
  reason?: string;
}

/**
 * Dependency analyzer that wraps dependency-cruiser
 */
export class DependencyAnalyzer {
  private cruiseFn: CruiseFn | null = null;

  /**
   * Load dependency-cruiser dynamically
   */
  async loadCruiser(): Promise<{ success: boolean; error?: string }> {
    try {
      // Dynamic import - dependency-cruiser is an optional peer dependency
      // Using Function constructor to avoid bundler static analysis
      const depCruiser = (await Function('return import("dependency-cruiser")')()) as {
        cruise: CruiseFn;
      };
      this.cruiseFn = depCruiser.cruise;
      return { success: true };
    } catch {
      return {
        success: false,
        error: 'dependency-cruiser not installed',
      };
    }
  }

  /**
   * Check if dependency-cruiser is available
   */
  isAvailable(): boolean {
    return this.cruiseFn !== null;
  }

  /**
   * Load dependency-cruiser configuration
   */
  async loadConfig(
    workspaceRoot: string,
    configFile: string
  ): Promise<Record<string, unknown> | null> {
    const configPath = join(workspaceRoot, configFile);

    if (!existsSync(configPath)) {
      return null;
    }

    try {
      const configModule = await import(configPath);
      return configModule.default || configModule;
    } catch {
      return null;
    }
  }

  /**
   * Get default cruise options when no config file exists
   */
  getDefaultCruiseOptions(): Record<string, unknown> {
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
   * Run dependency analysis
   */
  async analyze(
    filesToCruise: string[],
    cruiseOptions: Record<string, unknown>
  ): Promise<AnalysisResult> {
    if (!this.cruiseFn) {
      return {
        success: false,
        skipped: true,
        reason: 'dependency-cruiser not available',
      };
    }

    try {
      const cruiseResult = await this.cruiseFn(filesToCruise, {
        ...cruiseOptions,
        outputType: 'json',
      });

      return {
        success: true,
        result: cruiseResult.output as ICruiseResult,
      };
    } catch (error) {
      return {
        success: false,
        error: error instanceof Error ? error.message : 'Unknown error',
      };
    }
  }
}
