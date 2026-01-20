/**
 * Edda Stack Testing Utilities
 *
 * Provides mocks, fixtures, and validators for testing code
 * that interacts with the Edda Stack.
 *
 * @module @eddacraft/anvil-edda-stack/testing
 */

// =============================================================================
// Mocks
// =============================================================================

export {
  // Kindling Mocks
  createMockKindlingPort,
  mockKindlingWithObservations,
  mockKindlingEmpty,
  mockKindlingMultipleSessions,
  type MockKindlingPort,
  type MockKindlingPortOptions,
  // Ember Mocks
  createMockEmberPort,
  mockEmberWithProposals,
  mockEmberEmpty,
  mockEmberWithMixedStatuses,
  type MockEmberPort,
  type MockEmberPortOptions,
  // Edda Mocks
  createMockEddaPort,
  mockEddaWithMemories,
  mockEddaEmpty,
  mockEddaWithEvolutionChain,
  type MockEddaPort,
  type MockEddaPortOptions,
} from './mocks/index.js';

// =============================================================================
// Fixtures
// =============================================================================

export {
  // Proposal Fixtures
  createProposalFixture,
  createValidDecisionProposal,
  createValidPatternProposal,
  createValidWarningProposal,
  createValidLessonProposal,
  createValidAnomalyProposal,
  createValidConstraintProposal,
  createActiveProposal,
  createExpiredProposal,
  createPromotedProposal,
  createDismissedProposal,
  createProposalsOfAllTypes,
  createProposalsOfAllStatuses,
  type ProposalFixtureOverrides,
  // Memory Fixtures
  createMemoryFixture,
  createValidDecisionMemory,
  createValidPatternMemory,
  createValidConstraintMemory,
  createValidWarningMemory,
  createValidDoctrineMemory,
  createValidLessonMemory,
  createActiveMemory,
  createSupersededMemory,
  createRetiredMemory,
  createSupersedesMemory,
  createEvolutionChain,
  createMultiLevelEvolutionChain,
  createMemoriesOfAllTypes,
  createMemoriesOfAllStatuses,
  type MemoryFixtureOverrides,
} from './fixtures/index.js';

// =============================================================================
// Validators
// =============================================================================

export {
  // Main validator
  validateProvenanceChain,
  // Individual validators
  validateKindlingRefs,
  validateEmberRef,
  validateTemporalOrdering,
  validateSessionConsistency,
  validateNoDuplicates,
  // Utility functions
  createValidResult,
  createInvalidResult,
  hasValidationCode,
  getIssuesByCode,
  formatValidationResult,
  // Types and enums
  ProvenanceValidationCode,
  type ProvenanceValidationIssue,
  type ProvenanceValidationResult,
} from './validators/index.js';
