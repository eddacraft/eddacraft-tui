import { randomUUID } from 'node:crypto';
import Database from 'better-sqlite3';
import type { Database as DatabaseType } from 'better-sqlite3';
import { ProposalAlreadyResolvedError } from '../contracts/ember-proposal.js';
import {
  createProposalId,
  type MemoryId,
  type ProposalId,
  type SessionId,
} from '../contracts/identifiers.js';
import { calculateExpiry, now, parseTimestamp, type Timestamp } from '../contracts/temporal.js';
import type {
  CandidateProposal,
  CreateProposalInput,
  ProposalQuery,
  ProposalQueryResult,
  ProposalStatus,
  ProposalType,
} from '../contracts/ember-proposal.js';
import type {
  EmberStats,
  IEmberPort,
  ResolveProposalInput,
  UpdateProposalInput,
} from '../contracts/ports/ember.port.js';

interface ProposalRow {
  id: string;
  type: ProposalType;
  status: ProposalStatus;
  summary: string;
  rationale: string;
  confidence: number;
  metadata: string | null;
  signals: string | null;
  provenance: string;
  created_at: string;
  expires_at: string;
  ttl_days: number;
  updated_at: string | null;
  resolution: string | null;
}

interface SerialisedProposal {
  id: string;
  type: ProposalType;
  status: ProposalStatus;
  summary: string;
  rationale: string;
  confidence: number;
  metadata: string | null;
  signals: string;
  provenance: string;
  created_at: string;
  expires_at: string;
  ttl_days: number;
  updated_at: string | null;
  resolution: string | null;
}

const HOUR_MS = 60 * 60 * 1000;

const ALL_STATUSES: ProposalStatus[] = ['active', 'promoted', 'expired', 'dismissed'];
const ALL_TYPES: ProposalType[] = [
  'decision',
  'pattern',
  'warning',
  'lesson',
  'anomaly',
  'constraint',
];

const SORT_FIELD_MAP = {
  created_at: 'created_at',
  confidence: 'confidence',
  expires_at: 'expires_at',
} as const satisfies Record<string, string>;

type SortField = keyof typeof SORT_FIELD_MAP;

const SORT_DIRECTION_MAP = {
  asc: 'ASC',
  desc: 'DESC',
} as const satisfies Record<string, string>;

type SortDirection = keyof typeof SORT_DIRECTION_MAP;

export class ProposalStore implements IEmberPort {
  private readonly db: DatabaseType;

  constructor(dbPath: string) {
    this.db = new Database(dbPath);
    this.db.pragma('journal_mode = WAL');
    this.db.pragma('foreign_keys = ON');
    this.initialiseSchema();
  }

  static createInMemory(): ProposalStore {
    return new ProposalStore(':memory:');
  }

  close(): void {
    if (this.db.open) {
      this.db.close();
    }
  }

  createProposal(input: CreateProposalInput): Promise<CandidateProposal> {
    const id = createProposalId(randomUUID());
    const createdAt = now();
    const ttlDays = input.ttl_days ?? 30;

    const proposal: CandidateProposal = {
      id,
      type: input.type,
      status: 'active',
      summary: input.summary,
      rationale: input.rationale,
      confidence: input.confidence,
      metadata: input.metadata,
      signals: input.signals ?? [],
      provenance: input.provenance,
      created_at: createdAt,
      expires_at: calculateExpiry(createdAt, ttlDays),
      ttl_days: ttlDays,
    };

    const row = serialiseProposal(proposal);
    this.db
      .prepare(
        `INSERT INTO proposals (
          id, type, status, summary, rationale, confidence,
          metadata, signals, provenance, created_at, expires_at, ttl_days,
          updated_at, resolution
        ) VALUES (
          @id, @type, @status, @summary, @rationale, @confidence,
          @metadata, @signals, @provenance, @created_at, @expires_at, @ttl_days,
          @updated_at, @resolution
        )`
      )
      .run(row);

    return Promise.resolve(proposal);
  }

