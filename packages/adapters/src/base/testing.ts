/**
 * Adapter Testing Utilities
 *
 * Helpers for testing format adapters with common patterns and assertions.
 */

import type {
  FormatAdapter,
  ParseResult,
  SerializeResult,
  DetectionResult,
  ParseContext,
  AdapterOptions,
} from './types.js';
import type { APSPlan } from '@eddacraft/anvil-core';

/**
 * Test fixture for adapter testing
 */
export interface AdapterTestFixture {
  /** Name of the test case */
  name: string;
  /** Raw content in external format */
  content: string;
  /** Expected detection result (optional) */
  expectedDetection?: Partial<DetectionResult>;
  /** Expected parse success (optional) */
  expectParseSuccess?: boolean;
  /** Context for parsing */
  parseContext?: ParseContext;
  /** Options for operations */
  options?: AdapterOptions;
  /** Expected values in parsed plan (optional) */
  expectedPlan?: Partial<APSPlan>;
  /** Expected error codes (if parse should fail) */
  expectedErrors?: string[];
  /** Expected warning codes */
  expectedWarnings?: string[];
}

/**
 * Test helper for adapter detection
 */
export class AdapterTester {
  constructor(private adapter: FormatAdapter) {}

  /**
   * Test detection with a fixture
   */
  testDetection(fixture: AdapterTestFixture): DetectionResult {
    const result = this.adapter.detect(fixture.content);

    if (fixture.expectedDetection) {
      if (fixture.expectedDetection.detected !== undefined) {
        if (result.detected !== fixture.expectedDetection.detected) {
          throw new Error(
            `Detection mismatch for "${fixture.name}": expected ${fixture.expectedDetection.detected}, got ${result.detected}`
          );
        }
      }

      if (fixture.expectedDetection.confidence !== undefined) {
        if (result.confidence < fixture.expectedDetection.confidence) {
          throw new Error(
            `Confidence too low for "${fixture.name}": expected >=${fixture.expectedDetection.confidence}, got ${result.confidence}`
          );
        }
      }
    }

    return result;
  }

  /**
   * Test parsing with a fixture
   */
  async testParse(fixture: AdapterTestFixture): Promise<ParseResult> {
    const result = await this.adapter.parse(fixture.content, fixture.parseContext, fixture.options);

    if (fixture.expectParseSuccess !== undefined) {
      if (result.success !== fixture.expectParseSuccess) {
        throw new Error(
          `Parse success mismatch for "${fixture.name}": expected ${fixture.expectParseSuccess}, got ${result.success}`
        );
      }
    }

    if (fixture.expectedErrors && fixture.expectedErrors.length > 0) {
      if (!result.errors || result.errors.length === 0) {
        throw new Error(`Expected errors for "${fixture.name}" but got none`);
      }

      const errorCodes = result.errors.map((e) => e.code);
      for (const expectedCode of fixture.expectedErrors) {
        if (!errorCodes.includes(expectedCode)) {
          throw new Error(
            `Expected error code "${expectedCode}" for "${fixture.name}" but found: ${errorCodes.join(', ')}`
          );
        }
      }
    }

    if (fixture.expectedWarnings && fixture.expectedWarnings.length > 0) {
      if (!result.warnings || result.warnings.length === 0) {
        throw new Error(`Expected warnings for "${fixture.name}" but got none`);
      }

      const warningCodes = result.warnings.map((w) => w.code);
      for (const expectedCode of fixture.expectedWarnings) {
        if (!warningCodes.includes(expectedCode)) {
          throw new Error(
            `Expected warning code "${expectedCode}" for "${fixture.name}" but found: ${warningCodes.join(', ')}`
          );
        }
      }
    }

    if (fixture.expectedPlan && result.data) {
      this.assertPlanMatches(result.data, fixture.expectedPlan, fixture.name);
    }

    return result;
  }

  /**
   * Test serialization
   */
  async testSerialize(plan: APSPlan, options?: AdapterOptions): Promise<SerializeResult> {
    return this.adapter.serialize(plan, options);
  }

  /**
   * Test round-trip: parse then serialize then parse again
   */
  async testRoundTrip(fixture: AdapterTestFixture): Promise<{
    firstParse: ParseResult;
    serialized: SerializeResult;
    secondParse: ParseResult;
    planMatches: boolean;
  }> {
    // First parse
    const firstParse = await this.adapter.parse(
      fixture.content,
      fixture.parseContext,
      fixture.options
    );

    if (!firstParse.success || !firstParse.data) {
      throw new Error(`First parse failed for "${fixture.name}"`);
    }

    // Serialize
    const serialized = await this.adapter.serialize(firstParse.data, fixture.options);

    if (!serialized.success || !serialized.content) {
      throw new Error(`Serialization failed for "${fixture.name}"`);
    }

    // Second parse
    const secondParse = await this.adapter.parse(
      serialized.content,
      fixture.parseContext,
      fixture.options
    );

    if (!secondParse.success || !secondParse.data) {
      throw new Error(`Second parse failed for "${fixture.name}"`);
    }

    // Compare plans
    const planMatches = this.plansEqual(firstParse.data, secondParse.data);

    return {
      firstParse,
      serialized,
      secondParse,
      planMatches,
    };
  }

