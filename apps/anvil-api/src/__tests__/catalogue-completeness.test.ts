import { productCatalogue } from '@eddacraft/anvil-flags-catalogue';
import { inspectRoutes } from 'hono/dev';
import { afterEach, describe, expect, it, vi } from 'vitest';

vi.mock('../db/client.js', () => ({
  getClient: vi.fn(() => vi.fn(async () => [{ '?column?': 1 }])),
}));

vi.mock('../lib/licence.js', async (importOriginal) => {
  const actual = await importOriginal<typeof import('../lib/licence.js')>();
  return {
    ...actual,
    verifySigningKey: vi.fn(async () => ({ ok: true }) as const),
    verifyVerifyingKey: vi.fn(async () => ({ ok: true }) as const),
  };
});

vi.mock('../lib/resend-credentials.js', () => ({
  verifyResendKey: vi.fn(async () => 'ok' as const),
}));

import app from '../index.js';

afterEach(() => {
  vi.restoreAllMocks();
});

type ApiLocator = {
  method: string;
  path: string;
};

function normalise(locator: ApiLocator): string {
  return `${locator.method.toUpperCase()}\0${locator.path}`;
}

function catalogueSet(collection: 'deliverySurfaces' | 'excludedDeliverySurfaces'): string[] {
  const routes: string[] = [];
  for (const surface of productCatalogue()[collection]) {
    if (surface.status === 'active' && surface.locator.kind === 'api-route') {
      routes.push(normalise(surface.locator));
    }
  }
  return routes.sort();
}

function exactSetDiagnostics(actualLocators: readonly ApiLocator[], expected: string[]) {
  const actual = actualLocators.map(normalise).sort();
  const sortedExpected = [...expected].sort();
  const hostDuplicates = actual.filter((locator, index) => locator === actual[index - 1]);
  const catalogueDuplicates = sortedExpected.filter(
    (locator, index) => locator === sortedExpected[index - 1]
  );
  const actualSet = new Set(actual);
  const expectedSet = new Set(sortedExpected);

  return {
    hostDuplicates,
    catalogueDuplicates,
    missing: sortedExpected.filter((locator) => !actualSet.has(locator)),
    extra: actual.filter((locator) => !expectedSet.has(locator)),
  };
}

function expectExactSet(label: string, actualLocators: readonly ApiLocator[], expected: string[]) {
  expect({
    host: 'api',
    set: label,
    ...exactSetDiagnostics(actualLocators, expected),
  }).toEqual({
    host: 'api',
    set: label,
    hostDuplicates: [],
    catalogueDuplicates: [],
    missing: [],
    extra: [],
  });
}

type InspectedApiRoute = ReturnType<typeof inspectRoutes>[number];

function apiDeliveryProjection(routes: readonly InspectedApiRoute[] = inspectRoutes(app)) {
  const concreteGroups = new Map<string, InspectedApiRoute[]>();
  const pathsWithTerminalHandlers = new Set(
    routes.filter((route) => !route.isMiddleware).map((route) => route.path)
  );
  for (const route of routes) {
    if (route.path === '*' || route.path.endsWith('*')) {
      continue;
    }
    const locator = normalise(route);
    concreteGroups.set(locator, [...(concreteGroups.get(locator) ?? []), route]);
  }
  const unterminatedExactGroups = [...concreteGroups.entries()]
    .filter(([, handlers]) => {
      if (handlers.some((handler) => !handler.isMiddleware)) {
        return false;
      }
      const [{ method, path }] = handlers;
      return method !== 'ALL' || !pathsWithTerminalHandlers.has(path);
    })
    .map(([locator]) => locator);
  const unterminatedWildcardGroups = routes
    .filter((route) => route.isMiddleware && route.path !== '*' && route.path.endsWith('*'))
    .filter((route) => {
      const prefix = route.path.slice(0, -1);
      return !routes.some(
        (candidate) => !candidate.isMiddleware && candidate.path.startsWith(prefix)
      );
    })
    .map(normalise);
  const unterminatedConcreteGroups = [
    ...unterminatedExactGroups,
    ...unterminatedWildcardGroups,
  ].sort();

  const registeredRoutes = routes
    .filter((route) => !route.isMiddleware)
    .map((route) => ({
      method: route.method.toUpperCase(),
      path: route.path,
    }));
  const isCronRoute = (route: ApiLocator) =>
    route.path === '/api/v1/cron' || route.path.startsWith('/api/v1/cron/');

  return {
    unterminatedConcreteGroups,
    productDeliveries: registeredRoutes.filter((route) => !isCronRoute(route)),
    internalPlumbing: registeredRoutes.filter(isCronRoute),
  };
}

