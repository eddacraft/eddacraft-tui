# Plan: Migrate from Unosend to Resend

**Status:** Draft
**Date:** 2026-02-17

## Summary

Replace `@unosend/node` with `resend` across the website app and
infrastructure. Unosend is currently used solely for transactional waitlist
confirmation emails sent from `anvil@send.eddacraft.ai`.

## Scope of Changes

### 1. Application Code

**`apps/website/package.json`** — Swap dependency

- Remove `"@unosend/node": "^1.1.0"`
- Add `"resend": "^4.0.0"`

**`apps/website/lib/email.ts`** — Rewrite client

- Replace `import { Unosend } from '@unosend/node'` with
  `import { Resend } from 'resend'`
- Replace `Unosend` client instantiation with `new Resend(apiKey)`
- Change env var from `UNOSEND_API_KEY` to `RESEND_API_KEY`
- Update warning message to reference `RESEND_API_KEY`
- Adapt `emails.send()` call to Resend's API shape:
  - `from`, `to`, `subject`, `text`, `html` stay the same
  - `replyTo` → `reply_to`
  - `headers` stays the same (Resend supports custom headers)
  - `tags` format: Resend uses `[{ name, value }]` — same shape, no change
    needed
- Update error handling (Resend returns `{ data, error }` — same pattern)

**`apps/website/.env.local.example`** — Update env var docs

- Rename `UNOSEND_API_KEY` → `RESEND_API_KEY`
- Update comment to reference Resend docs (`https://resend.com`)
- Update key format hint from `un_...` to `re_...`

### 2. Infrastructure (Pulumi)

**`infra/src/vercel.ts`** — Update secret reference

- Rename Key Vault secret lookup from `unosend-api-key` to `resend-api-key`
- Rename env var from `UNOSEND_API_KEY` to `RESEND_API_KEY`

**`infra/src/dns/eddacraft-ai.ts`** — Replace DNS records for
`send.eddacraft.ai`

- **MX record**: Remove `mail.unosend.co` → not needed for Resend (Resend
  doesn't require inbound MX for sending)
- **SPF record**: Replace `include:_spf.unosend.co ip4:... ip6:...` with
  `include:amazonses.com` (Resend sends via AWS SES)
- **DKIM record**: Remove `unosend._domainkey` TXT record → Resend uses
  different DKIM selectors. New DKIM records will come from the Resend domain
  verification dashboard (typically 3 CNAME records)
- **Root TXT**: Remove the Unosend domain verification TXT value
  (`_cux88fmbdoc8oeyu9sy0paxt0yd4mzm`) from the root TXT record set
- **DMARC**: Keep as-is (`p=none` policy is provider-agnostic)
- Rename Pulumi resource names to remove "unosend" references

> **Note:** The exact DKIM CNAME values and SPF include will come from the
> Resend dashboard after adding `send.eddacraft.ai` as a domain. These are
> placeholder values based on Resend's standard setup. We'll need to update them
> with the actual values from the dashboard.

**`infra/README.md`** — Update documentation

- Rename `unosend-api-key` references to `resend-api-key`
- Update the Key Vault secret table entry
- Update the `az keyvault secret set` example command

**`infra/scripts/bootstrap-backend.sh`** — Update if it references the secret
name

### 3. Secrets Management

**Azure Key Vault** (manual step):

```bash
# Store new Resend API key
az keyvault secret set --vault-name kv-iac-anvil \
  --name resend-api-key --value '<RESEND_API_KEY>'

# Optionally disable old secret after verification
az keyvault secret set-attributes --vault-name kv-iac-anvil \
  --name unosend-api-key --enabled false
```

**Vercel env vars** — handled automatically by Pulumi after infra deploy.

### 4. Resend Account Setup (manual, pre-migration)

1. Create Resend account at https://resend.com
2. Add sending domain `send.eddacraft.ai`
3. Retrieve the DNS verification records (DKIM CNAMEs, SPF include)
4. Retrieve API key (format: `re_...`)
5. Store API key in Azure Key Vault as `resend-api-key`

### 5. Tests

**`infra/src/__tests__/vercel.test.ts`** — Update any assertions referencing
`UNOSEND_API_KEY` or `unosend-api-key`

**`infra/src/__tests__/dns.test.ts`** — Update assertions for the DNS records
(MX, SPF, DKIM resource names and values)

## Execution Order

1. **Resend account setup** — Create account, add domain, get DNS records and
   API key (manual)
2. **Store API key** — Add `resend-api-key` to Azure Key Vault (manual)
3. **DNS records** — Update Pulumi DNS config with Resend's DKIM/SPF values,
   deploy infra to propagate DNS changes
4. **Wait for DNS propagation** — Verify domain in Resend dashboard (can take
   up to 48h but usually minutes)
5. **Application code** — Swap package, rewrite `email.ts`, update env example
6. **Infrastructure code** — Update `vercel.ts` secret reference and env var
   name
7. **Deploy infra** — Pulumi up to push new env var to Vercel
8. **Deploy website** — Deploy the website app with new Resend integration
9. **Verify** — Send a test waitlist signup and confirm email delivery
10. **Cleanup** — Disable old `unosend-api-key` in Key Vault, remove
    `@unosend/node` from lockfile

## Files Changed (Code)

| File | Change |
|---|---|
| `apps/website/package.json` | Swap `@unosend/node` → `resend` |
| `apps/website/lib/email.ts` | Rewrite client + env var |
| `apps/website/.env.local.example` | Update env var + docs |
| `infra/src/vercel.ts` | Rename secret + env var |
| `infra/src/dns/eddacraft-ai.ts` | Replace DNS records |
| `infra/README.md` | Update secret docs |
| `infra/scripts/bootstrap-backend.sh` | Update secret name if present |
| `infra/src/__tests__/vercel.test.ts` | Update test assertions |
| `infra/src/__tests__/dns.test.ts` | Update test assertions |

## Risks & Notes

- **DNS propagation delay**: SPF/DKIM changes may take time. During this window
  emails could fail DKIM/SPF checks. Mitigate by deploying DNS first and
  waiting for Resend to verify the domain before switching the app code.
- **API shape differences**: Resend's SDK is very similar to Unosend's (both
  return `{ data, error }` and take similar `emails.send()` params). The
  migration is straightforward.
- **No downtime expected**: The app gracefully handles missing API keys (logs a
  warning, skips sending). During the cutover window, worst case is a few
  confirmation emails are missed — these are non-critical.
- **Rollback**: Revert the code changes and re-enable the old Key Vault secret.
  Keep the Unosend DNS records in git history for quick restoration.
