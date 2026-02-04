import { McpServer, ResourceTemplate } from '@modelcontextprotocol/sdk/server/mcp.js';
import { resolve, relative } from 'node:path';
import { realpathSync } from 'node:fs';

/**
 * Registers the `anvil://file/{path}/warnings` resource template on the given MCP server.
 *
 * Returns warnings for a specific file by running the Anvil analyser
 * against it. The `{path}` variable is the file path relative to workspace root.
 */
export function registerFileWarningsResource(
  server: McpServer,
  getWorkspaceRoot: () => string
): void {
  server.registerResource(
    'file-warnings',
    new ResourceTemplate('anvil://file/{path}/warnings', { list: undefined }),
    {
      title: 'File Warnings',
      description:
        'Architecture and anti-pattern warnings for a specific file. Use the file path relative to workspace root.',
      mimeType: 'application/json',
    },
    async (uri, { path }) => {
      try {
        const workspaceRoot = getWorkspaceRoot();
        const rawPath = Array.isArray(path) ? path.join('/') : String(path);
        // Decode URI-encoded path segments (e.g., %2F -> /)
        const filePath = decodeURIComponent(rawPath);

        // Reject paths that escape the workspace root (logical + symlink check)
        const absPath = resolve(workspaceRoot, filePath);
        const rel = relative(workspaceRoot, absPath);
        if (rel.startsWith('..') || resolve(workspaceRoot, rel) !== absPath) {
          return {
            contents: [
              {
                uri: uri.href,
                mimeType: 'application/json',
                text: JSON.stringify(
                  { error: `Path "${filePath}" resolves outside workspace root` },
                  null,
                  2
                ),
              },
            ],
          };
        }

        // Resolve symlinks to prevent escaping via symlink targets
        try {
          const realRoot = realpathSync(workspaceRoot);
          const realAbs = realpathSync(absPath);
          if (!realAbs.startsWith(realRoot + '/') && realAbs !== realRoot) {
            return {
              contents: [
                {
                  uri: uri.href,
                  mimeType: 'application/json',
                  text: JSON.stringify(
                    { error: `Path "${filePath}" resolves outside workspace root (symlink)` },
                    null,
                    2
                  ),
                },
              ],
            };
          }
        } catch {
          // realpathSync throws if the file doesn't exist — allow the
          // GateRunner to handle that case with its own error message.
        }

        const { GateRunner } = await import('@eddacraft/anvil-runtime');
        const runner = new GateRunner();

        const result = await runner.analyzeFiles([filePath], workspaceRoot);

        return {
          contents: [
            {
              uri: uri.href,
              mimeType: 'application/json',
              text: JSON.stringify(
                {
                  file: filePath,
                  warnings: result.warnings.warnings.map((w) => ({
                    id: w.id,
                    severity: w.severity,
                    title: w.title,
                    message: w.message,
                    suggestion: w.suggestion,
                    location: w.location,
                    category: w.category,
                  })),
                  summary: result.warnings.summary,
                  checksRun: result.checksRun,
                  hasBlockingWarnings: result.hasBlockingWarnings,
                },
                null,
                2
              ),
            },
          ],
        };
      } catch (error) {
        const message = error instanceof Error ? error.message : String(error);
        return {
          contents: [
            {
              uri: uri.href,
              mimeType: 'application/json',
              text: JSON.stringify({ error: message }, null, 2),
            },
          ],
        };
      }
    }
  );
}
