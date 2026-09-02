#!/usr/bin/env node
import { spawnSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import process from 'node:process';

const NOT_FINISHED = 2;
const FAILED = 1;

function usage() {
  process.stderr.write(
    'Usage: scripts/ci/pr-required-status.mjs [--repo owner/repo] [--pr N] [--base BRANCH] [--required-json FILE] [--checks-json FILE]\n'
  );
}

function parseArgs(argv) {
  const out = {
    repo: null,
    pr: null,
    base: null,
    requiredJson: null,
    checksJson: null,
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

function requiredFromRulesets(rulesets, baseBranch) {
  const wanted = new Set();
  const ref = `refs/heads/${baseBranch}`;
  for (const ruleset of rulesets) {
    if (ruleset.enforcement && ruleset.enforcement !== 'active') continue;
    const include = ruleset.conditions?.ref_name?.include ?? [];
    const exclude = ruleset.conditions?.ref_name?.exclude ?? [];
    const applies =
      include.length === 0 ||
      include.includes('~DEFAULT_BRANCH') ||
      include.includes('~ALL') ||
      include.includes(ref) ||
      include.includes(baseBranch);
    if (!applies) continue;
    if (exclude.includes(ref) || exclude.includes(baseBranch)) continue;
    for (const rule of ruleset.rules ?? []) {
      if (rule.type !== 'required_status_checks') continue;
      for (const check of rule.parameters?.required_status_checks ?? []) {
        if (check.context) wanted.add(check.context);
      }
    }
  }
  return [...wanted].sort();
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

function main() {
  const args = parseArgs(process.argv.slice(2));
  let required;
  let checks;

  if (args.requiredJson && args.checksJson) {
    required = loadJsonFile(args.requiredJson);
    checks = normalizeChecks(loadJsonFile(args.checksJson));
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
  process.stdout.write(
    `[pr-required-status] all required contexts reported and passed (${required.length})\n`
  );
  process.exit(0);
}

main();
