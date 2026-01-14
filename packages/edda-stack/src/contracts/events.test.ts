/**
 * Event Bus Schema Tests (STACK-008)
 *
 * Tests for event types, schemas, and utilities.
 */

import { describe, it, expect, vi } from 'vitest';
import {
  // Schemas
  SourceLayerSchema,
  BaseEventSchema,
  EventTypeSchema,
  ObservationRecordedPayloadSchema,
  SessionCompletedPayloadSchema,
  ProposalCreatedPayloadSchema,
  ProposalNearExpiryPayloadSchema,
  MemoryPromotedPayloadSchema,
  MemoryRetiredPayloadSchema,
  ObservationRecordedEventSchema,
  SessionCompletedEventSchema,
  ProposalCreatedEventSchema,
  ProposalNearExpiryEventSchema,
  MemoryPromotedEventSchema,
  MemoryRetiredEventSchema,
  StackEventSchema,
  // Types
  type IStackEventBus,
  type EventHandler,
  // Factory functions
  createEvent,
  createObservationRecordedEvent,
  createSessionCompletedEvent,
  createProposalCreatedEvent,
  createProposalNearExpiryEvent,
  createMemoryPromotedEvent,
  createMemoryRetiredEvent,
  // Type guards
  isObservationRecordedEvent,
  isSessionCompletedEvent,
  isProposalCreatedEvent,
  isProposalNearExpiryEvent,
  isMemoryPromotedEvent,
  isMemoryRetiredEvent,
  isFromLayer,
  isKindlingEvent,
  isEmberEvent,
  isEddaEvent,
  // Constants
  eventTypeDescriptions,
} from './events.js';

// =============================================================================
// Test Data
// =============================================================================

const validUuid = '550e8400-e29b-41d4-a716-446655440000';
const validTimestamp = '2024-01-15T14:30:00.000Z';

// =============================================================================
// Source Layer Tests
// =============================================================================

describe('SourceLayerSchema', () => {
  it('accepts valid layers', () => {
    expect(SourceLayerSchema.safeParse('kindling').success).toBe(true);
    expect(SourceLayerSchema.safeParse('ember').success).toBe(true);
    expect(SourceLayerSchema.safeParse('edda').success).toBe(true);
  });

  it('rejects invalid layers', () => {
    expect(SourceLayerSchema.safeParse('invalid').success).toBe(false);
    expect(SourceLayerSchema.safeParse('').success).toBe(false);
  });
});

// =============================================================================
// Base Event Tests
// =============================================================================

describe('BaseEventSchema', () => {
  it('accepts valid base event', () => {
    const event = {
      event_id: validUuid,
      timestamp: validTimestamp,
      source_layer: 'kindling',
    };
    expect(BaseEventSchema.safeParse(event).success).toBe(true);
  });

  it('rejects invalid event_id', () => {
    const event = {
      event_id: 'not-a-uuid',
      timestamp: validTimestamp,
      source_layer: 'kindling',
    };
    expect(BaseEventSchema.safeParse(event).success).toBe(false);
  });

  it('rejects invalid timestamp', () => {
    const event = {
      event_id: validUuid,
      timestamp: 'not-a-timestamp',
      source_layer: 'kindling',
    };
    expect(BaseEventSchema.safeParse(event).success).toBe(false);
  });
});

// =============================================================================
// Event Type Tests
// =============================================================================

describe('EventTypeSchema', () => {
  it('accepts all 6 event types', () => {
    const types = [
      'observation_recorded',
      'session_completed',
      'proposal_created',
      'proposal_near_expiry',
      'memory_promoted',
      'memory_retired',
    ];
    for (const type of types) {
      expect(EventTypeSchema.safeParse(type).success).toBe(true);
    }
  });

  it('rejects invalid types', () => {
    expect(EventTypeSchema.safeParse('invalid').success).toBe(false);
  });
});

// =============================================================================
// Payload Schema Tests
// =============================================================================

describe('ObservationRecordedPayloadSchema', () => {
  it('accepts valid payload', () => {
    const payload = {
      observation_id: validUuid,
      session_id: validUuid,
      observation_kind: 'gate_evaluated',
    };
    expect(ObservationRecordedPayloadSchema.safeParse(payload).success).toBe(true);
  });

  it('accepts payload with optional metadata', () => {
    const payload = {
      observation_id: validUuid,
      session_id: validUuid,
      observation_kind: 'gate_evaluated',
      metadata: { gate_id: 'coverage', passed: true },
    };
    expect(ObservationRecordedPayloadSchema.safeParse(payload).success).toBe(true);
  });
});

