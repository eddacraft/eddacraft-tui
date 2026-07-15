import { Hono } from 'hono';
import { getClient } from '../db/client.js';
import { insertTelemetryBeacon } from '../db/queries.js';
import { beaconSchema, SUPPORTED_TELEMETRY_SCHEMA_VERSIONS } from './telemetry-schemas.js';

// FLEET-005 (ADR-107 §6): the fleet telemetry ingest route. Beacons arrive
// from arbitrary installs, so the POST is deliberately unauthenticated; it
// sits behind the app-level shared rate limiter (rateLimiter() on '*' in
// index.ts) and the /api/v1 versioned base path like every other route.
// Schema evolution is double-versioned: the path version via the base path,
// and schema_version in the body (SUPPORTED_TELEMETRY_SCHEMA_VERSIONS).
//
// Privacy posture (ADR-107 §3), enforced here and in the storage layer:
//   - the payload is a strict allowlist; unknown fields are rejected, and
//     every string is a charset-capped token, so PII-shaped values cannot
//     validate;
//   - the request IP is never read and never reaches a query — the stored
//     row has no ip column at all (see 017-telemetry-beacons.sql);
//   - no timestamp is passed to the insert; arrival time coarsens to a
//     DATE via the received_on column default;
//   - error bodies never echo payload values back, so a rejected free-form
//     field is not reflected either.
//
// There is intentionally NO read/debug endpoint here: the fleet view is
// FLEET-007, and anything read-side must mount the admin-auth middleware.

export const telemetry = new Hono();

telemetry.post('/', async (c) => {
  try {
    const databaseUrl = process.env.DATABASE_URL;
    if (!databaseUrl) {
      console.error('DATABASE_URL not configured');
      return c.json({ error: 'Service unavailable' }, 503);
    }

    const contentType = c.req.header('content-type') ?? '';
    if (!contentType.toLowerCase().includes('application/json')) {
      return c.json({ error: 'Content-Type must be application/json' }, 400);
    }

    let body: unknown;
    try {
      body = await c.req.json();
    } catch {
      return c.json({ error: 'Invalid JSON payload' }, 400);
    }

    if (!body || typeof body !== 'object' || Array.isArray(body)) {
      return c.json({ error: 'Invalid JSON payload' }, 400);
    }

    // schema_version gates everything else: a missing or unknown version is
    // reported specifically so an evolved client fails loud against an old
    // server instead of drowning in field-level errors.
    const { schema_version: schemaVersion } = body as { schema_version?: unknown };
    if (schemaVersion === undefined) {
      return c.json({ error: 'schema_version is required' }, 400);
    }
    if (
      typeof schemaVersion !== 'number' ||
      !SUPPORTED_TELEMETRY_SCHEMA_VERSIONS.includes(schemaVersion)
    ) {
      return c.json(
        {
          error: 'Unsupported schema_version',
          supported: SUPPORTED_TELEMETRY_SCHEMA_VERSIONS,
        },
        400
      );
    }

    const parsed = beaconSchema.safeParse(body);
    if (!parsed.success) {
      // Field paths only — never the received values (they may be exactly
      // the free-form data the allowlist exists to keep out).
      const fields = [...new Set(parsed.error.issues.map((issue) => issue.path.join('.')))];
      return c.json({ error: 'Invalid beacon payload', fields }, 400);
    }

    const sql = getClient();
    await insertTelemetryBeacon(sql, parsed.data);

    return c.json({ accepted: true }, 202);
  } catch (error: unknown) {
    if (error instanceof Error) {
      console.error('Telemetry ingest error:', error.message);
    } else {
      console.error('Telemetry ingest error:', error);
    }
    return c.json({ error: 'Failed to record beacon' }, 500);
  }
});
