import { createRootRoute, createRoute, createRouter } from '@tanstack/react-router';

import { DashboardRootRoute } from './routes/__root';
import { DashboardIndexRoute } from './routes/index';
import { DashboardGatesRoute, DashboardGateDetailRoute } from './routes/gates';
import { DashboardPlanDetailRoute, DashboardPlansRoute } from './routes/plans';
import {
  DashboardWarningsBreakdownRoute,
  DashboardWarningsPatternsRoute,
  DashboardWarningsRoute,
} from './routes/warnings';
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

const gatesRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/gates',
  component: DashboardGatesRoute,
});

const gateDetailRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/gates/$id',
  component: function GateDetailRouteComponent() {
    const { id } = gateDetailRoute.useParams();
    return <DashboardGateDetailRoute id={id} />;
  },
});

const warningsRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/warnings',
  component: DashboardWarningsRoute,
  validateSearch: dashboardSearchSchema,
});

const warningsBreakdownRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/warnings/breakdown',
  component: DashboardWarningsBreakdownRoute,
});

const warningsPatternsRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/warnings/patterns',
  component: DashboardWarningsPatternsRoute,
});

const plansRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/plans',
  component: DashboardPlansRoute,
});

const planDetailRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/plans/$id',
  component: function PlanDetailRouteComponent() {
    const { id } = planDetailRoute.useParams();
    return <DashboardPlanDetailRoute id={id} />;
  },
});

const routeTree = rootRoute.addChildren([
  indexRoute,
  gatesRoute,
  gateDetailRoute,
  warningsRoute,
  warningsBreakdownRoute,
  warningsPatternsRoute,
  plansRoute,
  planDetailRoute,
]);

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