describe('SessionCompletedPayloadSchema', () => {
  it('accepts valid payload', () => {
    const payload = {
      session_id: validUuid,
      observation_count: 42,
      started_at: validTimestamp,
      ended_at: '2024-01-15T15:30:00.000Z',
    };
    expect(SessionCompletedPayloadSchema.safeParse(payload).success).toBe(true);
  });

  it('accepts payload with outcome', () => {
    const payload = {
      session_id: validUuid,
      observation_count: 42,
      started_at: validTimestamp,
      ended_at: '2024-01-15T15:30:00.000Z',
      outcome: 'success',
    };
    expect(SessionCompletedPayloadSchema.safeParse(payload).success).toBe(true);
  });

  it('accepts all outcome values', () => {
    const outcomes = ['success', 'failure', 'partial', 'cancelled'];
    for (const outcome of outcomes) {
      const payload = {
        session_id: validUuid,
        observation_count: 0,
        started_at: validTimestamp,
        ended_at: validTimestamp,
        outcome,
      };
      expect(SessionCompletedPayloadSchema.safeParse(payload).success).toBe(true);
    }
  });
});

describe('ProposalCreatedPayloadSchema', () => {
  it('accepts valid payload', () => {
    const payload = {
      proposal_id: validUuid,
      proposal_type: 'pattern',
      confidence: 0.75,
      summary: 'Detected recurring pattern in test execution',
      expires_at: '2024-02-14T14:30:00.000Z',
      source_observation_ids: [validUuid],
    };
    expect(ProposalCreatedPayloadSchema.safeParse(payload).success).toBe(true);
  });

  it('rejects invalid confidence', () => {
    const payload = {
      proposal_id: validUuid,
      proposal_type: 'pattern',
      confidence: 1.5, // Invalid: > 1
      summary: 'Test',
      expires_at: validTimestamp,
      source_observation_ids: [validUuid],
    };
    expect(ProposalCreatedPayloadSchema.safeParse(payload).success).toBe(false);
  });
});

describe('ProposalNearExpiryPayloadSchema', () => {
  it('accepts valid payload', () => {
    const payload = {
      proposal_id: validUuid,
      proposal_type: 'decision',
      confidence: 0.8,
      summary: 'Decision about API structure',
      expires_at: '2024-02-14T14:30:00.000Z',
      hours_remaining: 24,
    };
    expect(ProposalNearExpiryPayloadSchema.safeParse(payload).success).toBe(true);
  });

  it('rejects negative hours_remaining', () => {
    const payload = {
      proposal_id: validUuid,
      proposal_type: 'decision',
      confidence: 0.8,
      summary: 'Test',
      expires_at: validTimestamp,
      hours_remaining: -1,
    };
    expect(ProposalNearExpiryPayloadSchema.safeParse(payload).success).toBe(false);
  });
});

describe('MemoryPromotedPayloadSchema', () => {
  it('accepts valid payload', () => {
    const payload = {
      memory_id: validUuid,
      memory_type: 'decision',
      statement: 'We use TypeScript for all new code',
      promoted_by: 'user@example.com',
    };
    expect(MemoryPromotedPayloadSchema.safeParse(payload).success).toBe(true);
  });

  it('accepts payload with source_proposal_id', () => {
    const payload = {
      memory_id: validUuid,
      source_proposal_id: validUuid,
      memory_type: 'decision',
      statement: 'We use TypeScript for all new code',
      promoted_by: 'user@example.com',
      promotion_reason: 'Codifying team agreement',
    };
    expect(MemoryPromotedPayloadSchema.safeParse(payload).success).toBe(true);
  });
});

describe('MemoryRetiredPayloadSchema', () => {
  it('accepts valid payload', () => {
    const payload = {
      memory_id: validUuid,
      memory_type: 'constraint',
      statement: 'Node.js version must be 16+',
      retired_by: 'user@example.com',
      retirement_reason: 'Upgraded to Node.js 20 minimum',
    };
    expect(MemoryRetiredPayloadSchema.safeParse(payload).success).toBe(true);
  });

  it('accepts payload with superseded_by', () => {
    const payload = {
      memory_id: validUuid,
      memory_type: 'constraint',
      statement: 'Node.js version must be 16+',
      retired_by: 'user@example.com',
      retirement_reason: 'Upgraded to Node.js 20 minimum',
      superseded_by: '550e8400-e29b-41d4-a716-446655440001',
    };
    expect(MemoryRetiredPayloadSchema.safeParse(payload).success).toBe(true);
  });
});

