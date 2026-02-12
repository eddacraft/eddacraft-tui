/**
 * Configuration types
 */

/**
 * Configuration source type
 */
export type ConfigSource = 'file' | 'env' | 'default';

/**
 * Configuration entry with metadata
 */
export interface ConfigEntry<T = unknown> {
  /** The configuration value */
  value: T;
  /** Where the value came from */
  source: ConfigSource;
  /** Path to the configuration file (if file source) */
  filePath?: string;
}

/**
 * Configuration loader options
 */
export interface ConfigLoaderOptions {
  /** Base directory for configuration files */
  baseDir?: string;
  /** Environment variable prefix */
  envPrefix?: string;
  /** Configuration file names to search for */
  fileNames?: string[];
}
