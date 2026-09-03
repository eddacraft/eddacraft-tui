#!/usr/bin/env node

import { appendFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

const API_ORIGIN = 'https://console.neon.tech/api/v2';
const EXPECTED_PROJECT_NAME = 'anvil-api-test';
const EXPECTED_DATABASE_NAME = 'anvil_test';
const EXPECTED_ROLE_NAME = 'anvil_test_owner';
const BRANCH_NAME_PATTERN = /^ci-test-clawopen-011-[1-9]\d*-[1-9]\d*$/;
const RESOURCE_ID_PATTERN = /^[a-z0-9-]{1,60}$/;
const DIRECT_NEON_HOST = /^ep-[a-z0-9-]+(?:\.[a-z0-9-]+)*\.neon\.tech$/i;

function requireEnvironment(environment, key) {
  const value = environment[key];
  if (!value) throw new Error(`${key} is required`);
  return value;
}

function validateConfiguration(environment) {
  const apiKey = requireEnvironment(environment, 'NEON_API_KEY');
  const projectId = requireEnvironment(environment, 'NEON_PROJECT_ID');
  const branchName = requireEnvironment(environment, 'NEON_BRANCH_NAME');
  const expiresAt = requireEnvironment(environment, 'NEON_BRANCH_EXPIRES_AT');
  const outputPath = requireEnvironment(environment, 'GITHUB_OUTPUT');

  if (!RESOURCE_ID_PATTERN.test(projectId)) throw new Error('NEON_PROJECT_ID is invalid');
  if (!BRANCH_NAME_PATTERN.test(branchName)) {
    throw new Error('NEON_BRANCH_NAME must match the CLAWOPEN-011 CI test prefix');
  }
  if (!/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}Z$/.test(expiresAt)) {
    throw new Error('NEON_BRANCH_EXPIRES_AT must be an RFC 3339 UTC timestamp');
  }

  return { apiKey, projectId, branchName, expiresAt, outputPath };
}

async function requestApi(fetchImpl, apiKey, path, init = {}) {
  let response;
  try {
    response = await fetchImpl(`${API_ORIGIN}${path}`, {
      ...init,
      redirect: 'error',
      headers: {
        Accept: 'application/json',
        Authorization: `Bearer ${apiKey}`,
        ...(init.body === undefined ? {} : { 'Content-Type': 'application/json' }),
      },
    });
  } catch {
    throw new Error('Neon API request failed before receiving a response');
  }

  if (!response.ok) throw new Error(`Neon API request failed with status ${response.status}`);

  return response;
}

async function requestJson(fetchImpl, apiKey, path, init = {}) {
  const response = await requestApi(fetchImpl, apiKey, path, init);
  try {
    return await response.json();
  } catch {
    throw new Error('Neon API returned an invalid JSON response');
  }
}

async function requireDedicatedProject(fetchImpl, apiKey, projectId) {
  const projectResponse = await requestJson(fetchImpl, apiKey, `/projects/${projectId}`);
  if (projectResponse?.project?.name !== EXPECTED_PROJECT_NAME) {
    throw new Error(`Neon project name must equal ${EXPECTED_PROJECT_NAME}`);
  }
}

function escapeWorkflowCommandData(value) {
  return value.replaceAll('%', '%25').replaceAll('\r', '%0D').replaceAll('\n', '%0A');
}

function validateOutputValue(value, label) {
  if (!value || /[\r\n]/.test(value)) throw new Error(`Neon API returned an invalid ${label}`);
  return value;
}

function requireSafeDatabaseUrl(rawUrl) {
  let databaseUrl;
  try {
    databaseUrl = new URL(rawUrl);
  } catch {
    throw new Error('Neon API returned an invalid database URI');
  }

  const endpointId = databaseUrl.hostname.split('.')[0] ?? '';
  if (
    databaseUrl.protocol !== 'postgresql:' ||
    !DIRECT_NEON_HOST.test(databaseUrl.hostname) ||
    endpointId.endsWith('-pooler')
  ) {
    throw new Error('Neon API returned a non-direct database URI');
  }
  if (databaseUrl.username !== EXPECTED_ROLE_NAME) {
    throw new Error(`Neon API database role must equal ${EXPECTED_ROLE_NAME}`);
  }
  if (databaseUrl.pathname !== `/${EXPECTED_DATABASE_NAME}`) {
    throw new Error(`Neon API database name must equal ${EXPECTED_DATABASE_NAME}`);
  }
  if (databaseUrl.searchParams.get('sslmode') !== 'require') {
    throw new Error('Neon API database URI must require TLS');
  }
  if (!databaseUrl.password) throw new Error('Neon API database URI is missing a credential');

  return databaseUrl;
}