  updateProposal(id: ProposalId, input: UpdateProposalInput): Promise<CandidateProposal | null> {
    const updates: string[] = [];
    const params: Record<string, unknown> = { id };

    if (input.summary !== undefined) {
      updates.push('summary = @summary');
      params.summary = input.summary;
    }
    if (input.rationale !== undefined) {
      updates.push('rationale = @rationale');
      params.rationale = input.rationale;
    }
    if (input.confidence !== undefined) {
      updates.push('confidence = @confidence');
      params.confidence = input.confidence;
    }
    if (input.metadata !== undefined) {
      updates.push('metadata = @metadata');
      params.metadata = JSON.stringify(input.metadata);
    }

    if (updates.length === 0) {
      return this.getProposal(id);
    }

    const updatedAt = now();
    updates.push('updated_at = @updated_at');
    params.updated_at = updatedAt;

    const sql = `UPDATE proposals SET ${updates.join(', ')} WHERE id = @id`;
    const result = this.db.prepare(sql).run(params);
    if (result.changes === 0) {
      return Promise.resolve(null);
    }

    return this.getProposal(id);
  }

  async resolveProposal(
    id: ProposalId,
    input: ResolveProposalInput
  ): Promise<CandidateProposal | null> {
    const resolvedAt = now();
    const resolution = {
      resolved_at: resolvedAt,
      resolved_by: input.resolved_by,
      resolution_reason: input.resolution_reason,
      memory_id: input.memory_id,
    };

    // Compare-and-set on 'active' so a terminal resolution can never be
    // overwritten, even under concurrent resolvers (CIB-118).
    //
    // Allowed-transition matrix:
    //   active  -> promoted | expired | dismissed  (the CAS below)
    //   expired -> dismissed                       idempotent no-op success;
    //                                              the recorded expiry
    //                                              resolution is kept
    //   any other terminal transition              refused with
    //                                              ProposalAlreadyResolvedError
    const result = this.db
      .prepare(
        `UPDATE proposals
         SET status = @status,
             resolution = @resolution,
             updated_at = @updated_at
         WHERE id = @id AND status = 'active'`
      )
      .run({
        id,
        status: input.status,
        resolution: JSON.stringify(resolution),
        updated_at: resolvedAt,
      });

    if (result.changes === 0) {
      const current = await this.getProposal(id);
      if (current === null) {
        return null;
      }
      if (input.status === 'dismissed' && current.status === 'expired') {
        // Dismissing an expired proposal is a no-op success: both are
        // "closed without promotion" and the expiry record wins.
        return current;
      }
      throw new ProposalAlreadyResolvedError(id, current.status);
    }

    return this.getProposal(id);
  }

  getProposal(id: ProposalId): Promise<CandidateProposal | null> {
    const row = this.db.prepare('SELECT * FROM proposals WHERE id = ?').get(id) as
      | ProposalRow
      | undefined;
    if (!row) {
      return Promise.resolve(null);
    }
    return Promise.resolve(deserialiseRow(row));
  }

  queryProposals(query: ProposalQuery): Promise<ProposalQueryResult> {
    const where: string[] = [];
    const params: unknown[] = [];

    if (query.types && query.types.length > 0) {
      where.push(`type IN (${query.types.map(() => '?').join(', ')})`);
      params.push(...query.types);
    }

    if (query.statuses && query.statuses.length > 0) {
      where.push(`status IN (${query.statuses.map(() => '?').join(', ')})`);
      params.push(...query.statuses);
    }

    if (query.min_confidence !== undefined) {
      where.push('confidence >= ?');
      params.push(query.min_confidence);
    }

    if (query.created_after) {
      where.push('created_at > ?');
      params.push(query.created_after);
    }

    if (query.created_before) {
      where.push('created_at < ?');
      params.push(query.created_before);
    }

    if (!query.include_expired) {
      where.push("status != 'expired'");
    }

    if (query.session_id) {
      where.push(
        `EXISTS (
          SELECT 1
          FROM json_each(json_extract(provenance, '$.session_ids'))
          WHERE json_each.value = ?
        )`
      );
      params.push(query.session_id);
    }

    const whereClause = where.length > 0 ? `WHERE ${where.join(' AND ')}` : '';

    const sortBy = query.sort_by ?? 'created_at';
    const sortOrder = query.sort_order ?? 'desc';

    if (!(sortBy in SORT_FIELD_MAP)) {
      throw new Error(`Invalid sort field: ${sortBy}`);
    }
    if (!(sortOrder in SORT_DIRECTION_MAP)) {
      throw new Error(`Invalid sort direction: ${sortOrder}`);
    }

    const orderByField = SORT_FIELD_MAP[sortBy as SortField];
    const orderDirection = SORT_DIRECTION_MAP[sortOrder as SortDirection];

    const limit = query.limit ?? 100;
    const offset = query.offset ?? 0;

    const countStatement = this.db.prepare(
      `SELECT COUNT(*) as total FROM proposals ${whereClause}`
    );
    const countRow = countStatement.get(...params) as { total: number };
    const total = Number(countRow.total);

    const rows = this.db
      .prepare(
        `SELECT *
         FROM proposals
         ${whereClause}
         ORDER BY ${orderByField} ${orderDirection}
         LIMIT ? OFFSET ?`
      )
      .all(...params, limit, offset) as ProposalRow[];

    const proposals = rows.map((row) => deserialiseRow(row));

    return Promise.resolve({
      proposals,
      total,
      limit,
      offset,
      has_more: offset + proposals.length < total,
    });
  }