  /**
   * Assert that parsed plan matches expected values
   */
  private assertPlanMatches(actual: APSPlan, expected: Partial<APSPlan>, testName: string): void {
    if (expected.intent !== undefined && actual.intent !== expected.intent) {
      throw new Error(
        `Intent mismatch for "${testName}": expected "${expected.intent}", got "${actual.intent}"`
      );
    }

    if (expected.id !== undefined && actual.id !== expected.id) {
      throw new Error(
        `ID mismatch for "${testName}": expected "${expected.id}", got "${actual.id}"`
      );
    }

    if (
      expected.proposed_changes !== undefined &&
      actual.proposed_changes.length !== expected.proposed_changes.length
    ) {
      throw new Error(
        `Changes count mismatch for "${testName}": expected ${expected.proposed_changes.length}, got ${actual.proposed_changes.length}`
      );
    }
  }

  /**
   * Compare two plans for equality (ignoring hash and timestamps)
   */
  private plansEqual(plan1: APSPlan, plan2: APSPlan): boolean {
    // Compare intents
    if (plan1.intent !== plan2.intent) return false;

    // Compare changes count
    if (plan1.proposed_changes.length !== plan2.proposed_changes.length) return false;

    // Compare each change
    for (let i = 0; i < plan1.proposed_changes.length; i++) {
      const change1 = plan1.proposed_changes[i];
      const change2 = plan2.proposed_changes[i];

      if (change1.type !== change2.type) return false;
      if (change1.path !== change2.path) return false;
      if (change1.description !== change2.description) return false;
    }

    return true;
  }
}

/**
 * Create a test adapter tester
 */
export function createTester(adapter: FormatAdapter): AdapterTester {
  return new AdapterTester(adapter);
}

/**
 * Create a basic parse context for testing
 */
export function createTestContext(overrides?: Partial<ParseContext>): ParseContext {
  return {
    author: 'test-user',
    repositoryPath: '/test/repo',
    branch: 'main',
    commit: 'abc123',
    timestamp: '2024-01-01T00:00:00.000Z',
    ...overrides,
  };
}

/**
 * Create basic adapter options for testing
 */
export function createTestOptions(overrides?: Partial<AdapterOptions>): AdapterOptions {
  return {
    preserveComments: true,
    preserveMetadata: true,
    strict: false,
    ...overrides,
  };
}

/**
 * Assert detection result
 */
export function assertDetection(
  result: DetectionResult,
  expected: { detected: boolean; minConfidence?: number }
): void {
  if (result.detected !== expected.detected) {
    throw new Error(
      `Detection mismatch: expected ${expected.detected}, got ${result.detected}. Reason: ${result.reason}`
    );
  }

  if (expected.minConfidence !== undefined && result.confidence < expected.minConfidence) {
    throw new Error(
      `Confidence too low: expected >=${expected.minConfidence}, got ${result.confidence}`
    );
  }
}

/**
 * Assert parse success
 */
export function assertParseSuccess(
  result: ParseResult
): asserts result is ParseResult & { data: APSPlan } {
  if (!result.success) {
    const errorMsg =
      result.errors?.map((e) => `${e.code}: ${e.message}`).join(', ') || 'Unknown error';
    throw new Error(`Parse failed: ${errorMsg}`);
  }

  if (!result.data) {
    throw new Error('Parse succeeded but no data returned');
  }
}

/**
 * Assert parse failure
 */
export function assertParseFailure(result: ParseResult, expectedErrorCode?: string): void {
  if (result.success) {
    throw new Error('Expected parse to fail but it succeeded');
  }

  if (expectedErrorCode && result.errors) {
    const errorCodes = result.errors.map((e) => e.code);
    if (!errorCodes.includes(expectedErrorCode)) {
      throw new Error(
        `Expected error code "${expectedErrorCode}" but found: ${errorCodes.join(', ')}`
      );
    }
  }
}

/**
 * Assert serialize success
 */
export function assertSerializeSuccess(
  result: SerializeResult
): asserts result is SerializeResult & { content: string } {
  if (!result.success) {
    const errorMsg =
      result.errors?.map((e) => `${e.code}: ${e.message}`).join(', ') || 'Unknown error';
    throw new Error(`Serialize failed: ${errorMsg}`);
  }

  if (!result.content) {
    throw new Error('Serialize succeeded but no content returned');
  }
}
