/**
 * Event Bus Schemas (STACK-008)
 *
 * Defines event types and schemas for inter-layer communication in the Edda Stack.
 * Events enable loose coupling between Kindling, Ember, and Edda layers.
 *
 * Event flow:
 * - Kindling -> Ember: observation_recorded, session_completed
 * - Ember internal: proposal_created
 * - Ember -> notification: proposal_near_expiry
 * - Edda internal: memory_promoted, memory_retired
 *
 * @module @anvil/edda-stack/contracts/events
 */

import { z } from 'zod';
import {
  UuidSchema,
  ObservationIdSchema,
  SessionIdSchema,
  ProposalIdSchema,
  MemoryIdSchema,
} from './identifiers.js';
import { TimestampSchema, type Timestamp } from './temporal.js';
import { ProposalTypeSchema } from './ember-proposal.js';
import { MemoryTypeSchema } from './edda-memory.js';

// =============================================================================
// Source Layer
// =============================================================================

/**
 * The layer that originated an event
 */
export const SourceLayerSchema = z.enum(['kindling', 'ember', 'edda']);

export type SourceLayer = z.infer<typeof SourceLayerSchema>;

// =============================================================================
// Base Event Schema
// =============================================================================

/**
 * Base fields present on all stack events
 */
export const BaseEventSchema = z.object({
  /** Unique event identifier */
  event_id: UuidSchema.describe('Unique event identifier'),

  /** When the event was created */
  timestamp: TimestampSchema.describe('When the event was created'),

  /** The layer that originated this event */
  source_layer: SourceLayerSchema.describe('Originating layer'),
});

export type BaseEvent = z.infer<typeof BaseEventSchema>;

// =============================================================================
// Event Type Constants
// =============================================================================

/**
 * All supported event types
 */
export const EventTypeSchema = z.enum([
  'observation_recorded',
  'session_completed',
  'proposal_created',
  'proposal_near_expiry',
  'memory_promoted',
  'memory_retired',
]);

export type EventType = z.infer<typeof EventTypeSchema>;

// =============================================================================
// Event Payloads
// =============================================================================

/**
 * Payload for observation_recorded event
 * Emitted by Kindling when a new observation is recorded
 */
export const ObservationRecordedPayloadSchema = z.object({
  /** The recorded observation ID */
  observation_id: ObservationIdSchema,

  /** The session this observation belongs to */
  session_id: SessionIdSchema,

  /** The kind of observation (e.g., 'gate_evaluated', 'action_executed') */
  observation_kind: z.string().describe('Kind of observation recorded'),

  /** Optional metadata about the observation */
  metadata: z.record(z.string(), z.unknown()).optional(),
});

export type ObservationRecordedPayload = z.infer<typeof ObservationRecordedPayloadSchema>;

/**
 * Payload for session_completed event
 * Emitted by Kindling when a session ends
 */
export const SessionCompletedPayloadSchema = z.object({
  /** The completed session ID */
  session_id: SessionIdSchema,

  /** Number of observations in the session */
  observation_count: z.number().int().nonnegative(),

  /** Session start time */
  started_at: TimestampSchema,

  /** Session end time */
  ended_at: TimestampSchema,

  /** Session outcome */
  outcome: z.enum(['success', 'failure', 'partial', 'cancelled']).optional(),
});

export type SessionCompletedPayload = z.infer<typeof SessionCompletedPayloadSchema>;

/**
 * Payload for proposal_created event
 * Emitted by Ember when a new proposal is created
 */
export const ProposalCreatedPayloadSchema = z.object({
  /** The created proposal ID */
  proposal_id: ProposalIdSchema,

  /** Type of proposal */
  proposal_type: ProposalTypeSchema,

  /** Confidence score */
  confidence: z.number().min(0).max(1),

  /** Brief summary */
  summary: z.string(),

  /** When the proposal expires */
  expires_at: TimestampSchema,

  /** Source observation IDs */
  source_observation_ids: z.array(z.string().uuid()),
});

