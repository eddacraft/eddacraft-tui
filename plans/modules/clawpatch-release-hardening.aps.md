<!-- APS: See https://github.com/eddacraft/anvil-plan-spec for format reference -->

# Clawpatch Release Hardening

| ID      | Owner  | Priority | Status      | Progress |
| ------- | ------ | -------- | ----------- | -------- |
| CLAWREL | @aneki | P1       | In Progress | 0/1      |

> **Exclusive module.** CLAWREL-001 owns this bounded release-hardening change.
> It does not absorb the standing CIB backlog or the historical Clawpatch queue.

## Purpose

Close the release-relevant security and lifecycle defects found by Clawpatch run
`20260911T093427-fc4b6f` before the supported TypeScript packages are published
again.

## Design

Fix each defect at its owning boundary:

- redact credential-shaped material before provenance serialisation and before
  structured debug values reach the console;
- parse snapshots with the canonical feature-flag definition schema rather than
  maintaining a weaker parallel validator;
- make transport closure cancel the in-flight socket and reject any connection
  that completes after closure.

The change preserves public method signatures and clean-value output. Reverting
the single implementation PR is the rollback.

## Work Items

### CLAWREL-001: Close release-relevant credential, snapshot, and transport gaps

- **Status:** In Progress
- **Priority:** P1
- **Risk:** high
- **Claim:** [#4636](https://github.com/eddacraft/anvil-001/issues/4636)
- **Intent:** Supported TypeScript packages do not publish known credential
  disclosure, untrusted snapshot, or post-close connection defects.
- **Expected Outcome:**
  - Token-shaped SCP remote usernames, prompt message text, and nested structured
    debug values are redacted without changing ordinary Git remotes, messages,
    or debug context.
  - `loadSnapshot` accepts only complete definitions satisfying the canonical
    feature-flag schema and reports invalid definitions as `SnapshotLoadError`.
  - Closing either transport while connection or authentication is pending
    destroys the late socket, rejects `connect()` with
    `anvil-driver-closed`, and fires the close callback once.
- **Files:**
  - `packages/anvil/core/src/utils/credential-redaction.ts`
  - `packages/anvil/core/src/utils/credential-redaction.test.ts`
  - `packages/anvil/core/src/utils/debug.ts`
  - `packages/anvil/core/src/utils/debug.test.ts`
  - `packages/anvil/core/src/provenance/git-ai-standard/serializer.ts`
  - `packages/anvil/core/src/provenance/git-ai-standard/__tests__/credential-persistence.test.ts`
  - `packages/anvil/runtime/src/feature-flags/snapshot.ts`
  - `packages/anvil/runtime/src/feature-flags/snapshot.test.ts`
  - `packages/anvil-driver-client/src/transport/unix.ts`
  - `packages/anvil-driver-client/src/transport/unix.test.ts`
  - `packages/anvil-driver-client/src/transport/windows.ts`
  - `packages/anvil-driver-client/src/transport/windows.test.ts`
  - `packages/anvil-driver-client/README.md`
  - `packages/anvil-driver-client/ARCHITECTURE.md`
  - `plans/modules/clawpatch-release-hardening.aps.md`
  - `plans/index.aps.md`
- **Validation:**
  - `pnpm --filter @eddacraft/anvil-core test`
  - `pnpm --filter @eddacraft/anvil-core typecheck`
  - `pnpm --filter @eddacraft/anvil-core build`
  - `pnpm --filter @eddacraft/anvil-runtime test`
  - `pnpm --filter @eddacraft/anvil-runtime typecheck`
  - `pnpm --filter @eddacraft/anvil-runtime build`
  - `pnpm --filter @eddacraft/anvil-driver-client test`
  - `pnpm --filter @eddacraft/anvil-driver-client typecheck`
  - `pnpm --filter @eddacraft/anvil-driver-client build`
  - `pnpm validate:changed`
- **Finding IDs:**
  - `fnd_sig-feat-library-6f26fae528-464f_412ce217c1`
  - `fnd_sig-feat-library-6f26fae528-0948_3c6da74942`
  - `fnd_sig-feat-cli-command-a4f9ddbd8c-_cbb2efa54f`
  - `fnd_sig-feat-library-6b0f32ec35-2528_50ea851782`
  - `fnd_sig-feat-library-ed5b3b8728-752c_a685ac8946`
- **Dependencies:** none
- **Decision:** ready and started by operator instruction on 2026-09-11
- **Evidence:** focused red/green regressions cover credential-shaped SCP remotes,
  persisted prompt messages, structured debug objects and credential-named fields,
  malformed canonical snapshot definitions, and close/connect races on both
  transports. On 2026-09-11 all three affected package test, typecheck, and build
  commands exited 0; `pnpm validate:changed`, `pnpm format:check`,
  `pnpm lint:check`, `pnpm docs:check`, `pnpm aps:active-lint`, and
  `pnpm aps:index:check` also exited 0. Documentation impact is unaffected:
  `pnpm docs:redate --since 50c40cf374b927ae2b6ba4dd12618ac7754950ec`
  reported nothing owed, and no authoritative diagram depicts these internal
  validation and cancellation details.
- **Independent verification:** the first pass required repair for ordinary Git
  object IDs and non-credential debug keys being over-redacted, plus Windows
  closure losing error precedence after socket connection but before
  authentication. Repair cycle 1 added those adversarial regressions and the
  verifier passed tree `4bf98f5366fef1075f5611da21baa28fb5f23f82`.
- **Re-verification:** repair cycle 1 closed its original findings but exposed
  labelled hexadecimal credentials, semantic credential-key variants, and the
  staged driver architecture freshness gate. Repair cycle 2 covers those inputs
  and updates the owning component documentation. Council converged at PASS
  with no open findings or waivers; the subsequent CodeQL flow repair passed
  independent verification on tree `2ee4dcacb709560c094d20938e523152c7b024ea`.

## Non-scope

- The unused `ProvenanceStore` concurrency and retention findings.
- Snapshot throughput beyond 1,000 publications per second.
- Newline-bearing Git filenames, empty authorship logs, and zero line ranges.
- Release, merge, or branch-protection override authority.
