import assert from 'node:assert/strict';
import test from 'node:test';
import { cleanupNeonTestBranch, provisionNeonTestBranch } from './create-neon-test-branch.mjs';

const BRANCH_NAME = 'ci-test-clawopen-011-1234-1';
const EXPIRES_AT = '2026-08-31T08:00:00Z';
const DATABASE_URL =
  'postgresql://anvil_test_owner:generated-placeholder@ep-anvil-test.ap-southeast-2.aws.neon.tech/anvil_test?sslmode=require';

function environment(overrides = {}) {
  return {
    NEON_API_KEY: 'test-api-key',
    NEON_PROJECT_ID: 'test-project-id',
    NEON_BRANCH_NAME: BRANCH_NAME,
    NEON_BRANCH_EXPIRES_AT: EXPIRES_AT,
    GITHUB_OUTPUT: '/not-used-by-injected-writer',
    ...overrides,
  };
}

function jsonResponse(body, status = 200) {
  return new Response(JSON.stringify(body), {
    status,
    headers: { 'content-type': 'application/json' },
  });
}

test('rejects a non-dedicated project before attempting branch creation', async () => {
  const requests = [];
  const fetchImpl = async (url, init = {}) => {
    requests.push({ url: String(url), method: init.method ?? 'GET' });
    return jsonResponse({ project: { id: 'test-project-id', name: 'anvil-production' } });
  };

  await assert.rejects(
    provisionNeonTestBranch({
      environment: environment(),
      fetchImpl,
      appendOutput: () => assert.fail('unsafe preflight must not emit outputs'),
      emitWorkflowCommand: () => assert.fail('unsafe preflight must not emit masks'),
    }),
    /project name must equal anvil-api-test/
  );

  assert.deepEqual(requests, [
    {
      url: 'https://console.neon.tech/api/v2/projects/test-project-id',
      method: 'GET',
    },
  ]);
});

test('masks generated credentials before emitting the database URL output', async () => {
  const events = [];
  const requests = [];
  const fetchImpl = async (url, init = {}) => {
    const request = { url: String(url), method: init.method ?? 'GET' };
    requests.push(request);
    if (request.method === 'GET' && request.url.endsWith('/projects/test-project-id')) {
      return jsonResponse({ project: { id: 'test-project-id', name: 'anvil-api-test' } });
    }
    if (request.method === 'POST' && request.url.endsWith('/branches')) {
      return jsonResponse({ branch: { id: 'br-test-123', name: BRANCH_NAME } }, 201);
    }
    if (request.method === 'GET' && request.url.includes('/connection_uri?')) {
      return jsonResponse({ uri: DATABASE_URL });
    }
    assert.fail(`unexpected request: ${request.method} ${request.url}`);
  };

  await provisionNeonTestBranch({
    environment: environment(),
    fetchImpl,
    appendOutput: (line) => events.push({ type: 'output', value: line }),
    emitWorkflowCommand: (line) => events.push({ type: 'command', value: line }),
  });

  const fullUrlMask = events.findIndex(
    (event) => event.type === 'command' && event.value === `::add-mask::${DATABASE_URL}`
  );
  const passwordMask = events.findIndex(
    (event) => event.type === 'command' && event.value === '::add-mask::generated-placeholder'
  );
  const databaseUrlOutput = events.findIndex(
    (event) => event.type === 'output' && event.value === `db_url=${DATABASE_URL}`
  );

  assert.ok(fullUrlMask >= 0, 'the complete generated URI must be masked');
  assert.ok(passwordMask >= 0, 'the generated password must be masked independently');
  assert.ok(databaseUrlOutput > fullUrlMask, 'mask the URI before writing it to GITHUB_OUTPUT');
  assert.ok(databaseUrlOutput > passwordMask, 'mask the password before writing GITHUB_OUTPUT');
  assert.ok(
    requests.some(
      ({ url }) =>
        url.includes('database_name=anvil_test') &&
        url.includes('role_name=anvil_test_owner') &&
        url.includes('pooled=false')
    ),
    'retrieve only the exact direct test database URI'
  );
});

test('preserves exact cleanup identity when connection-URI retrieval fails', async () => {
  const outputs = [];
  const fetchImpl = async (url, init = {}) => {
    const requestUrl = String(url);
    const method = init.method ?? 'GET';
    if (method === 'GET' && requestUrl.endsWith('/projects/test-project-id')) {
      return jsonResponse({ project: { id: 'test-project-id', name: 'anvil-api-test' } });
    }
    if (method === 'POST' && requestUrl.endsWith('/branches')) {
      return jsonResponse({ branch: { id: 'br-partial-123', name: BRANCH_NAME } }, 201);
    }
    return jsonResponse({ detail: 'PRIVATE_RESPONSE_MARKER' }, 500);
  };

  await assert.rejects(
    provisionNeonTestBranch({
      environment: environment(),
      fetchImpl,
      appendOutput: (line) => outputs.push(line),
      emitWorkflowCommand: () => {},
    }),
    (error) => {
      assert.doesNotMatch(error.message, /PRIVATE_RESPONSE_MARKER/);
      return /Neon API request failed with status 500/.test(error.message);
    }
  );

  assert.deepEqual(outputs, [
    `branch_name=${BRANCH_NAME}`,
    `expires_at=${EXPIRES_AT}`,
    'branch_id=br-partial-123',
  ]);
});

test('cleanup resolves an ambiguous partial create by exact branch name', async () => {
  const requests = [];
  const fetchImpl = async (url, init = {}) => {
    const request = { url: String(url), method: init.method ?? 'GET' };
    requests.push(request);
    if (request.method === 'GET' && request.url.endsWith('/projects/test-project-id')) {
      return jsonResponse({ project: { id: 'test-project-id', name: 'anvil-api-test' } });
    }
    if (request.method === 'GET' && request.url.includes('/branches?')) {
      return jsonResponse({
        branches: [
          { id: 'br-unrelated', name: `${BRANCH_NAME}-other` },
          { id: 'br-exact-partial', name: BRANCH_NAME },
        ],
      });
    }
    if (request.method === 'DELETE' && request.url.endsWith('/branches/br-exact-partial')) {
      return new Response(null, { status: 204 });
    }
    assert.fail(`unexpected request: ${request.method} ${request.url}`);
  };

  await expectCleanup(
    cleanupNeonTestBranch({
      environment: environment({ NEON_BRANCH_ID: undefined }),
      fetchImpl,
    })
  );

  assert.equal(requests.at(-1)?.method, 'DELETE');
  assert.match(requests.at(-1)?.url ?? '', /\/branches\/br-exact-partial$/);
});

async function expectCleanup(cleanup) {
  assert.deepEqual(await cleanup, { branchId: 'br-exact-partial', deleted: true });
}