export type ProposalCreatedPayload = z.infer<typeof ProposalCreatedPayloadSchema>;

/**
 * Payload for proposal_near_expiry event
 * Emitted by Ember when a proposal is approaching expiry
 */
export const ProposalNearExpiryPayloadSchema = z.object({
  /** The expiring proposal ID */
  proposal_id: ProposalIdSchema,

  /** Type of proposal */
  proposal_type: ProposalTypeSchema,

  /** Confidence score */
  confidence: z.number().min(0).max(1),

  /** Brief summary */
  summary: z.string(),

  /** When the proposal expires */
  expires_at: TimestampSchema,

  /** Hours remaining until expiry */
  hours_remaining: z.number().nonnegative(),
});

export type ProposalNearExpiryPayload = z.infer<typeof ProposalNearExpiryPayloadSchema>;

/**
 * Payload for memory_promoted event
 * Emitted by Edda when a proposal is promoted to memory
 */
export const MemoryPromotedPayloadSchema = z.object({
  /** The new memory ID */
  memory_id: MemoryIdSchema,

  /** The source proposal ID (if promoted from Ember) */
  source_proposal_id: ProposalIdSchema.optional(),

  /** Type of memory */
  memory_type: MemoryTypeSchema,

  /** The memory statement */
  statement: z.string(),

  /** Who promoted the memory */
  promoted_by: z.string(),

  /** Reason for promotion */
  promotion_reason: z.string().optional(),
});

export type MemoryPromotedPayload = z.infer<typeof MemoryPromotedPayloadSchema>;

/**
 * Payload for memory_retired event
 * Emitted by Edda when a memory is retired
 */
export const MemoryRetiredPayloadSchema = z.object({
  /** The retired memory ID */
  memory_id: MemoryIdSchema,

  /** Type of memory */
  memory_type: MemoryTypeSchema,

  /** The memory statement */
  statement: z.string(),

  /** Who retired the memory */
  retired_by: z.string(),

  /** Reason for retirement */
  retirement_reason: z.string(),

  /** Memory that supersedes this one (if any) */
  superseded_by: MemoryIdSchema.optional(),
});

export type MemoryRetiredPayload = z.infer<typeof MemoryRetiredPayloadSchema>;

// =============================================================================
// Discriminated Union Event Types
// =============================================================================

/**
 * observation_recorded event (Kindling -> Ember)
 */
export const ObservationRecordedEventSchema = BaseEventSchema.extend({
  type: z.literal('observation_recorded'),
  source_layer: z.literal('kindling'),
  payload: ObservationRecordedPayloadSchema,
});

export type ObservationRecordedEvent = z.infer<typeof ObservationRecordedEventSchema>;

/**
 * session_completed event (Kindling -> Ember)
 */
export const SessionCompletedEventSchema = BaseEventSchema.extend({
  type: z.literal('session_completed'),
  source_layer: z.literal('kindling'),
  payload: SessionCompletedPayloadSchema,
});

export type SessionCompletedEvent = z.infer<typeof SessionCompletedEventSchema>;

/**
 * proposal_created event (Ember internal)
 */
export const ProposalCreatedEventSchema = BaseEventSchema.extend({
  type: z.literal('proposal_created'),
  source_layer: z.literal('ember'),
  payload: ProposalCreatedPayloadSchema,
});

export type ProposalCreatedEvent = z.infer<typeof ProposalCreatedEventSchema>;

/**
 * proposal_near_expiry event (Ember -> notification)
 */
export const ProposalNearExpiryEventSchema = BaseEventSchema.extend({
  type: z.literal('proposal_near_expiry'),
  source_layer: z.literal('ember'),
  payload: ProposalNearExpiryPayloadSchema,
});

export type ProposalNearExpiryEvent = z.infer<typeof ProposalNearExpiryEventSchema>;

/**
 * memory_promoted event (Edda internal)
 */