describe('API product catalogue completeness', () => {
  it('requires every concrete method and path to have a terminal handler', () => {
    expect(apiDeliveryProjection().unterminatedConcreteGroups).toEqual([]);
  });

  it('matches active product deliveries as an exact set', () => {
    expectExactSet(
      'product deliveries',
      apiDeliveryProjection().productDeliveries,
      catalogueSet('deliverySurfaces')
    );
  });

  it('matches active internal plumbing as a separate exact set', () => {
    expectExactSet(
      'internal plumbing',
      apiDeliveryProjection().internalPlumbing,
      catalogueSet('excludedDeliverySurfaces')
    );
  });

  it('reports duplicate concrete route registrations instead of collapsing them', () => {
    const routes = [
      ...inspectRoutes(app),
      {
        method: 'GET',
        path: '/api/v1/health',
        name: 'duplicateHealthHandler',
        isMiddleware: false,
      },
    ];

    expect(
      exactSetDiagnostics(
        apiDeliveryProjection(routes).productDeliveries,
        catalogueSet('deliverySurfaces')
      ).hostDuplicates
    ).toEqual(['GET\0/api/v1/health']);
  });

  it('reports duplicate catalogue locators instead of collapsing them', () => {
    const expected = catalogueSet('deliverySurfaces');

    expect(
      exactSetDiagnostics(apiDeliveryProjection().productDeliveries, [
        ...expected,
        'GET\0/api/v1/health',
      ]).catalogueDuplicates
    ).toEqual(['GET\0/api/v1/health']);
  });

  it('reports concrete ALL-method handlers instead of mistaking them for middleware', () => {
    const routes = [
      ...inspectRoutes(app),
      {
        method: 'ALL',
        path: '/api/v1/future',
        name: 'futureAllHandler',
        isMiddleware: false,
      },
    ];

    expect(
      exactSetDiagnostics(
        apiDeliveryProjection(routes).productDeliveries,
        catalogueSet('deliverySurfaces')
      ).extra
    ).toEqual(['ALL\0/api/v1/future']);
  });

  it('rejects method-specific routes that Hono classifies only as middleware', () => {
    const routes = [
      ...inspectRoutes(app),
      {
        method: 'GET',
        path: '/api/v1/future',
        name: 'futureArityTwoHandler',
        isMiddleware: true,
      },
    ];

    expect(apiDeliveryProjection(routes).unterminatedConcreteGroups).toEqual([
      'GET\0/api/v1/future',
    ]);
  });

  it('rejects orphan path-specific ALL middleware that can terminate requests', () => {
    const routes = [
      ...inspectRoutes(app),
      {
        method: 'ALL',
        path: '/api/v1/future',
        name: 'futureTerminalMiddleware',
        isMiddleware: true,
      },
    ];

    expect(apiDeliveryProjection(routes).unterminatedConcreteGroups).toEqual([
      'ALL\0/api/v1/future',
    ]);
  });

  it('rejects orphan path-scoped wildcard middleware that can terminate requests', () => {
    const routes = [
      ...inspectRoutes(app),
      {
        method: 'ALL',
        path: '/api/v1/future/*',
        name: 'futureWildcardTerminalMiddleware',
        isMiddleware: true,
      },
    ];

    expect(apiDeliveryProjection(routes).unterminatedConcreteGroups).toEqual([
      'ALL\0/api/v1/future/*',
    ]);
  });
});
