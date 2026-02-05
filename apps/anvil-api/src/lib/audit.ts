import { getClient } from '../db/client.js';
import { insertAuditLog } from '../db/queries.js';

/**
 * Log an auditable action.
 */
export async function audit(
  action: string,
  actor: string,
  metadata: Record<string, unknown> = {}
): Promise<void> {
  const sql = getClient();
  await insertAuditLog(sql, action, actor, metadata);
}
