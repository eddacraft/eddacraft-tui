import * as pulumi from '@pulumi/pulumi';
import { execFileSync } from 'node:child_process';
import { PerOperatorAdminKey } from './components/per-operator-admin-key.js';
import { getSecret } from './keyvault.js';
import { isTrustedStack, warnUntrustedSkip } from './stack-trust.js';

// ADMINCLIH-004: each entry in `seed` below results in an `admin_keys` row
// (and matching audit row) managed declaratively by Pulumi. Adding/removing
// entries is the provisioning workflow; the IaC review acts as the two-person
// rule. See docs/runbooks/admin-cli.md for operator-facing docs.

interface AdminKeySeed {
  name: string;
  actorEmail: string;
  note: string;
}

const seed: AdminKeySeed[] = [
  {
    name: 'josh-arkahna',
    actorEmail: 'josh@arkahna.io',
    note: 'founder; initial IaC-provisioned key',
  },
];

// `pulumi_commit_sha` on the audit row. CI sets GITHUB_SHA; for local runs
// we fall back to `git rev-parse HEAD`. If neither is available we use the
// sentinel `unknown` — the audit row is still written, just less useful.
function resolveCommitSha(): string {
  const fromCi = process.env['GITHUB_SHA'];
  if (fromCi && fromCi.trim()) return fromCi.trim();
  try {
    return execFileSync('git', ['rev-parse', 'HEAD'], {
      encoding: 'utf8',
      stdio: ['ignore', 'pipe', 'ignore'],
    }).trim();
  } catch {
    return 'unknown';
  }
}

// `change_actor` = who ran `pulumi up`. CI sets GITHUB_ACTOR; locally we fall
// back to the Pulumi-reported user, then finally to the OS username.
function resolveChangeActor(): string {
  return (
    process.env['GITHUB_ACTOR'] ?? process.env['PULUMI_USER'] ?? process.env['USER'] ?? 'unknown'
  );
}

// CIB-119: admin keys are rows in the PRODUCTION database, written via a
// local command with the production connection string in its environment.
// Only the trusted prod stack may define them; untrusted stacks (for example
// the PR-preview `dev` stack) provision nothing and read no secrets here.
function defineAdminKeys(): PerOperatorAdminKey[] {
  const databaseUrl = getSecret('anvil-api-database-url');
  const pepper = getSecret('admin-key-pepper');

  const commitSha = resolveCommitSha();
  const changeActor = resolveChangeActor();

  return seed.map(
    ({ name, actorEmail, note }) =>
      new PerOperatorAdminKey(`admin-key-${name}`, {
        actorEmail,
        note,
        changeActor,
        commitSha,
        databaseUrl,
        pepper,
      })
  );
}

const trusted = isTrustedStack();
if (!trusted) {
  warnUntrustedSkip('production per-operator admin keys');
}

export const adminKeys: PerOperatorAdminKey[] = trusted ? defineAdminKeys() : [];

// Bearer tokens, keyed by actor email. Secret Pulumi outputs — retrieve with:
//   pulumi stack output adminKeyBearers --show-secrets
export const adminKeyBearers = pulumi
  .all(adminKeys.map((k) => pulumi.all([k.actorEmail, k.bearerHex])))
  .apply((pairs) => {
    const out: Record<string, string> = {};
    for (const [email, hex] of pairs) {
      out[email] = hex;
    }
    return out;
  });
