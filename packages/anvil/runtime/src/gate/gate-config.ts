import { GateConfig, GateCheck, PolicyConfig, StackConfig } from '../types/gate.types.js';
import { readFileSync, existsSync, writeFileSync, mkdirSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { WatchConfigSchema, type WatchConfig } from '../watch/types.js';
import { createDebugger } from '@eddacraft/anvil-core';

const debug = createDebugger('gate');

/**
 * Configuration file locations in priority order
 */
const CONFIG_LOCATIONS = [
  '.anvilrc', // Root level (primary)
  '.anvil/config.json', // .anvil directory (alternative)
];

export interface ConfigLoadResult {
  config: GateConfig;
  path: string | null;
  isDefault: boolean;
  errors: string[];
}

export class GateConfigManager {
  private configPath: string | null = null;
  private workspaceRoot: string;

  constructor(workspaceRoot: string) {
    this.workspaceRoot = workspaceRoot;
    this.configPath = this.findConfigFile();
    debug(`GateConfigManager: workspace=${workspaceRoot} configPath=${this.configPath}`);
  }

  /**
   * Find the first existing config file from priority locations
   */
  private findConfigFile(): string | null {
    for (const location of CONFIG_LOCATIONS) {
      const fullPath = join(this.workspaceRoot, location);
      if (existsSync(fullPath)) {
        debug(`findConfigFile: found ${fullPath}`);
        return fullPath;
      }
    }
    debug('findConfigFile: no config found, will use defaults');
    return null;
  }

  /**
   * Get the path where config will be saved (prefers existing location, defaults to .anvilrc)
   */
  getConfigPath(): string {
    return this.configPath || join(this.workspaceRoot, '.anvilrc');
  }

  /**
   * Get all possible config locations
   */
  getConfigLocations(): string[] {
    return CONFIG_LOCATIONS.map((loc) => join(this.workspaceRoot, loc));
  }

  loadConfig(): GateConfig {
    const result = this.loadConfigWithDetails();
    return result.config;
  }

  /**
   * Load configuration with detailed result including path and validation errors
   */
  loadConfigWithDetails(): ConfigLoadResult {
    debug(`loadConfigWithDetails: path=${this.configPath}`);
    if (!this.configPath) {
      debug('loadConfigWithDetails: using default config');
      return {
        config: this.getDefaultConfig(),
        path: null,
        isDefault: true,
        errors: [],
      };
    }

    try {
      const content = readFileSync(this.configPath, 'utf-8');
      const rawConfig = JSON.parse(content);
      const { config, errors } = this.validateAndNormalizeConfigWithErrors(rawConfig);
      debug(
        `loadConfigWithDetails: loaded ${config.checks.length} checks, ${errors.length} errors`
      );

      return {
        config,
        path: this.configPath,
        isDefault: false,
        errors,
      };
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : 'Unknown error';
      console.warn(
        `Failed to load gate config from ${this.configPath}, using defaults:`,
        errorMessage
      );

      return {
        config: this.getDefaultConfig(),
        path: this.configPath,
        isDefault: true,
        errors: [`Failed to parse config: ${errorMessage}`],
      };
    }
  }

  saveConfig(config: GateConfig): void {
    const savePath = this.getConfigPath();
    debug(`saveConfig: path=${savePath} checks=${config.checks.length}`);
    const content = JSON.stringify(config, null, 2);
    // Ensure directory exists before writing file
    const dir = dirname(savePath);
    if (!existsSync(dir)) {
      mkdirSync(dir, { recursive: true });
    }
    writeFileSync(savePath, content, 'utf-8');
    // Update configPath to the saved location
    this.configPath = savePath;
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
        {
          name: 'policy',
          description: 'OPA/Rego policy evaluation',
          enabled: false, // Disabled by default until policies are configured
          config: {
            policy_dir: '.anvil/policies',
            severity_threshold: 'error',
          },
        },
        {
          name: 'architecture',
          description: 'Architecture validation using dependency-cruiser',
          enabled: false, // Disabled by default until dependency-cruiser is installed
          config: {
            config_file: '.anvil/dependency-cruiser.js',
            scope: 'affected',
            severity_threshold: 'error',
            fail_on_circular: true,
            fail_on_orphan: false,
          },
        },
      ],
      thresholds: {
        overall_score: 80,
      },
    };
  }

  /**
   * Validate and normalise configuration with detailed error tracking
   */
  private validateAndNormalizeConfigWithErrors(config: unknown): {
    config: GateConfig;
    errors: string[];
  } {
    const errors: string[] = [];

    // Type guard to ensure config is an object
    if (typeof config !== 'object' || config === null) {
      errors.push('Configuration must be an object');
      return { config: this.getDefaultConfig(), errors };
    }

    const configObj = config as Record<string, unknown>;

    // Validate version
    if (!configObj.version) {
      errors.push('Missing required field: version (defaulting to 1)');
      configObj.version = 1;
    } else if (typeof configObj.version !== 'number') {
      errors.push(`Invalid version type: expected number, got ${typeof configObj.version}`);
      configObj.version = 1;
    }

    // Validate checks
    if (!configObj.checks) {
      errors.push('Missing required field: checks (defaulting to empty array)');
      configObj.checks = [];
    } else if (!Array.isArray(configObj.checks)) {
      errors.push(`Invalid checks type: expected array, got ${typeof configObj.checks}`);
      configObj.checks = [];
    }

    // Validate thresholds
    if (!configObj.thresholds) {
      errors.push('Missing required field: thresholds (using default overall_score: 80)');
      configObj.thresholds = { overall_score: 80 };
    } else if (typeof configObj.thresholds !== 'object' || configObj.thresholds === null) {
      errors.push(`Invalid thresholds type: expected object, got ${typeof configObj.thresholds}`);
      configObj.thresholds = { overall_score: 80 };
    }

    // Validate each check
    const checksArray = Array.isArray(configObj.checks) ? configObj.checks : [];
    const validatedChecks: GateCheck[] = checksArray.map((check: unknown, index: number) => {
      if (typeof check !== 'object' || check === null) {
        errors.push(`checks[${index}]: expected object, got ${typeof check}`);
        return {
          name: 'unknown',
          description: '',
          enabled: false,
          config: {},
        };
      }

      const checkObj = check as Record<string, unknown>;

      // Validate check name
      if (typeof checkObj.name !== 'string' || !checkObj.name) {
        errors.push(`checks[${index}].name: missing or invalid name`);
      }

      // Validate check config if present
      if (
        checkObj.config !== undefined &&
        (typeof checkObj.config !== 'object' || checkObj.config === null)
      ) {
        errors.push(`checks[${index}].config: expected object, got ${typeof checkObj.config}`);
      }

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

    // Parse watch config if present
    let watchConfig: WatchConfig | undefined;
    if (configObj.watch !== undefined) {
      try {
        watchConfig = WatchConfigSchema.parse(configObj.watch);
      } catch (watchError) {
        errors.push(
          `watch: invalid configuration - ${watchError instanceof Error ? watchError.message : 'unknown error'}`
        );
      }
    }

    // Pass through policy config if present (bundles, verification settings)
    let policyConfig: PolicyConfig | undefined;
    if (
      configObj.policy !== undefined &&
      typeof configObj.policy === 'object' &&
      configObj.policy !== null
    ) {
      policyConfig = configObj.policy as PolicyConfig;
    }

    // Parse stack config if present (STACK-012)
    let stackConfig: StackConfig | undefined;
    if (
      configObj.stack !== undefined &&
      typeof configObj.stack === 'object' &&
      configObj.stack !== null
    ) {
      const stack = configObj.stack as Record<string, unknown>;

      // Validate layer configs
      for (const layer of ['kindling', 'ember', 'edda'] as const) {
        if (stack[layer] !== undefined) {
          if (typeof stack[layer] !== 'object' || stack[layer] === null) {
            errors.push(`stack.${layer}: expected object`);
          } else {
            const layerConfig = stack[layer] as Record<string, unknown>;
            if (layerConfig.enabled !== undefined && typeof layerConfig.enabled !== 'boolean') {
              errors.push(`stack.${layer}.enabled: expected boolean`);
            }
          }
        }
      }

      // Validate validation config
      if (stack.validation !== undefined) {
        if (typeof stack.validation !== 'object' || stack.validation === null) {
          errors.push('stack.validation: expected object');
        } else {
          const validation = stack.validation as Record<string, unknown>;
          for (const key of ['check_provenance_integrity', 'check_schema_compatibility']) {
            if (validation[key] !== undefined && typeof validation[key] !== 'boolean') {
              errors.push(`stack.validation.${key}: expected boolean`);
            }
          }
        }
      }

      stackConfig = stack as StackConfig;
    }

    const validatedConfig: GateConfig = {
      version: configObj.version as number,
      checks: validatedChecks,
      thresholds: configObj.thresholds as { overall_score: number; [key: string]: number },
      global_config: configObj.global_config as Record<string, unknown> | undefined,
      watch: watchConfig,
      policy: policyConfig,
      stack: stackConfig,
    };

    return { config: validatedConfig, errors };
  }

  /**
   * Get watch configuration (parsed and validated)
   */
  getWatchConfig(): WatchConfig | undefined {
    const config = this.loadConfig();
    // Internal config may have defaults applied - cast to base type
    return config.watch as WatchConfig | undefined;
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
