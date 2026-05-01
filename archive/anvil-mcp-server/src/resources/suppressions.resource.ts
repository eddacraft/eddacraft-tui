import type { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';

/**
 * Registers the `anvil://suppressions` resource on the given MCP server.
 *
 * Returns active suppressions and their expiry dates from the
 * `.anvil/suppressions.json` store.
 */
export function registerSuppressionsResource(
  server: McpServer,
  getWorkspaceRoot: () => string
): void {
  server.registerResource(
    'suppressions',
    'anvil://suppressions',
    {
      title: 'Active Suppressions',
      description:
        'Active warning suppressions with their expiry dates, scopes, and associated patterns.',
      mimeType: 'application/json',
    },
    async (uri) => {
      try {
        const workspaceRoot = getWorkspaceRoot();
        const { SuppressionStore } = await import('@eddacraft/anvil-core');
        const { join } = await import('node:path');

        const anvilDir = join(workspaceRoot, '.anvil');
        const store = new SuppressionStore(anvilDir);
        await store.load();

        const all = store.getAll();
        const now = new Date();
        const expired = store.getExpired(now);
        const expiredIds = new Set(expired.map((s) => s.id));

        const suppressions = all.map((s) => ({
          id: s.id,
          pattern_id: s.pattern_id,
          file: s.file,
          line: s.line,
          reason: s.reason,
          scope: s.scope,
          expires_at: s.expires_at ?? null,
          isExpired: expiredIds.has(s.id),
          commit: s.commit,
        }));

        return {
          contents: [
            {
              uri: uri.href,
              mimeType: 'application/json',
              text: JSON.stringify(
                {
                  suppressions,
                  summary: {
                    total: all.length,
                    active: all.length - expired.length,
                    expired: expired.length,
                  },
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