  getActiveProposals(): Promise<CandidateProposal[]> {
    const current = now();
    const rows = this.db
      .prepare(
        `SELECT *
         FROM proposals
         WHERE status = 'active' AND expires_at > ?
         ORDER BY created_at DESC`
      )
      .all(current) as ProposalRow[];
    return Promise.resolve(rows.map((row) => deserialiseRow(row)));
  }

  getProposalsBySession(sessionId: SessionId): Promise<CandidateProposal[]> {
    const rows = this.db
      .prepare(
        `SELECT *
         FROM proposals
         WHERE EXISTS (
           SELECT 1
           FROM json_each(json_extract(provenance, '$.session_ids'))
           WHERE json_each.value = ?
         )
         ORDER BY created_at DESC`
      )
      .all(sessionId) as ProposalRow[];

    return Promise.resolve(rows.map((row) => deserialiseRow(row)));
  }

  proposalExists(id: ProposalId): Promise<boolean> {
    const row = this.db.prepare('SELECT 1 as found FROM proposals WHERE id = ?').get(id) as
      | { found: number }
      | undefined;
    return Promise.resolve(Boolean(row?.found));
  }

  async markPromoted(id: ProposalId, memoryId: MemoryId, resolvedBy: string): Promise<void> {
    const resolvedAt = now();
    const resolution = {
      resolved_at: resolvedAt,
      resolved_by: resolvedBy,
      resolution_reason: 'Promoted to Edda memory',
      memory_id: memoryId,
    };

    // Compare-and-set on 'active': only one promotion can ever claim the
    // proposal, so a double-fire cannot overwrite the recorded memory link
    // (CIB-118).
    const result = this.db
      .prepare(
        `UPDATE proposals
         SET status = 'promoted',
             resolution = @resolution,
             updated_at = @updated_at
         WHERE id = @id AND status = 'active'`
      )
      .run({
        id,
        resolution: JSON.stringify(resolution),
        updated_at: resolvedAt,
      });

    if (result.changes === 0) {
      const current = await this.getProposal(id);
      if (current === null) {
        throw new Error(`Proposal not found: ${id}`);
      }
      if (current.status === 'promoted' && current.resolution?.memory_id === memoryId) {
        // Idempotent replay: the same promotion already succeeded.
        return;
      }
      throw new ProposalAlreadyResolvedError(id, current.status);
    }

    return;
  }

  async markDismissed(id: ProposalId, reason: string, resolvedBy: string): Promise<void> {
    const resolvedAt = now();
    const resolution = {
      resolved_at: resolvedAt,
      resolved_by: resolvedBy,
      resolution_reason: reason,
    };

    // Compare-and-set on 'active' so a terminal resolution can never be
    // overwritten (CIB-118).
    const result = this.db
      .prepare(
        `UPDATE proposals
         SET status = 'dismissed',
             resolution = @resolution,
             updated_at = @updated_at
         WHERE id = @id AND status = 'active'`
      )
      .run({
        id,
        resolution: JSON.stringify(resolution),
        updated_at: resolvedAt,
      });

    if (result.changes === 0) {
      const current = await this.getProposal(id);
      if (current === null) {
        throw new Error(`Proposal not found: ${id}`);
      }
      if (current.status === 'dismissed') {
        // Idempotent replay: the first dismissal record wins.
        return;
      }
      if (current.status === 'expired') {
        // Dismissing an expired proposal is a no-op success — the recorded
        // expiry resolution is kept (see the allowed-transition matrix on
        // resolveProposal).
        return;
      }
      throw new ProposalAlreadyResolvedError(id, current.status);
    }

    return;
  }

