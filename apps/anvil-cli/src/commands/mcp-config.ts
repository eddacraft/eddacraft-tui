import { Command } from 'commander';

// ---------------------------------------------------------------------------
// MCP config generation — inlined from @eddacraft/anvil-mcp-server/config
// to avoid pulling express + @modelcontextprotocol/sdk into the CLI publish tree.
// ---------------------------------------------------------------------------

interface McpConfigOptions {
  transport?: 'stdio' | 'http';
  port?: number;
}

interface McpConfig {
  target: string;
  configPath: string;
  content: Record<string, unknown>;
}

type McpConfigTarget = 'claude-code' | 'cursor' | 'windsurf' | 'vscode';

const SUPPORTED_TARGETS: McpConfigTarget[] = ['claude-code', 'cursor', 'windsurf', 'vscode'];

const SERVER_NAME = 'anvil';
const PACKAGE_NAME = '@eddacraft/anvil-mcp-server';

function generateMcpConfig(target: McpConfigTarget, options: McpConfigOptions = {}): McpConfig {
  const { transport = 'stdio', port = 3000 } = options;
  const httpUrl = `http://localhost:${port}/mcp`;

  const targetConfigs: Record<McpConfigTarget, { configPath: string; serverKey: string }> = {
    'claude-code': { configPath: '.claude/mcp.json', serverKey: 'mcpServers' },
    cursor: { configPath: '.cursor/mcp.json', serverKey: 'mcpServers' },
    windsurf: { configPath: '~/.codeium/windsurf/mcp_config.json', serverKey: 'mcpServers' },
    vscode: { configPath: '.vscode/mcp.json', serverKey: 'servers' },
  };

  const { configPath, serverKey } = targetConfigs[target];

  const serverEntry =
    transport === 'http'
      ? target === 'vscode'
        ? { type: 'sse', url: httpUrl }
        : { url: httpUrl }
      : target === 'vscode'
        ? { type: 'stdio', command: 'npx', args: [PACKAGE_NAME] }
        : { command: 'npx', args: [PACKAGE_NAME], env: {} };

  return {
    target,
    configPath,
    content: { [serverKey]: { [SERVER_NAME]: serverEntry } },
  };
}

// ---------------------------------------------------------------------------

export function createMcpConfigCommand(): Command {
  const command = new Command('mcp-config');

  command
    .description('Generate MCP configuration for AI code editors')
    .requiredOption(
      '-t, --target <target>',
      'Target editor (claude-code, cursor, windsurf, vscode)'
    )
    .option('--transport <type>', 'Transport type (stdio, http)', 'stdio')
    .option('--port <number>', 'HTTP port (only for http transport)', '3000')
    .option('--write', 'Write config file to disk (default: print to stdout)')
    .action(
      async (options: { target: string; transport: string; port: string; write?: boolean }) => {
        try {
          const target = options.target;
          if (!SUPPORTED_TARGETS.includes(target as McpConfigTarget)) {
            console.error(`Unknown target: ${target}. Supported: ${SUPPORTED_TARGETS.join(', ')}`);
            process.exit(1);
          }

          const transport = options.transport;
          if (transport !== 'stdio' && transport !== 'http') {
            console.error(`Unknown transport: ${transport}. Supported: stdio, http`);
            process.exit(1);
          }

          const port = parseInt(options.port, 10);
          if (!Number.isFinite(port) || port < 1 || port > 65535) {
            console.error(`Invalid port: ${options.port}. Must be an integer between 1 and 65535.`);
            process.exit(1);
          }

          const config = generateMcpConfig(target as McpConfigTarget, { transport, port });

          if (options.write) {
            const { writeFileSync, mkdirSync } = await import('node:fs');
            const { dirname, resolve } = await import('node:path');
            const { homedir } = await import('node:os');
            const expandedPath = config.configPath.startsWith('~/')
              ? config.configPath.replace('~', homedir())
              : config.configPath;
            const fullPath = resolve(process.cwd(), expandedPath);
            mkdirSync(dirname(fullPath), { recursive: true });
            writeFileSync(fullPath, JSON.stringify(config.content, null, 2) + '\n', 'utf-8');
            console.log(`Wrote ${config.configPath}`);
          } else {
            console.log(JSON.stringify(config.content, null, 2));
          }
        } catch (err) {
          console.error('Error:', err instanceof Error ? err.message : String(err));
          process.exit(1);
        }
      }
    );

  return command;
}
