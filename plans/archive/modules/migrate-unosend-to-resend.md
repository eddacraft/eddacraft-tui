<\!-- Archived: 2026-03-01 | Reason: Implementation complete — Unosend replaced with Resend -->

# Plan: Migrate from Unosend to Resend

**Status:** Implemented
**Date:** 2026-02-17

## Summary

Replace `@unosend/node` with `resend` across the website app and
infrastructure. Sending domain changed from `send.eddacraft.ai` to
`updates.eddacraft.ai`.

## Changes Made

### Application Code

| File | Change |
|---|---|
| `apps/website/package.json` | `@unosend/node` → `resend` |
| `apps/website/lib/email.ts` | Resend SDK, `RESEND_API_KEY`, from `anvil@updates.eddacraft.ai` |
| `apps/website/.env.local.example` | `RESEND_API_KEY="re_..."`, updated docs |

### Infrastructure (Pulumi)

| File | Change |
|---|---|
| `infra/src/vercel.ts` | `resend-api-key` secret, `RESEND_API_KEY` env var |
| `infra/src/dns/eddacraft-ai.ts` | Replaced all Unosend DNS records with Resend records for `updates.eddacraft.ai` |
| `infra/README.md` | Updated Key Vault secret name and examples |
| `infra/scripts/bootstrap-backend.sh` | Updated secret name in setup instructions |

### DNS Records (updates.eddacraft.ai)

| Type | Host | Value |
|---|---|---|
| TXT | `resend._domainkey.updates` | DKIM public key |
| MX | `send.updates` | `feedback-smtp.ap-northeast-1.amazonses.com` (priority 10) |
| TXT | `send.updates` | `v=spf1 include:amazonses.com ~all` |

Removed old Unosend records: MX for `send`, SPF for `send`, DKIM for
`unosend._domainkey`, and Unosend domain verification from root TXT.

### Tests

| File | Change |
|---|---|
| `infra/src/__tests__/vercel.test.ts` | `resend-api-key` mock, `RESEND_API_KEY` assertion |
| `infra/src/__tests__/dns.test.ts` | Updated record count (5) and resource names |

## Manual Steps Required

1. **Store API key in Azure Key Vault:**
   ```bash
   az keyvault secret set --vault-name kv-iac-anvil \
     --name resend-api-key --value '<RESEND_API_KEY>'
   ```

2. **Deploy infra** — `pulumi up` to create DNS records and push `RESEND_API_KEY`
   to Vercel

3. **Verify domain** — Check Resend dashboard for domain verification status

4. **Deploy website** — Deploy to pick up the new `resend` package

5. **Cleanup** — Disable old secret:
   ```bash
   az keyvault secret set-attributes --vault-name kv-iac-anvil \
     --name unosend-api-key --enabled false
   ```
