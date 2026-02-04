export interface McpConfigOptions {
  /** How to start the server: 'stdio' or 'http' */
  transport?: 'stdio' | 'http';
  /** Port for HTTP transport (default: 3000) */
  port?: number;
}

export interface McpConfig {
  /** Target tool name */
  target: string;
  /** Config file path relative to project root */
  configPath: string;
  /** The config content as a serializable object */
  content: Record<string, unknown>;
}
