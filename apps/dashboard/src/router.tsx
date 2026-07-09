import { createRootRoute, createRoute, createRouter } from '@tanstack/react-router';

import { DashboardRootRoute } from './routes/__root';
import { DashboardIndexRoute } from './routes/index';

const rootRoute = createRootRoute({
  component: DashboardRootRoute,
});

const indexRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/',
  component: DashboardIndexRoute,
});

const routeTree = rootRoute.addChildren([indexRoute]);

export function createDashboardRouter() {
  return createRouter({
    routeTree,
    defaultPreload: 'intent',
  });
}

export type DashboardRouter = ReturnType<typeof createDashboardRouter>;

declare module '@tanstack/react-router' {
  interface Register {
    router: DashboardRouter;
  }
}
