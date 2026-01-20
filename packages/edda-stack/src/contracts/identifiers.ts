/**
 * Common Identifier Schemas (STACK-001)
 *
 * Defines shared ID formats used across all Edda Stack layers.
 * All identifiers are UUIDs for global uniqueness and deterministic generation.
 *
 * @module @eddacraft/anvil-edda-stack/contracts/identifiers
 */

import { z } from 'zod';

// =============================================================================
// Base Identifier Schema
// =============================================================================

/**
 * Base UUID schema with validation
 */
export const UuidSchema = z.string().uuid().describe('UUID v4 identifier');

/**
 * Content hash schema (SHA-256 hex string)
 * Used for content-addressable references and deduplication
 */
export const ContentHashSchema = z
  .string()
  .regex(/^[a-f0-9]{64}$/, 'Must be a valid SHA-256 hex string')
  .describe('SHA-256 content hash');

// =============================================================================
// Layer-Specific Identifiers
// =============================================================================

/**
 * Kindling observation identifier
 * References a single observation record in Kindling
 */
export const ObservationIdSchema = UuidSchema.brand<'ObservationId'>();
export type ObservationId = z.infer<typeof ObservationIdSchema>;

/**
 * Kindling session identifier
 * Groups observations within a single Anvil execution
 */
export const SessionIdSchema = UuidSchema.brand<'SessionId'>();
export type SessionId = z.infer<typeof SessionIdSchema>;

/**
 * Ember proposal identifier
 * References a candidate memory proposal
 */
export const ProposalIdSchema = UuidSchema.brand<'ProposalId'>();
export type ProposalId = z.infer<typeof ProposalIdSchema>;

/**
 * Edda memory identifier
 * References a canonical memory object
 */
export const MemoryIdSchema = UuidSchema.brand<'MemoryId'>();
export type MemoryId = z.infer<typeof MemoryIdSchema>;

// =============================================================================
// Cross-Layer Reference Identifiers
// =============================================================================

/**
 * Plan identifier (from Anvil core)
 * References an APS plan being executed
 */
export const PlanIdSchema = z
  .string()
  .min(1)
  .describe('Plan identifier (e.g., "save-time-trust-v1")');
export type PlanId = z.infer<typeof PlanIdSchema>;

/**
 * Gate identifier (from Anvil core)
 * References a gate evaluation
 */
export const GateIdSchema = z
  .string()
  .min(1)
  .describe('Gate identifier (e.g., "architecture", "coverage")');
export type GateId = z.infer<typeof GateIdSchema>;

/**
 * Action identifier
 * References an executed action within a session
 */
export const ActionIdSchema = UuidSchema.brand<'ActionId'>();
export type ActionId = z.infer<typeof ActionIdSchema>;

/**
 * Gate evaluation identifier
 * References a specific gate evaluation instance
 */
export const GateEvalIdSchema = UuidSchema.brand<'GateEvalId'>();
export type GateEvalId = z.infer<typeof GateEvalIdSchema>;

/**
 * Error identifier
 * References a recorded error
 */
export const ErrorIdSchema = UuidSchema.brand<'ErrorId'>();
export type ErrorId = z.infer<typeof ErrorIdSchema>;

/**
 * Constraint identifier
 * References an applied constraint
 */
export const ConstraintIdSchema = z.string().min(1).describe('Constraint identifier');
export type ConstraintId = z.infer<typeof ConstraintIdSchema>;

// =============================================================================
// Identifier Utilities
// =============================================================================

/**
 * Create a branded identifier from a UUID string
 */
export function createObservationId(uuid: string): ObservationId {
  return ObservationIdSchema.parse(uuid);
}

export function createSessionId(uuid: string): SessionId {
  return SessionIdSchema.parse(uuid);
}

export function createProposalId(uuid: string): ProposalId {
  return ProposalIdSchema.parse(uuid);
}

export function createMemoryId(uuid: string): MemoryId {
  return MemoryIdSchema.parse(uuid);
}

export function createActionId(uuid: string): ActionId {
  return ActionIdSchema.parse(uuid);
}

export function createGateEvalId(uuid: string): GateEvalId {
  return GateEvalIdSchema.parse(uuid);
}

export function createErrorId(uuid: string): ErrorId {
  return ErrorIdSchema.parse(uuid);
}

/**
 * Validate any identifier string as a valid UUID
 */
export function isValidUuid(value: string): boolean {
  return UuidSchema.safeParse(value).success;
}

/**
 * Validate a content hash string
 */
export function isValidContentHash(value: string): boolean {
  return ContentHashSchema.safeParse(value).success;
}
