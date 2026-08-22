import { VercelApp } from './components/vercel-app.js';
import { getSecret } from './keyvault.js';
import { isTrustedStack, warnUntrustedSkip } from './stack-trust.js';

const gitRepo = 'eddacraft/anvil-001';

// CIB-119: every Vercel project here is a production resource with a fixed
// physical name (and, for most, production domains). An untrusted stack
// defining them would fight over the same live Vercel projects, so on
// untrusted stacks nothing below is defined and the exports are undefined.
const isProdStack = isTrustedStack();
if (!isProdStack) {
  warnUntrustedSkip('production Vercel projects');
}

function prodOnly<T>(define: () => T): T | undefined {
  return isProdStack ? define() : undefined;
}

// On untrusted stacks these resolve to explicit markers without contacting
// Key Vault (see keyvault.ts); no resource below consumes them in that case.
const databaseUrl = getSecret('anvil-api-database-url');
const resendApiKey = getSecret('resend-api-key');

// DOCSAUTH: GitHub OAuth secrets
const githubClientId = getSecret('github-oauth-client-id');
const githubClientSecret = getSecret('github-oauth-client-secret');
// GHCLIAUTH-002 (ADR-066): dedicated "Anvil CLI" GitHub OAuth app for the CLI
// device-authorization-grant login. Separate from the `eddacraft Docs` app
// above so CLI login and docs auth do not share rate limits, consent branding,
// or audit trails. Wired into anvil-api ONLY (never docs-shell).
const githubCliClientId = getSecret('github-cli-client-id');
const githubCliClientSecret = getSecret('github-cli-client-secret');
const licensePublicKey = getSecret('license-public-key');
const licenseSigningKey = getSecret('license-signing-key');
const docsStateSecret = getSecret('docs-state-secret');
const docsUpstreamSecret = getSecret('docs-upstream-secret');

// DOCSAUTH2: upstream Docusaurus hosts. These are the auto-generated
// .vercel.app hostnames (matching the project `name` in Track B). The
// docs-shell rewrites requests to these hosts and attaches a shared-secret
// header; the upstreams enforce the header in routing middleware.
// NOTE: these are string literals rather than Pulumi outputs because
// @pulumiverse/vercel does not expose a default-domain output. Keep in
// sync with the `name` fields on anvilDocsPrivate and docsPublic below.
const ANVIL_DOCS_PRIVATE_HOST = 'eddacraft-anvil-docs-private.vercel.app';
const DOCS_PUBLIC_HOST = 'eddacraft-docs-public.vercel.app';
export const API_CORS_ORIGINS =
  'https://eddacraft.ai,https://www.eddacraft.ai,https://docs.eddacraft.ai,https://*.vercel.app,http://localhost:3000';

// IAC-003: Website (Next.js) — static frontend, no server-side email/DB
export const website = prodOnly(
  () =>
    new VercelApp('website', {
      name: 'website',
      framework: 'nextjs',
      rootDirectory: 'apps/website',
      gitRepo,
      domains: ['eddacraft.ai', 'www.eddacraft.ai'],
      // www.eddacraft.ai exists in the prod Vercel project but is missing from
      // Pulumi state. Adopt it on the next prod `pulumi up`; remove this entry
      // once adoption succeeds (leaving it in is a no-op but warns on subsequent
      // runs). The import ID is a prod-specific Vercel project ID — safe here
      // because this whole definition is gated on the trusted prod stack.
      domainImports: {
        'www.eddacraft.ai': 'prj_b3egAMA3JZULn5KSTwiVs5Qr0vts/www.eddacraft.ai',
      },
      skipPreviewDeploys: true,
      envVars: {
        NEXT_PUBLIC_API_URL: 'https://api.eddacraft.ai',
      },
    })
);