// =============================================================================
// Full Event Schema Tests
// =============================================================================

describe('ObservationRecordedEventSchema', () => {
  it('accepts valid event', () => {
    const event = {
      event_id: validUuid,
      timestamp: validTimestamp,
      source_layer: 'kindling',
      type: 'observation_recorded',
      payload: {
        observation_id: validUuid,
        session_id: validUuid,
        observation_kind: 'gate_evaluated',
      },
    };
    expect(ObservationRecordedEventSchema.safeParse(event).success).toBe(true);
  });

  it('rejects wrong source_layer', () => {
    const event = {
      event_id: validUuid,
      timestamp: validTimestamp,
      source_layer: 'ember', // Wrong: should be kindling
      type: 'observation_recorded',
      payload: {
        observation_id: validUuid,
        session_id: validUuid,
        observation_kind: 'gate_evaluated',
      },
    };
    expect(ObservationRecordedEventSchema.safeParse(event).success).toBe(false);
  });
});

describe('SessionCompletedEventSchema', () => {
  it('accepts valid event', () => {
    const event = {
      event_id: validUuid,
      timestamp: validTimestamp,
      source_layer: 'kindling',
      type: 'session_completed',
      payload: {
        session_id: validUuid,
        observation_count: 10,
        started_at: validTimestamp,
        ended_at: '2024-01-15T15:30:00.000Z',
      },
    };
    expect(SessionCompletedEventSchema.safeParse(event).success).toBe(true);
  });
});

describe('ProposalCreatedEventSchema', () => {
  it('accepts valid event', () => {
    const event = {
      event_id: validUuid,
      timestamp: validTimestamp,
      source_layer: 'ember',
      type: 'proposal_created',
      payload: {
        proposal_id: validUuid,
        proposal_type: 'pattern',
        confidence: 0.75,
        summary: 'Test pattern',
        expires_at: '2024-02-14T14:30:00.000Z',
        source_observation_ids: [validUuid],
      },
    };
    expect(ProposalCreatedEventSchema.safeParse(event).success).toBe(true);
  });
});

describe('ProposalNearExpiryEventSchema', () => {
  it('accepts valid event', () => {
    const event = {
      event_id: validUuid,
      timestamp: validTimestamp,
      source_layer: 'ember',
      type: 'proposal_near_expiry',
      payload: {
        proposal_id: validUuid,
        proposal_type: 'decision',
        confidence: 0.8,
        summary: 'Test decision',
        expires_at: '2024-02-14T14:30:00.000Z',
        hours_remaining: 24,
      },
    };
    expect(ProposalNearExpiryEventSchema.safeParse(event).success).toBe(true);
  });
});

describe('MemoryPromotedEventSchema', () => {
  it('accepts valid event', () => {
    const event = {
      event_id: validUuid,
      timestamp: validTimestamp,
      source_layer: 'edda',
      type: 'memory_promoted',
      payload: {
        memory_id: validUuid,
        memory_type: 'decision',
        statement: 'Use TypeScript',
        promoted_by: 'user@example.com',
      },
    };
    expect(MemoryPromotedEventSchema.safeParse(event).success).toBe(true);
  });
});

describe('MemoryRetiredEventSchema', () => {
  it('accepts valid event', () => {
    const event = {
      event_id: validUuid,
      timestamp: validTimestamp,
      source_layer: 'edda',
      type: 'memory_retired',
      payload: {
        memory_id: validUuid,
        memory_type: 'constraint',
        statement: 'Old constraint',
        retired_by: 'user@example.com',
        retirement_reason: 'No longer applicable',
      },
    };
    expect(MemoryRetiredEventSchema.safeParse(event).success).toBe(true);
  });
});

// =============================================================================
// Discriminated Union Tests
// =============================================================================

