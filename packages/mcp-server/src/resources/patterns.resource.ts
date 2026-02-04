import type { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';

/**
 * Registers the `anvil://patterns` resource on the given MCP server.
 *
 * Returns the anti-pattern catalogue with explanations, severities,
 * and detection details for all built-in patterns.
 */
export function registerPatternsResource(server: McpServer): void {
  server.registerResource(
    'patterns',
    'anvil://patterns',
    {
      title: 'Anti-pattern Catalogue',
      description:
        'Built-in anti-pattern definitions with IDs, explanations, severities, and suggestions.',
      mimeType: 'application/json',
    },
    async (uri) => {
      try {
        const { PATTERNS } = await import('@eddacraft/anvil-core');

        const catalogue = PATTERNS.map((p) => ({
          id: p.id,
          name: p.name,
          category: p.category,
          severity: p.severity,
          confidence: p.confidence,
          title: p.title,
          explanation: p.explanation,
          suggestion: p.suggestion,
          enabled: p.enabled,
          optIn: p.optIn,
          allowlist: p.allowlist,
        }));

        return {
          contents: [
            {
              uri: uri.href,
              mimeType: 'application/json',
              text: JSON.stringify({ patterns: catalogue, count: catalogue.length }, null, 2),
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
