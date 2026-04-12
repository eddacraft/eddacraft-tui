import { VercelApp } from './components/vercel-app.js';
import { getSecret } from './keyvault.js';

const gitRepo = 'EddaCraft/anvil-001';

const databaseUrl = getSecret('website-database-url');
const resendApiKey = getSecret('resend-api-key');

// DOCSAUTH: GitHub OAuth secrets
const githubClientId = getSecret('github-oauth-client-id');
const githubClientSecret = getSecret('github-oauth-client-secret');
const licensePublicKey = getSecret('license-public-key');
const docsStateSecret = getSecret('docs-state-secret');
const docsUpstreamSecret = getSecret('docs-upstream-secret');

// DOCSAUTH2: upstream Docusaurus hosts. These are the auto-generated
// .vercel.app hostnames (matching the project `name` in Track B). The
// docs-shell rewrites requests to these hosts and attaches a shared-secret
// header; the upstreams enforce the header in routing middleware.
const ANVIL_DOCS_PRIVATE_HOST = 'eddacraft-anvil-docs-private.vercel.app';
const DOCS_PUBLIC_HOST = 'eddacraft-docs-public.vercel.app';

// IAC-003: Website (Next.js) — static frontend, no server-side email/DB
export const website = new VercelApp('website', {
  name: 'website',
  framework: 'nextjs',
  rootDirectory: 'apps/website',
  gitRepo,
  domains: ['eddacraft.ai'],
  skipPreviewDeploys: true,
  envVars: {
    NEXT_PUBLIC_API_URL: 'https://api.eddacraft.ai',
  },
});

// IAC-005: Anvil API (Hono)
export const api = new VercelApp('anvil-api', {
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
    WAITLIST_RESEND_ADMIN_TOKEN: getSecret('waitlist-resend-admin-token'),
    ANVIL_CORS_ORIGINS:
      'https://eddacraft.ai,https://docs.eddacraft.ai,https://*.vercel.app,http://localhost:3000',
    // BAUTH: audience management + device code flow
    RESEND_WAITLIST_AUDIENCE_ID: getSecret('resend-waitlist-audience-id'),
    RESEND_BETA_AUDIENCE_ID: getSecret('resend-beta-audience-id'),
    ACTIVATE_URL: 'https://eddacraft.ai/auth/activate',
    WAITLIST_ADMIN_EMAIL: 'josh@eddacraft.ai',
    CRON_SECRET: getSecret('cron-secret'),
    // BAUTH: pepper for SHA-256 token hashing (required in production)
    TOKEN_PEPPER: getSecret('token-pepper'),
    // DOCSAUTH: GitHub OAuth for docs auth gating
    GITHUB_CLIENT_ID: githubClientId,
    GITHUB_CLIENT_SECRET: githubClientSecret,
  },
});

// DOCSAUTH2: Anvil docs (private, Docusaurus). Protected via shared-secret
// header check in routing middleware — the docs-shell proxy injects
// X-Docs-Upstream-Secret on every upstream request.
export const anvilDocsPrivate = new VercelApp('anvil-docs-private', {
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
});

// DOCSAUTH2: Public docs (APS/Kindling/edda-stack/blog). Same header-based
// enforcement — direct .vercel.app hits return 401.
export const docsPublic = new VercelApp('docs-public', {
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
});

// DOCSAUTH2: Docs shell (Next.js) — public-facing, gates /anvil/* with
// licence JWT, proxies to the two protected upstreams.
export const docsShell = new VercelApp('docs-shell', {
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
    GITHUB_CLIENT_SECRET: githubClientSecret,
    BAUTH_API_URL: 'https://api.eddacraft.ai',
    ANVIL_DOCS_URL: `https://${ANVIL_DOCS_PRIVATE_HOST}`,
    PUBLIC_DOCS_URL: `https://${DOCS_PUBLIC_HOST}`,
    DOCS_UPSTREAM_SECRET: docsUpstreamSecret,
  },
});

// IAC-004: Docs Site (Docusaurus) — RETIRED. Domain moved to docsShell.
// Kept temporarily for rollback; remove once docs-shell is stable.
export const docsSite = new VercelApp('docs-site', {
  name: 'docs-site',
  framework: 'docusaurus-2',
  rootDirectory: 'apps/docs-site',
  gitRepo,
  domains: [],
  skipPreviewDeploys: true,
  extraWatchPaths: ['docs/public'],
  envVars: {
    // DOCSAUTH: ES256 public key for edge JWT verification
    LICENSE_PUBLIC_KEY: licensePublicKey,
    // DOCSAUTH: secret for encrypting OAuth state parameter (CSRF nonce)
    STATE_SECRET: docsStateSecret,
    // DOCSAUTH: BAUTH API URL for the callback function
    BAUTH_API_URL: 'https://api.eddacraft.ai',
    // DOCSAUTH: GitHub OAuth client ID (needed by login function for redirect)
    GITHUB_CLIENT_ID: githubClientId,
  },
});
