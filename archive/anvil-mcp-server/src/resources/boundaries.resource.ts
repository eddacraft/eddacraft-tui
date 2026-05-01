import type { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';

/**
 * Registers the `anvil://boundaries` resource on the given MCP server.
 *
 * Returns boundary rules (allowed/forbidden dependency edges) derived from
 * the architecture baseline layers and their `depends_on` declarations.
 */
export function registerBoundariesResource(
  server: McpServer,
  getWorkspaceRoot: () => string
): void {
  server.registerResource(
    'boundaries',
    'anvil://boundaries',
    {
      title: 'Boundary Rules',
      description:
        'Architecture boundary rules describing allowed and forbidden dependency edges between layers.',
      mimeType: 'application/json',
    },
    async (uri) => {
      try {
        const workspaceRoot = getWorkspaceRoot();
        const { loadBaseline, baselineExists } = await import('@eddacraft/anvil-core');

        if (!baselineExists(workspaceRoot)) {
          return {
            contents: [
              {
                uri: uri.href,
                mimeType: 'application/json',
                text: JSON.stringify(
                  {
                    error: 'no-baseline',
                    message: 'No architecture baseline found. Boundary rules require a baseline.',
                  },
                  null,
                  2
                ),
              },
            ],
          };
        }

        const baseline = loadBaseline(workspaceRoot);

        if (!baseline) {
          return {
            contents: [
              {
                uri: uri.href,
                mimeType: 'application/json',
                text: JSON.stringify(
                  {
                    error: 'baseline-load-failed',
                    message: 'Could not load architecture baseline.',
                  },
                  null,
                  2
                ),
              },
            ],
          };
        }

        // Extract layer dependency rules
        const layers = Object.entries(baseline.layers).map(([name, layer]) => ({
          name,
          patterns: layer.patterns,
          depends_on: layer.depends_on,
          description: layer.description,
        }));

        // Extract explicit boundary rules
        const boundaries = baseline.boundaries.map((b) => ({
          name: b.name,
          from: b.from,
          to: b.to,
          severity: b.severity,
          message: b.message,
        }));

        return {
          contents: [
            {
              uri: uri.href,
              mimeType: 'application/json',
              text: JSON.stringify({ layers, boundaries }, null, 2),
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
