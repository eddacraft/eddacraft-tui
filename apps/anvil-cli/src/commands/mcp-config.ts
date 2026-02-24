import { Command } from 'commander';
import { CliError } from '../utils/cli-error.js';

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
        ? { type: 'http', url: httpUrl }
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
    .option('-y, --yes', 'Skip confirmation when writing outside workspace')
    .action(
      async (options: {
        target: string;
        transport: string;
        port: string;
        write?: boolean;
        yes?: boolean;
      }) => {
        try {
          const target = options.target;
          if (!SUPPORTED_TARGETS.includes(target as McpConfigTarget)) {
            console.error(`Unknown target: ${target}. Supported: ${SUPPORTED_TARGETS.join(', ')}`);
            throw new CliError(`Unknown target: ${target}`);
          }

          const transport = options.transport;
          if (transport !== 'stdio' && transport !== 'http') {
            console.error(`Unknown transport: ${transport}. Supported: stdio, http`);
            throw new CliError(`Unknown transport: ${transport}`);
          }

          const port = parseInt(options.port, 10);
          if (!Number.isFinite(port) || port < 1 || port > 65535) {
            console.error(`Invalid port: ${options.port}. Must be an integer between 1 and 65535.`);
            throw new CliError(`Invalid port: ${options.port}`);
          }

          const config = generateMcpConfig(target as McpConfigTarget, { transport, port });

          if (options.write) {
            const { writeFileSync, mkdirSync, realpathSync } = await import('node:fs');
            const {
              dirname,
              resolve,
              basename,
              relative: pathRelative,
              sep,
            } = await import('node:path');
            const { homedir } = await import('node:os');
            const { createInterface } = await import('node:readline');
            const expandedPath = config.configPath.startsWith('~/')
              ? config.configPath.replace('~', homedir())
              : config.configPath;
            const fullPath = resolve(process.cwd(), expandedPath);

            // Resolve symlinks to prevent symlink-based bypass of outside-workspace check.
            // Try the full path first (catches symlinks at the final component, e.g.
            // .cursor/mcp.json -> /outside/file), then fall back to resolving the parent
            // directory when the file doesn't exist yet.
            let realCwd: string;
            try {
              realCwd = realpathSync(process.cwd());
            } catch {
              realCwd = process.cwd();
            }
            let realFullPath: string;
            try {
              realFullPath = realpathSync(fullPath);
            } catch {
              try {
                realFullPath = resolve(realpathSync(dirname(fullPath)), basename(fullPath));
              } catch {
                realFullPath = fullPath;
              }
            }
            const rel = pathRelative(realCwd, realFullPath);
            const isOutside = rel.startsWith('..') || rel.startsWith(sep) || /^[A-Za-z]:/.test(rel);
            if (isOutside && !options.yes) {
              if (!process.stdin.isTTY) {
                console.error(
                  `Target path is outside workspace: ${fullPath}\n` +
                    `Use --yes to skip confirmation in non-interactive mode.`
                );
                throw new CliError('Target path is outside workspace and no TTY available');
              }
              const rl = createInterface({ input: process.stdin, output: process.stdout });
              const answer = await new Promise<string>((res) => {
                rl.question(`Write config outside workspace to ${fullPath}? [y/N] `, res);
              });
              rl.close();
              if (answer.toLowerCase() !== 'y') {
                console.log('Aborted.');
                return;
              }
            }

            mkdirSync(dirname(fullPath), { recursive: true });
            writeFileSync(fullPath, JSON.stringify(config.content, null, 2) + '\n', 'utf-8');
            console.log(`Wrote ${config.configPath}`);
          } else {
            console.log(JSON.stringify(config.content, null, 2));
          }
        } catch (err) {
          if (err instanceof CliError) throw err;
          console.error('Error:', err instanceof Error ? err.message : String(err));
          throw new CliError(err instanceof Error ? err.message : String(err));
        }
      }
    );

  return command;
}
