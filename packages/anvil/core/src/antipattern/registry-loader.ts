/**
 * Registry loader — bridges the compiled `.anvil` pattern registry into the
 * scanner's in-memory `AntiPattern` shape.
 *
 * The compiled registry (`patterns/compiled/registry.json`) is produced by the
 * `patterns:compile` script. At runtime the scanner needs an `AntiPattern[]`
 * in the shape `patterns.ts` has always exposed, so this module reads the
 * JSON, validates it with Zod, and maps each `CompiledPattern` to an
 * `AntiPattern` with family provenance attached.
 *
 * Resolution order for the registry file:
 *   1. `opts.registryPath` explicit override (tests).
 *   2. `ANVIL_REGISTRY_PATH` env var.
 *   3. Upward walk from `process.cwd()` looking for
 *      `patterns/compiled/registry.json`.
 *   4. Upward walk from this module's file URL (handles running from
 *      `node_modules` when cwd is outside the monorepo).
 *
 * If no registry is found, the loader returns `{ patterns: [], warnings: [...] }`.
 * The scanner still works — only the legacy TS HTML/CSS catalogue fires —
 * so running outside a compiled tree degrades gracefully rather than crashing.
 */

import { existsSync, readFileSync } from 'node:fs';
import * as path from 'node:path';
import { fileURLToPath } from 'node:url';

import type { AntiPattern, DetectionConfig } from './types.js';
import {
  CompiledRegistrySchema,
  type CompiledPattern,
  type CompiledRegistry,
  type Detection as AnvilDetection,
} from './format/schemas.js';

export interface LoadRegistryOptions {
  /** Absolute path to a `registry.json` — overrides discovery. */
  registryPath?: string;
}

export interface LoadRegistryResult {
  registry: CompiledRegistry | null;
  /** Source path used, for diagnostics. */
  sourcePath: string | null;
  /** Non-fatal issues (missing file, parse errors). */
  warnings: string[];
}

const REGISTRY_RELATIVE_PATH = path.join('patterns', 'compiled', 'registry.json');

function walkUpwards(start: string): string | null {
  let current = path.resolve(start);
  while (true) {
    const candidate = path.join(current, REGISTRY_RELATIVE_PATH);
    if (existsSync(candidate)) return candidate;
    const parent = path.dirname(current);
    if (parent === current) return null;
    current = parent;
  }
}

function moduleStartDir(): string {
  try {
    return path.dirname(fileURLToPath(import.meta.url));
  } catch {
    return process.cwd();
  }
}

function resolveRegistryPath(opts: LoadRegistryOptions): string | null {
  if (opts.registryPath) {
    return existsSync(opts.registryPath) ? opts.registryPath : null;
  }

  const fromEnv = process.env.ANVIL_REGISTRY_PATH;
  if (fromEnv) {
    return existsSync(fromEnv) ? fromEnv : null;
  }

  const fromCwd = walkUpwards(process.cwd());
  if (fromCwd) return fromCwd;

  return walkUpwards(moduleStartDir());
}

/**
 * Map an anvil category (from compiled registry) to a valid AntiPattern
 * category. The compiled categories come from family definitions
 * (escape-hatch, type-evasion, error-handling, accountability, deferred-debt);
 * the AntiPattern enum accepts these directly after the extension in types.ts.
 * Unknown values fall back to 'code-quality' as a catch-all.
 */
function mapCategory(anvilCategory: string): AntiPattern['category'] {
  const known: ReadonlyArray<AntiPattern['category']> = [
    'escape-hatch',
    'error-handling',
    'code-quality',
    'type-safety',
    'html',
    'css',
    'type-evasion',
    'accountability',
    'deferred-debt',
  ];
  return (known as readonly string[]).includes(anvilCategory)
    ? (anvilCategory as AntiPattern['category'])
    : 'code-quality';
}

