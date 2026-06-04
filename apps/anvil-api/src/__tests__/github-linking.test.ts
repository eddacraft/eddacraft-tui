import { describe, expect, it, vi } from 'vitest';
import type { NeonClient } from '../db/client.js';
import {
  findUserByGitHubId,
  findActiveUserByAnyEmail,
  linkGitHubIdToUser,
  linkOrCreateGitHubUser,
  type GitHubIdentity,
} from '../db/queries.js';

/** A sql mock whose successive calls resolve to the queued results in order. */
function seqSql(...results: unknown[]): NeonClient {
  const fn = vi.fn();
  for (const r of results) fn.mockResolvedValueOnce(r);
  (fn as unknown as { transaction: unknown }).transaction = vi.fn();
  return fn as unknown as NeonClient;
}

function userRow(overrides: Record<string, unknown> = {}): Record<string, unknown> {
  return {
    id: 'user-1',
    email: 'alice@example.com',
    name: 'Alice',
    status: 'active',
    notes: null,
    github_id: null,
    created_at: '2026-06-01T00:00:00.000Z',
    updated_at: '2026-06-01T00:00:00.000Z',
    ...overrides,
  };
}

const ghUser: GitHubIdentity = {
  id: 42,
  login: 'alice',
  email: 'alice@users.noreply.github.com',
  verifiedEmails: ['alice@users.noreply.github.com', 'alice@example.com'],
};

describe('findUserByGitHubId', () => {
  it('returns the row matched on github_id', async () => {
    const sql = seqSql([userRow({ github_id: 42 })]);
    const user = await findUserByGitHubId(sql, 42);
    expect(user?.github_id).toBe(42);
    expect(vi.mocked(sql)).toHaveBeenCalledOnce();
  });

  it('returns null when no row matches', async () => {
    const sql = seqSql([]);
    expect(await findUserByGitHubId(sql, 99)).toBeNull();
  });
});

describe('findActiveUserByAnyEmail', () => {
  it('short-circuits with no query when the email list is empty', async () => {
    const sql = seqSql();
    expect(await findActiveUserByAnyEmail(sql, [])).toBeNull();
    expect(vi.mocked(sql)).not.toHaveBeenCalled();
  });

  it('returns an active row matched on any supplied email', async () => {
    const sql = seqSql([userRow()]);
    const user = await findActiveUserByAnyEmail(sql, ['x@y.z', 'alice@example.com']);
    expect(user?.email).toBe('alice@example.com');
  });
});

describe('linkGitHubIdToUser', () => {
  it('stores the github_id and returns the updated row', async () => {
    const sql = seqSql([userRow({ github_id: 42 })]);
    const user = await linkGitHubIdToUser(sql, 'user-1', 42);
    expect(user.github_id).toBe(42);
  });
});

describe('linkOrCreateGitHubUser', () => {
  it('1. matches on github_id without touching email (authoritative)', async () => {
    const sql = seqSql([userRow({ github_id: 42, email: 'renamed@example.com' })]);
    const { user, isNewPending, didFirstLink } = await linkOrCreateGitHubUser(sql, ghUser);
    expect(user.github_id).toBe(42);
    expect(isNewPending).toBe(false);
    expect(didFirstLink).toBe(false); // already linked, not a first-link
    expect(vi.mocked(sql)).toHaveBeenCalledOnce(); // only the github_id lookup
  });

  it('2. first-links an active invite via a verified secondary email', async () => {
    const sql = seqSql(
      [], // no github_id match
      [userRow({ email: 'alice@example.com', github_id: null })], // active email match
      [userRow({ email: 'alice@example.com', github_id: 42 })] // linked
    );
    const { user, isNewPending, didFirstLink } = await linkOrCreateGitHubUser(sql, ghUser);
    expect(user.github_id).toBe(42);
    expect(user.email).toBe('alice@example.com'); // invited email kept, not the noreply
    expect(isNewPending).toBe(false);
    expect(didFirstLink).toBe(true);
    expect(vi.mocked(sql)).toHaveBeenCalledTimes(3);
  });

  it('3. creates a pending row when nothing matches', async () => {
    const sql = seqSql(
      [], // no github_id match
      [], // no active email match
      [{ id: 'new-id' }], // insertPendingUser RETURNING id
      [userRow({ id: 'new-id', email: ghUser.email, status: 'pending' })] // findUserById
    );
    const { user, isNewPending } = await linkOrCreateGitHubUser(sql, ghUser);
    expect(user.status).toBe('pending');
    expect(isNewPending).toBe(true);
  });

  it('3b. surfaces an existing non-active row (insert conflict) without re-creating', async () => {
    const sql = seqSql(
      [], // no github_id match
      [], // no ACTIVE email match (the row exists but is pending/suspended)
      [], // insertPendingUser hits ON CONFLICT DO NOTHING -> no id
      [userRow({ email: ghUser.email, status: 'suspended' })] // findUserByEmail
    );
    const { user, isNewPending } = await linkOrCreateGitHubUser(sql, ghUser);
    expect(user.status).toBe('suspended');
    expect(isNewPending).toBe(false);
  });

  it('throws when the create path cannot resolve a row (DB inconsistency)', async () => {
    // no github_id, no active email, insert conflict (no id), follow-up find misses
    const sql = seqSql([], [], [], []);
    await expect(linkOrCreateGitHubUser(sql, ghUser)).rejects.toThrow(
      /failed to create or resolve/
    );
  });
});