  getExpiredProposals(): Promise<CandidateProposal[]> {
    const current = now();
    const rows = this.db
      .prepare(
        `SELECT *
         FROM proposals
         WHERE status = 'active' AND expires_at <= ?
         ORDER BY expires_at ASC`
      )
      .all(current) as ProposalRow[];

    return Promise.resolve(rows.map((row) => deserialiseRow(row)));
  }

  processExpiredProposals(): Promise<number> {
    const resolvedAt = now();
    const resolution = JSON.stringify({
      resolved_at: resolvedAt,
      resolution_reason: 'TTL expired',
    });

    const result = this.db
      .prepare(
        `UPDATE proposals
         SET status = 'expired',
             resolution = @resolution,
             updated_at = @updated_at
         WHERE status = 'active' AND expires_at <= @current_time`
      )
      .run({
        resolution,
        updated_at: resolvedAt,
        current_time: resolvedAt,
      });

    return Promise.resolve(result.changes);
  }

  expireStaleProposals(): Promise<number> {
    return this.processExpiredProposals();
  }

  isAvailable(): Promise<boolean> {
    try {
      this.db.prepare('SELECT 1').get();
      return Promise.resolve(true);
    } catch {
      return Promise.resolve(false);
    }
  }

  getStats(): Promise<EmberStats> {
    const totalRow = this.db.prepare('SELECT COUNT(*) as total FROM proposals').get() as {
      total: number;
    };
    const totalProposals = Number(totalRow.total);

    const byStatusRows = this.db
      .prepare('SELECT status, COUNT(*) as count FROM proposals GROUP BY status')
      .all() as Array<{ status: ProposalStatus; count: number }>;
    const byStatusMap = new Map<ProposalStatus, number>();
    for (const row of byStatusRows) {
      byStatusMap.set(row.status, Number(row.count));
    }

    const byTypeRows = this.db
      .prepare(
        `SELECT type, COUNT(*) as count, AVG(confidence) as avg_confidence
         FROM proposals
         GROUP BY type`
      )
      .all() as Array<{ type: ProposalType; count: number; avg_confidence: number | null }>;
    const byTypeMap = new Map<ProposalType, { count: number; avg_confidence: number }>();
    for (const row of byTypeRows) {
      byTypeMap.set(row.type, {
        count: Number(row.count),
        avg_confidence: row.avg_confidence ?? 0,
      });
    }

    const currentTime = Date.now();
    const expiringSoonThreshold = new Date(currentTime + HOUR_MS * 24).toISOString();
    const expiringSoonRow = this.db
      .prepare(
        `SELECT COUNT(*) as count
         FROM proposals
         WHERE status = 'active' AND expires_at <= ?`
      )
      .get(expiringSoonThreshold) as { count: number };

    const avgConfidenceRow = this.db
      .prepare("SELECT AVG(confidence) as avg_confidence FROM proposals WHERE status = 'active'")
      .get() as {
      avg_confidence: number | null;
    };

    const oldestActiveRow = this.db
      .prepare("SELECT MIN(created_at) as oldest_active FROM proposals WHERE status = 'active'")
      .get() as {
      oldest_active: string | null;
    };

    const mostRecentRow = this.db
      .prepare('SELECT MAX(created_at) as most_recent FROM proposals')
      .get() as {
      most_recent: string | null;
    };

    const promotedRow = this.db
      .prepare("SELECT COUNT(*) as count FROM proposals WHERE status = 'promoted'")
      .get() as { count: number };
    const resolvedRow = this.db
      .prepare("SELECT COUNT(*) as count FROM proposals WHERE status != 'active'")
      .get() as { count: number };

    return Promise.resolve({
      total_proposals: totalProposals,
      by_status: ALL_STATUSES.map((status) => ({
        status,
        count: byStatusMap.get(status) ?? 0,
      })),
      by_type: ALL_TYPES.map((type) => {
        const typeStats = byTypeMap.get(type);
        return {
          type,
          count: typeStats?.count ?? 0,
          avg_confidence: typeStats?.avg_confidence ?? 0,
        };
      }),
      expiring_soon: Number(expiringSoonRow.count),
      avg_confidence: avgConfidenceRow.avg_confidence ?? undefined,
      oldest_active: oldestActiveRow.oldest_active
        ? parseTimestamp(oldestActiveRow.oldest_active)
        : undefined,
      most_recent: mostRecentRow.most_recent
        ? parseTimestamp(mostRecentRow.most_recent)
        : undefined,
      promotion_rate:
        resolvedRow.count > 0 ? Number(promotedRow.count) / Number(resolvedRow.count) : undefined,
    });
  }

