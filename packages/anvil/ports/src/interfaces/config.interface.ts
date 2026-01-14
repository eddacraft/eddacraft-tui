/**
 * Config interface definitions
 *
 * Defines the contract for configuration providers.
 */

/**
 * Configuration provider interface
 */
export interface IConfigProvider {
  /** Get a configuration value by key */
  get<T>(key: string): T | undefined;

  /** Get a configuration value with a default */
  getOrDefault<T>(key: string, defaultValue: T): T;

  /** Check if a configuration key exists */
  has(key: string): boolean;

  /** Get all configuration values */
  getAll(): Record<string, unknown>;

  /** Reload configuration from source */
  reload?(): Promise<void>;
}
