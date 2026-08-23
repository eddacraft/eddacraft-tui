import { readFileSync, readdirSync } from 'node:fs';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';
import {
  FlagSurfaceManifestSchema,
  ProductCatalogueManifestSchema,
} from '@eddacraft/anvil-contracts';
import {
  flagAudiences,
  flagSurfaces,
  mustAlwaysBeOpenDeliverySurfaces,
  productCatalogue,
} from '../src/index.js';

describe('product catalogue v2 (flags/surfaces.json)', () => {
  it('loads the canonical registry through the strict v2 schema', () => {
    expect(ProductCatalogueManifestSchema.parse(productCatalogue()).schemaVersion).toBe(2);
  });

  it('resolves every feature and exclusion owner to a repository APS module', () => {
    const repositoryRoot = fileURLToPath(new URL('../../../../', import.meta.url));
    const moduleIds = new Set<string>();
    for (const relativeDirectory of ['plans/modules', 'plans/archive/modules']) {
      const directory = join(repositoryRoot, relativeDirectory);
      for (const filename of readdirSync(directory).filter((name) => name.endsWith('.aps.md'))) {
        const source = readFileSync(join(directory, filename), 'utf8');
        for (const match of source.matchAll(/^#+\s+([A-Z][A-Z0-9]*)-\d{3}/gm)) {
          moduleIds.add(match[1]);
        }
        for (const match of source.matchAll(/^\|\s*([A-Z][A-Z0-9]*)\s*\|/gm)) {
          moduleIds.add(match[1]);
        }
      }
    }

    const unresolved = [
      ...productCatalogue().productFeatures.map((feature) => ({
        kind: 'feature',
        key: feature.key,
        owner: feature.owner,
      })),
      ...productCatalogue().excludedDeliverySurfaces.map((surface) => ({
        kind: 'exclusion',
        key: surface.key,
        owner: surface.owner,
      })),
    ].filter(({ owner }) => !moduleIds.has(owner));

    expect(unresolved).toEqual([]);
  });
});

const LEGACY_GROUP_IDS = [
  'governance',
  'mcp',
  'dashboard',
  'save-time',
  'hooks',
  'admin',
  'tools',
  'setup',
  'foundational',
] as const;

const LEGACY_FEATURE_KEYS = [
  'check',
  'audit',
  'gate',
  'gate-config',
  'drift',
  'architecture',
  'policy',
  'export',
  'baseline',
  'audit-chain',
  'l4-validate',
  'validate',
  'mcp.install',
  'mcp.serve',
  'mcp.config',
  'dashboard.aps',
  'dashboard.architecture',
  'dashboard.drift',
  'dashboard.suppressions',
  'dashboard.saved',
  'watch',
  'intercept',
  'hook',
  'hooks',
  'admin.operations',
  'edda',
  'capsule',
  'insights',
  'kindling',
  'init',
  'ensure',
  'start',
  'welcome',
  'new',
  'wizard',
  'admin.credential',
  'auth',
  'config',
  'migrate',
  'update',
  'uninstall',
  'doctor',
  'version',
  'licenses',
  'tutorial',
  'workspace',
] as const;

function effectiveV1Access(
  surface: ReturnType<typeof flagSurfaces>['surfaces'][number],
  catalogue: ReturnType<typeof flagSurfaces>
): string | undefined {
  return (
    surface.access ??
    catalogue.categories.find((category) => category.id === surface.category)?.defaultAccess
  );
}

function effectiveV2Access(
  surface: ReturnType<typeof productCatalogue>['deliverySurfaces'][number]
): string | undefined {
  const catalogue = productCatalogue();
  const feature = catalogue.productFeatures.find(
    (candidate) => candidate.key === surface.featureKey
  );
  const group = catalogue.productFeatureGroups.find(
    (candidate) => candidate.key === feature?.groupKey
  );
  return surface.posture.access ?? group?.defaultSurfacePosture.access;
}

// ADR-076: the surface registry back-capture. These are the static checks the
// ADR puts in scope (existence, acyclicity, the MUST_ALWAYS_BE_OPEN floor,
// cross-inventory integrity). Runtime cascade-off / auth-list derivation are
// deferred and intentionally not exercised here.

describe('legacy surface registry compatibility projection', () => {
  it('validates against FlagSurfaceManifestSchema', () => {
    expect(FlagSurfaceManifestSchema.safeParse(flagSurfaces()).success).toBe(true);
  });

  it('carries the nine capability categories', () => {
    // Set comparison — category order in the JSON is not a schema invariant.
    expect(new Set(flagSurfaces().categories.map((c) => c.id))).toEqual(
      new Set([
        'governance',
        'mcp',
        'dashboard',
        'save-time',
        'hooks',
        'admin',
        'tools',
        'setup',
        'foundational',
      ])
    );
  });

  it('retains the exact 46-feature legacy CLI subset', () => {
    expect(flagSurfaces().surfaces).toHaveLength(46);
  });

  it('every surface references a defined category', () => {
    const categoryIds = new Set(flagSurfaces().categories.map((c) => c.id));
    for (const s of flagSurfaces().surfaces) {
      expect(categoryIds.has(s.category), `${s.key} -> ${s.category}`).toBe(true);
    }
  });

  it('every requires-target exists', () => {
    const keys = new Set(flagSurfaces().surfaces.map((s) => s.key));
    for (const s of flagSurfaces().surfaces) {
      for (const dep of s.requires ?? []) {
        expect(keys.has(dep), `${s.key} requires ${dep}`).toBe(true);
      }
    }
  });

  it('every gating audience exists in the audience inventory', () => {
    const audienceIds = new Set(flagAudiences().audiences.map((a) => a.id));
    for (const s of flagSurfaces().surfaces) {
      for (const a of s.audiences ?? []) {
        expect(audienceIds.has(a), `${s.key} -> ${a}`).toBe(true);
      }
    }
  });

  it('the MUST_ALWAYS_BE_OPEN floor uses v2 delivery identities and remains open', () => {
    // Intentionally hardcoded — adding a surface to the floor requires deliberate review.
    expect([...mustAlwaysBeOpenDeliverySurfaces()].sort()).toEqual([
      'api.auth-github-callback',
      'api.auth-github-device-poll',
      'api.auth-github-device-start',
      'api.auth-license-refresh',
      'api.auth-otp-request',
      'api.auth-otp-verify',
      'api.auth-session-refresh',
      'cli.admin-credential',
      'cli.auth-login',
      'cli.auth-refresh',
      'cli.login-alias',
      'docs.auth-callback',
      'docs.auth-login',
    ]);
    const byKey = new Map(
      productCatalogue().deliverySurfaces.map((surface) => [surface.key, surface])
    );
    for (const key of mustAlwaysBeOpenDeliverySurfaces()) {
      const surface = byKey.get(key)!;
      expect(effectiveV2Access(surface), `${key} must resolve open`).toBe('open');
    }

    // /auth/verify requires an already-valid credential, while the legacy
    // device flow cannot complete after its confirmation endpoint was removed.
    // They remain callable, but neither is an independent recovery path.
    for (const key of ['api.auth-device-poll', 'api.auth-device-start', 'api.auth-verify']) {
      const surface = byKey.get(key)!;
      expect(effectiveV2Access(surface), `${key} remains callable`).toBe('open');
      expect(surface.posture.mustAlwaysBeOpen, `${key} is not recovery-critical`).toBe(false);
    }
  });

  it('system-invoked surfaces are marked (git hooks must not be kill-switched)', () => {
    const hook = flagSurfaces().surfaces.find((s) => s.key === 'hook');
    expect(hook?.invocation).toBe('system');
  });

  it('the CIB-046 dashboard.aps surface is staff-gated', () => {
    const aps = flagSurfaces().surfaces.find((s) => s.key === 'dashboard.aps');
    expect(aps?.access).toBe('staff');
    expect(aps?.audiences).toContain('staff-internal-developer');
  });
});

describe('v1 compatibility projection', () => {
  it('preserves exact legacy identities and behaviour while canonical v2 reflects current posture', () => {
    const legacy = flagSurfaces();
    expect(legacy.categories.map((group) => group.id)).toEqual(LEGACY_GROUP_IDS);
    expect(legacy.surfaces.map((feature) => feature.key)).toEqual(LEGACY_FEATURE_KEYS);
    expect(legacy.surfaces).toHaveLength(46);

    const openOverrides = new Set([
      'baseline',
      'audit-chain',
      'l4-validate',
      'validate',
      'mcp.serve',
    ]);
    for (const surface of legacy.surfaces) {
      let expected = legacy.categories.find(
        (category) => category.id === surface.category
      )?.defaultAccess;
      if (openOverrides.has(surface.key)) expected = 'open';
      if (surface.key === 'dashboard.aps') expected = 'staff';
      if (surface.key === 'watch') expected = 'licence';
      expect(effectiveV1Access(surface, legacy), surface.key).toBe(expected);
    }
    expect(
      effectiveV1Access(legacy.surfaces.find((surface) => surface.key === 'welcome')!, legacy)
    ).toBe('licence');

    expect(
      Object.fromEntries(
        legacy.surfaces
          .filter((surface) => (surface.audiences?.length ?? 0) > 0)
          .map((surface) => [surface.key, surface.audiences])
      )
    ).toEqual({
      'dashboard.aps': ['staff-internal-developer'],
      'admin.operations': ['staff-anvil-internal'],
    });
    expect(
      legacy.surfaces
        .filter((surface) => surface.invocation === 'system')
        .map((surface) => surface.key)
    ).toEqual(['hook']);
    expect(
      Object.fromEntries(
        legacy.surfaces
          .filter((surface) => surface.requires.length > 0)
          .map((surface) => [surface.key, surface.requires])
      )
    ).toEqual({
      'gate-config': ['gate'],
      'dashboard.architecture': ['architecture'],
      'dashboard.drift': ['drift'],
      'dashboard.suppressions': ['policy'],
    });
    expect(
      legacy.surfaces
        .filter((surface) => surface.mustAlwaysBeOpen)
        .map((surface) => surface.key)
        .sort()
    ).toEqual(['admin.credential', 'auth']);

    const welcome = productCatalogue().deliverySurfaces.find(
      (surface) => surface.key === 'cli.welcome'
    )!;
    expect(effectiveV2Access(welcome)).toBe('open');
  });

  it('is a read-only compatibility view', () => {
    const legacy = flagSurfaces();
    expect(Object.isFrozen(legacy)).toBe(true);
    expect(Object.isFrozen(legacy.categories)).toBe(true);
    expect(Object.isFrozen(legacy.surfaces)).toBe(true);
    expect(Object.isFrozen(legacy.surfaces[0])).toBe(true);
  });
});

describe('current host inventory', () => {
  it('matches the CLI auth gate and recovery aliases at canonical delivery identities', () => {
    const cliSurfaces = productCatalogue().deliverySurfaces.filter(
      (surface) => surface.locator.kind === 'cli'
    );
    const licenceGated = cliSurfaces
      .filter((surface) => effectiveV2Access(surface) === 'licence')
      .map((surface) => surface.key)
      .sort();
    expect(licenceGated).toEqual([
      'cli.architecture',
      'cli.audit',
      'cli.auth-whoami',
      'cli.check',
      'cli.drift',
      'cli.ensure',
      'cli.export',
      'cli.gate',
      'cli.gate-config',
      'cli.init',
      'cli.mcp-config',
      'cli.mcp-install',
      'cli.mcp-pin',
      'cli.mcp-refresh',
      'cli.mcp-unpin',
      'cli.new',
      'cli.policy',
      'cli.skill-install',
      'cli.start',
      'cli.status',
      'cli.watch',
      'cli.whoami-alias',
      'cli.wizard',
    ]);
    expect(
      Object.fromEntries(
        cliSurfaces
          .filter((surface) => surface.featureKey === 'auth')
          .map((surface) => [
            surface.key,
            {
              path: surface.locator.kind === 'cli' ? surface.locator.commandPath : [],
              access: effectiveV2Access(surface),
            },
          ])
      )
    ).toEqual({
      'cli.auth': { path: ['auth'], access: 'open' },
      'cli.auth-login': { path: ['auth', 'login'], access: 'open' },
      'cli.login-alias': { path: ['login'], access: 'open' },
      'cli.auth-logout': { path: ['auth', 'logout'], access: 'open' },
      'cli.logout-alias': { path: ['logout'], access: 'open' },
      'cli.auth-whoami': { path: ['auth', 'whoami'], access: 'licence' },
      'cli.whoami-alias': { path: ['whoami'], access: 'licence' },
      'cli.auth-refresh': { path: ['auth', 'refresh'], access: 'open' },
    });
  });

  it('matches MCP requires_auth and API authentication middleware posture', () => {
    const mcp = productCatalogue().deliverySurfaces.filter(
      (surface) => surface.locator.kind === 'mcp-tool'
    );
    expect(
      mcp
        .filter((surface) => effectiveV2Access(surface) === 'licence')
        .map((surface) => surface.key)
        .sort()
    ).toEqual([
      'mcp-tool.apply-patch',
      'mcp-tool.fix',
      'mcp-tool.gate',
      'mcp-tool.suppress',
      'mcp-tool.validate-write',
    ]);
    expect(mcp.every((surface) => ['open', 'licence'].includes(effectiveV2Access(surface)!))).toBe(
      true
    );

    const api = productCatalogue().deliverySurfaces.filter(
      (surface) => surface.locator.kind === 'api-route'
    );
    expect(
      api
        .filter((surface) => effectiveV2Access(surface) === 'licence')
        .map((surface) => surface.key)
    ).toEqual(['api.account-activity']);
    expect(
      api
        .filter((surface) => effectiveV2Access(surface) === 'admin-key')
        .map((surface) => surface.key)
        .sort()
    ).toEqual(
      api
        .filter((surface) => surface.key.startsWith('api.admin-'))
        .map((surface) => surface.key)
        .sort()
    );
    expect(effectiveV2Access(api.find((surface) => surface.key === 'api.health')!)).toBe('open');
  });

  it('catalogues the independently usable docs-shell routes', () => {
    const docsKeys = productCatalogue()
      .deliverySurfaces.filter((surface) => surface.locator.kind === 'docs-route')
      .map((surface) => surface.key);
    expect(docsKeys).toEqual(
      expect.arrayContaining([
        'docs.shell-landing',
        'docs.auth-login',
        'docs.auth-callback',
        'docs.auth-logout',
        'docs.auth-pending',
        'docs.auth-error',
        'docs.robots',
        'docs.llms',
      ])
    );
  });

  it('covers all supported host identity kinds and reviewed exclusions', () => {
    expect(
      new Set(productCatalogue().deliverySurfaces.map((surface) => surface.key.split('.')[0]))
    ).toEqual(
      new Set([
        'cli',
        'mcp-tool',
        'mcp-resource',
        'api',
        'daemon',
        'dashboard',
        'docs',
        'hook',
        'integration',
      ])
    );
    expect(
      productCatalogue()
        .excludedDeliverySurfaces.map((surface) => surface.key)
        .sort()
    ).toEqual([
      'api.cron-cleanup',
      'cli.graph-base',
      'daemon.session-heartbeat',
      'daemon.session-register',
      'daemon.session-report-process',
      'daemon.session-unregister',
      'daemon.telemetry-subscribe',
      'daemon.telemetry-unsubscribe',
      'docs.assets',
      'docs.images',
      'docs.next-internal',
      'docs.pagefind',
    ]);
  });

  it('does not catalogue protocol constants absent from daemon dispatch', () => {
    const daemonMethods = productCatalogue()
      .deliverySurfaces.filter((surface) => surface.locator.kind === 'daemon-rpc')
      .map((surface) => surface.locator.method);
    expect(daemonMethods).not.toEqual(
      expect.arrayContaining([
        'publishDiagnostics',
        'enforcement/ack',
        'gate/request',
        'suppression/apply',
      ])
    );
  });
});

describe('surface registry schema rejects malformed registries', () => {
  const base = {
    schemaVersion: 1,
    categories: [
      { id: 'governance', name: 'Governance', defaultAccess: 'licence', defaultStatus: 'active' },
      { id: 'tools', name: 'Tools', defaultAccess: 'open', defaultStatus: 'active' },
    ],
  };

  it('rejects an unknown requires-target', () => {
    const result = FlagSurfaceManifestSchema.safeParse({
      ...base,
      surfaces: [{ key: 'a', name: 'a', category: 'tools', requires: ['nope'] }],
    });
    expect(result.success).toBe(false);
  });

  it('rejects a cycle in the requires graph', () => {
    const result = FlagSurfaceManifestSchema.safeParse({
      ...base,
      surfaces: [
        { key: 'a', name: 'a', category: 'tools', requires: ['b'] },
        { key: 'b', name: 'b', category: 'tools', requires: ['a'] },
      ],
    });
    expect(result.success).toBe(false);
    if (!result.success) {
      expect(JSON.stringify(result.error)).toContain('acyclic');
    }
  });

  it('rejects a self-loop in the requires graph', () => {
    const result = FlagSurfaceManifestSchema.safeParse({
      ...base,
      surfaces: [{ key: 'a', name: 'a', category: 'tools', requires: ['a'] }],
    });
    expect(result.success).toBe(false);
  });

  it('rejects duplicate category ids', () => {
    const result = FlagSurfaceManifestSchema.safeParse({
      schemaVersion: 1,
      categories: [
        { id: 'tools', name: 'Tools', defaultAccess: 'open', defaultStatus: 'active' },
        { id: 'tools', name: 'Tools 2', defaultAccess: 'open', defaultStatus: 'active' },
      ],
      surfaces: [],
    });
    expect(result.success).toBe(false);
  });

  it('rejects a mustAlwaysBeOpen surface that resolves to a gated access', () => {
    const result = FlagSurfaceManifestSchema.safeParse({
      ...base,
      surfaces: [{ key: 'a', name: 'a', category: 'governance', mustAlwaysBeOpen: true }],
    });
    expect(result.success).toBe(false);
  });

  it('rejects an unknown category reference', () => {
    const result = FlagSurfaceManifestSchema.safeParse({
      ...base,
      surfaces: [{ key: 'a', name: 'a', category: 'ghost' }],
    });
    expect(result.success).toBe(false);
  });

  it('rejects duplicate surface keys', () => {
    const result = FlagSurfaceManifestSchema.safeParse({
      ...base,
      surfaces: [
        { key: 'a', name: 'a', category: 'tools' },
        { key: 'a', name: 'a2', category: 'tools' },
      ],
    });
    expect(result.success).toBe(false);
  });
});
