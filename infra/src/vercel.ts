import { VercelApp } from './components/vercel-app.js';
import { getSecret } from './keyvault.js';

const gitRepo = 'EddaCraft/anvil-001';

const databaseUrl = getSecret('website-database-url');
const resendApiKey = getSecret('resend-api-key');

// DOCSAUTH: GitHub OAuth secrets
const githubClientId = getSecret('github-oauth-client-id');
const githubClientSecret = getSecret('github-oauth-client-secret');
const licensePublicKey = getSecret('license-public-key');

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

// IAC-004: Docs Site (Docusaurus)
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