// IAC-005: Anvil API (Hono)
export const api = prodOnly(
  () =>
    new VercelApp('anvil-api', {
      name: 'anvil-api',
      framework: 'hono',
      rootDirectory: 'apps/anvil-api',
      gitRepo,
      domains: ['api.eddacraft.ai'],
      skipPreviewDeploys: true,
      extraWatchPaths: ['packages/transactional'],
      envVars: {
        DATABASE_URL: databaseUrl,
        RESEND_API_KEY: resendApiKey,
        ADMIN_KEY: getSecret('anvil-admin-key'),
        // ADMINCLIH-002/004: per-operator admin keys. Keys are provisioned in
        // `infra/src/admin-keys.ts` and validated via HMAC-SHA-256 with this
        // pepper. Disable the feature by unsetting ADMIN_PER_OPERATOR_KEYS.
        ADMIN_KEY_PEPPER: getSecret('admin-key-pepper'),
        ADMIN_PER_OPERATOR_KEYS: '1',
        WAITLIST_RESEND_ADMIN_TOKEN: getSecret('waitlist-resend-admin-token'),
        ANVIL_CORS_ORIGINS: API_CORS_ORIGINS,
        // BAUTH: audience management + device code flow
        RESEND_WAITLIST_AUDIENCE_ID: getSecret('resend-waitlist-audience-id'),
        RESEND_BETA_AUDIENCE_ID: getSecret('resend-beta-audience-id'),
        ACTIVATE_URL: 'https://eddacraft.ai/auth/activate',
        WAITLIST_ADMIN_EMAIL: 'josh@eddacraft.ai',
        CRON_SECRET: getSecret('cron-secret'),
        // BAUTH: pepper for SHA-256 token hashing (required in production)
        TOKEN_PEPPER: getSecret('token-pepper'),
        // BAUTH: ES256 private key (PKCS#8 PEM) for signing licence JWTs
        LICENSE_SIGNING_KEY: licenseSigningKey,
        // CIB-066: ES256 public key (SPKI PEM) so the API can VERIFY the
        // licences it signs — /auth/verify accepts the licence JWT credential
        // that interactive logins store (whoami path), and the /health
        // verifying-key probe stops reporting degraded. Previously only the
        // docs apps received this key. DEPLOY ORDER: apply this (pulumi up or
        // set the var in the Vercel UI) BEFORE or with the code deploy — the
        // licence path 503s without it.
        LICENSE_PUBLIC_KEY: licensePublicKey,
        // DOCSAUTH: GitHub OAuth for docs auth gating
        GITHUB_CLIENT_ID: githubClientId,
        GITHUB_CLIENT_SECRET: githubClientSecret,
        // GHCLIAUTH-002 (ADR-066): dedicated "Anvil CLI" OAuth app for CLI device
        // flow. anvil-api only; requires Device Flow enabled on the GitHub app.
        GITHUB_CLI_CLIENT_ID: githubCliClientId,
        GITHUB_CLI_CLIENT_SECRET: githubCliClientSecret,
        // DBCON-003: WAITLIST_PAUSED is intentionally NOT declared here. It is a
        // kill switch for POST /waitlist (see waitlist.ts) toggled directly in the
        // Vercel project UI during the Neon cutover — if Pulumi managed it, the
        // next `pulumi up` would clobber the operator's toggle. Env changes require
        // an anvil-api redeploy to take effect (redeploy itself is zero-downtime).
        // Set to "true" and redeploy to return 503; unset and redeploy to restore
        // normal operation.
      },
    })
);

// DOCSAUTH2: Anvil docs (private, Docusaurus). Protected via shared-secret
// header check in routing middleware — the docs-shell proxy injects
// X-Docs-Upstream-Secret on every upstream request.
export const anvilDocsPrivate = prodOnly(
  () =>
    new VercelApp('anvil-docs-private', {
      name: 'eddacraft-anvil-docs-private',
      framework: 'docusaurus-2',
      rootDirectory: 'apps/anvil-docs-private',
      gitRepo,
      domains: [],
      skipPreviewDeploys: true,
      extraWatchPaths: ['docs/public/anvil', 'docs/public/beta'],
      envVars: {
        DOCS_UPSTREAM_SECRET: docsUpstreamSecret,
      },
    })
);

// DOCSAUTH2: Public docs (APS/Kindling/edda-stack/blog). Same header-based
// enforcement — direct .vercel.app hits return 401.
export const docsPublic = prodOnly(
  () =>
    new VercelApp('docs-public', {
      name: 'eddacraft-docs-public',
      framework: 'docusaurus-2',
      rootDirectory: 'apps/docs-public',
      gitRepo,
      domains: [],
      skipPreviewDeploys: true,
      extraWatchPaths: [
        'docs/public/aps',
        'docs/public/kindling',
        'docs/public/edda-stack',
        'apps/docs-public/blog',
      ],
      envVars: {
        DOCS_UPSTREAM_SECRET: docsUpstreamSecret,
      },
    })
);

// DOCSAUTH2: Docs shell (Next.js) — public-facing, gates /anvil/* with
// licence JWT, proxies to the two protected upstreams.
// DEPLOY NOTE: moving docs.eddacraft.ai between projects requires a
// two-step Pulumi apply — destroy the old ProjectDomain first with
// `pulumi destroy --target <old-domain-urn>`, then `pulumi up`.
export const docsShell = prodOnly(
  () =>
    new VercelApp('docs-shell', {
      name: 'eddacraft-docs-shell',
      framework: 'nextjs',
      rootDirectory: 'apps/docs-shell',
      gitRepo,
      domains: ['docs.eddacraft.ai'],
      buildCommand: 'pnpm nx build docs-shell',
      installCommand: 'pnpm install --frozen-lockfile',
      skipPreviewDeploys: true,
      envVars: {
        LICENSE_PUBLIC_KEY: licensePublicKey,
        DOCS_STATE_SECRET: docsStateSecret,
        GITHUB_CLIENT_ID: githubClientId,
        BAUTH_API_URL: 'https://api.eddacraft.ai',
        ANVIL_DOCS_URL: `https://${ANVIL_DOCS_PRIVATE_HOST}`,
        PUBLIC_DOCS_URL: `https://${DOCS_PUBLIC_HOST}`,
        DOCS_UPSTREAM_SECRET: docsUpstreamSecret,
      },
    })
);

// IAC-004 originally managed apps/docs-site. That host is gone; leaving the
// Vercel project connected made every main push fail against a missing
// rootDirectory. Do not re-add it.
