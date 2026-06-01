# Release Cadence And Support Policy

| Type  | Authority     | Owner                                                                                                          | Status | Freshness                                                                                                                                                                                                                |
| ----- | ------------- | -------------------------------------------------------------------------------------------------------------- | ------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Guide | Authoritative | DISTRIB ([`plans/modules/distribution-and-update.aps.md`](../../plans/modules/distribution-and-update.aps.md)) | Live   | Amended 2026-06-01: six-week minor-beta cadence hold retired (authorised by Josh — funding velocity). Prior: reviewed 2026-05-16 against `plans/specs/2026-05-14-release-plan-v0.7.0-sit-on.md` and `plans/aps-rules.md` |

| Upstream                                                                     | Downstream                                                      |
| ---------------------------------------------------------------------------- | --------------------------------------------------------------- |
| `plans/specs/2026-05-14-release-plan-v0.7.0-sit-on.md`, `plans/aps-rules.md` | Release runbooks, `README.md`, `CONTRIBUTING.md`, release notes |

This policy defines what Anvil users and operators can expect from beta release
cadence, version support, and hotfix response. It does not replace release
readiness checks, APS authorisation, or the release runbook; it sets the public
expectation those procedures must satisfy.

## Current Channel

The active distribution channel is `-beta`. Beta releases are expected to be
usable by senior internal users on real work, but they may still change quickly
when user signal shows friction, false positives, install breakage, or
protection claim gaps.

Through `v0.7.x` the project optimised for patch stability and adoption evidence
on the `v0.7.0-beta` slate. **As of 2026-06-01 the priority is continuous
feature delivery toward an investor-ready solution** (authorised by Josh):
`main` stays releasable, and minor betas cut when their slice is ready and the
release gates are green — not on a calendar. The six-week minor-beta cadence
hold described in earlier revisions of this policy is **retired**.

## Cadence

| Release shape          | Cadence                                      | Scope                                                                                                                                                           |
| ---------------------- | -------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `v0.7.x` patch         | Weekly while user signal is non-empty        | Bug fixes, false-positive reductions, documentation corrections, packaging fixes, and low-risk operational hardening.                                           |
| `v0.7.x` patch         | Within 48 hours of any P0 bug                | Crash, data loss, false protection claim, daemon corruption, install/update breakage that strands users, or security-relevant regression.                       |
| Next minor beta        | When ready — releasable `main` + green gates | Feature additions and non-breaking product expansion. No calendar gate; quality and APS authorisation gate the cut. (The six-week hold was retired 2026-06-01.) |
| Breaking beta or major | Demand-pulled                                | Triggered by a real adopter requirement or a Boring-Week-tier regression, not by backlog completion alone.                                                      |

Major releases are **demand-pulled** — triggered by a real adopter requirement
or a regression too serious for patch/minor repair, not by a calendar. The
six-week sit-on hold from earlier revisions of this policy is retired
(2026-06-01, authorised by Josh) in favour of continuous, quality-gated feature
delivery.

## Version Scope

Release scope follows the APS metadata convention in `plans/aps-rules.md`:

| Scope | Use for                                                    | Examples                                                                                             |
| ----- | ---------------------------------------------------------- | ---------------------------------------------------------------------------------------------------- |
| Patch | Same behaviour intent, safer or clearer execution          | Bug fixes, false-positive reductions, documentation corrections, release artefact repair.            |
| Minor | New capability without breaking existing workflows         | New command surfaces, additive JSON fields, new supported install path, new local-only insight view. |
| Major | Breaking behaviour, compatibility, or release-claim change | Removing a supported workflow, changing durable schema semantics, replacing the release claim.       |

When release metadata and implementation disagree, the safer higher scope wins
until the owner resolves the mismatch in APS and release notes.

## Beta Support Window

During the beta channel:

1. The latest minor receives bug fixes, false-positive reductions, and security
   fixes.
2. The previous minor receives security fixes and critical upgrade-path fixes.
3. Older beta minors are end-of-life unless an active incident response names a
   temporary exception.

For example, after `v0.7.1-beta` ships, `v0.7.x-beta` is the latest minor line
and `v0.6.x-beta` is the previous minor line for security and critical upgrade
support. Once `v0.8.0-beta` ships, `v0.6.x-beta` falls out of the default
support window.

## Hotfix Rules

Hotfixes are for urgent production repair. The normal path is still branch from
`main`, open a PR to `main`, validate the exact merge SHA, tag the patch, and
emit release evidence. Branching from the latest good tag is reserved for cases
where `main` is temporarily unreleasable.

A hotfix may narrow validation to the affected surface, but it does not waive:

1. APS tracking for the work.
2. A targeted review or Council pass.
3. Release readiness for the tagged SHA.
4. A release record proving what shipped.
5. Follow-up cleanup for any bypass or reduced validation.

Use [`docs/runbooks/emergency-hotfix.md`](../runbooks/emergency-hotfix.md) when
the change cannot wait for the normal patch cadence.

## End-Of-Life Notices

An end-of-life notice is required when a beta minor leaves the support window
and users may still plausibly be running it. The notice should name:

1. The unsupported version line.
2. The supported replacement line.
3. Whether security fixes still apply.
4. The upgrade command or installer path.

The notice can live in release notes unless the retirement has operational risk,
in which case create or update a runbook.

## Operator Checklist

Before cutting a release, confirm:

1. APS release metadata matches the intended patch, minor, or major scope.
2. Release notes describe user-visible support-window changes.
3. Any previous-minor security fix decision is explicit.
4. Any emergency hotfix bypass has a tracked follow-up.
5. The release record is sufficient for cleanup agents to advance APS lifecycle
   state from Merged to Released/Shipped.
