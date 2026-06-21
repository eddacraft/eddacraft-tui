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

import { spawn } from 'node:child_process';
import { describe, it, expect } from 'vitest';
import {
  cliBinaryAvailable,
  resolveCliBinary,
  runCliExpectSuccess,
} from '../helpers/cli-runner.js';
import { createE2EWorkspace } from '../helpers/workspace.js';

type JsonRpcResponse = {
  id?: number | string | null;
  result?: {
    tools?: Array<{ name?: string }>;
    content?: Array<{ type?: string; text?: string }>;
    isError?: boolean;
  };
  error?: { code?: number; message?: string };
};

type ValidateWritePayload = {
  decision?: string;
  safeDefault?: string;
  summary?: { total?: number; bySeverity?: { error?: number } };
  diagnostics?: Array<{ category?: string; source?: { rule_id?: string } }>;
  correlation?: { surface?: string; mode?: string; backend?: string };
};

function responseById(responses: JsonRpcResponse[], id: number): JsonRpcResponse {
  const response = responses.find((candidate) => candidate.id === id);
  if (!response) {
    throw new Error(`Missing JSON-RPC response id ${id}. Got: ${JSON.stringify(responses)}`);
  }
  return response;
}

function parseToolPayload(response: JsonRpcResponse): ValidateWritePayload {
  const text = response.result?.content?.[0]?.text;
  if (!text) {
    throw new Error(`Missing MCP tool text payload. Got: ${JSON.stringify(response)}`);
  }
  return JSON.parse(text) as ValidateWritePayload;
}

