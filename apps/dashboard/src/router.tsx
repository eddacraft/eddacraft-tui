import { createRootRoute, createRoute, createRouter } from '@tanstack/react-router';

import { DashboardRootRoute } from './routes/__root';
import { DashboardIndexRoute } from './routes/index';
import { DashboardPlansRoute } from './routes/plans';
import { dashboardSearchSchema } from './lib/search-params';

const rootRoute = createRootRoute({
  component: DashboardRootRoute,
});

const indexRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/',
  component: DashboardIndexRoute,
  validateSearch: dashboardSearchSchema,
});

const plansRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/plans',
  component: DashboardPlansRoute,
});

const routeTree = rootRoute.addChildren([indexRoute, plansRoute]);

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
