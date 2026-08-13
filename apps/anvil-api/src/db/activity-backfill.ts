/**
 * BACT-012 (ADR-121, OQ-B): one-shot operational backfill of
 * `beta_users.last_activity_at` from `max(refresh_tokens.created_at)`.
 *
 * Pre-BACT-008 token-era accounts refresh sessions without ever completing
 * a fresh interactive login, so `last_activity_at` starts NULL for them even
 * though they were recently using the product. This is a one-shot proxy —
 * it does not claim interactive login history and it must NEVER set
 * `first_login_at` / `last_login_at` / `last_login_method` (OQ-B); those
 * columns are owned exclusively by BACT-002's interactive-mint paths.
 *
 * Mirrors `db/migrate.ts`'s shape: a small `QueryRunner` interface plus a
 * pure-ish function the CLI wrapper (`scripts/backfill-activity.mjs`) drives
 * with a real `Pool`, and tests drive with an in-memory fake — no live
 * Postgres required to prove behaviour (see `__tests__/activity-backfill.test.ts`).
 *
 * Dry-run by default; the CLI requires an explicit `--apply` to write
 * (`apply: true` here). Idempotent: the `last_activity_at IS NULL` guard on
 * the UPDATE means a second apply run always affects 0 rows.
 */

export interface QueryRunner {
  query: (text: string, params?: unknown[]) => Promise<{ rows: unknown[] }>;
}

export interface RunOptions {
  /** Write changes. Defaults to false (dry-run, report only). */
  apply?: boolean;
  log?: (message: string) => void;
}

export interface BackfillResult {
  dryRun: boolean;
  /** Rows that were (or, in dry-run, would be) updated. */
  affected: number;
}

// Read-only: counts accounts eligible for backfill without writing anything.
// Eligible = last_activity_at IS NULL (only-null guard) AND at least one
// refresh_tokens row exists for the account. Used for the dry-run report.
export const BACKFILL_COUNT_SQL = `
SELECT count(*)::int AS affected
FROM beta_users u
WHERE u.last_activity_at IS NULL
  AND EXISTS (
    SELECT 1 FROM refresh_tokens rt WHERE rt.user_id = u.id
  )
`.trim();

// Writes ONLY last_activity_at and last_activity_kind, and ONLY for rows
// where last_activity_at IS NULL (the only-null guard that also makes this
// idempotent). Deliberately never names first_login_at / last_login_at /
// last_login_method — see the OQ-B proof in __tests__/activity-backfill.test.ts,
// which asserts this exact string never matches those column names.
export const BACKFILL_UPDATE_SQL = `
UPDATE beta_users u
SET last_activity_at = rt.last_created_at,
    last_activity_kind = 'refresh'
FROM (
  SELECT user_id, max(created_at) AS last_created_at
  FROM refresh_tokens
  GROUP BY user_id
) rt
WHERE u.id = rt.user_id
  AND u.last_activity_at IS NULL
RETURNING 1
`.trim();

export async function runActivityBackfill(
  runner: QueryRunner,
  options: RunOptions = {}
): Promise<BackfillResult> {
  const log = options.log ?? (() => {});
  const apply = options.apply === true;

  if (!apply) {
    const result = await runner.query(BACKFILL_COUNT_SQL);
    const row = result.rows[0] as { affected: number | string } | undefined;
    const affected = Number(row?.affected ?? 0);
    log(
      `--dry-run: would backfill last_activity_at (kind=refresh) for ${affected} account(s) ` +
        `from max(refresh_tokens.created_at). No rows written. Re-run with --apply to write.`
    );
    return { dryRun: true, affected };
  }

  const result = await runner.query(BACKFILL_UPDATE_SQL);
  const affected = result.rows.length;
  log(
    `backfilled last_activity_at (kind=refresh) for ${affected} account(s) ` +
      `from max(refresh_tokens.created_at).`
  );
  return { dryRun: false, affected };
}
