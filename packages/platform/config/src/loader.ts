/**
 * Configuration Loader
 *
 * Loads configuration from files and environment variables.
 * This is a placeholder - actual implementation will be enhanced.
 */

import type { ConfigLoaderOptions, ConfigEntry, ConfigSource } from './types.js';

/**
 * Configuration loader class
 */
export class ConfigLoader {
  private readonly options: Required<ConfigLoaderOptions>;
  private config: Map<string, ConfigEntry> = new Map();

  constructor(options: ConfigLoaderOptions = {}) {
    this.options = {
      baseDir: options.baseDir ?? process.cwd(),
      envPrefix: options.envPrefix ?? 'ANVIL_',
      fileNames: options.fileNames ?? ['.anvilrc', '.anvilrc.yaml', '.anvilrc.json', 'anvil.config.js'],
    };
  }

  /**
   * Get a configuration value
   */
  get<T>(key: string): T | undefined {
    const entry = this.config.get(key);
    return entry?.value as T | undefined;
  }

  /**
   * Get a configuration value with a default
   */
  getOrDefault<T>(key: string, defaultValue: T): T {
    return this.get<T>(key) ?? defaultValue;
  }

  /**
   * Check if a configuration key exists
   */
  has(key: string): boolean {
    return this.config.has(key);
  }

  /**
   * Set a configuration value
   */
  set<T>(key: string, value: T, source: ConfigSource = 'default'): void {
    this.config.set(key, { value, source });
  }

  /**
   * Get all configuration values
   */
  getAll(): Record<string, unknown> {
    const result: Record<string, unknown> = {};
    for (const [key, entry] of this.config) {
      result[key] = entry.value;
    }
    return result;
  }
}

/**
 * Create a configuration loader
 */
export function createConfigLoader(options?: ConfigLoaderOptions): ConfigLoader {
  return new ConfigLoader(options);
}
