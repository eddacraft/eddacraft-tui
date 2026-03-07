import { afterEach, describe, expect, it, vi } from 'vitest';
import { createMockEmberPort } from '../testing/mocks/ember.mock.js';
import {
  createSessionCompletedEvent,
  type EventHandler,
  type EventType,
  type IStackEventBus,
  type StackEvent,
} from '../contracts/events.js';
import { createSessionId } from '../contracts/identifiers.js';
import { CandidateService } from './candidate-service.js';
import { ObservationHook } from './observation-hook.js';

afterEach(() => {
  vi.restoreAllMocks();
});

class TestEventBus implements IStackEventBus {
  public readonly publish = vi.fn(async (event: StackEvent): Promise<void> => {
    const handlers = this.handlers.get(event.type) ?? [];
    await Promise.all(handlers.map(async (handler) => handler(event)));
  });

  public readonly subscribe = vi.fn(
    <T extends EventType>(eventType: T, handler: EventHandler): (() => void) => {
      const handlers = this.handlers.get(eventType) ?? [];
      handlers.push(handler);
      this.handlers.set(eventType, handlers);
      return () => {
        const existing = this.handlers.get(eventType) ?? [];
        this.handlers.set(
          eventType,
          existing.filter((registered) => registered !== handler)
        );
      };
    }
  );

  public readonly subscribeAll = vi.fn((_handler: EventHandler): (() => void) => () => {});

  private readonly handlers = new Map<EventType, EventHandler[]>();
}

describe('ObservationHook', () => {
  it('start subscribes to session_completed events', () => {
    const eventBus = new TestEventBus();
    const candidateService = new CandidateService({ store: createMockEmberPort() });
    const hook = new ObservationHook({ candidateService, eventBus });

    hook.start();

    expect(eventBus.subscribe).toHaveBeenCalledTimes(1);
    expect(eventBus.subscribe.mock.calls[0][0]).toBe('session_completed');
  });

  it('stop unsubscribes from event bus', () => {
    const unsubscribe = vi.fn();
    const eventBus = {
      publish: vi.fn().mockResolvedValue(undefined),
      subscribe: vi.fn().mockReturnValue(unsubscribe),
      subscribeAll: vi.fn().mockReturnValue(() => {}),
    } as unknown as IStackEventBus;
    const candidateService = new CandidateService({ store: createMockEmberPort() });
    const hook = new ObservationHook({ candidateService, eventBus });

    hook.start();
    hook.stop();

    expect(unsubscribe).toHaveBeenCalledTimes(1);
    expect(hook.isActive()).toBe(false);
  });

  it('isActive reports current state', () => {
    const eventBus = new TestEventBus();
    const candidateService = new CandidateService({ store: createMockEmberPort() });
    const hook = new ObservationHook({ candidateService, eventBus });

    expect(hook.isActive()).toBe(false);

    hook.start();
    expect(hook.isActive()).toBe(true);

    hook.stop();
    expect(hook.isActive()).toBe(false);
  });

  it('processSession delegates to candidateService', async () => {
    const candidateService = new CandidateService({ store: createMockEmberPort() });
    const processSessionSpy = vi
      .spyOn(candidateService, 'processSession')
      .mockResolvedValueOnce([]);
    const hook = new ObservationHook({ candidateService });

    await hook.processSession('550e8400-e29b-41d4-a716-446655440040');

    expect(processSessionSpy).toHaveBeenCalledWith('550e8400-e29b-41d4-a716-446655440040');
  });

  it('processes session when session.completed event is emitted', async () => {
    const eventBus = new TestEventBus();
    const candidateService = new CandidateService({ store: createMockEmberPort() });
    const processSessionSpy = vi
      .spyOn(candidateService, 'processSession')
      .mockResolvedValueOnce([]);
    const hook = new ObservationHook({ candidateService, eventBus });

    hook.start();

    const event = createSessionCompletedEvent({
      session_id: createSessionId('550e8400-e29b-41d4-a716-446655440041'),
      observation_count: 8,
      started_at: '2026-01-10T12:00:00.000Z',
      ended_at: '2026-01-10T12:10:00.000Z',
      outcome: 'success',
    });
    await eventBus.publish(event);

    expect(processSessionSpy).toHaveBeenCalledWith('550e8400-e29b-41d4-a716-446655440041');
  });

  it('logs and suppresses session processing failures from the event handler', async () => {
    const eventBus = new TestEventBus();
    const candidateService = {
      processSession: vi.fn(async () => {
        throw new Error('processing failed');
      }),
    };
    const consoleErrorSpy = vi.spyOn(console, 'error').mockImplementation(() => undefined);
    const hook = new ObservationHook({ candidateService, eventBus });

    hook.start();

    const event = createSessionCompletedEvent({
      session_id: createSessionId('550e8400-e29b-41d4-a716-446655440042'),
      observation_count: 4,
      started_at: '2026-01-10T12:00:00.000Z',
      ended_at: '2026-01-10T12:10:00.000Z',
      outcome: 'failure',
    });

    await expect(eventBus.publish(event)).resolves.toBeUndefined();
    expect(consoleErrorSpy).toHaveBeenCalledWith(
      'ObservationHook failed to process session 550e8400-e29b-41d4-a716-446655440042',
      expect.objectContaining({ message: 'processing failed' })
    );
  });
});