  countProposals(status?: ProposalStatus): Promise<number> {
    const row = status
      ? (this.db
          .prepare('SELECT COUNT(*) as count FROM proposals WHERE status = ?')
          .get(status) as {
          count: number;
        })
      : (this.db.prepare('SELECT COUNT(*) as count FROM proposals').get() as {
          count: number;
        });

    return Promise.resolve(Number(row.count));
  }

  pruneProposals(olderThan: Timestamp): Promise<number> {
    const result = this.db
      .prepare(
        `DELETE FROM proposals
         WHERE status != 'active'
           AND resolution IS NOT NULL
           AND json_extract(resolution, '$.resolved_at') < ?`
      )
      .run(olderThan);

    return Promise.resolve(result.changes);
  }

  private initialiseSchema(): void {
    this.db.exec(`
      CREATE TABLE IF NOT EXISTS proposals (
        id TEXT PRIMARY KEY,
        type TEXT NOT NULL,
        status TEXT NOT NULL DEFAULT 'active',
        summary TEXT NOT NULL,
        rationale TEXT NOT NULL,
        confidence REAL NOT NULL,
        metadata TEXT,
        signals TEXT,
        provenance TEXT NOT NULL,
        created_at TEXT NOT NULL,
        expires_at TEXT NOT NULL,
        ttl_days INTEGER NOT NULL,
        updated_at TEXT,
        resolution TEXT
      );

      CREATE INDEX IF NOT EXISTS idx_proposals_status ON proposals(status);
      CREATE INDEX IF NOT EXISTS idx_proposals_type ON proposals(type);
      CREATE INDEX IF NOT EXISTS idx_proposals_expires_at ON proposals(expires_at);
      CREATE INDEX IF NOT EXISTS idx_proposals_created_at ON proposals(created_at);
      CREATE INDEX IF NOT EXISTS idx_proposals_confidence ON proposals(confidence);
    `);
  }
}

export function serialiseProposal(proposal: CandidateProposal): SerialisedProposal {
  return {
    id: proposal.id,
    type: proposal.type,
    status: proposal.status,
    summary: proposal.summary,
    rationale: proposal.rationale,
    confidence: proposal.confidence,
    metadata: proposal.metadata ? JSON.stringify(proposal.metadata) : null,
    signals: JSON.stringify(proposal.signals),
    provenance: JSON.stringify(proposal.provenance),
    created_at: proposal.created_at,
    expires_at: proposal.expires_at,
    ttl_days: proposal.ttl_days,
    updated_at: proposal.updated_at ?? null,
    resolution: proposal.resolution ? JSON.stringify(proposal.resolution) : null,
  };
}

export function deserialiseRow(row: ProposalRow): CandidateProposal {
  const metadata = row.metadata ? (JSON.parse(row.metadata) as Record<string, unknown>) : undefined;
  const signals = row.signals
    ? (JSON.parse(row.signals) as CandidateProposal['signals'])
    : ([] as CandidateProposal['signals']);
  const provenance = JSON.parse(row.provenance) as CandidateProposal['provenance'];
  const resolution = row.resolution
    ? (JSON.parse(row.resolution) as NonNullable<CandidateProposal['resolution']>)
    : undefined;

  return {
    id: createProposalId(row.id),
    type: row.type,
    status: row.status,
    summary: row.summary,
    rationale: row.rationale,
    confidence: row.confidence,
    metadata,
    signals,
    provenance,
    created_at: parseTimestamp(row.created_at),
    expires_at: parseTimestamp(row.expires_at),
    ttl_days: row.ttl_days,
    updated_at: row.updated_at ? parseTimestamp(row.updated_at) : undefined,
    resolution: resolution
      ? {
          resolved_at: parseTimestamp(resolution.resolved_at),
          resolved_by: resolution.resolved_by,
          resolution_reason: resolution.resolution_reason,
          memory_id: resolution.memory_id,
        }
      : undefined,
  };
}
