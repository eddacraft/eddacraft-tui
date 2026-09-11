# Post-merge: fix-cib-172-windows-first-run-recipe

PR: #NNN
Branch: `fix/cib-172-windows-first-run-recipe`
APS: CIB-172
Merged: <!-- filled by cleanup agent -->
Verified: <!-- filled by cleanup agent -->

## Steps

- [ ] Confirm the `windows-latest` matrix job in `ci.yml` on the post-merge
      push-to-main run passes `commands::start::tests` — in particular
      `first_run_recipe_matches_fixture`, which on Windows exercises the
      `cfg!(windows)`-selected `RECIPE_CLEANUP_WINDOWS` (`del`) path that PR CI
      (Linux-only for this branch prefix) cannot reach (agent: yes)

## Notes

CIB-172: the first-run smoke recipe's cleanup step printed
`rm .anvil-smoke-test.ts` unconditionally; under cmd.exe that fails with
`'rm' is not recognized`. The fix branches the cleanup line via a
`recipe_cleanup_line()` selector (`rm` on Unix, `del` on Windows), mirroring the
tutorial's `create_policy_directory_command` pattern.

Why post-merge verification is warranted: the macOS/Windows matrix job in
`ci.yml` only runs on `release/`/`hotfix/` branches and pushes to `main`, so the
platform-*selected* Windows line is first compiled-and-asserted by CI after the
merge lands. Pre-merge coverage is still strong — both cleanup variants are
compiled and named on every host, and
`first_run_recipe_cleanup_is_platform_branched` asserts the Windows variant uses
`del` and contains no `rm` — so this step is confirmation, not a gap.

Known out-of-scope follow-up (logged in the CI log entry): step 1's
`echo 'const KEY = "…"'` single-quoting is a cmd.exe quirk (cmd echoes the
quotes literally); worth a follow-up CIB if the Windows smoke path is exercised
end-to-end.
