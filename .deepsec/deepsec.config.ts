import { defineConfig, type DeepsecPlugin } from 'deepsec/config';
import { anvilHonoRoute } from './matchers/anvil-hono-route.js';
import { anvilInterceptTrustBoundary } from './matchers/anvil-intercept-trust-boundary.js';
import { anvilMcpToolEntry } from './matchers/anvil-mcp-tool-entry.js';
import { anvilDaemonWireInput } from './matchers/anvil-daemon-wire-input.js';
import { rsSpawnDynamic } from './matchers/rs-spawn-dynamic.js';

const anvilPlugin: DeepsecPlugin = {
  name: 'anvil-001',
  matchers: [
    anvilHonoRoute,
    anvilInterceptTrustBoundary,
    anvilMcpToolEntry,
    anvilDaemonWireInput,
    rsSpawnDynamic,
  ],
};

export default defineConfig({
  projects: [
    {
      id: 'anvil-001',
      root: '..',
      // Highest-signal runtime trust boundaries first (see data/anvil-001/INFO.md):
      // Hono admin/auth API, docs-shell proxy, website API routes, and the Rust
      // CLI + intercept daemon local trust boundary.
      priorityPaths: [
        'apps/anvil-api',
        'apps/docs-shell',
        'apps/website/app/api',
        'crates/anvil-cli',
        'crates/anvil-intercept',
      ],
    },
    // <deepsec:projects-insert-above>
  ],
  plugins: [anvilPlugin],
});