export const MemoryPromotedEventSchema = BaseEventSchema.extend({
  type: z.literal('memory_promoted'),
  source_layer: z.literal('edda'),
  payload: MemoryPromotedPayloadSchema,
});

export type MemoryPromotedEvent = z.infer<typeof MemoryPromotedEventSchema>;

/**
 * memory_retired event (Edda internal)
 */
export const MemoryRetiredEventSchema = BaseEventSchema.extend({
  type: z.literal('memory_retired'),
  source_layer: z.literal('edda'),
  payload: MemoryRetiredPayloadSchema,
});

export type MemoryRetiredEvent = z.infer<typeof MemoryRetiredEventSchema>;

// =============================================================================
// Union Stack Event Schema
// =============================================================================

/**
 * Discriminated union of all stack events
 */
export const StackEventSchema = z.discriminatedUnion('type', [
  ObservationRecordedEventSchema,
  SessionCompletedEventSchema,
  ProposalCreatedEventSchema,
  ProposalNearExpiryEventSchema,
  MemoryPromotedEventSchema,
  MemoryRetiredEventSchema,
]);

export type StackEvent = z.infer<typeof StackEventSchema>;

// =============================================================================
// Event Bus Interface
// =============================================================================

/**
 * Handler function type for event subscriptions
 */
export type EventHandler<T extends StackEvent = StackEvent> = (event: T) => void | Promise<void>;

/**
 * Unsubscribe function returned by subscribe
 */
export type Unsubscribe = () => void;

/**
 * Stack Event Bus Interface
 *
 * Provides publish/subscribe semantics for inter-layer communication.
 */
export interface IStackEventBus {
  /**
   * Publish an event to all subscribers
   * @param event - The event to publish
   */
  publish(event: StackEvent): Promise<void>;

  /**
   * Subscribe to events of a specific type
   * @param eventType - The event type to subscribe to
   * @param handler - Handler function called when event is published
   * @returns Unsubscribe function to remove the subscription
   */
  subscribe<T extends EventType>(
    eventType: T,
    handler: EventHandler<Extract<StackEvent, { type: T }>>
  ): Unsubscribe;

  /**
   * Subscribe to all events
   * @param handler - Handler function called for all events
   * @returns Unsubscribe function to remove the subscription
   */
  subscribeAll(handler: EventHandler): Unsubscribe;
}

// =============================================================================
// Event Factory Functions
// =============================================================================

/**
 * Create base event fields with auto-generated ID and timestamp
 */
function createBaseEvent(
  sourceLayer: SourceLayer
): Omit<BaseEvent, 'event_id'> & { event_id: string } {
  return {
    event_id: crypto.randomUUID(),
    timestamp: new Date().toISOString() as Timestamp,
    source_layer: sourceLayer,
  };
}

/**
 * Create an observation_recorded event
 */
export function createObservationRecordedEvent(
  payload: ObservationRecordedPayload
): ObservationRecordedEvent {
  return {
    ...createBaseEvent('kindling'),
    type: 'observation_recorded',
    source_layer: 'kindling',
    payload,
  };
}

/**
 * Create a session_completed event
 */
export function createSessionCompletedEvent(
  payload: SessionCompletedPayload
): SessionCompletedEvent {
  return {
    ...createBaseEvent('kindling'),
    type: 'session_completed',
    source_layer: 'kindling',
    payload,
  };
}

/**
 * Create a proposal_created event
 */
export function createProposalCreatedEvent(payload: ProposalCreatedPayload): ProposalCreatedEvent {
  return {
    ...createBaseEvent('ember'),
    type: 'proposal_created',
    source_layer: 'ember',
    payload,
  };
}

/**
 * Create a proposal_near_expiry event
 */
export function createProposalNearExpiryEvent(
  payload: ProposalNearExpiryPayload
): ProposalNearExpiryEvent {
  return {
    ...createBaseEvent('ember'),
    type: 'proposal_near_expiry',
    source_layer: 'ember',
    payload,
  };
}

/**
 * Create a memory_promoted event
 */
