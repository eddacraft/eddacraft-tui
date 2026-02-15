import type { McpConfig, McpConfigOptions } from './types.js';

const SERVER_NAME = 'anvil';
const PACKAGE_NAME = '@eddacraft/anvil-mcp-server';
const CONFIG_PATH = '.vscode/mcp.json';

export function generateVscodeConfig(options: McpConfigOptions = {}): McpConfig {
  const { transport = 'stdio', port = 3000 } = options;

  if (transport === 'http') {
    return {
      target: 'vscode',
      configPath: CONFIG_PATH,
      content: {
        servers: {
          [SERVER_NAME]: {
            type: 'http',
            url: `http://localhost:${port}/mcp`,
          },
        },
      },
    };
  }

  return {
    target: 'vscode',
    configPath: CONFIG_PATH,
    content: {
      servers: {
        [SERVER_NAME]: {
          type: 'stdio',
          command: 'npx',
          args: [PACKAGE_NAME],
        },
      },
    },
  };
}
