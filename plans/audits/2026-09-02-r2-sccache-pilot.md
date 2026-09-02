# DEVENV-008: Cloudflare R2-backed sccache pilot

| Field | Value |
| --- | --- |
| Type | APS spike evidence and operator runbook |
| Work item | DEVENV-008 |
| Date | 2026-09-02 |
| Branch | `perf/devenv-008-r2-sccache-pilot` |
| Status | Ready to run after environment configuration; no performance verdict yet |

This is a bounded experiment, not a production CI migration. It measures
whether compiler-object reuse from Cloudflare R2 is valuable enough to replace
part of the current GitHub-hosted Rust target cache. The normal Rust workflows
and required checks are unchanged.

## Trust boundary

The pilot is deliberately isolated in `.github/workflows/r2-sccache-pilot.yml`:

- `workflow_dispatch` is its only trigger;
- the job rejects every event except `workflow_dispatch` and every ref except
  `refs/heads/main` before checkout;
- credentials come from the protected `r2-sccache` GitHub environment;
- an ephemeral GitHub-hosted runner and per-run `sccache` socket prevent a
  cancelled pilot leaving a credential-bearing daemon for a later job;
- no pull-request, push, or scheduled job receives the R2 credentials;
- a repository fixture fails if a pull-request, push, or schedule trigger is
  added, or if the main-ref/environment guards are removed.

The `r2-sccache` GitHub environment exists with a custom deployment-branch
policy allowing `main` only. Keep that policy in place. Do not move these
credentials into repository-wide secrets consumed by a workflow that runs
proposed code.

## One-time operator setup

1. Create a private R2 bucket dedicated to this cache, for example
   `anvil-sccache`.
2. Create an R2 API token scoped only to that bucket with Object Read & Write.
3. In the existing GitHub environment named `r2-sccache`, retain the custom
   `main`-only deployment policy and add:

   | Kind | Name | Value |
   | --- | --- | --- |
   | Environment variable | `R2_SCCACHE_BUCKET` | R2 bucket name |
   | Environment variable | `R2_SCCACHE_ENDPOINT` | `https://<ACCOUNT_ID>.r2.cloudflarestorage.com` |
   | Environment secret | `R2_SCCACHE_ACCESS_KEY_ID` | R2 token access-key ID |
   | Environment secret | `R2_SCCACHE_SECRET_ACCESS_KEY` | R2 token secret access key |

The workflow pins `sccache` 0.17.0, uses R2's required `auto` region, and scopes
objects below `anvil-001/rust/v1` so the bucket can be inspected or retired
without ambiguity.

## Experiment protocol

Run `R2 sccache Pilot` from `main` using the `check` surface:

1. Run the `none` backend once for the same-runner cold baseline.
2. Run the `rust-cache` backend once to restore the current production
   `rust-ci` cache on the same runner and compile surface.
3. Run the `r2` backend in `READ_WRITE` mode once to seed the bucket.
4. Run `r2` in `READ_ONLY` mode twice at the same commit to measure warm remote
   reuse without changing the cache.
5. Repeat the five runs after a representative Rust change reaches `main`.
6. Repeat with `test-build` only if `check` shows a material hit rate.

Each run uses a fresh ephemeral `ubuntu-24.04` runner, fetches dependencies
before the timer starts, and uploads its wall time, exit status, runner/toolchain
identity, and (for R2) `sccache --show-stats` for 30 days. Record run URLs and
results below rather than relying on the Actions retention window.

| Run | Commit | Surface | Mode | Wall time | Cache hits | Cache misses | Notes |
| --- | --- | --- | --- | ---: | ---: | ---: | --- |
| Cold control | — | `check` | `none` | — | — | — | Pending |
| Current cache | — | `check` | `rust-cache` | — | — | — | Pending |
| Seed | — | `check` | `READ_WRITE` | — | — | — | Pending environment setup |
| Warm 1 | — | `check` | `READ_ONLY` | — | — | — | Pending |
| Warm 2 | — | `check` | `READ_ONLY` | — | — | — | Pending |

## Decision bar

Recommend a broader CI trial only when both warm runs:

- complete at least 20% faster than both the no-cache and current-cache control
  arms on the same surface and runner image;
- show compiler cache hits rather than merely faster dependency download;
- have no cache errors or unexplained non-cacheable compilations;
- keep projected R2 storage and operation costs below the GitHub cache/run-time
  savings they replace.

Even with a positive result, pull-request access needs a separate security
decision. Long-lived write credentials must not be exposed to proposed code.
A production design may retain the current PR cache, use a public/read-only
cache only after a source-disclosure review, or introduce short-lived scoped
credentials from a trusted broker.

## Failure modes and rollback

- Missing variables or secrets: the configuration step fails before compiling
  and names only the missing setting, never its value.
- Invalid endpoint: the workflow rejects non-HTTPS or non-R2 endpoints.
- R2 outage or token failure: the pilot fails; required CI is unaffected.
- Cache poisoning or unexpected objects: revoke the bucket token, delete the
  `anvil-001/rust/v1` prefix or the dedicated bucket, and rotate the credentials.
- No useful speedup: remove the pilot workflow and GitHub environment; the
  existing Rust cache remains authoritative throughout the experiment.

## Exit condition

This report becomes input to DEVENV-008's wider nx-cache versus `sccache` versus
wave-1 comparison. DEVENV-008 remains open until that comparison exits through
the required go/no-go ADR; this pilot alone does not authorise a production
migration.

## References

- [`sccache` 0.17.0 S3 and R2 configuration](https://github.com/mozilla/sccache/blob/v0.17.0/docs/S3.md)
- [Cloudflare R2 API-token guidance](https://developers.cloudflare.com/r2/api/s3/tokens/)
- [GitHub deployment environments](https://docs.github.com/en/actions/how-tos/deploy/configure-and-manage-deployments/manage-environments)
