/**
 * Plan Emitter (KINDLING-006)
 *
 * Emits plan lifecycle observations: created, edited, approved, rejected.
 * Plans are the governance artifacts that authorize actions.
 */

import { randomUUID } from 'node:crypto';
import type { KindlingService } from '../kindling-service.js';
import type {
  PlanCreatedObservation,
  PlanEditedObservation,
  PlanApprovedObservation,
  PlanRejectedObservation,
} from '../observation-contract.js';

// =============================================================================
// Input Types
// =============================================================================

export interface PlanCreatedInput {
  session_id: string;
  plan_id?: string;
  plan_version: string;
  plan_path: string;
  plan_hash: string;
  created_by: 'human' | 'ai' | 'system';
  source?: string;
}

export interface PlanEditedInput {
  session_id: string;
  plan_id: string;
  previous_version: string;
  new_version: string;
  previous_hash: string;
  new_hash: string;
  edited_by: 'human' | 'ai' | 'system';
  change_summary?: string;
}

export interface PlanApprovedInput {
  session_id: string;
  plan_id: string;
  plan_version: string;
  approved_by: string;
  approval_method: 'cli_confirm' | 'explicit_flag' | 'ci_gate';
}

export interface PlanRejectedInput {
  session_id: string;
  plan_id: string;
  plan_version: string;
  rejected_by: string;
  rejection_reason?: string;
}

// =============================================================================
// Emitters
// =============================================================================

/**
 * Emit a plan_created observation.
 *
 * If no plan_id is provided, one is generated.
 *
 * @param service - KindlingService instance
 * @param plan - Plan creation details
 * @returns The plan_id (generated or provided)
 */
export function emitPlanCreated(service: KindlingService, plan: PlanCreatedInput): string {
  const planId = plan.plan_id ?? randomUUID();

  const observation: PlanCreatedObservation = {
    kind: 'plan_created',
    session_id: plan.session_id,
    timestamp: new Date().toISOString(),
    plan_id: planId,
    plan_version: plan.plan_version,
    plan_path: plan.plan_path,
    plan_hash: plan.plan_hash,
    created_by: plan.created_by,
    source: plan.source,
  };

  // Fire-and-forget
  service.emit(observation).catch(() => {
    // Silently swallow
  });

  return planId;
}

/**
 * Emit a plan_edited observation.
 *
 * @param service - KindlingService instance
 * @param edit - Plan edit details
 * @returns The plan_id
 */
export function emitPlanEdited(service: KindlingService, edit: PlanEditedInput): string {
  const observation: PlanEditedObservation = {
    kind: 'plan_edited',
    session_id: edit.session_id,
    timestamp: new Date().toISOString(),
    plan_id: edit.plan_id,
    previous_version: edit.previous_version,
    new_version: edit.new_version,
    previous_hash: edit.previous_hash,
    new_hash: edit.new_hash,
    edited_by: edit.edited_by,
    change_summary: edit.change_summary,
  };

  // Fire-and-forget
  service.emit(observation).catch(() => {
    // Silently swallow
  });

  return edit.plan_id;
}

/**
 * Emit a plan_approved observation.
 *
 * @param service - KindlingService instance
 * @param approval - Plan approval details
 * @returns The plan_id
 */
export function emitPlanApproved(service: KindlingService, approval: PlanApprovedInput): string {
  const observation: PlanApprovedObservation = {
    kind: 'plan_approved',
    session_id: approval.session_id,
    timestamp: new Date().toISOString(),
    plan_id: approval.plan_id,
    plan_version: approval.plan_version,
    approved_by: approval.approved_by,
    approval_method: approval.approval_method,
  };

  // Fire-and-forget
  service.emit(observation).catch(() => {
    // Silently swallow
  });

  return approval.plan_id;
}

/**
 * Emit a plan_rejected observation.
 *
 * @param service - KindlingService instance
 * @param rejection - Plan rejection details
 * @returns The plan_id
 */
export function emitPlanRejected(service: KindlingService, rejection: PlanRejectedInput): string {
  const observation: PlanRejectedObservation = {
    kind: 'plan_rejected',
    session_id: rejection.session_id,
    timestamp: new Date().toISOString(),
    plan_id: rejection.plan_id,
    plan_version: rejection.plan_version,
    rejected_by: rejection.rejected_by,
    rejection_reason: rejection.rejection_reason,
  };

  // Fire-and-forget
  service.emit(observation).catch(() => {
    // Silently swallow
  });

  return rejection.plan_id;
}
