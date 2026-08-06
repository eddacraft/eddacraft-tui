// Shared exit-code taxonomy for docs-check surfaces (CIB-278).
//
// A docs surface can fail in two materially different ways, and a contributor
// needs to tell them apart at a glance:
//
//   1 — the docs are wrong. The contributor's change is the suspect.
//   2 — the check could not RUN. The environment is the suspect, and the docs
//       carry no signal at all.
//
// Code 2 is not invented here — scripts/aps/drift-check.mjs already exits 2 on
// usage errors (lines 48, 59). What was missing was anything downstream that
// honoured the distinction: the delegates forwarded 2 as an opaque non-zero
// status and the orchestrator rendered every non-zero status as `FAIL`, so a
// broken toolchain read as a docs defect.
//
// Honouring code 2 downstream only works if every surface means the same thing
// by it, so all of DEFAULT_SURFACES was audited rather than assumed. Two sites
// were exiting 2 for conditions that are plainly *content* defects — a missing
// RELEASE-PLAN.md (check-release-plan.mjs) and a missing plans/decisions/ or
// DECISION-LOG.md (adr-integrity.sh) — and were moved to exit 1. Leaving them
// would have re-created the very misattribution this fixes, just inverted:
// deleting a governed doc would have been reported as somebody else's broken
// toolchain.
//
// So: if you add a surface, exit 2 ONLY when the check could not run. A missing
// or malformed input document is content, and exits 1.
//
// Signals get the same treatment as code 2. A surface killed by SIGKILL (OOM,
// a CI runner reaping the job) has told us nothing about the corpus, so
// reporting it as a content failure would be the same lie in a different coat.

import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import process from 'node:process';

/** The check ran and the docs are clean. */
export const EXIT_PASS = 0;
/** The check ran and found a real content defect. */
export const EXIT_CONTENT_FAILURE = 1;
/** The check could not run. Says nothing about the docs. */
export const EXIT_TOOLING_FAILURE = 2;

/** Repository root, resolved from this file rather than from the caller's cwd. */
export const REPO_ROOT = fileURLToPath(new URL('../../..', import.meta.url));

/**
 * Classify a finished child process into the taxonomy above.
 *
 * `spawnSync` reports three distinct outcomes that all need mapping: a spawn
 * that never happened (`error`), a termination by signal (`status === null`),
 * and a normal exit with a code.
 */
export function classify(result) {
  if (result.error) return EXIT_TOOLING_FAILURE;
  if (result.status === null) return EXIT_TOOLING_FAILURE;
  if (result.status === EXIT_TOOLING_FAILURE) return EXIT_TOOLING_FAILURE;
  return result.status === EXIT_PASS ? EXIT_PASS : EXIT_CONTENT_FAILURE;
}

/**
 * Run one underlying check as a labelled docs-check surface.
 *
 * The delegates deliberately invoke their checker directly — `node <script>` or
 * `bash <script>` — instead of re-entering the package manager. Re-entering
 * meant every surface inherited the package manager's health, which is how a
 * broken corepack shim came to be reported as `FAIL aps` / `FAIL adr` on a
 * clean corpus (CIB-278).
 *
 * @param {object} options
 * @param {string} options.surface  Label used to prefix output, e.g. `aps`.
 * @param {string} options.command  Executable to run.
 * @param {string[]} options.args   Arguments for the executable.
 * @param {string} options.isolate  The command to reproduce this surface on its
 *                                  own, shown when the check could not run.
 * @returns {never}
 */
export function runSurfaceDelegate({ surface, command, args, isolate }) {
  const result = spawnSync(command, args, {
    // The underlying checks root themselves at the working directory, so pin it
    // to the repository rather than inheriting wherever the caller stood.
    cwd: REPO_ROOT,
    // stdin is ignored rather than inherited: neither underlying check reads it,
    // and under the orchestrator there is nothing meaningful to inherit.
    stdio: ['ignore', 'pipe', 'pipe'],
    encoding: 'utf8',
  });

  const stdout = (result.stdout ?? '').trimEnd();
  const stderr = (result.stderr ?? '').trimEnd();
  for (const line of stdout.split('\n')) if (line) console.log(`[${surface}] ${line}`);
  for (const line of stderr.split('\n')) if (line) console.error(`[${surface}] ${line}`);

  const code = classify(result);
  if (code === EXIT_TOOLING_FAILURE) {
    const cause = result.error
      ? result.error.message
      : result.status === null
        ? `terminated by signal ${result.signal}`
        : `exited ${EXIT_TOOLING_FAILURE} (could not run)`;
    // The underlying error was already streamed above, so pointing back at it
    // would be circular. What is actually useful under `docs:check` — where nine
    // surfaces interleave their output — is the command that reproduces this one
    // in isolation.
    console.error(`[${surface}] tooling failure: ${cause}`);
    console.error(`[${surface}] this is not a docs content defect; the docs were never checked.`);
    console.error(`[${surface}] reproduce in isolation with: ${isolate}`);
  }
  process.exit(code);
}