describe('StackEventSchema (discriminated union)', () => {
  it('discriminates by type field', () => {
    const observationEvent = {
      event_id: validUuid,
      timestamp: validTimestamp,
      source_layer: 'kindling',
      type: 'observation_recorded',
      payload: {
        observation_id: validUuid,
        session_id: validUuid,
        observation_kind: 'test',
      },
    };
    expect(StackEventSchema.safeParse(observationEvent).success).toBe(true);

    const memoryEvent = {
      event_id: validUuid,
      timestamp: validTimestamp,
      source_layer: 'edda',
      type: 'memory_promoted',
      payload: {
        memory_id: validUuid,
        memory_type: 'decision',
        statement: 'Test',
        promoted_by: 'user',
      },
    };
    expect(StackEventSchema.safeParse(memoryEvent).success).toBe(true);
  });

  it('rejects unknown event types', () => {
    const unknownEvent = {
      event_id: validUuid,
      timestamp: validTimestamp,
      source_layer: 'kindling',
      type: 'unknown_type',
      payload: {},
    };
    expect(StackEventSchema.safeParse(unknownEvent).success).toBe(false);
  });
});

// =============================================================================
// Factory Function Tests
// =============================================================================

describe('Event Factory Functions', () => {
  describe('createObservationRecordedEvent', () => {
    it('creates valid event with auto-generated fields', () => {
      const payload = {
        observation_id: validUuid as `${string}-${string}-${string}-${string}-${string}`,
        session_id: validUuid as `${string}-${string}-${string}-${string}-${string}`,
        observation_kind: 'gate_evaluated',
      };
      const event = createObservationRecordedEvent(payload);

      expect(event.type).toBe('observation_recorded');
      expect(event.source_layer).toBe('kindling');
      expect(event.payload).toEqual(payload);
      expect(event.event_id).toBeDefined();
      expect(event.timestamp).toBeDefined();
      expect(ObservationRecordedEventSchema.safeParse(event).success).toBe(true);
    });
  });

  describe('createSessionCompletedEvent', () => {
    it('creates valid event', () => {
      const payload = {
        session_id: validUuid as `${string}-${string}-${string}-${string}-${string}`,
        observation_count: 10,
        started_at: validTimestamp,
        ended_at: '2024-01-15T15:30:00.000Z',
      };
      const event = createSessionCompletedEvent(payload);

      expect(event.type).toBe('session_completed');
      expect(event.source_layer).toBe('kindling');
      expect(SessionCompletedEventSchema.safeParse(event).success).toBe(true);
    });
  });

  describe('createProposalCreatedEvent', () => {
    it('creates valid event', () => {
      const payload = {
        proposal_id: validUuid as `${string}-${string}-${string}-${string}-${string}`,
        proposal_type: 'pattern' as const,
        confidence: 0.75,
        summary: 'Test pattern',
        expires_at: '2024-02-14T14:30:00.000Z',
        source_observation_ids: [validUuid],
      };
      const event = createProposalCreatedEvent(payload);

      expect(event.type).toBe('proposal_created');
      expect(event.source_layer).toBe('ember');
      expect(ProposalCreatedEventSchema.safeParse(event).success).toBe(true);
    });
  });

  describe('createProposalNearExpiryEvent', () => {
    it('creates valid event', () => {
      const payload = {
        proposal_id: validUuid as `${string}-${string}-${string}-${string}-${string}`,
        proposal_type: 'decision' as const,
        confidence: 0.8,
        summary: 'Test decision',
        expires_at: '2024-02-14T14:30:00.000Z',
        hours_remaining: 24,
      };
      const event = createProposalNearExpiryEvent(payload);

      expect(event.type).toBe('proposal_near_expiry');
      expect(event.source_layer).toBe('ember');
      expect(ProposalNearExpiryEventSchema.safeParse(event).success).toBe(true);
    });
  });

  describe('createMemoryPromotedEvent', () => {
    it('creates valid event', () => {
      const payload = {
        memory_id: validUuid as `${string}-${string}-${string}-${string}-${string}`,
        memory_type: 'decision' as const,
        statement: 'Use TypeScript',
        promoted_by: 'user@example.com',
      };
      const event = createMemoryPromotedEvent(payload);

      expect(event.type).toBe('memory_promoted');
      expect(event.source_layer).toBe('edda');
      expect(MemoryPromotedEventSchema.safeParse(event).success).toBe(true);
    });
  });

  describe('createMemoryRetiredEvent', () => {
    it('creates valid event', () => {
      const payload = {
        memory_id: validUuid as `${string}-${string}-${string}-${string}-${string}`,
        memory_type: 'constraint' as const,
        statement: 'Old constraint',
        retired_by: 'user@example.com',
        retirement_reason: 'No longer applicable',
      };
      const event = createMemoryRetiredEvent(payload);

      expect(event.type).toBe('memory_retired');
      expect(event.source_layer).toBe('edda');
      expect(MemoryRetiredEventSchema.safeParse(event).success).toBe(true);
    });
  });

  describe('createEvent (generic factory)', () => {
    it('creates observation_recorded event', () => {
      const event = createEvent('observation_recorded', {
        observation_id: validUuid as `${string}-${string}-${string}-${string}-${string}`,
        session_id: validUuid as `${string}-${string}-${string}-${string}-${string}`,
        observation_kind: 'test',
      });
      expect(event.type).toBe('observation_recorded');
      expect(StackEventSchema.safeParse(event).success).toBe(true);
    });

    it('creates memory_promoted event', () => {
      const event = createEvent('memory_promoted', {
        memory_id: validUuid as `${string}-${string}-${string}-${string}-${string}`,
        memory_type: 'decision',
        statement: 'Test',
        promoted_by: 'user',
      });
      expect(event.type).toBe('memory_promoted');
      expect(StackEventSchema.safeParse(event).success).toBe(true);
    });
  });
});

