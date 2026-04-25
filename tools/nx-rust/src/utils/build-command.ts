import type { ExecutorContext } from '@nx/devkit';
import type { BaseCargoOptions } from '../models/base-options';

// Keys handled out-of-band (toolchain becomes `+toolchain`, args goes after
// `--`, package becomes `-p <pkg>`). Never converted to a `--key value` flag.
const HANDLED_OUT_OF_BAND = new Set<string>(['toolchain', 'args', 'package']);

// Base cargo options every executor accepts. Extra subcommand keys are passed
// in by the executor (e.g. `'all-targets'`, `'no-run'` for the test executor).
// The allowlist is the union — anything else (e.g. vitest's `--coverage`,
// `--run`, `--reporter` leaking through `nx run-many -t test -- <flags>`) is
// silently dropped so cargo never sees flags it doesn't understand.
const BASE_CARGO_KEYS: ReadonlySet<string> = new Set([
  'toolchain',
  'target',
  'profile',
  'release',
  'target-dir',
  'features',
  'all-features',
  'no-default-features',
  'locked',
  'frozen',
  'offline',
  'package',
  'args',
]);

/**
 * Convert `allTargets` → `all-targets`. Executor schemas use camelCase keys
 * because that's what Nx's schema validator prefers, but cargo expects
 * kebab-case flags. This is idempotent for keys that are already kebab-cased.
 */
function toKebabFlag(key: string): string {
  if (key.includes('-')) return key;
  return key.replace(/([A-Z])/g, '-$1').toLowerCase();
}

/**
 * Turn a cargo subcommand + a normalised option bag into the argv cargo wants.
 *
 * Shape:
 *   [+toolchain?] <subcommand> [--key value | --flag]* -p <package> [-- <args>]
 *
 * Only options in `BASE_CARGO_KEYS` ∪ `extraKeys` are forwarded; everything
 * else is silently ignored. This is what stops vitest flags (`--coverage`,
 * `--run`, etc.) leaking into cargo when CI runs
 * `nx run-many -t test -- --coverage` and Nx fans out to every project's
 * `test` target.
 *
 * Handles kebab-case option keys, scalar flags (`--release`), string values
 * (`--target x86_64-...`), array values — `--features` and `--bin` are joined
 * (features comma-separated, bins repeated as one string) and everything else
 * repeats — plus passthrough `args` split between `cargo <sub>` and the binary
 * under `--`.
 *
 * Kept as a pure function so it's unit-testable without touching cargo.
 */
export function buildCargoArgs<T extends BaseCargoOptions>(
  subcommand: string,
  options: T,
  context: Pick<ExecutorContext, 'projectName'>,
  extraKeys: ReadonlyArray<string> = []
): string[] {
  const allowed: ReadonlySet<string> = extraKeys.length
    ? new Set([...BASE_CARGO_KEYS, ...extraKeys])
    : BASE_CARGO_KEYS;

  // The iterator below uses Object.entries, which at runtime sees every own
  // enumerable property regardless of declared type.
  const opts = options as unknown as Record<string, unknown>;
  const out: string[] = [];

  if (options.toolchain && options.toolchain !== 'stable') {
    out.push(`+${options.toolchain}`);
  }

  out.push(subcommand);

  // `release` is a bool, but `profile` is a string — profile wins and we drop
  // the --release flag entirely so cargo doesn't complain about conflicts.
  const hasProfile = typeof options.profile === 'string' && options.profile.length > 0;

  for (const [rawKey, rawValue] of Object.entries(opts)) {
    if (!allowed.has(rawKey)) continue;
    if (HANDLED_OUT_OF_BAND.has(rawKey)) continue;
    if (rawValue === undefined || rawValue === null) continue;
    if (rawKey === 'release' && hasProfile) continue;

    const flag = `--${toKebabFlag(rawKey)}`;

    if (typeof rawValue === 'boolean') {
      if (rawValue) out.push(flag);
    } else if (Array.isArray(rawValue)) {
      if (rawKey === 'features') {
        const joined = rawValue
          .filter((v) => v !== undefined && v !== null && v !== '')
          .map((v) => String(v))
          .join(',');
        if (joined) out.push(flag, joined);
      } else {
        for (const item of rawValue) {
          if (item === undefined || item === null) continue;
          out.push(flag, String(item));
        }
      }
    } else {
      out.push(flag, String(rawValue));
    }
  }

  // Scope to the Nx project's cargo package unless the caller already set one.
  const pkg = options.package ?? context.projectName;
  if (pkg && !out.includes('--package') && !out.includes('-p')) {
    out.push('-p', pkg);
  }

  if (options.args !== undefined) {
    out.push('--');
    if (Array.isArray(options.args)) {
      for (const a of options.args) out.push(String(a));
    } else {
      out.push(String(options.args));
    }
  }

  return out;
}