async function runMcpLaunchShim(
  cwd: string,
  frames: Array<Record<string, unknown>>
): Promise<JsonRpcResponse[]> {
  const binary = resolveCliBinary();
  if (!binary) {
    throw new Error('anvil CLI binary not found');
  }

  return new Promise((resolve, reject) => {
    const child = spawn(binary, ['--no-tui', 'mcp', 'serve', '--stdio'], {
      cwd,
      env: {
        ...process.env,
        CI: 'true',
        FORCE_COLOR: '0',
        NO_COLOR: '1',
        NO_TUI: '1',
      },
      stdio: ['pipe', 'pipe', 'pipe'],
    });

    let stdout = '';
    let stderr = '';
    let settled = false;

    const finish = (error?: Error) => {
      if (settled) return;
      settled = true;
      clearTimeout(timeout);
      if (error) reject(error);
    };

    const timeout = setTimeout(() => {
      child.kill();
      finish(new Error(`Timed out waiting for MCP launch shim. stderr: ${stderr}`));
    }, 10_000);

    child.stdout.setEncoding('utf8');
    child.stderr.setEncoding('utf8');
    child.stdout.on('data', (chunk: string) => {
      stdout += chunk;
    });
    child.stderr.on('data', (chunk: string) => {
      stderr += chunk;
    });
    child.on('error', (error) => finish(error));
    child.on('close', (code) => {
      if (settled) return;
      settled = true;
      clearTimeout(timeout);
      if (code !== 0) {
        reject(new Error(`MCP launch shim exited ${code}. stderr: ${stderr}`));
        return;
      }

      try {
        const responses = stdout
          .split(/\r?\n/)
          .map((line) => line.trim())
          .filter(Boolean)
          .map((line) => JSON.parse(line) as JsonRpcResponse);
        resolve(responses);
      } catch (error) {
        reject(new Error(`MCP stdout contained non-JSON frames: ${stdout}\n${String(error)}`));
      }
    });

    for (const frame of frames) {
      child.stdin.write(`${JSON.stringify(frame)}\n`);
    }
    child.stdin.end();
  });
}

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
  // antipattern scanner + drift detection archived under ADR-033
  // → anvil-archive/anvil-ts-scanner/. The Rust scanner is the sole engine;
  // drift was scoped to anti-pattern deltas and has no live equivalent.

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
  // Gate runner + export utilities archived under ADR-033
  // → anvil-archive/anvil-ts-scanner/runtime-gate/, runtime-export/.
  // The Rust CLI / RMCP shim are the gate-evaluation path now.

  it('exports cache providers', async () => {
    const mod = await import('@eddacraft/anvil-runtime/cache');
    expect(mod).toBeDefined();
  });

  it('exports watch utilities', async () => {
    const mod = await import('@eddacraft/anvil-runtime/watch');
    expect(typeof mod.createFileWatcher).toBe('function');
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

// ─── Surface: MCP Server (archived per ADR-033) ─────────────────
// Smoke tests for `@eddacraft/anvil-mcp-server` removed —
// package archived to `anvil-archive/anvil-mcp-server/`. The launch MCP
// path runs through RMCP (`anvil mcp serve --stdio` in the Rust
// binary); RMCPF will replace these contracts in Rust and own its
// own smoke tests.

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

// ─── Surface: Rust MCP Launch Shim (binary exists) ───────────────

describe('Smoke › Rust MCP launch shim', () => {
  const maybeIt = cliBinaryAvailable() ? it : it.skip;

  maybeIt('lists tools and validates safe and blocked proposed writes over stdio', async () => {
    const workspace = createE2EWorkspace({
      files: {
        'src/existing.ts': 'export const existing = true;\n',
      },
    });

    try {
      const responses = await runMcpLaunchShim(workspace.root, [
        {
          jsonrpc: '2.0',
          id: 100,
          method: 'initialize',
          params: {
            protocolVersion: '2024-11-05',
            capabilities: {},
            clientInfo: { name: 'anvil-e2e-smoke', version: '0.0.0' },
          },
        },
        {
          jsonrpc: '2.0',
          method: 'notifications/initialized',
        },
        {
          jsonrpc: '2.0',
          id: 101,
          method: 'tools/list',
        },
        {
          jsonrpc: '2.0',
          id: 102,
          method: 'tools/call',
          params: {
            name: 'anvil_validate_write',
            arguments: {
              workspaceRoot: workspace.root,
              path: 'src/safe.ts',
              operation: 'create',
              proposedContent: 'export const value = 1;\n',
            },
          },
        },
        {
          jsonrpc: '2.0',
          id: 103,
          method: 'tools/call',
          params: {
            name: 'anvil_validate_write',
            arguments: {
              workspaceRoot: workspace.root,
              path: 'config/credentials.example',
              operation: 'create',
              proposedContent: "export const token = 'ghp_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa';\n",
            },
          },
        },
        {
          jsonrpc: '2.0',
          id: 104,
          method: 'shutdown',
        },
        {
          jsonrpc: '2.0',
          method: 'exit',
        },
      ]);

      expect(responseById(responses, 100).result).toBeDefined();

      const tools = responseById(responses, 101).result?.tools ?? [];
      expect(tools.some((tool) => tool.name === 'anvil_validate_write')).toBe(true);

      const safePayload = parseToolPayload(responseById(responses, 102));
      expect(safePayload.decision).toBe('allow');
      expect(safePayload.summary?.total).toBe(0);
      expect(safePayload.correlation?.surface).toBe('mcp');
      expect(safePayload.correlation?.mode).toBe('preWrite');

      const blockedResponse = responseById(responses, 103);
      expect(blockedResponse.result?.isError).toBe(true);

      const blockedPayload = parseToolPayload(blockedResponse);
      expect(blockedPayload.decision).toBe('block');
      expect(blockedPayload.safeDefault).toBe('do-not-write');
      expect(blockedPayload.summary?.bySeverity?.error).toBe(1);
      expect(blockedPayload.diagnostics?.[0]?.category).toBe('secret');
      expect(blockedPayload.diagnostics?.[0]?.source?.rule_id).toBe('secret-detection');
    } finally {
      workspace.cleanup();
    }
  });
});
