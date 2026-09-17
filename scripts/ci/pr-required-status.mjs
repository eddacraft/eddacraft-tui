#!/usr/bin/env node
import { spawnSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import process from 'node:process';

const NOT_FINISHED = 2;
const FAILED = 1;
// A merge gate that is not a status check: the ruleset can also require every
// review thread to be resolved. Distinct from FAILED so a caller can tell
// "a check went red" from "a human conversation is still open".
const UNRESOLVED_THREADS = 3;
// The PR has no merge candidate at all: GitHub cannot build the merge commit,
// so no workflow runs and no required context can ever report. Distinct from
// NOT_FINISHED because the two demand opposite actions — NOT_FINISHED says
// wait, this says repair the base (CIB-404).
const MERGE_CONFLICT = 4;

function usage() {
  process.stderr.write(
    'Usage: scripts/ci/pr-required-status.mjs [--repo owner/repo] [--pr N] [--base BRANCH] [--required-json FILE] [--checks-json FILE] [--threads-json FILE] [--merge-json FILE] [--gh PATH]\n'
  );
}

function parseArgs(argv) {
  const out = {
    repo: null,
    pr: null,
    base: null,
    requiredJson: null,
    checksJson: null,
    threadsJson: null,
    mergeJson: null,
    gh: process.env.GH_BIN || 'gh',
  };
  for (let i = 0; i < argv.length; i += 1) {
    const arg = argv[i];
    if (arg === '--help' || arg === '-h') {
      usage();
      process.exit(0);
    }
    const keys = {
      '--repo': 'repo',
      '--pr': 'pr',
      '--base': 'base',
      '--required-json': 'requiredJson',
      '--checks-json': 'checksJson',
      '--threads-json': 'threadsJson',
      '--merge-json': 'mergeJson',
      '--gh': 'gh',
    };
    if (keys[arg]) {
      const value = argv[i + 1];
      if (value === undefined || value.startsWith('--')) {
        process.stderr.write(`pr-required-status.mjs: ${arg} requires a value\n`);
        process.exit(NOT_FINISHED);
      }
      out[keys[arg]] = value;
      i += 1;
      continue;
    }
    process.stderr.write(`pr-required-status.mjs: unknown argument: ${arg}\n`);
    process.exit(NOT_FINISHED);
  }
  return out;
}

function ghJson(gh, args) {
  const result = spawnSync(gh, args, { encoding: 'utf8' });
  if (result.status !== 0) {
    process.stderr.write(result.stderr || `gh ${args.join(' ')} failed\n`);
    process.exit(NOT_FINISHED);
  }
  try {
    return JSON.parse(result.stdout);
  } catch {
    process.stderr.write(
      `[pr-required-status] incomplete API page: invalid JSON from gh ${args.join(' ')}\n`
    );
    process.exit(NOT_FINISHED);
  }
}

/// Discovery that cannot finish must not look like a pass. Incomplete GraphQL
/// or ruleset pages are the same class of lie as a missing check: wait/retry,
/// never green.
function failClosed(message) {
  process.stderr.write(`[pr-required-status] ${message}\n`);
  process.exit(NOT_FINISHED);
}

function loadJsonFile(path) {
  return JSON.parse(readFileSync(path, 'utf8'));
}

/// The active rulesets whose ref conditions cover `baseBranch`. Shared by the
/// status-check and thread-resolution readers so the two cannot disagree about
/// which rulesets apply.
function rulesetsForBranch(rulesets, baseBranch) {
  const ref = `refs/heads/${baseBranch}`;
  return rulesets.filter((ruleset) => {
    if (ruleset.enforcement && ruleset.enforcement !== 'active') return false;
    const include = ruleset.conditions?.ref_name?.include ?? [];
    const exclude = ruleset.conditions?.ref_name?.exclude ?? [];
    const applies =
      include.length === 0 ||
      include.includes('~DEFAULT_BRANCH') ||
      include.includes('~ALL') ||
      include.includes(ref) ||
      include.includes(baseBranch);
    if (!applies) return false;
    return !(exclude.includes(ref) || exclude.includes(baseBranch));
  });
}

function requiredFromRulesets(rulesets, baseBranch) {
  const wanted = new Set();
  for (const ruleset of rulesetsForBranch(rulesets, baseBranch)) {
    for (const rule of ruleset.rules ?? []) {
      if (rule.type !== 'required_status_checks') continue;
      for (const check of rule.parameters?.required_status_checks ?? []) {
        if (check.context) wanted.add(check.context);
      }
    }
  }
  return [...wanted].sort();
}

/// True when an active ruleset for `baseBranch` requires every review thread to
/// be resolved before merge. Green checks are not sufficient on such a branch:
/// GitHub reports `mergeStateStatus: BLOCKED` while a thread is open, so a tool
/// that only reads status checks would call a still-blocked PR a pass.
function threadResolutionRequired(rulesets, baseBranch) {
  for (const ruleset of rulesetsForBranch(rulesets, baseBranch)) {
    for (const rule of ruleset.rules ?? []) {
      if (rule.type !== 'pull_request') continue;
      if (rule.parameters?.required_review_thread_resolution) return true;
    }
  }
  return false;
}

function normalizeChecks(raw) {
  if (Array.isArray(raw)) {
    return raw.map((item) => ({
      name: item.name || item.context || '',
      status: item.status || (item.state === 'PENDING' ? 'IN_PROGRESS' : 'COMPLETED'),
      conclusion: item.conclusion || item.state || null,
    }));
  }
  const rollup = raw.statusCheckRollup || raw.checkRollup || [];
  return rollup.map((item) => ({
    name: item.name || item.context || '',
    status: item.status || 'COMPLETED',
    conclusion: item.conclusion || item.state || null,
  }));
}

function isPassing(check) {
  if (!check || check.status !== 'COMPLETED') return false;
  const conclusion = (check.conclusion || '').toUpperCase();
  return conclusion === 'SUCCESS' || conclusion === 'NEUTRAL' || conclusion === 'SKIPPED';
}

function isFailed(check) {
  if (!check || check.status !== 'COMPLETED') return false;
  return !isPassing(check);
}

/// Count unresolved review threads via GraphQL. REST cannot answer this — it
/// does not carry resolution state — which is why the ruleset's thread gate is
/// invisible to `gh pr checks`. Follow `pageInfo` until `hasNextPage` is false;
/// a truncated page is fail-closed (CIB-430), never a pass on the first 100.
const MAX_THREAD_PAGES = 100;

function unresolvedThreadCount(gh, repo, prNumber) {
  const [owner, name] = repo.split('/');
  const query = `query($owner:String!,$name:String!,$pr:Int!,$after:String){
    repository(owner:$owner,name:$name){
      pullRequest(number:$pr){
        reviewThreads(first:100, after:$after){
          pageInfo { hasNextPage endCursor }
          nodes { isResolved }
        }
      }
    }
  }`;
  let after = null;
  let unresolved = 0;
  const seenCursors = new Set();
  for (let page = 0; page < MAX_THREAD_PAGES; page += 1) {
    const args = [
      'api',
      'graphql',
      '-f',
      `query=${query}`,
      '-F',
      `owner=${owner}`,
      '-F',
      `name=${name}`,
      '-F',
      `pr=${prNumber}`,
    ];
    if (after) args.push('-f', `after=${after}`);
    const data = ghJson(gh, args);
    if (Array.isArray(data?.errors) && data.errors.length > 0) {
      failClosed('incomplete review-thread page: GraphQL errors');
    }
    const threads = data?.data?.repository?.pullRequest?.reviewThreads;
    const pageInfo = threads?.pageInfo;
    const nodes = threads?.nodes;
    if (
      !threads ||
      !pageInfo ||
      typeof pageInfo.hasNextPage !== 'boolean' ||
      !Array.isArray(nodes)
    ) {
      failClosed('incomplete review-thread page: missing pageInfo or nodes');
    }
    unresolved += nodes.filter((thread) => !thread.isResolved).length;
    if (!pageInfo.hasNextPage) return unresolved;
    if (!pageInfo.endCursor) {
      failClosed('incomplete review-thread page: hasNextPage without endCursor');
    }
    if (seenCursors.has(pageInfo.endCursor)) {
      failClosed('incomplete review-thread page: repeated cursor');
    }
    seenCursors.add(pageInfo.endCursor);
    after = pageInfo.endCursor;
  }
  failClosed('incomplete review-thread page: exceeded page limit');
}

function loadLiveRulesets(gh, repo) {
  // --paginate alone emits one JSON document per page; JSON.parse then fails
  // once a second page exists. --slurp wraps pages in one array; flatten so
  // detail lookups see every id (CIB-430 Copilot review).
  const pages = ghJson(gh, ['api', '--paginate', '--slurp', `repos/${repo}/rulesets`]);
  if (!Array.isArray(pages)) {
    failClosed('incomplete ruleset page: slurped list is not an array');
  }
  const rulesetList = pages.every((page) => Array.isArray(page)) ? pages.flat() : pages;
  if (!Array.isArray(rulesetList)) {
    failClosed('incomplete ruleset page: list is not an array');
  }
  return rulesetList.map((entry) => {
    if (entry == null || entry.id == null) {
      failClosed('incomplete ruleset page: missing id');
    }
    return ghJson(gh, ['api', `repos/${repo}/rulesets/${entry.id}`]);
  });
}

/// Classify the PR's merge candidate from `gh pr view`'s `mergeable` /
/// `mergeStateStatus`. Returns `null` when no merge state was supplied at all,
/// which is "the question was not asked" — never a verdict.
///
/// Three outcomes, deliberately not two:
///   - `conflicting`: `mergeable: CONFLICTING` or `mergeStateStatus: DIRTY`.
///     Nothing is pending because nothing can start.
///   - `unknown`: GitHub computes mergeability asynchronously, so `UNKNOWN` is
///     itself a not-yet-a-verdict. It is not a conflict (calling it one sends a
///     caller to rebase a healthy branch) and it is not ordinary
///     checks-not-started either, so it gets its own message.
///   - neither: the PR has a merge candidate; classify the checks as before.
function classifyMergeability(raw) {
  if (raw === null || raw === undefined) return null;
  const mergeable = String(raw.mergeable ?? '').toUpperCase();
  const stateStatus = String(raw.mergeStateStatus ?? '').toUpperCase();
  if (mergeable === 'CONFLICTING' || stateStatus === 'DIRTY') {
    return { conflicting: true, unknown: false, mergeable, stateStatus };
  }
  if (mergeable === 'UNKNOWN' || mergeable === '') {
    return { conflicting: false, unknown: true, mergeable: mergeable || 'UNKNOWN', stateStatus };
  }
  return { conflicting: false, unknown: false, mergeable, stateStatus };
}

function main() {
  const args = parseArgs(process.argv.slice(2));
  let required;
  let checks;
  // `null` means "thread resolution is not required on this base", which is
  // different from "required and zero unresolved". Only the latter is a pass.
  let unresolved = null;
  // `null` means "no merge state was supplied", which is how every pre-CIB-404
  // caller behaves. Absent data must not manufacture a verdict either way.
  let mergeState = null;

  if (args.requiredJson && args.checksJson) {
    required = loadJsonFile(args.requiredJson);
    checks = normalizeChecks(loadJsonFile(args.checksJson));
    if (args.threadsJson) {
      const threads = loadJsonFile(args.threadsJson);
      unresolved = Array.isArray(threads)
        ? threads.filter((thread) => !thread.isResolved).length
        : Number(threads.unresolved ?? 0);
    }
    if (args.mergeJson) {
      mergeState = classifyMergeability(loadJsonFile(args.mergeJson));
    }
  } else {
    const prView = ghJson(args.gh, [
      'pr',
      'view',
      ...(args.pr ? [args.pr] : []),
      '--json',
      'number,baseRefName,url,statusCheckRollup,mergeable,mergeStateStatus',
      ...(args.repo ? ['--repo', args.repo] : []),
    ]);
    const repo =
      args.repo || ghJson(args.gh, ['repo', 'view', '--json', 'nameWithOwner']).nameWithOwner;
    const base = args.base || prView.baseRefName;
    const rulesets = loadLiveRulesets(args.gh, repo);
    required = requiredFromRulesets(rulesets, base);
    checks = normalizeChecks(prView);
    mergeState = classifyMergeability({
      mergeable: prView.mergeable,
      mergeStateStatus: prView.mergeStateStatus,
    });
    if (threadResolutionRequired(rulesets, base)) {
      unresolved = unresolvedThreadCount(args.gh, repo, prView.number);
    }
  }

  // Answered first, ahead of even the required-set check: whether the PR has a
  // merge candidate is a fact about the PR, not about how many contexts the
  // branch happens to configure. A conflicting PR on a branch with zero
  // required contexts is still a conflict, and reporting it as NOT_FINISHED —
  // by any route — is exactly the lie CIB-404 exists to remove. This reports;
  // it never rebases, merges, or otherwise repairs the branch.
  if (mergeState?.conflicting) {
    process.stdout.write(
      `[pr-required-status] conflict: the PR has no merge candidate (mergeable: ${mergeState.mergeable}, mergeStateStatus: ${mergeState.stateStatus || 'unknown'}); GitHub builds no merge commit, so required contexts cannot report — rebase or update the branch on its base, then re-run\n`
    );
    process.exit(MERGE_CONFLICT);
  }

  if (!Array.isArray(required) || required.length === 0) {
    process.stderr.write('[pr-required-status] no required contexts found\n');
    process.exit(NOT_FINISHED);
  }

  // Deliberately *after* the required-set check, unlike the conflict guard.
  // Both exit NOT_FINISHED, so this is a choice of message, not of verdict, and
  // the caller should get the one that stays true: "no required contexts found"
  // is a configuration fact that no amount of waiting changes, while UNKNOWN is
  // GitHub's transient "ask again in a moment". Reporting UNKNOWN first on such
  // a branch would send a caller off to poll a question that can never resolve
  // into a useful answer.
  if (mergeState?.unknown) {
    process.stdout.write(
      `[pr-required-status] mergeability unresolved: GitHub has not finished computing it (mergeable: ${mergeState.mergeable}); the merge candidate is not yet decided either way, and no check verdict is implied — re-run once it settles\n`
    );
    process.exit(NOT_FINISHED);
  }

  const byName = new Map();
  for (const check of checks) {
    if (check.name) byName.set(check.name, check);
  }

  const pending = [];
  const failed = [];
  for (const name of required) {
    const check = byName.get(name);
    if (!check || check.status !== 'COMPLETED') {
      pending.push(name);
      continue;
    }
    if (isFailed(check)) failed.push(name);
  }

  if (pending.length > 0) {
    process.stdout.write(
      `[pr-required-status] not finished: ${pending.join(', ')} ${pending.length === 1 ? 'has' : 'have'} not reported\n`
    );
    process.exit(NOT_FINISHED);
  }
  if (failed.length > 0) {
    process.stdout.write(`[pr-required-status] failed: ${failed.join(', ')}\n`);
    process.exit(FAILED);
  }
  if (unresolved !== null && unresolved > 0) {
    process.stdout.write(
      `[pr-required-status] blocked: ${unresolved} unresolved review ${unresolved === 1 ? 'thread' : 'threads'}; required contexts are green but the branch requires thread resolution\n`
    );
    process.exit(UNRESOLVED_THREADS);
  }
  const threadNote = unresolved === null ? '' : '; review threads resolved';
  process.stdout.write(
    `[pr-required-status] all required contexts reported and passed (${required.length})${threadNote}\n`
  );
  process.exit(0);
}

main();