/**
 * Convert a single compiled pattern into the scanner's `AntiPattern` shape.
 *
 * The compiled pattern carries richer metadata (family, spectrum, targets)
 * than the legacy AntiPattern, so the extra fields are preserved via the
 * optional `family` / `definitionRef` / `spectrumPosition` / `targets`
 * properties added in Phase 2. Downstream warning emission surfaces them on
 * the `Warning` object.
 */
/**
 * The compiled registry's Detection schema uses `ast_query` (snake_case to
 * match YAML convention); the legacy scanner's `DetectionConfig` uses
 * `astQuery` (camelCase). Convert between them so the scanner sees a single
 * shape regardless of where the pattern originated.
 */
function mapDetection(d: AnvilDetection): DetectionConfig {
  if (d.type === 'regex') {
    return {
      type: 'regex',
      pattern: d.pattern,
      ...(d.flags ? { flags: d.flags } : {}),
    };
  }
  return { type: 'ast', astQuery: d.ast_query };
}

export function compiledToAntiPattern(cp: CompiledPattern): AntiPattern {
  return {
    id: cp.id,
    name: cp.title,
    category: mapCategory(cp.category),
    severity: cp.severity,
    confidence: cp.confidence,
    detection: mapDetection(cp.detection),
    title: cp.title,
    explanation: cp.explanation,
    suggestion: cp.suggestion,
    nudge: cp.nudge,
    ...(cp.file_extensions ? { fileExtensions: cp.file_extensions } : {}),
    ...(cp.allowlist.length > 0 ? { allowlist: [...cp.allowlist] } : {}),
    enabled: cp.enabled,
    optIn: cp.opt_in,
    family: cp.family,
    definitionRef: cp.definition_ref,
    spectrumPosition: cp.spectrum_position,
    targets: [...cp.targets],
  };
}

let cached: LoadRegistryResult | null = null;
let cachedKey: string | null = null;

/**
 * Load and validate the compiled registry.
 *
 * Caches the result per-path. Pass `{ registryPath }` in tests to target
 * a fixture; omit in production to let discovery find the workspace registry.
 */
export function loadCompiledRegistry(opts: LoadRegistryOptions = {}): LoadRegistryResult {
  const resolved = resolveRegistryPath(opts);
  const key = resolved ?? '__none__';

  if (cached && cachedKey === key) return cached;

  if (!resolved) {
    cached = {
      registry: null,
      sourcePath: null,
      warnings: ['Compiled pattern registry not found; legacy HTML/CSS patterns only.'],
    };
    cachedKey = key;
    return cached;
  }

  try {
    const raw = readFileSync(resolved, 'utf-8');
    const json: unknown = JSON.parse(raw);
    const parsed = CompiledRegistrySchema.safeParse(json);
    if (!parsed.success) {
      cached = {
        registry: null,
        sourcePath: resolved,
        warnings: [`Registry at ${resolved} failed schema validation: ${parsed.error.message}`],
      };
      cachedKey = key;
      return cached;
    }
    cached = { registry: parsed.data, sourcePath: resolved, warnings: [] };
    cachedKey = key;
    return cached;
  } catch (err) {
    cached = {
      registry: null,
      sourcePath: resolved,
      warnings: [
        `Failed to read registry at ${resolved}: ${err instanceof Error ? err.message : String(err)}`,
      ],
    };
    cachedKey = key;
    return cached;
  }
}

/**
 * Reset the cached registry. Intended for tests that need to exercise
 * discovery or simulate a different registry per case.
 */
export function resetRegistryCache(): void {
  cached = null;
  cachedKey = null;
}

/**
 * Load the registry and return the mapped anti-patterns in the shape the
 * scanner expects. Returns `[]` if no registry is available.
 */
export function loadRegistryPatterns(opts: LoadRegistryOptions = {}): AntiPattern[] {
  const { registry } = loadCompiledRegistry(opts);
  if (!registry) return [];
  return registry.patterns.map(compiledToAntiPattern);
}
