import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { chmodSync, mkdtempSync, writeFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import test from 'node:test';

const SCRIPT = resolve(import.meta.dirname, 'pr-required-status.mjs');

function run(required, checks, threads, merge) {
  const dir = mkdtempSync(join(tmpdir(), 'pr-req-'));
  const requiredPath = join(dir, 'required.json');
  const checksPath = join(dir, 'checks.json');
  writeFileSync(requiredPath, JSON.stringify(required));
  writeFileSync(checksPath, JSON.stringify(checks));
  const argv = [SCRIPT, '--required-json', requiredPath, '--checks-json', checksPath];
  if (threads !== undefined) {
    const threadsPath = join(dir, 'threads.json');
    writeFileSync(threadsPath, JSON.stringify(threads));
    argv.push('--threads-json', threadsPath);
  }
  if (merge !== undefined) {
    const mergePath = join(dir, 'merge.json');
    writeFileSync(mergePath, JSON.stringify(merge));
    argv.push('--merge-json', mergePath);
  }
  try {
    return spawnSync(process.execPath, argv, { encoding: 'utf8' });
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
}

const GREEN = [
  { name: 'Docs Lint', status: 'COMPLETED', conclusion: 'SUCCESS' },
  { name: 'Lint & Format', status: 'COMPLETED', conclusion: 'SUCCESS' },
];
const REQUIRED = ['Docs Lint', 'Lint & Format'];

test('pending required context is not finished, not a pass', () => {
  const r = run(
    ['Docs Lint', 'Lint & Format'],
    [{ name: 'Lint & Format', status: 'COMPLETED', conclusion: 'SUCCESS' }]
  );
  assert.equal(r.status, 2, r.stdout + r.stderr);
  assert.match(r.stdout, /not finished/i);
  assert.match(r.stdout, /Docs Lint/);
  assert.doesNotMatch(r.stdout, /all required contexts reported and passed/i);
});

test('fully reported green required set is success', () => {
  const r = run(
    ['Docs Lint', 'Lint & Format'],
    [
      { name: 'Docs Lint', status: 'COMPLETED', conclusion: 'SUCCESS' },
      { name: 'Lint & Format', status: 'COMPLETED', conclusion: 'SUCCESS' },
    ]
  );
  assert.equal(r.status, 0, r.stdout + r.stderr);
  assert.match(r.stdout, /all required contexts reported and passed/);
});

test('reported failure is a failure, not a pending sample', () => {
  const r = run(['Docs Lint'], [{ name: 'Docs Lint', status: 'COMPLETED', conclusion: 'FAILURE' }]);
  assert.equal(r.status, 1, r.stdout + r.stderr);
  assert.match(r.stdout, /failed: Docs Lint/);
  assert.doesNotMatch(r.stdout, /not finished/i);
});

test('in-progress required context is not finished', () => {
  const r = run(['Docs Lint'], [{ name: 'Docs Lint', status: 'IN_PROGRESS', conclusion: null }]);
  assert.equal(r.status, 2, r.stdout + r.stderr);
  assert.match(r.stdout, /not finished/i);
  assert.match(r.stdout, /Docs Lint/);
});

// A branch whose ruleset requires thread resolution is not mergeable on green
// checks alone. Observed on PR #4341: every required context reported and
// passed while GitHub held the PR at `BLOCKED` on one unresolved thread, so a
// checks-only reading called a still-blocked PR a pass.
test('unresolved review thread blocks even when every required check is green', () => {
  const r = run(REQUIRED, GREEN, [{ isResolved: false }, { isResolved: true }]);
  assert.equal(r.status, 3, r.stdout + r.stderr);
  assert.match(r.stdout, /blocked: 1 unresolved review thread\b/);
  assert.doesNotMatch(r.stdout, /all required contexts reported and passed/);
});

test('resolved threads with green checks is a pass, and says so', () => {
  const r = run(REQUIRED, GREEN, [{ isResolved: true }, { isResolved: true }]);
  assert.equal(r.status, 0, r.stdout + r.stderr);
  assert.match(r.stdout, /all required contexts reported and passed/);
  assert.match(r.stdout, /review threads resolved/);
});

test('thread state absent means the gate does not apply, not that it passed', () => {
  const r = run(REQUIRED, GREEN);
  assert.equal(r.status, 0, r.stdout + r.stderr);
  assert.match(r.stdout, /all required contexts reported and passed/);
  assert.doesNotMatch(r.stdout, /review threads resolved/);
});

// Pending checks outrank an open thread: the honest report is "not finished",
// because the checks have not reached a verdict at all yet.
test('pending checks still report not-finished even with an open thread', () => {
  const r = run(
    REQUIRED,
    [{ name: 'Lint & Format', status: 'COMPLETED', conclusion: 'SUCCESS' }],
    [{ isResolved: false }]
  );
  assert.equal(r.status, 2, r.stdout + r.stderr);
  assert.match(r.stdout, /not finished/i);
});

// --- CIB-404: a PR with no merge candidate ------------------------------------
// When a PR conflicts with its base, GitHub builds no merge commit, so no
// workflow runs and no required context can ever report. Reporting that as
// "not finished" tells a polling caller to wait when the truth is "repair the
// base" — observed on PR #4372, where the poller burned its full 52-minute
// budget on a branch that needed a thirty-second rebase.
const NOTHING_REPORTED = [];

test('a conflicting PR is its own exit code, not "not finished"', () => {
  const r = run(REQUIRED, NOTHING_REPORTED, undefined, {
    mergeable: 'CONFLICTING',
    mergeStateStatus: 'DIRTY',
  });
  assert.equal(r.status, 4, r.stdout + r.stderr);
  assert.match(r.stdout, /conflict/i);
  assert.doesNotMatch(r.stdout, /not finished/i);
  assert.doesNotMatch(r.stdout, /all required contexts reported and passed/i);
});

test('the conflict report names the base repair, not the missing contexts', () => {
  const r = run(REQUIRED, NOTHING_REPORTED, undefined, {
    mergeable: 'CONFLICTING',
    mergeStateStatus: 'DIRTY',
  });
  assert.equal(r.status, 4, r.stdout + r.stderr);
  assert.match(r.stdout, /mergeable: CONFLICTING/);
  assert.match(r.stdout, /mergeStateStatus: DIRTY/);
  assert.match(r.stdout, /rebase|repair|update the branch/i);
});

test('mergeStateStatus DIRTY alone is a conflict even if mergeable lags', () => {
  const r = run(REQUIRED, NOTHING_REPORTED, undefined, {
    mergeable: 'UNKNOWN',
    mergeStateStatus: 'DIRTY',
  });
  assert.equal(r.status, 4, r.stdout + r.stderr);
  assert.match(r.stdout, /conflict/i);
});

// The other arm: the tool must not start calling healthy, merely-early PRs
// conflicted. A mergeable PR whose checks genuinely have not started is still
// exit 2 and still names what is missing.
test('a healthy PR whose checks have not started is still not finished', () => {
  const r = run(REQUIRED, NOTHING_REPORTED, undefined, {
    mergeable: 'MERGEABLE',
    mergeStateStatus: 'CLEAN',
  });
  assert.equal(r.status, 2, r.stdout + r.stderr);
  assert.match(r.stdout, /not finished/i);
  assert.match(r.stdout, /Docs Lint/);
  assert.match(r.stdout, /Lint & Format/);
  assert.doesNotMatch(r.stdout, /conflict/i);
});

test('a mergeable PR with green checks still passes', () => {
  const r = run(REQUIRED, GREEN, undefined, {
    mergeable: 'MERGEABLE',
    mergeStateStatus: 'BLOCKED',
  });
  assert.equal(r.status, 0, r.stdout + r.stderr);
  assert.match(r.stdout, /all required contexts reported and passed/);
});

// GitHub computes mergeability asynchronously, so UNKNOWN is itself a
// not-yet-a-verdict: it is neither a conflict nor evidence that the checks are
// merely early. It gets its own message so a caller polling on exit 2 can tell
// the two apart, and is never reported as a conflict.
test('mergeable UNKNOWN is its own case, not a conflict', () => {
  const r = run(REQUIRED, NOTHING_REPORTED, undefined, {
    mergeable: 'UNKNOWN',
    mergeStateStatus: 'UNKNOWN',
  });
  assert.equal(r.status, 2, r.stdout + r.stderr);
  assert.match(r.stdout, /mergeability (is )?(still )?(not|un)/i);
  assert.match(r.stdout, /UNKNOWN/);
  assert.doesNotMatch(r.stdout, /conflict/i);
  assert.doesNotMatch(r.stdout, /all required contexts reported and passed/i);
});

test('mergeable UNKNOWN is distinguishable from plain not-started checks', () => {
  const unknown = run(REQUIRED, NOTHING_REPORTED, undefined, {
    mergeable: 'UNKNOWN',
    mergeStateStatus: 'UNKNOWN',
  });
  const early = run(REQUIRED, NOTHING_REPORTED, undefined, {
    mergeable: 'MERGEABLE',
    mergeStateStatus: 'CLEAN',
  });
  assert.equal(unknown.status, 2, unknown.stdout + unknown.stderr);
  assert.equal(early.status, 2, early.stdout + early.stderr);
  assert.notEqual(unknown.stdout, early.stdout);
});

// Absent mergeability data must not invent a verdict: every pre-CIB-404 caller
// passes no merge state at all and must keep its old behaviour.
test('absent merge state leaves the existing classification untouched', () => {
  const r = run(REQUIRED, NOTHING_REPORTED);
  assert.equal(r.status, 2, r.stdout + r.stderr);
  assert.match(r.stdout, /not finished/i);
  assert.doesNotMatch(r.stdout, /conflict/i);
  assert.doesNotMatch(r.stdout, /mergeab/i);
});

// --- CIB-404 repair: the conflict guard must outrank the required-set exit ----
// `required.length === 0` exits NOT_FINISHED before any classification runs, so
// a conflict guard placed after it leaves the original misreport alive on a
// branch that configures no required contexts — a reachable case, and one where
// a poller waits its full budget exactly as it did on #4372.
test('a conflicting PR is exit 4 even with an empty required set', () => {
  const r = run([], NOTHING_REPORTED, undefined, {
    mergeable: 'CONFLICTING',
    mergeStateStatus: 'DIRTY',
  });
  assert.equal(r.status, 4, r.stdout + r.stderr);
  assert.match(r.stdout, /conflict/i);
  assert.doesNotMatch(r.stdout + r.stderr, /no required contexts found/);
});

// The early exit must survive the reorder: a mergeable PR on a branch with no
// required contexts is still "the question cannot be answered", not a pass.
test('an empty required set on a mergeable PR is still exit 2 and says so', () => {
  const r = run([], NOTHING_REPORTED, undefined, {
    mergeable: 'MERGEABLE',
    mergeStateStatus: 'CLEAN',
  });
  assert.equal(r.status, 2, r.stdout + r.stderr);
  assert.match(r.stderr, /no required contexts found/);
});

// Ordering call for UNKNOWN, the other side of the same decision: both paths
// exit 2, so the caller should get the message that stays true. "No required
// contexts found" is a configuration fact; UNKNOWN is transient.
test('an empty required set outranks the transient UNKNOWN message', () => {
  const r = run([], NOTHING_REPORTED, undefined, {
    mergeable: 'UNKNOWN',
    mergeStateStatus: 'UNKNOWN',
  });
  assert.equal(r.status, 2, r.stdout + r.stderr);
  assert.match(r.stderr, /no required contexts found/);
  assert.doesNotMatch(r.stdout, /mergeability unresolved/);
});

// --- CIB-430: paginate live discovery -----------------------------------------
// Fixture injection never talks to GraphQL or the ruleset list, so a later
// unresolved thread or required context on page 2 was invisible. Live mode
// must follow every page; an incomplete page is fail-closed, not a pass.

const FAKE_GH = `#!/usr/bin/env node
'use strict';
const fs = require('fs');
const fixture = JSON.parse(fs.readFileSync(process.env.PR_REQUIRED_STATUS_FIXTURE, 'utf8'));
const args = process.argv.slice(2);

function valueOf(flag) {
  for (let i = 0; i < args.length; i += 1) {
    if ((args[i] === '-F' || args[i] === '-f') && args[i + 1] && args[i + 1].startsWith(flag + '=')) {
      return args[i + 1].slice(flag.length + 1);
    }
  }
  return undefined;
}

function reply(body) {
  process.stdout.write(JSON.stringify(body));
}

if (args[0] === 'pr' && args[1] === 'view') {
  reply(fixture.prView);
  process.exit(0);
}

if (args.includes('graphql')) {
  const after = valueOf('after') || '';
  const pages = fixture.threadPages || {};
  if (!Object.prototype.hasOwnProperty.call(pages, after)) {
    process.stderr.write('fake-gh: unexpected reviewThreads cursor ' + JSON.stringify(after) + '\\n');
    process.exit(1);
  }
  reply({
    data: {
      repository: {
        pullRequest: {
          reviewThreads: pages[after],
        },
      },
    },
  });
  process.exit(0);
}

const pathArg = args.find((a) => typeof a === 'string' && a.startsWith('repos/'));
if (pathArg) {
  const pathOnly = pathArg.split('?')[0];
  const detail = pathOnly.match(/\\/rulesets\\/(\\d+)$/);
  if (detail) {
    const id = detail[1];
    const body = (fixture.rulesetDetails || {})[id];
    if (!body) {
      process.stderr.write('fake-gh: unknown ruleset ' + id + '\\n');
      process.exit(1);
    }
    reply(body);
    process.exit(0);
  }
  if (/\\/rulesets$/.test(pathOnly)) {
    const pages = fixture.rulesetPages || [];
    // Real gh: --paginate alone emits one JSON document per page; --paginate
    // --slurp returns one JSON array of those page documents.
    if (args.includes('--paginate') && args.includes('--slurp')) {
      reply(pages);
      process.exit(0);
    }
    if (args.includes('--paginate')) {
      for (const page of pages) {
        process.stdout.write(JSON.stringify(page) + '\\n');
      }
      process.exit(0);
    }
    reply(pages[0] || []);
    process.exit(0);
  }
}

process.stderr.write('fake-gh: unhandled ' + args.join(' ') + '\\n');
process.exit(1);
`;

function livePrView(checks = GREEN) {
  return {
    number: 42,
    baseRefName: 'main',
    url: 'https://github.com/eddacraft/anvil-001/pull/42',
    mergeable: 'MERGEABLE',
    mergeStateStatus: 'BLOCKED',
    statusCheckRollup: checks,
  };
}

function liveRuleset({ id, contexts = [], threadResolution = false }) {
  const rules = [];
  if (contexts.length > 0) {
    rules.push({
      type: 'required_status_checks',
      parameters: {
        required_status_checks: contexts.map((context) => ({ context })),
      },
    });
  }
  if (threadResolution) {
    rules.push({
      type: 'pull_request',
      parameters: { required_review_thread_resolution: true },
    });
  }
  return {
    id,
    enforcement: 'active',
    conditions: { ref_name: { include: ['~DEFAULT_BRANCH'] } },
    rules,
  };
}

function runLive(fixture) {
  const dir = mkdtempSync(join(tmpdir(), 'pr-req-live-'));
  const fixturePath = join(dir, 'fixture.json');
  const ghPath = join(dir, 'fake-gh');
  writeFileSync(fixturePath, JSON.stringify(fixture));
  writeFileSync(ghPath, FAKE_GH, { mode: 0o755 });
  chmodSync(ghPath, 0o755);
  try {
    return spawnSync(
      process.execPath,
      [SCRIPT, '--repo', 'eddacraft/anvil-001', '--pr', '42', '--gh', ghPath],
      {
        encoding: 'utf8',
        env: { ...process.env, PR_REQUIRED_STATUS_FIXTURE: fixturePath },
      }
    );
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
}

test('live mode counts an unresolved thread from the second GraphQL page', () => {
  const firstPage = liveRuleset({
    id: 1,
    contexts: REQUIRED,
    threadResolution: true,
  });
  const r = runLive({
    prView: livePrView(),
    rulesetPages: [[{ id: 1 }]],
    rulesetDetails: { 1: firstPage },
    threadPages: {
      '': {
        pageInfo: { hasNextPage: true, endCursor: 'thread-page-2' },
        nodes: Array.from({ length: 100 }, () => ({ isResolved: true })),
      },
      'thread-page-2': {
        pageInfo: { hasNextPage: false, endCursor: null },
        nodes: [{ isResolved: false }],
      },
    },
  });
  assert.equal(r.status, 3, r.stdout + r.stderr);
  assert.match(r.stdout, /blocked: 1 unresolved review thread\b/);
  assert.doesNotMatch(r.stdout, /all required contexts reported and passed/);
});

test('live mode treats a required context from a later ruleset page as not finished', () => {
  const pageOne = liveRuleset({ id: 1, contexts: REQUIRED });
  const pageTwo = liveRuleset({ id: 2, contexts: ['Secret Scan'] });
  const r = runLive({
    prView: livePrView(),
    rulesetPages: [[{ id: 1 }], [{ id: 2 }]],
    rulesetDetails: { 1: pageOne, 2: pageTwo },
    threadPages: {
      '': {
        pageInfo: { hasNextPage: false, endCursor: null },
        nodes: [],
      },
    },
  });
  assert.equal(r.status, 2, r.stdout + r.stderr);
  assert.match(r.stdout, /not finished/i);
  assert.match(r.stdout, /Secret Scan/);
  assert.doesNotMatch(r.stdout, /all required contexts reported and passed/);
});

test('live mode fail-closes on an incomplete review-thread page, not a pass', () => {
  const firstPage = liveRuleset({
    id: 1,
    contexts: REQUIRED,
    threadResolution: true,
  });
  const r = runLive({
    prView: livePrView(),
    rulesetPages: [[{ id: 1 }]],
    rulesetDetails: { 1: firstPage },
    threadPages: {
      '': {
        pageInfo: { hasNextPage: true, endCursor: null },
        nodes: Array.from({ length: 100 }, () => ({ isResolved: true })),
      },
    },
  });
  assert.equal(r.status, 2, r.stdout + r.stderr);
  assert.doesNotMatch(r.stdout, /all required contexts reported and passed/);
});
