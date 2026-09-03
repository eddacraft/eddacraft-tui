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

function usage() {
  process.stderr.write(
    'Usage: scripts/ci/pr-required-status.mjs [--repo owner/repo] [--pr N] [--base BRANCH] [--required-json FILE] [--checks-json FILE] [--threads-json FILE] [--gh PATH]\n'
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
  return JSON.parse(result.stdout);
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
/// invisible to `gh pr checks`.
function unresolvedThreadCount(gh, repo, prNumber) {
  const [owner, name] = repo.split('/');
  const query = `query($owner:String!,$name:String!,$pr:Int!){
    repository(owner:$owner,name:$name){
      pullRequest(number:$pr){
        reviewThreads(first:100){ nodes { isResolved } }
      }
    }
  }`;
  const data = ghJson(gh, [
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
  ]);
  const nodes = data?.data?.repository?.pullRequest?.reviewThreads?.nodes ?? [];
  return nodes.filter((thread) => !thread.isResolved).length;
}

function main() {
  const args = parseArgs(process.argv.slice(2));
  let required;
  let checks;
  // `null` means "thread resolution is not required on this base", which is
  // different from "required and zero unresolved". Only the latter is a pass.
  let unresolved = null;

  if (args.requiredJson && args.checksJson) {
    required = loadJsonFile(args.requiredJson);
    checks = normalizeChecks(loadJsonFile(args.checksJson));
    if (args.threadsJson) {
      const threads = loadJsonFile(args.threadsJson);
      unresolved = Array.isArray(threads)
        ? threads.filter((thread) => !thread.isResolved).length
        : Number(threads.unresolved ?? 0);
    }
  } else {
    const prView = ghJson(args.gh, [
      'pr',
      'view',
      ...(args.pr ? [args.pr] : []),
      '--json',
      'number,baseRefName,url,statusCheckRollup',
      ...(args.repo ? ['--repo', args.repo] : []),
    ]);
    const repo =
      args.repo || ghJson(args.gh, ['repo', 'view', '--json', 'nameWithOwner']).nameWithOwner;
    const base = args.base || prView.baseRefName;
    const rulesetList = ghJson(args.gh, ['api', `repos/${repo}/rulesets`]);
    const rulesets = rulesetList.map((entry) =>
      ghJson(args.gh, ['api', `repos/${repo}/rulesets/${entry.id}`])
    );
    required = requiredFromRulesets(rulesets, base);
    checks = normalizeChecks(prView);
    if (threadResolutionRequired(rulesets, base)) {
      unresolved = unresolvedThreadCount(args.gh, repo, prView.number);
    }
  }

  if (!Array.isArray(required) || required.length === 0) {
    process.stderr.write('[pr-required-status] no required contexts found\n');
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
