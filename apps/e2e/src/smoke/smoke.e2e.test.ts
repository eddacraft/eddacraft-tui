/**
 * Smoke Tests — All Surfaces
 *
 * Quick canary tests that verify each surface can be loaded and responds
 * to basic operations. These are intentionally shallow — they catch
 * import failures, missing exports, and startup crashes without testing
 * business logic in depth.
 *
 * Run these first: `pnpm --filter @eddacraft/anvil-e2e test:smoke`
 *
 * Surface: ALL
 */

import { describe, it, expect } from 'vitest';

// ─── Surface: Contracts ─────────────────────────────────────────

describe('Smoke › @eddacraft/anvil-contracts', () => {
  it('exports APS_SCHEMA_VERSION', async () => {
    const mod = await import('@eddacraft/anvil-contracts');
    expect(mod.APS_SCHEMA_VERSION).toBeDefined();
    expect(typeof mod.APS_SCHEMA_VERSION).toBe('string');
  });

  it('exports APSPlanSchema', async () => {
    const mod = await import('@eddacraft/anvil-contracts');
    expect(mod.APSPlanSchema).toBeDefined();
    expect(typeof mod.APSPlanSchema.safeParse).toBe('function');
  });

  it('exports createPlan', async () => {
    const mod = await import('@eddacraft/anvil-contracts');
    expect(typeof mod.createPlan).toBe('function');
  });
});

// ─── Surface: Core ──────────────────────────────────────────────

describe('Smoke › @eddacraft/anvil-core', () => {
  it('exports antipattern scanning', async () => {
    const mod = await import('@eddacraft/anvil-core/antipattern');
    expect(typeof mod.scanFile).toBe('function');
    expect(typeof mod.scanFiles).toBe('function');
    expect(typeof mod.getEnabledPatterns).toBe('function');
  });

  it('exports drift detection', async () => {
    const mod = await import('@eddacraft/anvil-core/drift');
    expect(typeof mod.createEmptySnapshot).toBe('function');
    expect(typeof mod.compareSnapshots).toBe('function');
  });

  it('exports validation', async () => {
    const mod = await import('@eddacraft/anvil-core/validation');
    expect(typeof mod.validateAPSPlan).toBe('function');
  });

  it('exports crypto utilities', async () => {
    const mod = await import('@eddacraft/anvil-core/crypto');
    expect(typeof mod.generateHash).toBe('function');
  });
});

// ─── Surface: Runtime ───────────────────────────────────────────

describe('Smoke › @eddacraft/anvil-runtime', () => {
  it('exports gate runner', async () => {
    const mod = await import('@eddacraft/anvil-runtime/gate');
    expect(mod.GateRunner).toBeDefined();
    expect(mod.GateConfigManager).toBeDefined();
  });

  it('exports cache providers', async () => {
    const mod = await import('@eddacraft/anvil-runtime/cache');
    expect(mod).toBeDefined();
  });

  it('exports watch utilities', async () => {
    const mod = await import('@eddacraft/anvil-runtime/watch');
    expect(typeof mod.createFileWatcher).toBe('function');
  });

  it('exports export utilities', async () => {
    const mod = await import('@eddacraft/anvil-runtime/export');
    expect(mod).toBeDefined();
  });
});

// ─── Surface: APS Parser ────────────────────────────────────────

describe('Smoke › @eddacraft/anvil-aps', () => {
  it('exports parser and loader', async () => {
    const mod = await import('@eddacraft/anvil-aps');
    expect(mod).toBeDefined();
    // APS should export parsing / loading functionality
    const exportNames = Object.keys(mod);
    expect(exportNames.length).toBeGreaterThan(0);
  });
});

// ─── Surface: Adapters ──────────────────────────────────────────

describe('Smoke › @eddacraft/anvil-adapters', () => {
  it('exports the adapter registry', async () => {
    const mod = await import('@eddacraft/anvil-adapters');
    expect(mod.registry).toBeDefined();
    expect(typeof mod.registry.detect).toBe('function');
    expect(typeof mod.registry.getAll).toBe('function');
  });

  it('has registered adapters on import', async () => {
    const mod = await import('@eddacraft/anvil-adapters');
    expect(mod.registry.getAll().length).toBeGreaterThan(0);
  });
});

// ─── Surface: MCP Server ────────────────────────────────────────

describe('Smoke › @eddacraft/anvil-mcp-server', () => {
  it('exports createAnvilMcpServer', async () => {
    const mod = await import('@eddacraft/anvil-mcp-server');
    expect(typeof mod.createAnvilMcpServer).toBe('function');
  });

  it('exports config generation', async () => {
    const mod = await import('@eddacraft/anvil-mcp-server');
    expect(typeof mod.generateMcpConfig).toBe('function');
    expect(Array.isArray(mod.SUPPORTED_TARGETS)).toBe(true);
  });
});

// ─── Surface: Edda Stack ────────────────────────────────────────

describe('Smoke › @eddacraft/anvil-edda-stack', () => {
  it('exports package metadata', async () => {
    const mod = await import('@eddacraft/anvil-edda-stack');
    expect(mod.PACKAGE_NAME).toBe('@eddacraft/anvil-edda-stack');
    expect(mod.PACKAGE_VERSION).toBeDefined();
  });

  it('exports contracts', async () => {
    const mod = await import('@eddacraft/anvil-edda-stack');
    expect(mod).toBeDefined();
    const exportNames = Object.keys(mod);
    expect(exportNames.length).toBeGreaterThan(2); // more than just PACKAGE_NAME/VERSION
  });
});

// ─── Surface: API ───────────────────────────────────────────────

describe('Smoke › @eddacraft/anvil-api', () => {
  it('exports a Hono app', async () => {
    const mod = await import('@eddacraft/anvil-api');
    const app = mod.default;
    expect(app).toBeDefined();
    expect(typeof app.request).toBe('function');
  });

  it('health endpoint responds', async () => {
    const mod = await import('@eddacraft/anvil-api');
    const res = await mod.default.request(
      new Request('http://localhost/api/v1/health')
    );
    expect(res.status).toBe(200);
  });
});

// ─── Surface: CLI (binary exists) ───────────────────────────────

describe('Smoke › CLI binary', () => {
  it('built CLI entry point exists', async () => {
    const { existsSync } = await import('node:fs');
    const { resolve } = await import('node:path');
    const cliPath = resolve(__dirname, '../../../../anvil-cli/dist/index.js');
    // This test fails fast if the CLI hasn't been built
    expect(existsSync(cliPath)).toBe(true);
  });
});
