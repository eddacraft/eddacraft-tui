import { z } from 'zod';

// FLEET-005 (ADR-107 §3): the beacon payload is a hard-enumerated dimension
// allowlist — nothing else. Every string field is a charset-constrained,
// length-capped token, so free-form values (paths, repo names, hostnames,
// emails, arguments) are rejected by construction, not by inspection.
// Adding any dimension requires a dated amendment to ADR-107.

/** The beacon wire-format version this deployment understands. */
export const TELEMETRY_SCHEMA_VERSION = 1;

/**
 * Versions the ingest route accepts. Schema evolution adds the new version
 * here alongside a discriminated schema; unknown versions are rejected with
 * a specific error so old servers fail loud, not lossy.
 */
export const SUPPORTED_TELEMETRY_SCHEMA_VERSIONS: readonly number[] = [TELEMETRY_SCHEMA_VERSION];

/** Install-method detection values (LAUNCH-013), per ADR-107 §3. */
export const INSTALL_METHODS = [
  'homebrew',
  'scoop',
  'winget',
  'cargo_dist',
  'cargo_install',
  'dev_build',
  'unknown',
] as const;

export type InstallMethod = (typeof INSTALL_METHODS)[number];

/**
 * One `(feature key, count)` usage stat per feature actually used since the
 * last beacon (FLAGS-design contract). The flag catalogue is small; the cap
 * bounds a hostile payload, it does not constrain legitimate use.
 */
export const MAX_FEATURES_PER_BEACON = 128;

// Token charsets. Length caps live inside the patterns so a single regex
// check enforces both; none of them admit whitespace, `/`, `@`, or `\`,
// which is what keeps paths / emails / hostnames out by construction.
const VERSION_PATTERN = /^[0-9A-Za-z][0-9A-Za-z.+-]{0,63}$/; // e.g. 0.9.0-beta
const PLATFORM_TRIPLE_PATTERN = /^[0-9A-Za-z_][0-9A-Za-z_.-]{0,63}$/; // e.g. x86_64-unknown-linux-gnu
const CHANNEL_PATTERN = /^[0-9a-z][0-9a-z_-]{0,31}$/; // e.g. stable / beta / nightly
const FLAG_SNAPSHOT_VERSION_PATTERN = /^[0-9A-Za-z][0-9A-Za-z.-]{0,63}$/;
const FEATURE_KEY_PATTERN = /^[0-9A-Za-z][0-9A-Za-z._-]{0,127}$/; // e.g. anvil.check

const featureUsageSchema = z.strictObject({
  key: z.string().regex(FEATURE_KEY_PATTERN),
  count: z.number().int().min(0).max(1_000_000_000),
});

/**
 * Schema-version-1 beacon. `strictObject` rejects unknown keys rather than
 * stripping them: an out-of-allowlist field is a contract violation we want
 * to hear about (400), never data we quietly accept or discard.
 */
export const beaconSchema = z.strictObject({
  schema_version: z.literal(TELEMETRY_SCHEMA_VERSION),
  install_id: z.uuid(),
  version: z.string().regex(VERSION_PATTERN),
  install_method: z.enum(INSTALL_METHODS),
  platform: z.string().regex(PLATFORM_TRIPLE_PATTERN),
  channel: z.string().regex(CHANNEL_PATTERN),
  flag_snapshot_version: z.string().regex(FLAG_SNAPSHOT_VERSION_PATTERN),
  features: z.array(featureUsageSchema).max(MAX_FEATURES_PER_BEACON),
});

export type TelemetryBeacon = z.infer<typeof beaconSchema>;
