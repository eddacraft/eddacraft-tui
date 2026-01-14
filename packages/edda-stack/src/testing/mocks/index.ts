/**
 * Testing Mocks
 *
 * Mock implementations of port interfaces for testing.
 *
 * @module @anvil/edda-stack/testing/mocks
 */

// Kindling Mocks
export {
  createMockKindlingPort,
  mockKindlingWithObservations,
  mockKindlingEmpty,
  mockKindlingMultipleSessions,
  type MockKindlingPort,
  type MockKindlingPortOptions,
} from './kindling.mock.js';

// Ember Mocks
export {
  createMockEmberPort,
  mockEmberWithProposals,
  mockEmberEmpty,
  mockEmberWithMixedStatuses,
  type MockEmberPort,
  type MockEmberPortOptions,
} from './ember.mock.js';

// Edda Mocks
export {
  createMockEddaPort,
  mockEddaWithMemories,
  mockEddaEmpty,
  mockEddaWithEvolutionChain,
  type MockEddaPort,
  type MockEddaPortOptions,
} from './edda.mock.js';
