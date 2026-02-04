import { Command } from 'commander';

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
          const { generateMcpConfig, SUPPORTED_TARGETS } =
            await import('@eddacraft/anvil-mcp-server');

          type McpConfigTarget = (typeof SUPPORTED_TARGETS)[number];
          const target = options.target;
          if (!SUPPORTED_TARGETS.includes(target as McpConfigTarget)) {
            console.error(`Unknown target: ${target}. Supported: ${SUPPORTED_TARGETS.join(', ')}`);
            process.exit(1);
          }

          const config = generateMcpConfig(target as McpConfigTarget, {
            transport: options.transport as 'stdio' | 'http',
            port: parseInt(options.port, 10),
          });

          if (options.write) {
            const { writeFileSync, mkdirSync } = await import('node:fs');
            const { dirname, resolve } = await import('node:path');
            const fullPath = resolve(process.cwd(), config.configPath);
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
