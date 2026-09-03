import { describe, expect, it } from 'vitest';
import { requireSafeNeonTestDatabase } from './support/neon-test-database.js';

const SAFE_URL =
  'postgresql://anvil_test_owner:test-password@ep-anvil-test-123.ap-southeast-2.aws.neon.tech/anvil_test?sslmode=require';

function safeEnvironment(overrides: Record<string, string | undefined> = {}) {
  return {
    ANVIL_API_TEST_DATABASE_URL: SAFE_URL,
    ANVIL_API_TEST_DATABASE_PROJECT_NAME: 'anvil-api-test',
    ANVIL_API_TEST_DATABASE_BRANCH_NAME: 'ci-test-clawopen-011-1234-1',
    ...overrides,
  };
}

describe('Neon integration-test safety gate', () => {
  it('accepts only the dedicated test URL and test-labelled project and branch', () => {
    expect(requireSafeNeonTestDatabase(safeEnvironment())).toEqual({ url: SAFE_URL });
  });

  it('never falls back to DATABASE_URL', () => {
    expect(() =>
      requireSafeNeonTestDatabase({
        DATABASE_URL: SAFE_URL,
        ANVIL_API_TEST_DATABASE_PROJECT_NAME: 'anvil-api-test',
        ANVIL_API_TEST_DATABASE_BRANCH_NAME: 'ci-test-clawopen-011-1234-1',
      })
    ).toThrow('ANVIL_API_TEST_DATABASE_URL is required');
  });

  it.each([
    {
      name: 'non-Neon host',
      environment: safeEnvironment({
        ANVIL_API_TEST_DATABASE_URL:
          'postgresql://anvil_test:test-password@database.example.test/anvil_test?sslmode=require',
      }),
      message: 'must target a direct Neon endpoint',
    },
    {
      name: 'pooled Neon endpoint',
      environment: safeEnvironment({
        ANVIL_API_TEST_DATABASE_URL:
          'postgresql://anvil_test:test-password@ep-anvil-test-123-pooler.ap-southeast-2.aws.neon.tech/anvil_test?sslmode=require',
      }),
      message: 'must target a direct Neon endpoint',
    },
    {
      name: 'near-match project name',
      environment: safeEnvironment({ ANVIL_API_TEST_DATABASE_PROJECT_NAME: 'anvil-api-testing' }),
      message: 'project name must equal anvil-api-test',
    },
    {
      name: 'wrong test branch prefix',
      environment: safeEnvironment({
        ANVIL_API_TEST_DATABASE_BRANCH_NAME: 'ci-test-other-1234-1',
      }),
      message: 'branch name must match the CLAWOPEN-011 CI test prefix',
    },
    {
      name: 'near-match database role',
      environment: safeEnvironment({
        ANVIL_API_TEST_DATABASE_URL:
          'postgresql://anvil_test_writer:test-password@ep-anvil-test-123.ap-southeast-2.aws.neon.tech/anvil_test?sslmode=require',
      }),
      message: 'role must equal anvil_test_owner',
    },
    {
      name: 'near-match database name',
      environment: safeEnvironment({
        ANVIL_API_TEST_DATABASE_URL:
          'postgresql://anvil_test_owner:test-password@ep-anvil-test-123.ap-southeast-2.aws.neon.tech/anvil_test_archive?sslmode=require',
      }),
      message: 'database must equal anvil_test',
    },
    {
      name: 'unencrypted connection option',
      environment: safeEnvironment({
        ANVIL_API_TEST_DATABASE_URL:
          'postgresql://anvil_test_owner:test-password@ep-anvil-test-123.ap-southeast-2.aws.neon.tech/anvil_test?sslmode=disable',
      }),
      message: 'must require TLS',
    },
  ])('rejects $name without exposing credentials', ({ environment, message }) => {
    let error: unknown;
    try {
      requireSafeNeonTestDatabase(environment);
    } catch (caught) {
      error = caught;
    }

    expect(error).toBeInstanceOf(Error);
    expect((error as Error).message).toContain(message);
    expect((error as Error).message).not.toContain('test-password');
    expect((error as Error).message).not.toContain(environment.ANVIL_API_TEST_DATABASE_URL ?? '');
  });
});
