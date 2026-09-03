type TestEnvironment = Readonly<Record<string, string | undefined>>;

const DIRECT_NEON_HOST = /^ep-[a-z0-9-]+(?:\.[a-z0-9-]+)*\.neon\.tech$/i;
const EXPECTED_PROJECT_NAME = 'anvil-api-test';
const EXPECTED_DATABASE_NAME = 'anvil_test';
const EXPECTED_ROLE_NAME = 'anvil_test_owner';
const TEST_BRANCH_NAME = /^ci-test-clawopen-011-[1-9]\d*-[1-9]\d*$/;

/**
 * Reject unsafe live-database configuration before a test creates schema
 * objects or synthetic rows. Deliberately reads only the test-specific URL;
 * production DATABASE_URL is never a fallback.
 */
export function requireSafeNeonTestDatabase(environment: TestEnvironment = process.env): {
  url: string;
} {
  const rawUrl = environment['ANVIL_API_TEST_DATABASE_URL'];
  if (!rawUrl) {
    throw new Error('ANVIL_API_TEST_DATABASE_URL is required for Neon integration tests');
  }

  let databaseUrl: URL;
  try {
    databaseUrl = new URL(rawUrl);
  } catch {
    throw new Error('ANVIL_API_TEST_DATABASE_URL must be a valid PostgreSQL URL');
  }

  const endpointId = databaseUrl.hostname.split('.')[0] ?? '';
  const isDirectNeonEndpoint =
    (databaseUrl.protocol === 'postgres:' || databaseUrl.protocol === 'postgresql:') &&
    DIRECT_NEON_HOST.test(databaseUrl.hostname) &&
    !endpointId.endsWith('-pooler');
  if (!isDirectNeonEndpoint) {
    throw new Error('ANVIL_API_TEST_DATABASE_URL must target a direct Neon endpoint');
  }

  const databaseName = databaseUrl.pathname.slice(1);
  if (databaseUrl.username !== EXPECTED_ROLE_NAME) {
    throw new Error(`Neon test database role must equal ${EXPECTED_ROLE_NAME}`);
  }
  if (databaseName !== EXPECTED_DATABASE_NAME) {
    throw new Error(`Neon test database name must equal ${EXPECTED_DATABASE_NAME}`);
  }

  const sslMode = databaseUrl.searchParams.get('sslmode');
  if (sslMode !== 'require' && sslMode !== 'verify-ca' && sslMode !== 'verify-full') {
    throw new Error('ANVIL_API_TEST_DATABASE_URL must require TLS');
  }

  if (environment['ANVIL_API_TEST_DATABASE_PROJECT_NAME'] !== EXPECTED_PROJECT_NAME) {
    throw new Error(`Neon test database project name must equal ${EXPECTED_PROJECT_NAME}`);
  }
  if (!TEST_BRANCH_NAME.test(environment['ANVIL_API_TEST_DATABASE_BRANCH_NAME'] ?? '')) {
    throw new Error('Neon test database branch name must match the CLAWOPEN-011 CI test prefix');
  }

  return { url: rawUrl };
}