export async function provisionNeonTestBranch({
  environment = process.env,
  fetchImpl = globalThis.fetch,
  appendOutput,
  emitWorkflowCommand = (line) => process.stdout.write(`${line}\n`),
} = {}) {
  const { apiKey, projectId, branchName, expiresAt, outputPath } =
    validateConfiguration(environment);
  const writeOutput =
    appendOutput ?? ((line) => appendFileSync(outputPath, `${line}\n`, { encoding: 'utf8' }));

  await requireDedicatedProject(fetchImpl, apiKey, projectId);

  writeOutput(`branch_name=${branchName}`);
  writeOutput(`expires_at=${expiresAt}`);

  const branchResponse = await requestJson(fetchImpl, apiKey, `/projects/${projectId}/branches`, {
    method: 'POST',
    body: JSON.stringify({
      branch: { name: branchName, expires_at: expiresAt },
      endpoints: [{ type: 'read_write' }],
    }),
  });
  const branchId = validateOutputValue(branchResponse?.branch?.id, 'branch ID');
  writeOutput(`branch_id=${branchId}`);
  if (branchResponse?.branch?.name !== branchName) {
    throw new Error('Neon API created a branch with an unexpected name');
  }

  const connectionParameters = new URLSearchParams({
    branch_id: branchId,
    database_name: EXPECTED_DATABASE_NAME,
    role_name: EXPECTED_ROLE_NAME,
    pooled: 'false',
  });
  const connectionResponse = await requestJson(
    fetchImpl,
    apiKey,
    `/projects/${projectId}/connection_uri?${connectionParameters}`
  );
  const rawDatabaseUrl = validateOutputValue(connectionResponse?.uri, 'database URI');
  const databaseUrl = requireSafeDatabaseUrl(rawDatabaseUrl);

  const encodedPassword = databaseUrl.password;
  const decodedPassword = decodeURIComponent(encodedPassword);
  for (const value of new Set([rawDatabaseUrl, encodedPassword, decodedPassword])) {
    emitWorkflowCommand(`::add-mask::${escapeWorkflowCommandData(value)}`);
  }
  writeOutput(`db_url=${rawDatabaseUrl}`);

  return { branchId, branchName, expiresAt };
}

export async function cleanupNeonTestBranch({
  environment = process.env,
  fetchImpl = globalThis.fetch,
} = {}) {
  const { apiKey, projectId, branchName } = validateConfiguration(environment);
  await requireDedicatedProject(fetchImpl, apiKey, projectId);

  let branchId = environment['NEON_BRANCH_ID'];
  if (branchId && !RESOURCE_ID_PATTERN.test(branchId)) {
    throw new Error('NEON_BRANCH_ID is invalid');
  }
  if (!branchId) {
    const searchParameters = new URLSearchParams({ search: branchName, limit: '10' });
    const branchList = await requestJson(
      fetchImpl,
      apiKey,
      `/projects/${projectId}/branches?${searchParameters}`
    );
    const exactMatches = (branchList?.branches ?? []).filter(
      (branch) => branch?.name === branchName
    );
    if (exactMatches.length === 0) return { branchId: null, deleted: false };
    if (exactMatches.length !== 1) {
      throw new Error('Neon API returned multiple branches with the exact CI test name');
    }
    branchId = validateOutputValue(exactMatches[0]?.id, 'branch ID');
  }

  await requestApi(fetchImpl, apiKey, `/projects/${projectId}/branches/${branchId}`, {
    method: 'DELETE',
  });
  return { branchId, deleted: true };
}

if (process.argv[1] && fileURLToPath(import.meta.url) === process.argv[1]) {
  const operation =
    process.argv[2] === '--delete' ? cleanupNeonTestBranch : provisionNeonTestBranch;
  operation().catch((error) => {
    const message = error instanceof Error ? error.message : 'unknown error';
    process.stderr.write(`Neon test branch provisioning failed: ${message}\n`);
    process.exitCode = 1;
  });
}
