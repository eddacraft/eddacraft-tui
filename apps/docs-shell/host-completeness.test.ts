import { dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

import { productCatalogue } from '@eddacraft/anvil-flags-catalogue';
import { discoverRoutes } from 'next/dist/build/route-discovery.js';
import {
  normalizeMetadataPageToRoute,
  normalizeMetadataRoute,
} from 'next/dist/lib/metadata/get-metadata-route.js';
import { normalizeAppPath } from 'next/dist/shared/lib/router/utils/app-paths.js';
import { describe, expect, it, vi } from 'vitest';

vi.hoisted(() => {
  process.env['ANVIL_DOCS_URL'] = 'https://private-docs.example.test';
  process.env['PUBLIC_DOCS_URL'] = 'https://public-docs.example.test';
  process.env['DOCS_UPSTREAM_SECRET'] = 'test-upstream-secret';
});

import { config } from './proxy';

const INTERNAL_ROUTE_PREFIXES = new Set(['/assets', '/_next', '/img', '/pagefind']);

type AppRouteDiscovery = Pick<
  Awaited<ReturnType<typeof discoverRoutes>>,
  'appRoutes' | 'appRouteHandlers'
>;

function routePrefixesFromDiscovery(discovery: AppRouteDiscovery): string[] {
  return [...discovery.appRoutes, ...discovery.appRouteHandlers].map(({ route }) => route);
}

async function appRoutePrefixes(): Promise<string[]> {
  const appRoot = fileURLToPath(new URL('./app/', import.meta.url));
  const discovery = await discoverRoutes({
    appDir: appRoot,
    pageExtensions: ['ts', 'tsx', 'js', 'jsx'],
    isDev: false,
    baseDir: dirname(appRoot),
    appDirOnly: true,
  });
  return routePrefixesFromDiscovery(discovery);
}

function normaliseMatcher(matcher: string): string {
  return matcher.replace(/\/:path\*$/, '');
}

function matcherRoutePrefixes(matchers: readonly string[]) {
  const variants = new Map<string, { exact: number; wildcard: number }>();
  for (const matcher of matchers) {
    const pathPrefix = normaliseMatcher(matcher);
    const counts = variants.get(pathPrefix) ?? { exact: 0, wildcard: 0 };
    if (matcher.endsWith('/:path*')) counts.wildcard += 1;
    else counts.exact += 1;
    variants.set(pathPrefix, counts);
  }

  return {
    prefixes: [...variants].flatMap(([pathPrefix, counts]) =>
      Array.from({ length: Math.max(counts.exact, counts.wildcard) }, () => pathPrefix)
    ),
    missingCatchAll: [...variants]
      .filter(([, counts]) => counts.exact > 0 && counts.wildcard === 0)
      .map(([pathPrefix]) => pathPrefix)
      .sort(),
  };
}

async function docsDeliveryProjection(matchers: readonly string[] = config.matcher) {
  const matcherProjection = matcherRoutePrefixes(matchers);
  const allPrefixes = [...(await appRoutePrefixes()), ...matcherProjection.prefixes, '/_next'];

  return {
    productDeliveries: allPrefixes.filter((pathPrefix) => !INTERNAL_ROUTE_PREFIXES.has(pathPrefix)),
    internalPlumbing: allPrefixes.filter((pathPrefix) => INTERNAL_ROUTE_PREFIXES.has(pathPrefix)),
    matcherCoverageGaps: matcherProjection.missingCatchAll,
  };
}

function catalogueSet(collection: 'deliverySurfaces' | 'excludedDeliverySurfaces'): string[] {
  const routes: string[] = [];
  for (const surface of productCatalogue()[collection]) {
    if (surface.status === 'active' && surface.locator.kind === 'docs-route') {
      routes.push(surface.locator.pathPrefix);
    }
  }
  return routes.sort();
}

function exactSetDiagnostics(actualPrefixes: readonly string[], expected: string[]) {
  const actual = [...actualPrefixes].sort();
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

function expectExactSet(label: string, actualPrefixes: readonly string[], expected: string[]) {
  expect({
    host: 'docs shell',
    set: label,
    ...exactSetDiagnostics(actualPrefixes, expected),
  }).toEqual({
    host: 'docs shell',
    set: label,
    hostDuplicates: [],
    catalogueDuplicates: [],
    missing: [],
    extra: [],
  });
}

describe('docs-shell product catalogue completeness', () => {
  it('matches active product deliveries as an exact set', async () => {
    expectExactSet(
      'product deliveries',
      (await docsDeliveryProjection()).productDeliveries,
      catalogueSet('deliverySurfaces')
    );
  });

  it('matches active internal plumbing as a separate exact set', async () => {
    expectExactSet(
      'internal plumbing',
      (await docsDeliveryProjection()).internalPlumbing,
      catalogueSet('excludedDeliverySurfaces')
    );
  });

  it('requires each exact proxy prefix to retain its catch-all matcher', async () => {
    expect((await docsDeliveryProjection()).matcherCoverageGaps).toEqual([]);
  });

  it('reports a proxied internal prefix removed from the live matcher', async () => {
    const matchersWithoutAssets = config.matcher.filter(
      (matcher) => normaliseMatcher(matcher) !== '/assets'
    );

    expect(
      exactSetDiagnostics(
        (await docsDeliveryProjection(matchersWithoutAssets)).internalPlumbing,
        catalogueSet('excludedDeliverySurfaces')
      ).missing
    ).toEqual(['/assets']);
  });

  it('reports an exact matcher whose catch-all transport was removed', async () => {
    const exactOnlyAnvil = config.matcher.filter((matcher) => matcher !== '/anvil/:path*');

    expect((await docsDeliveryProjection(exactOnlyAnvil)).matcherCoverageGaps).toEqual(['/anvil']);
  });

  it('reports repeated matcher declarations without rejecting an exact/wildcard pair', async () => {
    const duplicateExactMatcher = await docsDeliveryProjection([...config.matcher, '/anvil']);

    expect(
      exactSetDiagnostics(duplicateExactMatcher.productDeliveries, catalogueSet('deliverySurfaces'))
        .hostDuplicates
    ).toEqual(['/anvil']);
  });

  it('reports duplicate catalogue prefixes instead of collapsing them', async () => {
    const expected = catalogueSet('deliverySurfaces');

    expect(
      exactSetDiagnostics((await docsDeliveryProjection()).productDeliveries, [
        ...expected,
        '/anvil',
      ]).catalogueDuplicates
    ).toEqual(['/anvil']);
  });

  it('uses Next-owned route normalization for route groups and metadata routes', () => {
    const groupedRoute = normalizeAppPath('/(marketing)/about/page');
    const metadataRoute = normalizeAppPath(
      normalizeMetadataPageToRoute(normalizeMetadataRoute('/sitemap'), false)
    );

    expect(
      routePrefixesFromDiscovery({
        appRoutes: [{ route: groupedRoute, filePath: 'synthetic-group/page.tsx' }],
        appRouteHandlers: [{ route: metadataRoute, filePath: 'synthetic-sitemap.ts' }],
      })
    ).toEqual(['/about', '/sitemap.xml']);
  });
});
