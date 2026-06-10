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
    { id: 'anvil-001', root: '..' },
    // <deepsec:projects-insert-above>
  ],
  plugins: [anvilPlugin],
});
