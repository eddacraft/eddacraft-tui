import { GateConfig, GateCheck } from '../types/gate.types.js';
import { readFileSync, existsSync, writeFileSync, mkdirSync } from 'fs';
import { join, dirname } from 'path';

export class GateConfigManager {
  private configPath: string;

  constructor(workspaceRoot: string) {
    this.configPath = join(workspaceRoot, '.anvilrc');
  }

  loadConfig(): GateConfig {
    if (!existsSync(this.configPath)) {
      return this.getDefaultConfig();
    }

    try {
      const content = readFileSync(this.configPath, 'utf-8');
      const config = JSON.parse(content);
      return this.validateAndNormalizeConfig(config);
    } catch (error) {
      console.warn('Failed to load gate config, using defaults:', error);
      return this.getDefaultConfig();
    }
  }

  saveConfig(config: GateConfig): void {
    const content = JSON.stringify(config, null, 2);
    // Ensure directory exists before writing file
    const dir = dirname(this.configPath);
    if (!existsSync(dir)) {
      mkdirSync(dir, { recursive: true });
    }
    writeFileSync(this.configPath, content, 'utf-8');
  }

  getDefaultConfig(): GateConfig {
    return {
      version: 1,
      checks: [
        {
          name: 'eslint',
          description: 'Code quality checks',
          enabled: true,
          config: {
            min_score: 80,
          },
        },
        {
          name: 'coverage',
          description: 'Test coverage validation',
          enabled: true,
          config: {
            min_score: 80,
            thresholds: {
              lines: 80,
              functions: 80,
              branches: 80,
              statements: 80,
            },
          },
        },
        {
          name: 'secret',
          description: 'Secret scanning',
          enabled: true,
          config: {},
        },
        {
          name: 'dependency',
          description: 'Dependency vulnerability scanning',
          enabled: true,
          config: {
            min_severity: 'moderate',
            fail_on_critical: true,
            fail_on_high: true,
            fail_on_moderate: false,
          },
        },
      ],
      thresholds: {
        overall_score: 80,
      },
    };
  }

  private validateAndNormalizeConfig(config: unknown): GateConfig {
    // Type guard to ensure config is an object
    const configObj =
      typeof config === 'object' && config !== null ? (config as Record<string, unknown>) : {};

    // Ensure required fields exist
    if (!configObj.version) configObj.version = 1;
    if (!configObj.checks) configObj.checks = [];
    if (!configObj.thresholds) configObj.thresholds = { overall_score: 80 };

    // Validate checks structure
    const checksArray = Array.isArray(configObj.checks) ? configObj.checks : [];
    configObj.checks = checksArray.map((check: unknown) => {
      const checkObj =
        typeof check === 'object' && check !== null ? (check as Record<string, unknown>) : {};
      return {
        name: typeof checkObj.name === 'string' ? checkObj.name : 'unknown',
        description: typeof checkObj.description === 'string' ? checkObj.description : '',
        enabled: checkObj.enabled !== false,
        config:
          typeof checkObj.config === 'object' && checkObj.config !== null
            ? (checkObj.config as Record<string, unknown>)
            : {},
      };
    });

    return {
      version: configObj.version as number,
      checks: configObj.checks as GateCheck[],
      thresholds: configObj.thresholds as { overall_score: number; [key: string]: number },
      global_config: configObj.global_config as Record<string, unknown> | undefined,
    };
  }

  updateCheck(name: string, updates: Partial<GateCheck>): void {
    const config = this.loadConfig();
    const checkIndex = config.checks.findIndex((c) => c.name === name);

    if (checkIndex >= 0) {
      config.checks[checkIndex] = { ...config.checks[checkIndex], ...updates };
    } else {
      config.checks.push({
        name,
        description: updates.description || '',
        enabled: updates.enabled !== false,
        config: updates.config || {},
      });
    }

    this.saveConfig(config);
  }

  enableCheck(name: string): void {
    this.updateCheck(name, { enabled: true });
  }

  disableCheck(name: string): void {
    this.updateCheck(name, { enabled: false });
  }
}