// =============================================================================
// Type Guard Tests
// =============================================================================

describe('Type Guards', () => {
  const observationEvent = createObservationRecordedEvent({
    observation_id: validUuid as `${string}-${string}-${string}-${string}-${string}`,
    session_id: validUuid as `${string}-${string}-${string}-${string}-${string}`,
    observation_kind: 'test',
  });

  const sessionEvent = createSessionCompletedEvent({
    session_id: validUuid as `${string}-${string}-${string}-${string}-${string}`,
    observation_count: 10,
    started_at: validTimestamp,
    ended_at: validTimestamp,
  });

  const proposalCreatedEvent = createProposalCreatedEvent({
    proposal_id: validUuid as `${string}-${string}-${string}-${string}-${string}`,
    proposal_type: 'pattern',
    confidence: 0.75,
    summary: 'Test',
    expires_at: validTimestamp,
    source_observation_ids: [validUuid],
  });

  const proposalExpiryEvent = createProposalNearExpiryEvent({
    proposal_id: validUuid as `${string}-${string}-${string}-${string}-${string}`,
    proposal_type: 'decision',
    confidence: 0.8,
    summary: 'Test',
    expires_at: validTimestamp,
    hours_remaining: 24,
  });

  const memoryPromotedEvent = createMemoryPromotedEvent({
    memory_id: validUuid as `${string}-${string}-${string}-${string}-${string}`,
    memory_type: 'decision',
    statement: 'Test',
    promoted_by: 'user',
  });

  const memoryRetiredEvent = createMemoryRetiredEvent({
    memory_id: validUuid as `${string}-${string}-${string}-${string}-${string}`,
    memory_type: 'constraint',
    statement: 'Test',
    retired_by: 'user',
    retirement_reason: 'Reason',
  });

  describe('isObservationRecordedEvent', () => {
    it('returns true for observation_recorded events', () => {
      expect(isObservationRecordedEvent(observationEvent)).toBe(true);
    });

    it('returns false for other events', () => {
      expect(isObservationRecordedEvent(memoryPromotedEvent)).toBe(false);
    });
  });

  describe('isSessionCompletedEvent', () => {
    it('returns true for session_completed events', () => {
      expect(isSessionCompletedEvent(sessionEvent)).toBe(true);
    });

    it('returns false for other events', () => {
      expect(isSessionCompletedEvent(observationEvent)).toBe(false);
    });
  });

  describe('isProposalCreatedEvent', () => {
    it('returns true for proposal_created events', () => {
      expect(isProposalCreatedEvent(proposalCreatedEvent)).toBe(true);
    });

    it('returns false for other events', () => {
      expect(isProposalCreatedEvent(observationEvent)).toBe(false);
    });
  });

  describe('isProposalNearExpiryEvent', () => {
    it('returns true for proposal_near_expiry events', () => {
      expect(isProposalNearExpiryEvent(proposalExpiryEvent)).toBe(true);
    });

    it('returns false for other events', () => {
      expect(isProposalNearExpiryEvent(proposalCreatedEvent)).toBe(false);
    });
  });

  describe('isMemoryPromotedEvent', () => {
    it('returns true for memory_promoted events', () => {
      expect(isMemoryPromotedEvent(memoryPromotedEvent)).toBe(true);
    });

    it('returns false for other events', () => {
      expect(isMemoryPromotedEvent(observationEvent)).toBe(false);
    });
  });

  describe('isMemoryRetiredEvent', () => {
    it('returns true for memory_retired events', () => {
      expect(isMemoryRetiredEvent(memoryRetiredEvent)).toBe(true);
    });

    it('returns false for other events', () => {
      expect(isMemoryRetiredEvent(memoryPromotedEvent)).toBe(false);
    });
  });

  describe('isFromLayer', () => {
    it('correctly identifies kindling events', () => {
      expect(isFromLayer(observationEvent, 'kindling')).toBe(true);
      expect(isFromLayer(observationEvent, 'ember')).toBe(false);
    });

    it('correctly identifies ember events', () => {
      expect(isFromLayer(proposalCreatedEvent, 'ember')).toBe(true);
      expect(isFromLayer(proposalCreatedEvent, 'kindling')).toBe(false);
    });

    it('correctly identifies edda events', () => {
      expect(isFromLayer(memoryPromotedEvent, 'edda')).toBe(true);
      expect(isFromLayer(memoryPromotedEvent, 'ember')).toBe(false);
    });
  });

  describe('isKindlingEvent', () => {
    it('returns true for kindling events', () => {
      expect(isKindlingEvent(observationEvent)).toBe(true);
      expect(isKindlingEvent(sessionEvent)).toBe(true);
    });

    it('returns false for non-kindling events', () => {
      expect(isKindlingEvent(proposalCreatedEvent)).toBe(false);
      expect(isKindlingEvent(memoryPromotedEvent)).toBe(false);
    });
  });

  describe('isEmberEvent', () => {
    it('returns true for ember events', () => {
      expect(isEmberEvent(proposalCreatedEvent)).toBe(true);
      expect(isEmberEvent(proposalExpiryEvent)).toBe(true);
    });

    it('returns false for non-ember events', () => {
      expect(isEmberEvent(observationEvent)).toBe(false);
      expect(isEmberEvent(memoryPromotedEvent)).toBe(false);
    });
  });

  describe('isEddaEvent', () => {
    it('returns true for edda events', () => {
      expect(isEddaEvent(memoryPromotedEvent)).toBe(true);
      expect(isEddaEvent(memoryRetiredEvent)).toBe(true);
    });

    it('returns false for non-edda events', () => {
      expect(isEddaEvent(observationEvent)).toBe(false);
      expect(isEddaEvent(proposalCreatedEvent)).toBe(false);
    });
  });
});

