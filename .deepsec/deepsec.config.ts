import { defineConfig, type DeepsecPlugin } from 'deepsec/config';
import { anvilHonoRoute } from './matchers/anvil-hono-route.js';
import { anvilInterceptTrustBoundary } from './matchers/anvil-intercept-trust-boundary.js';

const anvilPlugin: DeepsecPlugin = {
  name: 'anvil-001',
  matchers: [anvilHonoRoute, anvilInterceptTrustBoundary],
};

export default defineConfig({
  projects: [
    { id: 'anvil-001', root: '..' },
    // <deepsec:projects-insert-above>
  ],
  plugins: [anvilPlugin],
});