export function createMemoryPromotedEvent(payload: MemoryPromotedPayload): MemoryPromotedEvent {
  return {
    ...createBaseEvent('edda'),
    type: 'memory_promoted',
    source_layer: 'edda',
    payload,
  };
}

/**
 * Create a memory_retired event
 */
export function createMemoryRetiredEvent(payload: MemoryRetiredPayload): MemoryRetiredEvent {
  return {
    ...createBaseEvent('edda'),
    type: 'memory_retired',
    source_layer: 'edda',
    payload,
  };
}

/**
 * Generic event factory
 * Creates an event of the specified type with the given payload
 */
export function createEvent<T extends EventType>(
  type: T,
  payload: Extract<StackEvent, { type: T }>['payload']
): Extract<StackEvent, { type: T }> {
  const factories: Record<EventType, (payload: unknown) => StackEvent> = {
    observation_recorded: (p) => createObservationRecordedEvent(p as ObservationRecordedPayload),
    session_completed: (p) => createSessionCompletedEvent(p as SessionCompletedPayload),
    proposal_created: (p) => createProposalCreatedEvent(p as ProposalCreatedPayload),
    proposal_near_expiry: (p) => createProposalNearExpiryEvent(p as ProposalNearExpiryPayload),
    memory_promoted: (p) => createMemoryPromotedEvent(p as MemoryPromotedPayload),
    memory_retired: (p) => createMemoryRetiredEvent(p as MemoryRetiredPayload),
  };

  return factories[type](payload) as Extract<StackEvent, { type: T }>;
}

// =============================================================================
// Type Guards
// =============================================================================

/**
 * Check if an event is an observation_recorded event
 */
export function isObservationRecordedEvent(event: StackEvent): event is ObservationRecordedEvent {
  return event.type === 'observation_recorded';
}

/**
 * Check if an event is a session_completed event
 */
export function isSessionCompletedEvent(event: StackEvent): event is SessionCompletedEvent {
  return event.type === 'session_completed';
}

/**
 * Check if an event is a proposal_created event
 */
export function isProposalCreatedEvent(event: StackEvent): event is ProposalCreatedEvent {
  return event.type === 'proposal_created';
}

/**
 * Check if an event is a proposal_near_expiry event
 */
export function isProposalNearExpiryEvent(event: StackEvent): event is ProposalNearExpiryEvent {
  return event.type === 'proposal_near_expiry';
}

/**
 * Check if an event is a memory_promoted event
 */
export function isMemoryPromotedEvent(event: StackEvent): event is MemoryPromotedEvent {
  return event.type === 'memory_promoted';
}

/**
 * Check if an event is a memory_retired event
 */
export function isMemoryRetiredEvent(event: StackEvent): event is MemoryRetiredEvent {
  return event.type === 'memory_retired';
}

/**
 * Check if an event originated from a specific layer
 */
export function isFromLayer(event: StackEvent, layer: SourceLayer): boolean {
  return event.source_layer === layer;
}

/**
 * Check if an event is from Kindling
 */
export function isKindlingEvent(event: StackEvent): boolean {
  return isFromLayer(event, 'kindling');
}

/**
 * Check if an event is from Ember
 */
export function isEmberEvent(event: StackEvent): boolean {
  return isFromLayer(event, 'ember');
}

/**
 * Check if an event is from Edda
 */
export function isEddaEvent(event: StackEvent): boolean {
  return isFromLayer(event, 'edda');
}

// =============================================================================
// Event Descriptions
// =============================================================================

/**
 * Human-readable descriptions of event types
 */
export const eventTypeDescriptions: Record<EventType, string> = {
  observation_recorded: 'A new observation was recorded in Kindling',
  session_completed: 'An Anvil session has completed',
  proposal_created: 'A new candidate memory proposal was created in Ember',
  proposal_near_expiry: 'A proposal is approaching its expiry time',
  memory_promoted: 'A proposal was promoted to canonical memory in Edda',
  memory_retired: 'A memory was retired from active use',
};
