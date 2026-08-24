import { describe, expect, it } from 'vitest';

import { productCatalogue } from '../../../packages/anvil/flags-catalogue/src/index.js';
import { createDashboardRouter } from './router';

function isDashboardServerPath(path: string): boolean {
  return path === '/healthz' || path === '/openapi.json' || path.startsWith('/api/v1/');
}

function catalogueSet(collection: 'deliverySurfaces' | 'excludedDeliverySurfaces'): string[] {
  const routes: string[] = [];
  for (const surface of productCatalogue()[collection]) {
    if (
      surface.status === 'active' &&
      surface.locator.kind === 'dashboard-route' &&
      !isDashboardServerPath(surface.locator.path)
    ) {
      routes.push(surface.locator.path);
    }
  }
  return routes.sort();
}

function exactSetDiagnostics(actualPaths: readonly string[], expected: string[]) {
  const actual = [...actualPaths].sort();
  const sortedExpected = [...expected].sort();
  const hostDuplicates = actual.filter((path, index) => path === actual[index - 1]);
  const catalogueDuplicates = sortedExpected.filter(
    (path, index) => path === sortedExpected[index - 1]
  );
  const actualSet = new Set(actual);
  const expectedSet = new Set(sortedExpected);

  return {
    hostDuplicates,
    catalogueDuplicates,
    missing: sortedExpected.filter((path) => !actualSet.has(path)),
    extra: actual.filter((path) => !expectedSet.has(path)),
  };
}

function expectExactSet(label: string, actualPaths: readonly string[], expected: string[]) {
  expect({
    host: 'dashboard SPA',
    set: label,
    ...exactSetDiagnostics(actualPaths, expected),
  }).toEqual({
    host: 'dashboard SPA',
    set: label,
    hostDuplicates: [],
    catalogueDuplicates: [],
    missing: [],
    extra: [],
  });
}

function dashboardSpaDeliveryProjection() {
  const productDeliveries = Object.values(createDashboardRouter().routesById)
    .filter((route) => !route.isRoot)
    .map((route) => route.fullPath);

  return {
    productDeliveries,
    internalPlumbing: [] as const,
  };
}

describe('dashboard SPA product catalogue completeness', () => {
  it('matches active product deliveries as an exact set', () => {
    expectExactSet(
      'product deliveries',
      dashboardSpaDeliveryProjection().productDeliveries,
      catalogueSet('deliverySurfaces')
    );
  });

  it('matches active internal plumbing as a separate exact set', () => {
    expectExactSet(
      'internal plumbing',
      dashboardSpaDeliveryProjection().internalPlumbing,
      catalogueSet('excludedDeliverySurfaces')
    );
  });

  it('reports duplicate catalogue routes instead of collapsing them', () => {
    const expected = catalogueSet('deliverySurfaces');
    const duplicate = expected[0]!;

    expect(
      exactSetDiagnostics(dashboardSpaDeliveryProjection().productDeliveries, [
        ...expected,
        duplicate,
      ]).catalogueDuplicates
    ).toEqual([duplicate]);
  });
});
