// CIB-119: single source of truth for which Pulumi stacks are trusted to
// read production secrets and define production resources.
//
// The CI PR preview runs an untrusted `ci-preview` stack with PR-controlled
// code (CIB-136), so it must
// never receive live secret values or define resources whose physical names
// collide with production (Vercel projects, the signing account, admin-key
// rows in the production database). Only stacks named here are trusted;
// everything else fails closed — secret reads resolve to an explicit marker
// and production resources are simply not defined.

import * as pulumi from '@pulumi/pulumi';

const TRUSTED_STACKS: ReadonlySet<string> = new Set(['prod']);

/** True only for stacks authorised to touch production secrets/resources. */
export function isTrustedStack(stackName: string = pulumi.getStack()): boolean {
  return TRUSTED_STACKS.has(stackName);
}

/**
 * Explicit, traceable stand-in for a secret value on an untrusted stack.
 * Deliberately unusable as a credential so misconfiguration surfaces loudly
 * instead of being masked by a plausible-looking placeholder.
 */
export function untrustedSecretMarker(secretName: string): string {
  return `<untrusted-stack-secret:${secretName}>`;
}

/** Standard warning when a production-only definition is skipped. */
export function warnUntrustedSkip(what: string): void {
  pulumi.log.warn(
    `Skipping ${what}: stack '${pulumi.getStack()}' is not authorised to define ` +
      `production resources (CIB-119). Run the 'prod' stack to provision them.`
  );
}
