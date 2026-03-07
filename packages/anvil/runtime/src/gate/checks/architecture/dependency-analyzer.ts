/**
 * Dependency Analyser - Wrapper for dependency-cruiser
 *
 * Handles loading and executing dependency-cruiser for architectural analysis.
 */

import { existsSync } from 'node:fs';
import { join } from 'node:path';
import { createDebugger } from '@eddacraft/anvil-core';

const log = createDebugger('check');

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
 * Dependency analyser that wraps dependency-cruiser
 */
export class DependencyAnalyser {
  private cruiseFn: CruiseFn | null = null;

  /**
   * Load dependency-cruiser dynamically
   */
  async loadCruiser(): Promise<{ success: boolean; error?: string }> {
    log('dependency-analyzer: loading dependency-cruiser');
    try {
      // Dynamic import - dependency-cruiser is an optional peer dependency
      // Using Function constructor to avoid bundler static analysis
      const depCruiser = (await Function('return import("dependency-cruiser")')()) as {
        cruise: CruiseFn;
      };
      this.cruiseFn = depCruiser.cruise;
      log('dependency-analyzer: dependency-cruiser loaded successfully');
      return { success: true };
    } catch {
      log('dependency-analyzer: dependency-cruiser not installed');
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
      log(`dependency-analyzer: config file not found at ${configPath}`);
      return null;
    }

    try {
      const configModule = await import(configPath);
      log(`dependency-analyzer: config loaded from ${configPath}`);
      return configModule.default || configModule;
    } catch {
      log(`dependency-analyzer: failed to load config from ${configPath}`);
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
      log('dependency-analyzer: cruise function not available, skipping analysis');
      return {
        success: false,
        skipped: true,
        reason: 'dependency-cruiser not available',
      };
    }

    log(`dependency-analyzer: analysing ${filesToCruise.length} files/patterns`);
    try {
      const cruiseResult = await this.cruiseFn(filesToCruise, {
        ...cruiseOptions,
        outputType: 'json',
      });

      const summary = cruiseResult.output.summary;
      log('dependency-analyzer: analysis complete', {
        totalCruised: summary.totalCruised,
        violations: summary.violations.length,
        error: summary.error,
        warn: summary.warn,
        info: summary.info,
      });

      return {
        success: true,
        result: cruiseResult.output as ICruiseResult,
      };
    } catch (error) {
      log(
        `dependency-analyzer: analysis failed: ${error instanceof Error ? error.message : 'Unknown error'}`
      );
      return {
        success: false,
        error: error instanceof Error ? error.message : 'Unknown error',
      };
    }
  }
}
