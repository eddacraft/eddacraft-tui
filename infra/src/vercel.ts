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

// DOCSAUTH2: upstream Docusaurus hosts. These are the auto-generated
// .vercel.app hostnames (matching the project `name`). The shell rewrites
// requests to these hosts and attaches a protection-bypass secret.
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

// DOCSAUTH2: Anvil docs (private, Docusaurus) — locked behind deployment
// protection, reached only via docs-shell rewrites with a bypass secret.
// Declared before docsShell so its host string is in scope for shell env vars.
export const anvilDocsPrivate = new VercelApp('anvil-docs-private', {
  name: 'eddacraft-anvil-docs-private',
  framework: 'docusaurus-2',
  rootDirectory: 'apps/anvil-docs-private',
  gitRepo,
  domains: [],
  skipPreviewDeploys: true,
  deploymentProtection: 'all_deployments',
});

// DOCSAUTH2: Public docs (Kindling/APS/edda-stack/blog) — also locked behind
// deployment protection. Public-facing users reach it only via docs-shell;
// direct .vercel.app hits return 401.
export const docsPublic = new VercelApp('docs-public', {
  name: 'eddacraft-docs-public',
  framework: 'docusaurus-2',
  rootDirectory: 'apps/docs-public',
  gitRepo,
  domains: [],
  skipPreviewDeploys: true,
  deploymentProtection: 'all_deployments',
});

// DOCSAUTH2: Docs shell (Next.js) — public-facing, gates /anvil/* with
// licence JWT, rewrites to the two protected upstreams. Domain cutover to
// docs.eddacraft.ai lives in a later task (currently still on docsSite).
export const docsShell = new VercelApp('docs-shell', {
  name: 'eddacraft-docs-shell',
  framework: 'nextjs',
  rootDirectory: 'apps/docs-shell',
  gitRepo,
  domains: [],
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
  },
});

// IAC-004: Docs Site (Docusaurus) — will be retired once docsShell takes
// over the docs.eddacraft.ai domain (see DOCSAUTH2 Task 24/26).
export const docsSite = new VercelApp('docs-site', {
  name: 'docs-site',
  framework: 'docusaurus-2',
  rootDirectory: 'apps/docs-site',
  gitRepo,
  domains: ['docs.eddacraft.ai'],
  skipPreviewDeploys: true,
  extraWatchPaths: ['docs/public'],
  envVars: {
    // DOCSAUTH: ES256 public key for edge JWT verification
    LICENSE_PUBLIC_KEY: licensePublicKey,
    // DOCSAUTH: secret for encrypting OAuth state parameter (CSRF nonce)
    STATE_SECRET: getSecret('docs-state-secret'),
    // DOCSAUTH: BAUTH API URL for the callback function
    BAUTH_API_URL: 'https://api.eddacraft.ai',
    // DOCSAUTH: GitHub OAuth client ID (needed by login function for redirect)
    GITHUB_CLIENT_ID: githubClientId,
  },
});
