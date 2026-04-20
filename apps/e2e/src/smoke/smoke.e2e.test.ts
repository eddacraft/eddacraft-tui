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
import { cliBinaryAvailable, runCliExpectSuccess } from '../helpers/cli-runner.js';

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
    expect(typeof mod.registry.detectAdapter).toBe('function');
    expect(typeof mod.registry.listAdapters).toBe('function');
  });

  it('has registered adapters on import', async () => {
    const mod = await import('@eddacraft/anvil-adapters');
    expect(mod.registry.listAdapters().length).toBeGreaterThan(0);
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
    const res = await mod.default.request(new Request('http://localhost/api/v1/health'));
    // Smoke: endpoint is wired up. 200 = healthy, 503 = degraded (no DB/signing
    // key in test env). Both prove the route responds.
    expect([200, 503]).toContain(res.status);
    const body = (await res.json()) as { status?: string };
    expect(body.status).toBeDefined();
  });
});

// ─── Surface: CLI (binary exists) ───────────────────────────────
//
// The CLI is now a Rust binary in crates/anvil-cli/ (ADR-011, ADR-011a). When
// built via `cargo build` it lands at target/{debug,release}/anvil. The smoke
// test skips visibly when no binary is present (rust.yml validates the build)
// and, when present, actually invokes the binary so "discoverable" is not a
// lie — discovery alone would pass against a zero-byte or wrong-arch file.

describe('Smoke › CLI binary', () => {
  const maybeIt = cliBinaryAvailable() ? it : it.skip;

  maybeIt('built Rust CLI responds to --version', async () => {
    const result = await runCliExpectSuccess(['--version']);
    expect(result.stdout.toLowerCase()).toContain('anvil');
  });
});
