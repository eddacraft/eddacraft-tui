# Post-merge: fix-broadcast-preview-token-real-send

PR: #2064
Branch: `fix/broadcast-preview-token-real-send`
Issue: #1926 ([Clawpatch] POST /admin/broadcast real-send rejects
preview-token-only sends)
Council: pass-with-minors
Merged: <!-- filled by cleanup agent -->
Verified: <!-- filled by cleanup agent -->

## Steps

- [ ] `pnpm exec vitest run apps/anvil-api/src/__tests__/admin-broadcast.test.ts`
      passes on `main` (32 tests, including the new preview-token-only
      real-send acceptance cases and the dry-run template/audience-required
      refine cases). (agent: yes, pre-merge)
- [ ] `pnpm exec nx run anvil-api:typecheck` succeeds on `main` — the schema
      change makes `template`/`audience` optional, so the handler's
      dry-run-only narrowing must still typecheck. (agent: yes, pre-merge)
- [ ] `pnpm exec nx run anvil-api:lint` clean on `main`. (agent: yes, pre-merge)
- [ ] `pnpm exec oxfmt --check apps/anvil-api/README.md
      apps/anvil-api/src/routes/admin.ts apps/anvil-api/src/routes/admin-schemas.ts
      apps/anvil-api/src/__tests__/admin-broadcast.test.ts` clean. (agent: yes,
      pre-merge)
- [ ] Manual smoke (human required): against a deployed/preview API, issue a
      dry-run `POST /admin/broadcast` to mint a preview token, then issue a
      real-send with only `{ dryRun: false, previewToken }` (no template /
      audience) and confirm it is accepted and the consumed snapshot drives the
      send. (human required)
- [ ] Manual smoke (human required): issue a real-send with a `previewToken`
      AND a contradicting request-time `template`/`audience`/`templateProps`,
      and confirm the request-time fields are ignored — the consumed snapshot
      remains the source of truth (no bait-and-switch). (human required)

## Notes

Root cause (Clawpatch #1926): `broadcastSchema` required `template` and
`audience` unconditionally, so the shared request schema rejected a valid
preview-token-only real-send before the handler reached snapshot consumption.

Fix splits validation by leg:
- `template`/`audience` are now optional in `broadcastSchema`, required only
  when `dryRun` is true via a cross-field `refine`.
- The handler validates + snapshots request-time `template`/`audience`/
  `templateProps` on the dry-run leg ONLY. On a real-send the consumed preview
  snapshot is the sole source of truth (EMAIL-010 / #1926); contradicting
  request-time fields are ignored, not rejected.

`/admin/send-migration` shim is unchanged. README endpoint table documents
`POST /admin/broadcast` (and the shim) — docs-only.
