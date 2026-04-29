import type { McpConfig, McpConfigOptions } from './types.js';

const SERVER_NAME = 'anvil';
const PACKAGE_NAME = '@eddacraft/anvil-mcp-server';
const CONFIG_PATH = '.cursor/mcp.json';

export function generateCursorConfig(options: McpConfigOptions = {}): McpConfig {
  const { transport = 'stdio', port = 3000 } = options;

  if (transport === 'http') {
    return {
      target: 'cursor',
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
    target: 'cursor',
    configPath: CONFIG_PATH,
    content: {
      mcpServers: {
        [SERVER_NAME]: {
          command: 'npx',
          args: [PACKAGE_NAME],
        },
      },
    },
  };
}
