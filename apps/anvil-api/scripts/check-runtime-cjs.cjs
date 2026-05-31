#!/usr/bin/env node
/* eslint-disable @typescript-eslint/no-require-imports --
 * The whole point of this script is to exercise the CommonJS require()
 * path that Vercel's Node runtime hits at cold start. ESM imports would
 * not reproduce the failure mode this guard is meant to catch.
 */
// Vercel's Node runtime loads the compiled API as ESM, but `resend` and
// other declared CommonJS deps still resolve their own transitive deps
// via require(). An ESM-only transitive dep pulled in by a global
// override (e.g. an over-broad `uuid` floor) crashes those CJS packages
// on cold start with ERR_REQUIRE_ESM.
//
// This script reproduces that require chain against the package's public
// CJS entrypoint. It runs as part of the anvil-api build so the
// regression cannot reach prod again.
//
// History: resend@<=6.12.3 pulled svix (CJS), which required uuid;
// uuid v14 is ESM-only, so a parent-scoped `svix>uuid` override pinned
// svix to uuid v10. resend@6.12.4 dropped svix entirely, so the probe
// now targets `resend` directly (the declared public entrypoint) rather
// than a transitive package that can disappear on a patch bump.

const cjsModulesToProbe = ['resend'];

let failed = false;
for (const id of cjsModulesToProbe) {
  try {
    require(id);
    console.log(`ok: require('${id}')`);
  } catch (err) {
    failed = true;
    console.error(`FAIL: require('${id}') threw ${err.code ?? err.name}: ${err.message}`);
    if (err.code === 'ERR_REQUIRE_ESM') {
      console.error(
        'Hint: a transitive ESM-only dep was pulled in by an override. ' +
          'Check pnpm.overrides for a uuid (or similar) floor that needs a ' +
          'parent-scoped exception (e.g. "<cjs-parent>>uuid": "^10.0.0").'
      );
    }
  }
}

process.exit(failed ? 1 : 0);
