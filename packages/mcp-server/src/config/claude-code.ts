import type { McpConfig, McpConfigOptions } from './types.js';

const SERVER_NAME = 'anvil';
const PACKAGE_NAME = '@eddacraft/anvil-mcp-server';
const CONFIG_PATH = '.claude/mcp.json';

export function generateClaudeCodeConfig(options: McpConfigOptions = {}): McpConfig {
  const { transport = 'stdio', port = 3000 } = options;

  if (transport === 'http') {
    return {
      target: 'claude-code',
      configPath: CONFIG_PATH,
      content: {
        mcpServers: {
          [SERVER_NAME]: {
            url: `http://localhost:${port}/mcp`,
          },
        },
      },
    };
  }

  return {
    target: 'claude-code',
    configPath: CONFIG_PATH,
    content: {
      mcpServers: {
        [SERVER_NAME]: {
          command: 'npx',
          args: [PACKAGE_NAME],
          env: {},
        },
      },
    },
  };
}
