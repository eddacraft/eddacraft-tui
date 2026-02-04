import type { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';

/**
 * Registers the `anvil://drift` resource on the given MCP server.
 *
 * Returns the current drift status by comparing the latest snapshot
 * against the baseline. If two snapshots exist, compares the most recent
 * pair to show the trend.
 */
export function registerDriftResource(server: McpServer, getWorkspaceRoot: () => string): void {
  server.registerResource(
    'drift',
    'anvil://drift',
    {
      title: 'Drift Status',
      description:
        'Current architecture drift status comparing the latest snapshot against the baseline or a previous snapshot.',
      mimeType: 'application/json',
    },
    async (uri) => {
      try {
        const workspaceRoot = getWorkspaceRoot();
        const { getLatestSnapshot, listSnapshots, loadSnapshot, compareSnapshots } =
          await import('@eddacraft/anvil-core');

        const snapshots = await listSnapshots(workspaceRoot);

        if (snapshots.length === 0) {
          return {
            contents: [
              {
                uri: uri.href,
                mimeType: 'application/json',
                text: JSON.stringify(
                  {
                    status: 'no-snapshots',
                    message:
                      'No drift snapshots found. Run `anvil snapshot` to capture the current state.',
                    snapshotCount: 0,
                  },
                  null,
                  2
                ),
              },
            ],
          };
        }

        const latest = await getLatestSnapshot(workspaceRoot);

        if (!latest) {
          return {
            contents: [
              {
                uri: uri.href,
                mimeType: 'application/json',
                text: JSON.stringify(
                  {
                    status: 'snapshot-load-failed',
                    message: 'Could not load the latest snapshot.',
                  },
                  null,
                  2
                ),
              },
            ],
          };
        }

        // If only one snapshot, return it with basic metrics
        if (snapshots.length === 1) {
          return {
            contents: [
              {
                uri: uri.href,
                mimeType: 'application/json',
                text: JSON.stringify(
                  {
                    status: 'single-snapshot',
                    message:
                      'Only one snapshot available. Capture another to compare drift over time.',
                    snapshotCount: 1,
                    latest: {
                      name: latest.name,
                      created_at: latest.created_at,
                      metrics: latest.metrics,
                      hotspots: latest.hotspots,
                    },
                  },
                  null,
                  2
                ),
              },
            ],
          };
        }

        // Compare the two most recent snapshots
        // snapshots are sorted newest first by listSnapshots
        const previousMeta = snapshots[1];
        const previous = await loadSnapshot(workspaceRoot, previousMeta.filename);

        if (!previous) {
          return {
            contents: [
              {
                uri: uri.href,
                mimeType: 'application/json',
                text: JSON.stringify(
                  {
                    status: 'comparison-failed',
                    message: 'Could not load previous snapshot for comparison.',
                    snapshotCount: snapshots.length,
                    latest: {
                      name: latest.name,
                      created_at: latest.created_at,
                      metrics: latest.metrics,
                    },
                  },
                  null,
                  2
                ),
              },
            ],
          };
        }

        const comparison = compareSnapshots(previous, latest);

        return {
          contents: [
            {
              uri: uri.href,
              mimeType: 'application/json',
              text: JSON.stringify(
                {
                  status: 'ok',
                  snapshotCount: snapshots.length,
                  comparison,
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
