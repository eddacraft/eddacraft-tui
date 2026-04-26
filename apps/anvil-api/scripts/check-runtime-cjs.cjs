#!/usr/bin/env node
// Vercel's Node runtime loads the compiled API as ESM, but transitive
// CommonJS deps (notably svix, pulled by resend) still resolve uuid via
// require(). uuid v14 is ESM-only, so a global `uuid` override that
// floors at >=14 will crash svix on cold start with ERR_REQUIRE_ESM.
//
// This script reproduces that exact require chain. It runs as part of
// the anvil-api build so the regression cannot reach prod again.
//
// See: pnpm.overrides["svix>uuid"] in the workspace root package.json.

const cjsModulesToProbe = ['svix'];

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
          'parent-scoped exception (e.g. "svix>uuid": "^10.0.0").',
      );
    }
  }
}

process.exit(failed ? 1 : 0);