// =============================================================================
// Event Descriptions Tests
// =============================================================================

describe('eventTypeDescriptions', () => {
  it('has descriptions for all event types', () => {
    const types = [
      'observation_recorded',
      'session_completed',
      'proposal_created',
      'proposal_near_expiry',
      'memory_promoted',
      'memory_retired',
    ] as const;

    for (const type of types) {
      expect(eventTypeDescriptions[type]).toBeDefined();
      expect(typeof eventTypeDescriptions[type]).toBe('string');
      expect(eventTypeDescriptions[type].length).toBeGreaterThan(0);
    }
  });
});

// =============================================================================
// Interface Compliance Tests
// =============================================================================

describe('IStackEventBus interface', () => {
  it('can be implemented', () => {
    // Mock implementation to verify interface is usable
    const mockEventBus: IStackEventBus = {
      publish: vi.fn().mockResolvedValue(undefined),
      subscribe: vi.fn().mockReturnValue(() => {}),
      subscribeAll: vi.fn().mockReturnValue(() => {}),
    };

    const event = createObservationRecordedEvent({
      observation_id: validUuid as `${string}-${string}-${string}-${string}-${string}`,
      session_id: validUuid as `${string}-${string}-${string}-${string}-${string}`,
      observation_kind: 'test',
    });

    // Test publish
    mockEventBus.publish(event);
    expect(mockEventBus.publish).toHaveBeenCalledWith(event);

    // Test subscribe
    const handler: EventHandler = vi.fn();
    const unsubscribe = mockEventBus.subscribe('observation_recorded', handler);
    expect(mockEventBus.subscribe).toHaveBeenCalledWith('observation_recorded', handler);
    expect(typeof unsubscribe).toBe('function');

    // Test subscribeAll
    const allHandler: EventHandler = vi.fn();
    const unsubscribeAll = mockEventBus.subscribeAll(allHandler);
    expect(mockEventBus.subscribeAll).toHaveBeenCalledWith(allHandler);
    expect(typeof unsubscribeAll).toBe('function');
  });
});
