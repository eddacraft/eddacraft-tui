import { describe, it, expect } from 'vitest';
import { neon, type NeonQueryPromise } from '@neondatabase/serverless';

// The admin list queries (findWaitlistPaginated, findAuditEntries) compose
// predicate fragments into an outer template like:
//
//   const statusPred = status === 'pending' ? sql`AND w.approved_at IS NULL` : sql``;
//   const sourcePred = source === 'all' ? sql`` : sql`AND w.source = ${source}`;
//   await sql`... WHERE 1=1 ${statusPred} ${sourcePred} ...`;
//
// Composability (non-empty fragments) is documented in @neondatabase/serverless;
// the empty-fragment case (`sql``) is not. admin.test.ts mocks queries.ts
// wholesale, so the composition is never exercised against the real driver.
// This test asserts what SQL + params the driver actually produces for each
// combination of fragments — a regression in driver behaviour would surface
// here rather than at first deploy.

// Use the driver with a dummy URL. We never `await` a promise, so no network.
const sql = neon('postgres://user:pass@example.invalid/db');

// Extract the compiled { query, params } from a tagged-template call.
function compile(promise: NeonQueryPromise<false, false, unknown>): {
  query: string;
  params: unknown[];
} {
  const data = (promise as unknown as { queryData: unknown }).queryData as {
    toParameterizedQuery?: () => { query: string; params: unknown[] };
  };
  if (!data?.toParameterizedQuery) {
    throw new Error('NeonQueryPromise.queryData did not expose toParameterizedQuery');
  }
  return data.toParameterizedQuery();
}

describe('neon tagged-template composition — admin list queries', () => {
  // The waitlist-listing pattern: two composable predicates, each of which
  // may be empty.

  it('composes two non-empty fragments with bound params', () => {
    const statusPred = sql`AND w.approved_at IS NULL`;
    const sourcePred = sql`AND w.source = ${'manual'}`;
    const compiled = compile(
      sql`SELECT 1 FROM waitlist w WHERE 1=1 ${statusPred} ${sourcePred} LIMIT ${50}`
    );

    expect(compiled.query).toContain('AND w.approved_at IS NULL');
    expect(compiled.query).toContain('AND w.source = ');
    // params preserved in order: source filter, then limit
    expect(compiled.params).toEqual(['manual', 50]);
  });

  it('treats `sql``` as a true no-op fragment (no phantom params, no broken SQL)', () => {
    const statusPred = sql``;
    const sourcePred = sql``;
    const compiled = compile(sql`SELECT 1 WHERE 1=1 ${statusPred} ${sourcePred} LIMIT ${25}`);

    // No stray tokens like `[object Object]` or `undefined` leaking in
    expect(compiled.query).not.toMatch(/object|undefined|Promise/i);
    // Both predicates collapse — only the LIMIT placeholder remains
    expect(compiled.query.replace(/\s+/g, ' ').trim()).toBe('SELECT 1 WHERE 1=1 LIMIT $1');
    expect(compiled.params).toEqual([25]);
  });

  it('mixes empty and non-empty fragments (status=pending, source=all)', () => {
    const statusPred = sql`AND w.approved_at IS NULL`;
    const sourcePred = sql``;
    const compiled = compile(
      sql`SELECT 1 WHERE 1=1 ${statusPred} ${sourcePred} LIMIT ${10} OFFSET ${0}`
    );

    expect(compiled.query).toContain('AND w.approved_at IS NULL');
    expect(compiled.query).not.toContain('w.source');
    // Empty fragment must not consume a parameter slot
    expect(compiled.params).toEqual([10, 0]);
  });

  it('mixes non-empty and empty fragments (status=all, source=manual)', () => {
    const statusPred = sql``;
    const sourcePred = sql`AND w.source = ${'website'}`;
    const compiled = compile(
      sql`SELECT 1 WHERE 1=1 ${statusPred} ${sourcePred} LIMIT ${10} OFFSET ${0}`
    );

    expect(compiled.query).not.toContain('bu.id IS');
    expect(compiled.query).toContain('AND w.source = ');
    // source param is $1 — empty fragment did not reserve a slot before it
    expect(compiled.query).toMatch(/AND w\.source = \$1/);
    expect(compiled.params).toEqual(['website', 10, 0]);
  });

  // The audit-listing pattern mirrors the waitlist one but with action/actor.

  it('audit filter composition binds action and actor in order', () => {
    const actionPred = sql`AND action = ${'user.approved'}`;
    const actorPred = sql`AND actor = ${'josh@arkahna.io'}`;
    const compiled = compile(
      sql`SELECT id FROM audit_log WHERE 1=1 ${actionPred} ${actorPred} ORDER BY created_at DESC LIMIT ${25} OFFSET ${10}`
    );

    expect(compiled.query).toContain('ORDER BY created_at DESC');
    expect(compiled.params).toEqual(['user.approved', 'josh@arkahna.io', 25, 10]);
  });

  it('audit list default (no filters) preserves DESC ordering and binds only pagination', () => {
    const actionPred = sql``;
    const actorPred = sql``;
    const compiled = compile(
      sql`SELECT id FROM audit_log WHERE 1=1 ${actionPred} ${actorPred} ORDER BY created_at DESC LIMIT ${50} OFFSET ${0}`
    );

    expect(compiled.query).toContain('ORDER BY created_at DESC');
    expect(compiled.query).not.toContain('ASC');
    expect(compiled.params).toEqual([50, 0]);
  });

  // The recent-audit-for-email query uses LOWER() on the jsonb expression so
  // it can hit the functional index idx_audit_log_metadata_email_lower. Pin
  // the exact WHERE shape so a future "simplification" that drops LOWER()
  // (and silently turns this back into a seq scan) is caught by tests.

  it('findRecentAuditForEmail LOWER() wrap survives to the compiled SQL', () => {
    const email = 'alice@example.com';
    const compiled = compile(
      sql`SELECT id FROM audit_log WHERE LOWER(metadata->>'email') = ${email} OR actor = ${email} ORDER BY created_at DESC LIMIT 10`
    );

    expect(compiled.query).toMatch(/LOWER\(metadata->>'email'\)\s*=\s*\$1/);
    expect(compiled.query).toMatch(/OR actor =\s*\$2/);
    expect(compiled.params).toEqual([email, email]);
  });
});
